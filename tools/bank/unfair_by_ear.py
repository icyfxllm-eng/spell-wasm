#!/usr/bin/env python3
"""Unfair-by-ear candidate sweep for the English bank. Read-only: writes reports/unfair-by-ear-candidates.csv.

Classes: A irregular spelling given the sound (gap between the spelling and a grapheme-to-phoneme model trained on
common English, from tools/en_growth/score.py), B homophone with both members in the bank, C both recognizers
heard a different real word. Quarantines nothing; Eric rules per word.
Run: .corpus-cache/wf/v/bin/python tools/bank/unfair_by_ear.py   (needs wordfreq, like score.py)
"""
import csv, json, os, re, sys
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "tools/en_growth"))
import score as S
from common import *

TIERS = ["easy", "medium", "hard", "expert"]
# Players reached: Junior sees easy+medium, Standard sees all four; an Easy word is hit by every player.
WEIGHT = {"easy": 1.0, "medium": 0.8, "hard": 0.4, "expert": 0.2}  # share of players a tier reaches (a judgment, not a measurement)
A_MIN = 0.985       # irregularity percentile alone (the aligner is a screen, not a verdict: it is noisy near the margin)
A_SILENT_MIN = 0.90 # lower bar when a known silent-letter pattern is present
SILENT = re.compile(r"^(kn|wr|ps|pn|gn)|^(hour|honest|honor|heir)|mb$|mn$|ght|lm|ough|eau|ough|colonel|island|isle|debris|corps|rendezvous|yacht|receipt|subtle|doubt|debt|czar|tsch|chor|^ch[ao]r|psych|coup|^[a-z]*sc[ie]|ique$|cque|vieu|eur$|ois$|aisle|sword|answer|whole|^who|calm|salmon|palm|folk|walk|talk|chalk|castle|listen|fasten|often|wednesday|friend|guess|guard|guest|build|biscuit|rhyme|rhythm|thyme|gauge|bouquet|ballet|buffet|sieve|people|leopard|jeopardy|queue|quay|lieutenant|mortgage|sergeant|clerk|seize|their|height|aunt|laugh|cough|tough|enough|bury|busy|women|says|said|once|^one$|two$|choir|chaos|ache|anchor|stomach|schedule|machine|chef|chute|moustache|ocean|sugar|sure")
def silent_pat(w): return bool(SILENT.search(w))
NUMBERS = set("one two three four five six seven eight nine ten eleven twelve zero".split())

bank = bank_rows()
tier_of = {w: t for t in TIERS for w in bank[t]}
quar = {}
for t in TIERS:
    p = os.path.join(ROOT, "assets/words-quarantine/en", t + ".txt")
    if os.path.exists(p):
        for l in open(p, encoding="utf-8"):
            if l.strip():
                quar[l.strip()] = t
signed = {r["word"]: r["class"] for r in csv.DictReader(open(os.path.join(ROOT, "tools/bank/purity_overrides.csv"))) if r["lang"] == "en"}
diff = json.load(open(os.path.join(ROOT, "assets/words/en/en_difficulty.json")))["components"]
pron = json.load(open(os.path.join(ROOT, "assets/words/en/en_pron.json")))["keys"]
verd = json.load(open(os.path.join(ROOT, "config/audio-verdicts.json")))["clips"]
trans = json.load(open(os.path.join(ROOT, "audio_clarity/transcripts.json")))
sys.path.insert(0, os.path.join(ROOT, "tools"))
from soundalikes import soundalikes
sa = soundalikes("en")["sets"]
svd = json.load(open(os.path.join(ROOT, "assets/words/en/soundalike-verdicts.json")))["sets"]
sov = json.load(open(os.path.join(ROOT, "assets/words/en/soundalike-overrides.json")))["sets"]
cue_mode = json.load(open(os.path.join(ROOT, "config/sense-cue.json")))["languages"]["en"]["cueMode"]

# worst grapheme unit per word, from the same aligner score.py uses
zipf = zipf_fn()
cmu = load_cmu_stress()
train_words = sorted((w for w in cmu if len(w) >= 3 and zipf(w) >= S.AL["train_zipf_min"]), key=lambda w: (-zipf(w), w))[: S.AL["train_max_words"]]
model = S.train([(w, cmu[w][0]) for w in train_words])
def worst_unit(w):
    best = None
    for phs in cmu.get(w, []):
        r, a = S.raw_irregularity(model, w, list(phs))
        if r is not None and (best is None or r < best[0]):
            best = (r, a)
    if not best:
        return ""
    a = best[1]
    cap = S.AL["unit_cost_cap"]
    ph, g = max(a, key=lambda u: min(cap, model.cost(*u)))
    silent = [g2 for ph2, g2 in a if ph2 == S.NULL and g2 and set(g2) - set("aeiouy")]
    return f"{g or '(none)'} spells {'nothing' if ph == S.NULL else re.sub(chr(92)+'d','',ph)}" + (f"; silent letters: {'+'.join(silent)}" if silent else "")
