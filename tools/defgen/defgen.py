#!/usr/bin/env python3
"""CC-LAUNCH-EN v1.1 rev 3, D-SRC4: the English definition and sentence generator.

    tools/defgen/.venv/bin/python tools/defgen/defgen.py pilot [--n 50] [--seed 20261108]
    tools/defgen/.venv/bin/python tools/defgen/defgen.py resume build/defgen/<run>
    tools/defgen/.venv/bin/python tools/defgen/defgen.py dry-run [--n 50]

WHAT GOES INTO A PROMPT (D-SRC1, D-SRC1a, I5): the headword, a part of speech
and a sense, both taken from Princeton WordNet 3.0 (its licence allows this; its
gloss never ships, it only anchors the meaning), and the tier. Nothing else.
`build_prompt` takes exactly those four values, so no Wiktionary, dictionary-API
or other BY-SA text can reach a prompt; `canary_check` proves it on every prompt
log (A15.10).

FLOW (Message Batches API, 50% price):
  1. generation batch: one request per word -> two candidates, each a
     definition and a sentence (structured JSON output);
  2. validator (code, not trust): length, no headword in the definition, the
     headword exactly once in the sentence, reading grade for Easy/Medium, the
     profanity and kid word lists;
  3. safety batch: every candidate field that passed goes through the existing
     prescreen prompt (tools/llm-prescreen/prompts/wordlist-v1.txt, unchanged);
  4. per field, the first candidate that passed both is kept; if neither did, the
     field stays dark and is listed for Eric.

COST GUARD: hard cap $25 for the whole English set (D-SRC4). The pilot projects
the full cost from its own measured usage and says STOP if the projection is over.
Credentials: the SDK's usual resolution (ANTHROPIC_API_KEY or an `ant auth
login` profile). Nothing is printed or written about the key.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import random
import re
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
MODEL = "claude-sonnet-5-5"   # D-SRC4: a current Sonnet-class model
EFFORT = "low"                # Sonnet 5.5 rejects non-default temperature; low effort is the lever
PROMPT_VERSION = "gen-v1"
SAFETY_PROMPT = ROOT / "tools" / "llm-prescreen" / "prompts" / "wordlist-v1.txt"
CAP_USD = 25.0
# Batch prices are half the list price; Sonnet 5.5 lists $2 / $10 per MTok.
BATCH_IN_PER_MTOK, BATCH_OUT_PER_MTOK = 1.0, 5.0
TIERS = ["easy", "medium", "hard", "expert"]
NLTK_DIR = HERE / ".nltk"

GEN_SCHEMA = {
    "type": "object",
    "properties": {
        "candidates": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {"definition": {"type": "string"}, "sentence": {"type": "string"}},
                "required": ["definition", "sentence"],
                "additionalProperties": False,
            },
        }
    },
    "required": ["candidates"],
    "additionalProperties": False,
}

# ---------------------------------------------------------------- inputs

def bank() -> dict[str, str]:
    """served English word -> tier (legacy bank; the re-tier is off for this release)."""
    out = {}
    for t in TIERS:
        for l in (ROOT / "assets" / "words" / "en" / f"{t}.txt").read_text(encoding="utf-8").splitlines():
            w = l.strip()
            if w and not w.startswith("#"):
                out.setdefault(w, t)
    return out


_WN = None


def wordnet():
    global _WN
    if _WN is None:
        import nltk
        nltk.data.path.insert(0, str(NLTK_DIR))
        from nltk.corpus import wordnet as wn
        _WN = wn
    return _WN


POS_NAMES = {"n": "noun", "v": "verb", "a": "adjective", "s": "adjective", "r": "adverb"}
# Closed-class English words. WordNet covers only nouns, verbs, adjectives and
# adverbs, and gives some function words rare noun senses ("it" = information
# technology, "us" = United States), so these get no WordNet anchor at all.
FUNCTION_WORDS = set("""
a an the this that these those my your his her its our their mine yours hers ours theirs
i me you he him she it we us they them myself yourself himself herself itself ourselves themselves
who whom whose which what where when why how whoever whatever
and or but nor so yet if than because although though while unless until since whether
in on at by for from to of with without about above below under over into onto out up down off
through across along around behind beside between beyond near among against toward towards upon within
be am is are was were been being do does did done have has had having
can could will would shall should may might must
not no yes all any some each every both either neither few many much more most other another such
there here then now very too also just only even again
""".split())


def sense_for(word: str) -> tuple[str, str | None, str | None]:
    """(part of speech, WordNet sense gloss, synset name).

    For each part of speech WordNet reduces `word` to its base form (morphy:
    stopped -> stop, windows -> window) and sums how often that base is used
    (SemCor lemma counts). The most-used part of speech wins, ties in the order
    noun, verb, adjective, adverb; the sense is that part of speech's FIRST synset
    for the base form (D-SRC1a). Function words and words WordNet lacks:
    ("function word", None, None) / ("other", None, None)."""
    if word.lower() in FUNCTION_WORDS:
        return "function word", None, None
    wn = wordnet()
    best = None   # (count, order, pos, base)
    for order, p in enumerate(("n", "v", "a", "r")):
        # Every base WordNet can reduce `word` to for this part of speech
        # (windows -> Windows, window); the most-used base wins.
        for base in dict.fromkeys(b.lower() for b in wn._morphy(word.lower(), p)):
            syns = wn.synsets(base, pos=p)
            if not syns:
                continue
            c = sum(l.count() for s in syns for l in s.lemmas() if l.name().lower() == base)
            if best is None or (c, -order) > (best[0], -best[1]):
                best = (c, order, p, base)
    if best is None:
        return "other", None, None
    _, _, p, base = best
    first = wn.synsets(base, pos=p)[0]
    return POS_NAMES[p], first.definition(), first.name()


def build_prompt(headword: str, pos: str, sense: str | None, tier: str) -> str:
    """The ONLY prompt builder. Four inputs, nothing else (I5 / A15.10)."""
    tmpl = (HERE / "prompts" / f"{PROMPT_VERSION}.txt").read_text(encoding="utf-8")
    return (tmpl.replace("{HEADWORD}", headword).replace("{POS}", pos)
            .replace("{SENSE}", sense or "not given; use the most common everyday meaning")
            .replace("{TIER}", tier))


def gen_params(prompt: str) -> dict:
    return {
        "model": MODEL,
        "max_tokens": 2000,
        "output_config": {"effort": EFFORT, "format": {"type": "json_schema", "schema": GEN_SCHEMA}},
        "messages": [{"role": "user", "content": prompt}],
    }


def safety_params(items: list[dict]) -> dict:
    prompt = (SAFETY_PROMPT.read_text(encoding="utf-8")
              .replace("{LANGUAGE_NAME}", "English").replace("{LANG_CODE}", "en"))
    return {
        "model": MODEL,
        "max_tokens": 4000,
        "output_config": {"effort": EFFORT},
        "messages": [{"role": "user", "content": prompt + "\n\n" + json.dumps(items, ensure_ascii=False)}],
    }

# ---------------------------------------------------------------- validator

SUFFIXES = ("ing", "ed", "es", "s", "er", "est", "ly", "ies", "ied")


def tokens(text: str) -> list[str]:
    return re.findall(r"[a-z]+(?:'[a-z]+)?", text.lower())


def forms(word: str) -> set[str]:
    w = word.lower()
    out = {w, w + "s", w + "es", w + "ed", w + "d", w + "ing", w + "er", w + "est", w + "ly"}
    if w.endswith("e"):
        out |= {w[:-1] + "ing", w[:-1] + "ed"}
    if w.endswith("y"):
        out |= {w[:-1] + "ies", w[:-1] + "ied", w[:-1] + "ier", w[:-1] + "iest", w[:-1] + "ily"}
    if len(w) >= 3 and w[-1] not in "aeiouwxy" and w[-2] in "aeiou" and w[-3] not in "aeiou":
        out |= {w + w[-1] + "ing", w + w[-1] + "ed", w + w[-1] + "er"}
    return out


def stem(word: str) -> str:
    w = word.lower()
    for suf in sorted(SUFFIXES, key=len, reverse=True):
        if w.endswith(suf) and len(w) - len(suf) >= 4:
            return w[: -len(suf)]
    return w


def syllables(word: str) -> int:
    w = word.lower().strip("'")
    groups = re.findall(r"[aeiouy]+", w)
    n = len(groups)
    if w.endswith("e") and n > 1 and not w.endswith(("le", "ee", "ye")):
        n -= 1
    return max(1, n)


_FAMILIAR = None


FAMILIAR_RANK = 8000   # roughly a grade-4 reading vocabulary


def familiar() -> set[str]:
    """Words a grade-4 reader knows: the Easy and Medium banks with their
    inflected forms, the function words, and the FAMILIAR_RANK most frequent
    English words. The frequency list (tools/wordpipe/sources/freq_en.txt,
    OpenSubtitles, CC BY-SA) is used as a CHECK only: none of its text reaches a
    prompt or ships (D-NOBYSA allows check-only use)."""
    global _FAMILIAR
    if _FAMILIAR is None:
        b = bank()
        _FAMILIAR = set(FUNCTION_WORDS)
        freq = ROOT / "tools" / "wordpipe" / "sources" / "freq_en.txt"
        for n, line in enumerate(freq.read_text(encoding="utf-8").splitlines()):
            if n >= FAMILIAR_RANK:
                break
            _FAMILIAR.add(line.split(" ", 1)[0].lower())
        for w, t in b.items():
            if t in ("easy", "medium"):
                _FAMILIAR |= forms(w)
    return _FAMILIAR


def hard_words(text: str) -> list[str]:
    """D-SRC4 "reads at about grade 4", in the Dale-Chall style: a word of three
    or more syllables that is not a familiar word. (Flesch-Kincaid on one short
    sentence is noise: "vegetables" alone scores grade 6.7.)"""
    fam = familiar()
    return [w for w in tokens(text) if syllables(w) >= 3 and w not in fam]


def blocked_terms() -> tuple[set[str], set[str]]:
    kid = {l.strip().lower() for l in (ROOT / "assets/words/kid-exclude/en.txt").read_text(encoding="utf-8").splitlines()
           if l.strip() and not l.startswith("#")}
    block = {l.strip().lower() for l in (ROOT / "backend/blocklist.txt").read_text(encoding="utf-8").splitlines()
             if l.strip() and not l.startswith("#")}
    return kid, block


def check_field(field: str, text: str, headword: str, tier: str, lists=None) -> list[str]:
    """Problems with one generated field; [] means it passes the code checks."""
    kid, block = lists or blocked_terms()
    t = text.strip()
    ws = tokens(t)
    hw = headword.lower()
    bad = []
    if not t:
        return ["empty"]
    if t.count(".") + t.count("!") + t.count("?") > 1 or "\n" in t:
        bad.append("more than one sentence")
    if field == "definition":
        if len(ws) > 15:
            bad.append(f"{len(ws)} words (max 15)")
        st = stem(hw)
        fs = forms(hw)
        if any(w in fs or (len(st) >= 4 and w.startswith(st)) for w in ws):
            bad.append("contains the headword or its stem")
    else:
        if not 6 <= len(ws) <= 14:
            bad.append(f"{len(ws)} words (6-14)")
        if ws.count(hw) != 1:
            bad.append(f"headword appears {ws.count(hw)} times (exactly 1)")
        if any(w in forms(hw) and w != hw for w in ws):
            bad.append("an inflected form of the headword")
    if tier in ("easy", "medium"):
        hw_forms = forms(hw)
        hard = [w for w in hard_words(t) if w not in hw_forms]
        if hard:
            bad.append("above grade 4: " + ", ".join(hard))
    for w in ws:
        if w != hw and w in kid:
            bad.append(f"kid-excluded word '{w}'")
        if any(len(b) >= 4 and b in w for b in block) or w in block:
            bad.append(f"blocklisted term in '{w}'")
    return bad

# ---------------------------------------------------------------- canary (A15.10)

def byasa_texts() -> list[str]:
    """Every BY-SA or unknown-licence English text in circulation: the Definition
    Match pool (Wiktionary) and the server's dictionary cache. Read from the
    newest definitions sheet export, which holds all of them."""
    sheets = sorted((ROOT / "build" / "defs-audit").glob("en-defs-*/sheet.csv"))
    out = []
    if sheets:
        with open(sheets[-1], encoding="utf-8") as f:
            next(f)
            for r in csv.DictReader(f):
                out.append(r["text"])
    return out


def canary_check(prompts: list[str], texts: list[str], min_len: int = 30) -> list[str]:
    """Texts of `min_len`+ characters that appear in any prompt (should be none)."""
    hay = "\n".join(p.lower() for p in prompts)
    return [t for t in texts if len(t) >= min_len and t.lower() in hay]

# ---------------------------------------------------------------- run

KEYCHAIN_SERVICE = "spell-defgen-anthropic"


def client():
    """The SDK's own credential resolution, plus one fallback: a macOS Keychain
    item Eric creates himself (so the key never sits in a file or a chat):
        security add-generic-password -a "$USER" -s spell-defgen-anthropic -w
    (it prompts for the key without echoing it). Nothing here prints the key."""
    import subprocess
    import anthropic
    if not os.environ.get("ANTHROPIC_API_KEY") and not os.environ.get("ANTHROPIC_AUTH_TOKEN"):
        r = subprocess.run(["security", "find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"],
                           capture_output=True, text=True)
        if r.returncode == 0 and r.stdout.strip():
            return anthropic.Anthropic(api_key=r.stdout.strip())
    return anthropic.Anthropic()


def submit(c, reqs: list[tuple[str, dict]]):
    from anthropic.types.message_create_params import MessageCreateParamsNonStreaming
    from anthropic.types.messages.batch_create_params import Request
    return c.messages.batches.create(requests=[Request(custom_id=i, params=MessageCreateParamsNonStreaming(**p)) for i, p in reqs])


def wait(c, batch_id: str):
    while True:
        b = c.messages.batches.retrieve(batch_id)
        if b.processing_status == "ended":
            return b
        print(f"  batch {batch_id}: {b.processing_status} ({b.request_counts.processing} processing)", flush=True)
        time.sleep(30)


def results(c, batch_id: str) -> dict[str, dict]:
    out = {}
    for r in c.messages.batches.results(batch_id):
        if r.result.type != "succeeded":
            out[r.custom_id] = {"error": r.result.type}
            continue
        m = r.result.message
        text = next((b.text for b in m.content if b.type == "text"), "")
        out[r.custom_id] = {"stop": m.stop_reason, "text": text,
                            "in": m.usage.input_tokens, "out": m.usage.output_tokens}
    return out


def pick_words(n: int, seed: int) -> list[tuple[str, str]]:
    b = bank()
    rng = random.Random(seed)
    per = {t: sorted(w for w, tt in b.items() if tt == t) for t in TIERS}
    picked = []
    for i, t in enumerate(TIERS):
        k = n // len(TIERS) + (1 if i < n % len(TIERS) else 0)
        picked += [(w, t) for w in rng.sample(per[t], k)]
    return picked


def plan(words: list[tuple[str, str]]) -> list[dict]:
    rows = []
    for w, t in words:
        pos, sense, syn = sense_for(w)
        rows.append({"word": w, "tier": t, "pos": pos, "sense": sense, "synset": syn,
                     "prompt": build_prompt(w, pos, sense, t)})
    return rows


def cost(usage: list[dict]) -> float:
    i = sum(u.get("in", 0) for u in usage)
    o = sum(u.get("out", 0) for u in usage)
    return i / 1e6 * BATCH_IN_PER_MTOK + o / 1e6 * BATCH_OUT_PER_MTOK


def run(run_dir: Path):
    st_path = run_dir / "state.json"
    st = json.loads(st_path.read_text())
    save = lambda: st_path.write_text(json.dumps(st, indent=1, ensure_ascii=False))
    c = client()
    rows = st["rows"]
    if "gen_batch" not in st:
        b = submit(c, [(f"g{i}", gen_params(r["prompt"])) for i, r in enumerate(rows)])
        st["gen_batch"] = b.id; save()
        print(f"generation batch {b.id} submitted ({len(rows)} words)")
    if "gen" not in st:
        wait(c, st["gen_batch"])
        st["gen"] = results(c, st["gen_batch"]); save()
    lists = blocked_terms()
    items = []
    for i, r in enumerate(rows):
        g = st["gen"].get(f"g{i}", {})
        r["candidates"] = []
        if g.get("stop") == "end_turn" and g.get("text"):
            try:
                r["candidates"] = json.loads(g["text"]).get("candidates", [])[:2]
            except ValueError:
                pass
        r["gen_status"] = g.get("error") or g.get("stop") or "missing"
        for j, cand in enumerate(r["candidates"]):
            for field in ("definition", "sentence"):
                cand[field + "_problems"] = check_field(field, cand.get(field, ""), r["word"], r["tier"], lists)
                if not cand[field + "_problems"]:
                    items.append({"id": f"{i}.{j}.{field}", "word": r["word"], "gloss": cand[field]})
    save()
    if "safety_batch" not in st:
        chunks = [items[k:k + 50] for k in range(0, len(items), 50)]
        st["safety_chunks"] = [[x["id"] for x in ch] for ch in chunks]
        b = submit(c, [(f"s{k}", safety_params([{"word": x["word"], "gloss": x["gloss"]} for x in ch]))
                       for k, ch in enumerate(chunks)])
        st["safety_batch"] = b.id; save()
        print(f"safety batch {b.id} submitted ({len(items)} fields)")
    if "safety" not in st:
        wait(c, st["safety_batch"])
        st["safety"] = results(c, st["safety_batch"]); save()
    # A flag names a word; map it back onto the fields of that chunk whose gloss was flagged.
    flagged, safety_failed = set(), set()
    for k, ids in enumerate(st["safety_chunks"]):
        res = st["safety"].get(f"s{k}", {})
        try:
            flags = json.loads(res.get("text", "")) if res.get("stop") == "end_turn" else None
        except ValueError:
            flags = None
        if flags is None:
            safety_failed |= set(ids)   # unreadable -> fail closed
            continue
        bad_words = {f.get("word") for f in flags if f.get("category") == "inappropriate"}
        for fid in ids:
            i = int(fid.split(".")[0])
            if rows[i]["word"] in bad_words:
                flagged.add(fid)
    out_rows, dark = [], []
    for i, r in enumerate(rows):
        for field in ("definition", "sentence"):
            kept = None
            for j, cand in enumerate(r["candidates"]):
                fid = f"{i}.{j}.{field}"
                if not cand[field + "_problems"] and fid not in flagged and fid not in safety_failed:
                    kept = cand[field]
                    break
            reasons = "; ".join(
                f"c{j + 1}: " + (", ".join(cand[field + "_problems"]) or
                                 ("safety flag" if f"{i}.{j}.{field}" in flagged else "safety unreadable"))
                for j, cand in enumerate(r["candidates"])) or r["gen_status"]
            out_rows.append({"word": r["word"], "tier": r["tier"], "pos": r["pos"], "synset": r["synset"] or "",
                             "field": field, "text": kept or "", "status": "ok" if kept else "dark",
                             "reasons": "" if kept else reasons,
                             "source": f"spell-gen-{PROMPT_VERSION}", "source_license": "original"})
            if not kept:
                dark.append(out_rows[-1])
    with open(run_dir / "rows.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(out_rows[0].keys()))
        w.writeheader(); w.writerows(out_rows)
    usage = list(st["gen"].values()) + list(st["safety"].values())
    spent = cost(usage)
    full_words = len(bank())
    projected = spent * full_words / len(rows)
    leak = canary_check([r["prompt"] for r in rows], byasa_texts())
    report = {
        "run": run_dir.name, "model": MODEL, "effort": EFFORT, "prompt": PROMPT_VERSION,
        "words": len(rows), "fields": len(out_rows), "kept": len(out_rows) - len(dark), "dark": len(dark),
        "spentUsd": round(spent, 4), "projectedFullUsd": round(projected, 2), "fullWords": full_words,
        "capUsd": CAP_USD, "overCap": projected > CAP_USD,
        "canaryLeaks": len(leak), "noWordNetSense": sum(1 for r in rows if not r["sense"]),
    }
    (run_dir / "report.json").write_text(json.dumps(report, indent=1))
    print(json.dumps(report, indent=1))
    if leak:
        print("CANARY: BY-SA text found in a prompt -- do not use this run"); sys.exit(2)
    if projected > CAP_USD:
        print(f"STOP: projected ${projected:.2f} for {full_words} words is over the ${CAP_USD} cap"); sys.exit(3)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("pilot", "dry-run"):
        p = sub.add_parser(name)
        p.add_argument("--n", type=int, default=50)
        p.add_argument("--seed", type=int, default=20261108)
    r = sub.add_parser("resume")
    r.add_argument("run_dir")
    a = ap.parse_args()
    if a.cmd == "resume":
        run(Path(a.run_dir))
        return
    rows = plan(pick_words(a.n, a.seed))
    leak = canary_check([r["prompt"] for r in rows], byasa_texts())
    if a.cmd == "dry-run":
        print(json.dumps({"words": len(rows), "noWordNetSense": sum(1 for r in rows if not r["sense"]),
                          "canaryLeaks": len(leak)}, indent=1))
        print(rows[0]["prompt"])
        return
    run_dir = ROOT / "build" / "defgen" / f"pilot-{time.strftime('%Y%m%d-%H%M%S')}"
    run_dir.mkdir(parents=True)
    # Every prompt is logged before anything is sent (D-SRC4).
    with open(run_dir / "prompts.jsonl", "w", encoding="utf-8") as f:
        for i, r in enumerate(rows):
            f.write(json.dumps({"id": f"g{i}", "word": r["word"], "prompt": r["prompt"],
                                "sha256": hashlib.sha256(r["prompt"].encode()).hexdigest()}, ensure_ascii=False) + "\n")
    if leak:
        print("CANARY: BY-SA text found in a prompt -- refusing to submit"); sys.exit(2)
    (run_dir / "state.json").write_text(json.dumps({"rows": rows}, indent=1, ensure_ascii=False))
    print(f"run dir {run_dir}")
    run(run_dir)


if __name__ == "__main__":
    main()
