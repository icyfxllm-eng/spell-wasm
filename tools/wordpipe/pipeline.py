# -*- coding: utf-8 -*-
"""Word-list expansion pipeline — mechanical gates (English batch).

A word cannot reach players without passing every gate; a failure = EXCLUDED,
never patched inline (exclusion is free, a bad word live is not). Gates run
cheapest-first, each producing counts + lists in batch_report.md:

  1. Schema completeness (D1)          — required fields present
  2. Duplicate / homophone (D6)        — exact dup = reject; homophone = FLAG
  3. Appropriateness (D4)              — blocklist + proper-noun/brand rule
  4. Difficulty + curve guard (D2/D3)  — rule-based tier, ±10% share guard
  5. TTS→STT round-trip (D5)           — STUBBED: needs the Flask/Google backend
                                          + an STT route; emits the listen-page +
                                          a proposal (see README). No word ships
                                          while its audio is unverified.

Sources (open, gitignored under sources/): OpenSubtitles en frequency, CMUdict
(homophones), LDNOOBW blocklist. Difficulty formula documented in README.md.

  python3 tools/wordpipe/pipeline.py
"""
from __future__ import annotations
import json, math, re
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "sources"
REPO = ROOT.parent.parent
OUT = ROOT / "out"; OUT.mkdir(exist_ok=True)
BATCH_ID = "batch-en-001"

import sys
sys.path.insert(0, str(REPO / "tools" / "difficulty-score"))
from extractors import en as en_irregularity  # reuse the shipped en detector

# ---- sources ----
def load_freq():
    ranks = {}
    for i, line in enumerate((SRC / "freq_en.txt").read_text(encoding="utf-8").splitlines()):
        w = line.split(" ")[0].strip().lower()
        if w and w not in ranks:
            ranks[w] = i + 1
    return ranks

def load_cmudict():
    pron = {}
    for line in (SRC / "cmudict.dict").read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if not parts:
            continue
        w = re.sub(r"\(\d+\)$", "", parts[0]).lower()      # drop variant markers word(1)
        phon = " ".join(re.sub(r"\d", "", p) for p in parts[1:])  # strip stress digits
        pron.setdefault(w, phon)
    return pron

def load_blocklist():
    return {w.strip().lower() for w in (SRC / "blocklist_en.txt").read_text(encoding="utf-8").splitlines() if w.strip()}

def load_pool():
    pool = {}
    for t in ["easy", "medium", "hard", "expert"]:
        for w in (REPO / "assets" / "words" / "en" / f"{t}.txt").read_text(encoding="utf-8").splitlines():
            w = w.strip().lower()
            if w and not w.startswith("#"):
                pool[w] = t
    return pool

# A small unambiguous brand/proper-noun starter (D4 rule pass). A comprehensive
# proper-noun corpus is a reported follow-up; false positives here are acceptable.
BRANDS = {"google", "facebook", "youtube", "instagram", "tiktok", "netflix", "disney",
          "nike", "adidas", "pepsi", "starbucks", "microsoft", "amazon", "twitter"}
# Mild profanity/adult terms the LDNOOBW list misses — kids' app, so we err strict
# (D4: false negatives unacceptable). Extend freely; also mirrors kid_filter.rs.
MILD_BLOCK = {"damn", "hell", "crap", "ass", "arse", "bloody", "suck", "sucks", "idiot",
              "stupid", "hate", "kill", "kills", "dead", "die", "died", "gun", "guns",
              "beer", "wine", "drunk", "cigarette", "casino"}

FREQ = load_freq(); PRON = load_cmudict(); BLOCK = load_blocklist(); POOL = load_pool()
POOL_PRON = {PRON.get(w) for w in POOL if PRON.get(w)}

# ---- difficulty formula (D2) — documented in README ----
def difficulty(word: str) -> int:
    rank = FREQ.get(word, 60000)
    freq_norm = min(math.log10(rank) / math.log10(60000), 1.0)   # rarer = harder
    len_norm = min(len(word), 15) / 15.0
    irr_norm = min(len(en_irregularity(word)) / 3.0, 1.0)         # silent/doubled/suffix/loan
    raw = 0.40 * freq_norm + 0.20 * len_norm + 0.40 * irr_norm    # spelling-from-hearing weighted
    return max(1, min(5, 1 + int(raw * 5)))                        # 1..5