def arpa(w):
    p = pron.get(w) or []
    return " / ".join(p)
def status(w):
    s = []
    if w in quar: s.append("quarantined")
    if w in signed: s.append("signed-override:" + signed[w])
    if verd.get(f"en|{w}|normal", {}).get("v") == "ExemptHomophone": s.append("exempt-homophone-audio")
    return ";".join(s) or "none"
def norm(x): return re.sub(r"[^a-z0-9]", "", x.lower())

# Class B: kind=true sets with >=2 members that are bank words
setmap = {}
for s in sa:
    if s["kind"] != "true": continue
    ov = sov.get(s["setId"], {}); drop = set(ov.get("drop", [])); tonly = ov.get("targetOnly", [])
    mem = [m["memberId"] for m in s["members"] if m["memberId"] not in drop]
    inb = [m for m in mem if m in tier_of]
    if len(inb) >= 2:
        setmap[s["setId"]] = (mem, inb, s["confirmed"], tonly, svd.get(s["setId"], {}).get("verdict", ""))

rows = {}
def row(w):
    return rows.setdefault(w, {"word": w, "tier": tier_of[w], "pron": arpa(w), "classes": [], "evidence": [], "score": 0.0})

for w, t in tier_of.items():
    c = diff.get(w)
    wu = worst_unit(w) if c and c[0] >= A_SILENT_MIN else ""
    if c and (c[0] >= A_MIN or (silent_pat(w) and c[0] >= A_SILENT_MIN)):
        r = row(w); r["classes"].append("A")
        r["evidence"].append(f"irregularity pct {c[0]:.3f} (spelling vs sound-model guess); worst unit: {wu}" + ("; known irregular/silent-letter pattern" if silent_pat(w) else ""))
        r["score"] = max(r["score"], c[0] * (0.6 + 0.4 * c[1]) + (0.15 if silent_pat(w) else 0.0))   # rarer words are less likely to be known by sight, so less fair
for sid, (mem, inb, conf, tonly, v) in setmap.items():
    for w in inb:
        r = row(w); r["classes"].append("B")
        others = [m for m in mem if m != w]
        mit = "accept-all active (F2a), either spelling is graded correct"
        if tonly and w not in tonly: mit = "target-only set: typing the other member is NOT accepted when this word is the prompt"
        r["evidence"].append(f"homophone of {'/'.join(others)} (both in bank: {'/'.join(inb)}); en cueMode={cue_mode} so no meaning cue shown; {mit}" + (f"; Eric verdict {v}" if v else ""))
        r["score"] = max(r["score"], 0.2 if "NOT accepted" not in mit else 0.6)
        r.setdefault("setids", []).append(sid)
for w, t in tier_of.items():
    h = trans.get(f"en|{w}|normal")
    if not h or len(h) < 2: continue
    n = [norm(x) for x in h]
    homo = {m for sid, (mem, inb, *_r) in setmap.items() if w in mem for m in mem}
    # a real different word: alphabetic, two letters or more, not the same sound-alike set (that is class B), not a digit or letter name
    if n[0] and n[0] == n[1] and n[0] != norm(w) and n[0].isalpha() and len(n[0]) >= 2 and n[0] not in homo and not (len(norm(w)) <= 5 and n[0] in NUMBERS):
        r = row(w); r["classes"].append("C")
        r["evidence"].append(f"both recognizers heard \"{h[0]}\" / \"{h[1]}\" (a different real word); clip verdict {verd.get(f'en|{w}|normal', {}).get('v', '?')}")
        r["score"] = max(r["score"], 0.7 if verd.get(f"en|{w}|normal", {}).get("v") == "Fail" else 0.4)

out = []
for w, r in rows.items():
    r["w"] = WEIGHT[r["tier"]]
    r["status"] = status(w)
    r["score"] += 0.05 * (len(set(r["classes"])) - 1)
    r["rank_key"] = r["score"] * r["w"]
    r["zipf"] = round(zipf(w), 2)
    out.append(r)
