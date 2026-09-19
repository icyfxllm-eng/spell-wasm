#!/usr/bin/env python3
"""Render census.py's JSON into the tables of human_audio_census.md.

Prints markdown to stdout; the prose findings around it are written by hand
into docs/census/human_audio_census.md, because they are judgments, not counts.
Also writes one gap list per language (tier<TAB>entry) for Gig C sizing.
"""
import json
import pathlib
import sys

TIERS = ["easy", "medium", "hard", "expert"]


def pct(a, b):
    return f"{100 * a / b:.1f}%" if b else "—"


def main(path, gap_dir):
    r = json.loads(pathlib.Path(path).read_text())
    gap_dir = pathlib.Path(gap_dir)
    gap_dir.mkdir(parents=True, exist_ok=True)

    print("## Summary per language\n")
    print("| Lang | Variety | LL categories | Commons files | Bank | Raw | **Usable (I8)** | Usable if residence counts | Gap | D5 held out | Usable speakers | Top speaker share |")
    print("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    order = sorted(r, key=lambda l: -sum(t["usable"] for t in r[l]["tiers"].values()) /
                   max(1, sum(t["n"] for t in r[l]["tiers"].values())))
    for lang in order:
        d = r[lang]
        n = sum(t["n"] for t in d["tiers"].values())
        raw = sum(t["raw"] for t in d["tiers"].values())
        us = sum(t["usable"] for t in d["tiers"].values())
        ur = sum(t["usable_residence"] for t in d["tiers"].values())
        hg = sum(t["homographs"] for t in d["tiers"].values())
        top = d["top_speaker"]
        top_s = f"{100 * top['share']:.0f}% ({top['name']})" if top else "—"
        print(f"| {lang} | {d['variety']} | {', '.join(d['isos'])} | {d['commons_titles']:,} | {n:,} | "
              f"{pct(raw, n)} | **{pct(us, n)}** | {pct(ur, n)} | {d['gap']:,} | {hg or '—'} | {d['speakers_usable']} | {top_s} |")

    print("\n## Per tier: raw / usable (I8) / usable if residence counts\n")
    print("D3 enables a tier at ≥80% usable *verified*; these are ceilings. ✓ marks ≥80% under I8.\n")
    print("| Lang | " + " | ".join(TIERS) + " |")
    print("|---|" + "---|" * len(TIERS))
    for lang in order:
        cells = []
        for t in TIERS:
            x = r[lang]["tiers"][t]
            mark = " ✓" if x["n"] and x["usable"] / x["n"] >= 0.8 else ""
            cells.append(f"{pct(x['raw'], x['n'])} / {pct(x['usable'], x['n'])}{mark} / {pct(x['usable_residence'], x['n'])}")
        print(f"| {lang} | " + " | ".join(cells) + " |")

    print("\n## License mix (every matched clip, before variety and D5 filters)\n")
    print("| Lang | CC0 | CC BY | CC BY-SA | Excluded |")
    print("|---|---:|---:|---:|---:|")
    for lang in order:
        m = r[lang]["license_mix"]
        print(f"| {lang} | {m.get('CC0', 0):,} | {m.get('CC BY', 0):,} | {m.get('CC BY-SA', 0):,} | {m.get('excluded', 0):,} |")

    print("\n## Why matched clips were excluded (clip counts)\n")
    reasons = sorted({k for d in r.values() for k in d["exclusions"]})
    print("| Lang | " + " | ".join(reasons) + " |")
    print("|---|" + "---:|" * len(reasons))
    for lang in order:
        e = r[lang]["exclusions"]
        print(f"| {lang} | " + " | ".join(f"{e.get(k, 0):,}" if e.get(k) else "·" for k in reasons) + " |")

    print("\n## Where the variety-excluded speakers learned the language (top countries, clip counts)\n")
    for lang in order:
        v = r[lang]["variety_countries_excluded"]
        if v:
            print(f"- **{lang}** ({r[lang]['variety']}): " + ", ".join(f"{k} {n:,}" for k, n in v.items()))

    for lang, d in r.items():
        (gap_dir / f"{lang}.tsv").write_text("".join(f"{t}\t{e}\n" for t, e in d["gap_entries"]))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