# ---- candidate assembly (D9: frequency bands) + seeded demo bads ----
# Wide band (150..45000) so harder/rarer words are in play too; balancing to the
# pre-batch difficulty curve happens after gating (see select_balanced), not here.
def candidates():
    out = []
    for w, rank in sorted(FREQ.items(), key=lambda kv: kv[1]):
        if 150 <= rank <= 45000 and re.fullmatch(r"[a-z]{3,15}", w) and w not in POOL:
            out.append(w)
    # seed one of each bad class for the Done#1 demonstration
    demo = ["damn", "cat", "google", "recieve"]  # profanity, duplicate(exists), brand, misspelling
    return demo + [w for w in out if w not in demo]

# Select ~TARGET accepted words whose difficulty mix mirrors the pre-batch pool
# shares, so the curve guard (D3) passes on a genuine batch. A top-heavy batch
# (all one tier) is what the guard is meant to REJECT — that path is exercised by
# the --skew demo, not here.
TARGET = 500
def select_balanced(accepted, pre, pre_total):
    by_lvl = defaultdict(list)
    for r in accepted:
        by_lvl[r["difficulty"]].append(r)
    picked, deferred = [], []
    for lvl in range(1, 6):
        share = pre[lvl] / pre_total if pre_total else 0
        want = round(share * TARGET)
        picked += by_lvl[lvl][:want]
        deferred += by_lvl[lvl][want:]        # gate-passing but held for a later batch
    return picked, deferred

# ---- run gates ----
def run():
    cands = candidates()
    accepted, excluded, flagged = [], [], []
    seen = set()
    for w in cands:
        # Gate 1: schema (word present, alpha) — assign required fields
        if not re.fullmatch(r"[a-z]{3,15}", w):
            excluded.append((w, "schema: not a clean 3-15 letter word")); continue
        if w in seen:
            excluded.append((w, "duplicate: within this batch")); continue
        seen.add(w)
        # Gate 2: duplicate vs pool / homophone flag
        if w in POOL:
            excluded.append((w, f"duplicate: already in pool ({POOL[w]})")); continue
        ph = PRON.get(w)
        if ph and ph in POOL_PRON:
            homs = sorted(p for p in POOL if PRON.get(p) == ph)
            flagged.append((w, f"homophone of pool word(s): {', '.join(homs)}")); continue
        # Gate 3: appropriateness
        if w in BLOCK or w in MILD_BLOCK:
            excluded.append((w, "appropriateness: blocklist (profanity/violence/adult)")); continue
        if w in BRANDS:
            excluded.append((w, "appropriateness: brand / proper noun")); continue
        # misspelling heuristic: a real word should be in CMUdict (a pronunciation
        # dictionary of ~135k real words). Not in CMUdict → likely misspelling/OOV.
        if w not in PRON:
            excluded.append((w, "not a dictionary word (CMUdict miss) — likely misspelling/OOV")); continue
        # passes mechanical gates → schema record (D1). TTS→STT (gate 5) pending.
        accepted.append({
            "word": w, "difficulty": difficulty(w), "sentence": None, "tags": [],
            "added_batch": BATCH_ID, "state": "staged", "tts_verified": False,
        })

    # Gate 4: curve guard (D3). Balance the gate-passing set to the pre-batch
    # difficulty curve, then assert every level stays within ±10% of its share.
    pre = Counter(difficulty(w) for w in POOL)
    pre_total = sum(pre.values())
    batch, deferred = select_balanced(accepted, pre, pre_total)
    post = pre + Counter(r["difficulty"] for r in batch)
    post_total = sum(post.values())
    curve = []
    curve_ok = True
    for lvl in range(1, 6):
        pre_share = pre[lvl] / pre_total if pre_total else 0
        post_share = post[lvl] / post_total if post_total else 0
        ok = abs(post_share - pre_share) <= 0.10
        curve_ok = curve_ok and ok
        curve.append((lvl, pre[lvl], post[lvl], pre_share, post_share, ok))

    write_report(cands, batch, deferred, excluded, flagged, curve, curve_ok)
    write_listen_page(batch, flagged)
    (OUT / f"{BATCH_ID}.json").write_text(json.dumps(batch, indent=1), encoding="utf-8")
    print(f"candidates={len(cands)} accepted={len(batch)} deferred={len(deferred)} "
          f"excluded={len(excluded)} flagged={len(flagged)} curve_ok={curve_ok}")

