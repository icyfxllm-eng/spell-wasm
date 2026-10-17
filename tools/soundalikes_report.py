#!/usr/bin/env python3
"""CC-SENSE-CUE v1.1 F8 + I3 -- the shared sound-alike census report and the proof test.

    python3 tools/soundalikes_report.py           # write reports/soundalikes-report.{json,md}
    python3 tools/soundalikes_report.py --check   # gate: fail if the committed report is stale

One scan, every consumer reads it: CC-SOUNDALIKE-BONUS (C2, C7) and CC-IMPOSSIBLE
(A4) read the JSON and never rescan.

THE PROOF TEST (I3). For a language with a spoken-key generator, group the bank
by key: any two rows with an equal key must have identical accepted-answer sets,
which for this table means they sit in the same kind-"true" set. A language
passes only if no such pair is split; failing pairs are listed by name.
`none-by-construction` additionally needs every equal-key group to be a single
row (nothing to accept), and is never granted without a generator: a language
with no spoken key cannot be proved safe, so it stays "missing".
"""
import json
import pathlib
import sys
import unicodedata
from collections import defaultdict

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "bank"))
from soundalikes import LANGS, ROOT, layer1_voices, soundalikes  # noqa: E402

OUT_JSON = ROOT / "reports" / "soundalikes-report.json"
OUT_MD = ROOT / "reports" / "soundalikes-report.md"


def generators():
    """lang -> spoken-key function for the bank, from tools/build_collisions.py.
    The English key returns every pronunciation (list) or None for heteronyms."""
    import build_collisions as bc
    out = {"es": bc.KEYS["es"], "ru": bc.KEYS["ru"]}
    if not bc.missing_en_inputs():
        out["en"] = bc.key_en()
    return out, bc.bank


def proof(lang, key, bank_rows, table):
    groups = defaultdict(set)
    for w in bank_rows:
        k = key(w)
        if not k:
            continue
        for kk in (k if isinstance(k, list) else [k]):
            groups[kk if isinstance(kk, str) else tuple(kk)].add(unicodedata.normalize("NFC", w))
    sets_of = defaultdict(set)          # a word may sit in several sets (English)
    for s in table["sets"]:
        if s["kind"] != "true":
            continue
        for m in s["members"]:
            sets_of[m["memberId"]].add(s["setId"])
    # Pairs an AUDITOR separated on a locked sheet (F6): a "no" verdict (kind accent,
    # rule auditor-distinguishes) or a dropped member. The key cannot know those, so
    # they are allowed splits, not failures.
    separated = set()
    for s in table["sets"]:
        ms = [m["memberId"] for m in s["members"]]
        if s["kind"] == "accent" and "auditor-distinguishes" in s.get("mergerRules", []):
            separated |= {frozenset((a, b)) for a in ms for b in ms if a != b}
        for d in s.get("membersDropped", []):
            separated |= {frozenset((d, m)) for m in ms + s.get("membersDropped", []) if m != d}
    split = []
    multi = 0
    for k, rows in groups.items():
        if len(rows) < 2:
            continue
        multi += 1
        rows_l = sorted(rows)
        bad = [(a, b) for i, a in enumerate(rows_l) for b in rows_l[i + 1:]
               if not (sets_of.get(a, set()) & sets_of.get(b, set())) and frozenset((a, b)) not in separated]
        if bad:
            split.append(rows_l)
    return {"ran": True, "equalKeyGroups": multi, "passes": not split,
            "splitGroups": sorted(split)[:50], "splitCount": len(split),
            "noneByConstructionEligible": multi == 0 and not split}


def profanity(lang):
    from census_c6 import fold_lenient, load
    lst = load(str(ROOT / "assets" / "words" / "profanity" / f"{lang}.txt")) or set()
    return lambda w: fold_lenient(w) in lst