out.sort(key=lambda r: (-r["rank_key"], -r["zipf"], r["word"]))
os.makedirs(os.path.join(ROOT, "reports"), exist_ok=True)
with open(os.path.join(ROOT, "reports/unfair-by-ear-candidates.csv"), "w", newline="", encoding="utf-8") as f:
    wr = csv.writer(f); wr.writerow(["rank", "word", "tier", "pronunciation_arpabet", "classes", "severity", "hit_share", "zipf", "evidence", "status", "sense_cue_set", "eric_verdict"])
    for i, r in enumerate(out, 1):
        wr.writerow([i, r["word"], r["tier"], r["pron"], "+".join(sorted(set(r["classes"]))), f"{r['score']:.3f}", r["w"], r["zipf"], " | ".join(r["evidence"]),
                     r["status"], "yes" if "B" in r["classes"] else "no", ""])
print(len(out), "candidates")

# ---- short summary ----
T = TIERS
def cnt(c): return [sum(1 for x in out if c in x["classes"] and x["tier"] == t) for t in T]
L = ["# Unfair-by-ear candidates (English bank)", "",
     "Read-only sweep after the coup ruling (09751197). Nothing is quarantined here; Eric rules per word, a later session applies the quarantine with the same mechanism. "
     "Regenerate: `.corpus-cache/wf/v/bin/python tools/bank/unfair_by_ear.py`. Full list: `reports/unfair-by-ear-candidates.csv` (the `eric_verdict` column is blank for yes/no).", "",
     "Rank = severity x hit share. Hit share is a judgment, not a measurement: Easy 1.0 (every player), Medium 0.8, Hard 0.4, Expert 0.2 (Junior plays Easy+Medium, Standard plays all four). "
     "Severity: class A is the irregularity percentile scaled by rarity, +0.15 on a known silent-letter pattern; class B 0.6 on a target-only set else 0.2; class C 0.7 when the clip verdict is Fail, 0.4 when it is ExemptHomophone; +0.05 per extra class. "
     "No candidate is already quarantined (quarantined words are out of the bank).", "",
     "| Class | Easy | Medium | Hard | Expert | Total |", "|---|---|---|---|---|---|"]
names = {"A": "A irregular or silent letters", "B": "B homophone, both in bank, no cue shown", "C": "C both recognizers heard another real word"}
for c in "ABC":
    v = cnt(c); L.append(f"| {names[c]} | " + " | ".join(map(str, v)) + f" | {sum(v)} |")
v = [sum(1 for x in out if x["tier"] == t) for t in T]
L.append("| Distinct words | " + " | ".join(map(str, v)) + f" | {sum(v)} |")
L += ["", f"{sum(1 for x in out if 'exempt' in x['status'])} candidates already carry the audio exempt-homophone verdict.", "",
      "## Caveats", "",
      "- Class A screens with the spelling-given-sound irregularity percentile (tools/en_growth/score.py) plus a silent-letter pattern list. The aligner is noisy near the margin, so expect false positives among common Easy words (leaf, fee, look).",
      "- Class B is already graded fairly: English accepts any member of a true sound-alike set (accept-all, F2a), and en cueMode is off, so no meaning cue is shown anywhere. The residual unfairness is the target-only sets (read/lead; the CSV marks them NOT accepted).",
      "- Class C excludes members of the same homophone set and number words. It lists clips where both recognizers agreed on a different word, which points at a clip or voice problem as much as an unfair word.", "",
      "## Top 25 for yes/no", "", "| # | Word | Tier | Classes | Evidence | Status |", "|---|---|---|---|---|---|"]
for i, x in enumerate(out[:25], 1):
    ev = x["evidence"][-1] if "C" in x["classes"] and "A" not in x["classes"] else x["evidence"][0]
    ev = ev.replace("(spelling vs sound-model guess)", "").replace("; en cueMode=off so no meaning cue shown", "")[:170]
    L.append(f"| {i} | {x['word']} | {x['tier']} | {'+'.join(sorted(set(x['classes'])))} | {ev} | {x['status']} |")
L += ["", "## Coup-like words in Hard and Expert (outside the Easy/Medium ranking)", "",
      ", ".join(f"{x['word']} ({x['tier']}, {'+'.join(sorted(set(x['classes'])))})" for x in out if x["tier"] in ("hard", "expert") and "A" in x["classes"])]
open(os.path.join(ROOT, "reports/unfair-by-ear-candidates.md"), "w").write("\n".join(L) + "\n")