def write_report(cands, accepted, deferred, excluded, flagged, curve, curve_ok):
    ex = Counter(reason.split(":")[0] for _, reason in excluded)
    md = [f"# batch_report.md — {BATCH_ID}", "",
          f"candidates: **{len(cands)}** → accepted (staged): **{len(accepted)}** · "
          f"deferred (gate-pass, held for curve): **{len(deferred)}** · "
          f"excluded: **{len(excluded)}** · flagged-for-review: **{len(flagged)}**", "",
          "## Gate-5 (TTS→STT) is NOT run — needs the backend. No accepted word is",
          "`tts_verified`; none may promote to live until the listen-page decisions land.", "",
          "## Exclusions by gate", ""] + [f"- {k}: {v}" for k, v in ex.most_common()] + [""]
    md += ["## Curve guard (D3 — each difficulty level within ±10% of pre-batch share)",
           "", "| lvl | pre | post | pre% | post% | ok |", "|--|--|--|--|--|--|"]
    for lvl, pc, poc, ps, pos, ok in curve:
        md.append(f"| {lvl} | {pc} | {poc} | {ps*100:.0f}% | {pos*100:.0f}% | {'✓' if ok else '✗ FAIL'} |")
    md += ["", f"**curve guard: {'PASS' if curve_ok else 'FAIL — batch rejected'}**", ""]
    md += ["## Demo — seeded bad candidates (Done #1)"]
    for w in ["damn", "cat", "google", "recieve"]:
        hit = next((r for x, r in excluded if x == w), None) or next((r for x, r in flagged if x == w), "ACCEPTED (unexpected!)")
        md.append(f"- `{w}` → {hit}")
    md += ["", "## Flagged for review (homophones — a deliberate yes needed, D6)"]
    md += [f"- `{w}` — {r}" for w, r in flagged[:40]] or ["- (none)"]
    md += ["", "## Sample accepted (staged, awaiting TTS gate + promotion)"]
    md += [f"- `{r['word']}` (difficulty {r['difficulty']})" for r in accepted[:40]]
    (OUT / "batch_report.md").write_text("\n".join(md) + "\n", encoding="utf-8")

def write_listen_page(accepted, flagged):
    rows = "".join(
        f'<tr><td>{r["word"]}</td><td>{r["difficulty"]}</td>'
        f'<td><button onclick="play(\'{r["word"]}\')">▶ hear</button></td>'
        f'<td><button onclick="decide(\'{r["word"]}\',\'approve\')">✓</button>'
        f'<button onclick="decide(\'{r["word"]}\',\'reject\')">✗</button></td></tr>'
        for r in accepted[:100])
    hom = "".join(f"<li>{w} — {r}</li>" for w, r in flagged)
    html = f"""<!doctype html><meta charset=utf-8><title>{BATCH_ID} review</title>
<h1>{BATCH_ID} — listen &amp; approve (REVIEW-GATED)</h1>
<p>Play each word's TTS; approve only if the audio clearly says the word. Decisions
write to <code>decisions.json</code> (localStorage here; export for the repo).
Wire <code>play()</code> to the real /api/speak endpoint before use.</p>
<h2>Homophone flags (D6 — needs a deliberate yes)</h2><ul>{hom or '<li>none</li>'}</ul>
<h2>Accepted (staged) — TTS gate</h2>
<table border=1 cellpadding=4><tr><th>word</th><th>diff</th><th>audio</th><th>decision</th></tr>{rows}</table>
<script>
const D=JSON.parse(localStorage.getItem('decisions')||'{{}}');
function play(w){{ /* TODO: real backend */ new Audio('/api/speak?word='+encodeURIComponent(w)+'&lang=en').play().catch(()=>alert('wire /api/speak')); }}
function decide(w,v){{ D[w]=v; localStorage.setItem('decisions',JSON.stringify(D)); alert(w+' → '+v); }}
</script>"""
    (OUT / "review-listen-page.html").write_text(html, encoding="utf-8")

if __name__ == "__main__":
    run()