def build_report():
    gens, bank = generators()
    voices = layer1_voices()
    rows = {}
    import build_collisions as bc
    for lang, variant in [(l, None) for l in LANGS] + list(bc.VARIANTS):
        t = soundalikes(lang, variant=variant)
        bank_rows = bank(lang, variant) if variant else bank(lang)
        banked = set(unicodedata.normalize("NFC", w) for w in bank_rows)
        is_prof = profanity(lang)
        kinds = defaultdict(int)
        confirmed = flags = 0
        bonus_words, impossible_words = set(), set()
        for s in t["sets"]:
            kinds[s["kind"]] += 1
            confirmed += 1 if s.get("confirmed") else 0
            flags += 1 if s.get("loopbackFlag") else 0
            members = [m["memberId"] for m in s["members"]]
            in_bank = [m for m in members if m in banked]
            impossible_words.update(in_bank)
            if s["kind"] == "true":
                for w in in_bank:
                    # Proper nouns: bank rows already pass PURITY's proper-noun gate,
                    # and generated members are lowercase dictionary forms.
                    others = [o for o in members if o != w and not is_prof(o)]
                    if others and not is_prof(w):
                        bonus_words.add(w)
        pr = proof(lang, gens[lang], bank_rows, t) if lang in gens else {
            "ran": False, "reason": "no spoken-key generator for this language yet; cannot be proved safe"}
        rows[f"{lang}+{variant}" if variant else lang] = {
            "status": t["status"],
            "voice": t.get("voice"),
            "voiceMatchesLayer1": t.get("voice") == voices.get(lang),
            **({"servedOnlyWhen": "en.retier is on (not Spell Jr / Kid Mode)"} if variant else {}),
            "generator": t.get("generator"),
            "bankRows": len(bank_rows),
            "sets": dict(sorted(kinds.items())),
            "confirmedSets": confirmed,
            "loopbackFlags": flags,
            "bonusEligibleBankWords": len(bonus_words),
            "impossibleFilterRemoves": len(impossible_words),
            "proof": pr,
        }
    return {"$doc": "GENERATED by tools/soundalikes_report.py -- CC-SENSE-CUE v1.1 F8 shared census. "
                    "Consumers read this; none rescans.", "languages": rows}


def render_md(rep):
    lines = ["# Sound-alike tables: shared census (generated)", "",
             "CC-SENSE-CUE v1.1 F8. Regenerate with `python3 tools/soundalikes_report.py`.", "",
             "| Lang | Status | Voice matches | Generator | Bank rows | true sets | accent sets | Confirmed | Loopback flags | Bonus-eligible words | Impossible removes | Proof test |",
             "|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for lang, r in rep["languages"].items():
        p = r["proof"]
        proof = ("pass" if p.get("passes") else f"FAIL ({p['splitCount']} split)") if p["ran"] else "not runnable"
        lines.append(f"| {lang} | {r['status']} | {'yes' if r['voiceMatchesLayer1'] else 'NO'} | {r['generator'] or '-'} | "
                     f"{r['bankRows']:,} | {r['sets'].get('true', 0)} | {r['sets'].get('accent', 0)} | {r['confirmedSets']} | "
                     f"{r['loopbackFlags']} | {r['bonusEligibleBankWords']} | {r['impossibleFilterRemoves']} | {proof} |")
    lines += ["", "Status stays `missing` for every language until the F5 voice loopback has run (Phase B). "
              "`none-by-construction` is granted only by the proof test with a spoken-key generator; no language has one yet "
              "beyond en, es and ru, whose banks do contain sound-alikes."]
    return "\n".join(lines) + "\n"


def main():
    rep = build_report()
    js = json.dumps(rep, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    md = render_md(rep)
    if "--check" in sys.argv:
        stale = [p for p, body in ((OUT_JSON, js), (OUT_MD, md))
                 if not p.exists() or p.read_text(encoding="utf-8") != body]
        failed = [l for l, r in rep["languages"].items() if r["proof"]["ran"] and not r["proof"]["passes"]]
        mismatched = [l for l, r in rep["languages"].items() if not r["voiceMatchesLayer1"]]
        ok = True
        if stale:
            print("soundalikes-report: STALE --", ", ".join(str(p.relative_to(ROOT)) for p in stale),
                  "-- regenerate with python3 tools/soundalikes_report.py"); ok = False
        if failed:
            print("soundalikes-report: proof test FAILS for", failed); ok = False
        if mismatched:
            print("soundalikes-report: table voice is not the backend's Layer-1 voice for", mismatched,
                  "-- regenerate the tables (I6)"); ok = False
        if ok:
            print("soundalikes-report: OK -- report current, proof tests pass, voices match")
        return 0 if ok else 1
    OUT_JSON.write_text(js, encoding="utf-8")
    OUT_MD.write_text(md, encoding="utf-8")
    print(md)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
