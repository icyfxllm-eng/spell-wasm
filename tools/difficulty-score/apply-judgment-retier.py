#!/usr/bin/env python3
"""Job A (judgment version) — hand-audited Expert/Hard swaps.

Unlike retier.py's heuristic (which fails on `yacht`), these swaps are Claude's
linguistic judgment about spelling-from-hearing difficulty. Each promotes a
genuine orthographic TRAP out of Hard and demotes a long-but-phonetically-
REGULAR word out of Expert. Counts per tier are preserved (balance + no-repeat
invariants hold). Every swap carries a one-line rationale for the native
reviewer who will confirm before this ships to the public.

Non-Latin scripts (th, ko, ja, zh) and the near-perfectly-phonetic languages
(vi tone-clusters, tr) are intentionally NOT touched here — held for a native
speaker, per the standing constraint.

    python3 tools/difficulty-score/apply-judgment-retier.py            # dry run
    python3 tools/difficulty-score/apply-judgment-retier.py --apply
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
WORDS = ROOT / "assets" / "words"

# lang -> list of (promote_from_hard, demote_from_expert, rationale)
SWAPS = {
    "en": [
        ("colonel", "phenomenon", "colonel /ˈkɜːrnəl/ — spelling shares nothing with the sound; a top-5 English trap. phenomenon is phonetic."),
        ("lieutenant", "protagonist", "lieutenant — 'lieu' is wholly unpredictable. protagonist spells as it sounds."),
        ("gauge", "archipelago", "gauge — 'au'=/eɪ/ silent trap. archipelago is regular (ch=k aside)."),
        ("liaison", "circumlocution", "liaison — two 'ai' digraphs + silent structure. circumlocution is long but regular."),
        ("maneuver", "juxtaposition", "maneuver — 'eu'=/uː/ + variant manoeuvre. juxtaposition is phonetic."),
    ],
    "es": [
        ("almohada", "perpendicular", "almohada — silent intervocalic h (a classic Spanish dictation trap). perpendicular is fully regular."),
        ("zanahoria", "kilogramo", "zanahoria — silent h + z(seseo). kilogramo is regular."),
    ],
    "fr": [
        ("écureuil", "obstacle", "écureuil — the 'euil' cluster (accueil family) is French's hardest vowel ending. obstacle is regular."),
        ("coquillage", "kilomètre", "coquillage — 'qu'=/k/ + 'ill'. kilomètre is regular (accent aside)."),
    ],
    "de": [
        ("Rhythmus", "Krankenhaus", "Rhythmus — rh + y + th, all un-German graphemes. Krankenhaus is a regular compound."),
        ("Xylophon", "Dinosaurier", "Xylophon — X=/ks/, y=/y/, ph=/f/. Dinosaurier is regular."),
    ],
    "it": [
        ("specchio", "transatlantico", "specchio — 'cch' cluster you must simply know. transatlantico is regular."),
        ("chitarra", "perpendicolare", "chitarra — ch=/k/ + double r. perpendicolare is regular."),
    ],
    "pt": [
        ("xilofone", "perpendicular", "xilofone — Portuguese x is notoriously multi-valued (here /ʃ/). perpendicular is regular."),
    ],
    "nl": [
        ("pinguïn", "sinaasappel", "pinguïn — the ï trema (dieresis) is a real orthographic decision. sinaasappel is regular double-vowel."),
    ],
    "pl": [
        ("żółw", "temperatura", "żółw — ż AND ó, two sound-identical-letter traps in four letters. temperatura is a regular loan."),
        ("łyżka", "uniwersytet", "łyżka — ł + ż. uniwersytet is a regular loan."),
    ],
    "sv": [
        ("skjorta", "kilometer", "skjorta — 'skj' is the sj-sound (/ɧ/), spellable many ways. kilometer is regular."),
    ],
    "nb": [
        ("skjorte", "kilometer", "skjorte — 'skj' sj-sound. kilometer is regular."),
        ("kjøkken", "universitet", "kjøkken — 'kj' sound + double k + ø. universitet is regular."),
    ],
}


def read_words(p: Path) -> list[str]:
    return [w.strip() for w in p.read_text(encoding="utf-8").splitlines()
            if w.strip() and not w.startswith("#")]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()

    out = ROOT / "tools" / "difficulty-score" / "out"
    out.mkdir(exist_ok=True)
    md = ["# Job A — judgment-based Expert re-tier", "",
          "Hand-audited by spelling-from-hearing difficulty. Each row promotes an",
          "orthographic trap into Expert and demotes a long-but-regular word out of",
          "it. Counts per tier unchanged. **Native reviewer: confirm each rationale",
          "before public release.**", ""]
    errors, total = [], 0
    for lang, swaps in SWAPS.items():
        hp, ep = WORDS / lang / "hard.txt", WORDS / lang / "expert.txt"
        hard, expert = read_words(hp), read_words(ep)
        md.append(f"## {lang}")
        for up, down, why in swaps:
            if up not in hard:
                errors.append(f"{lang}: promote '{up}' not found in hard.txt"); continue
            if down not in expert:
                errors.append(f"{lang}: demote '{down}' not found in expert.txt"); continue
            hard.remove(up); expert.append(up)
            expert.remove(down); hard.append(down)
            md.append(f"- ↑ **{up}** / ↓ {down}")
            md.append(f"  - _{why}_")
            total += 1
            print(f"[{lang}] ↑{up}  ↓{down}")
        md.append("")
        if args.apply and not errors:
            hp.write_text("\n".join(sorted(hard)) + "\n", encoding="utf-8")
            ep.write_text("\n".join(sorted(expert)) + "\n", encoding="utf-8")

    md.insert(6, f"**{total} swaps across {len(SWAPS)} languages. Held for native review: "
                 f"th, ko, ja, zh (non-Latin), vi + tr (near-phonetic).**\n")
    (out / "judgment-retier.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    if errors:
        print("\nERRORS (nothing applied):", file=sys.stderr)
        for e in errors:
            print("  " + e, file=sys.stderr)
        sys.exit(1)
    print(f"\n{'APPLIED' if args.apply else 'DRY RUN'} — {total} swaps; "
          f"report → tools/difficulty-score/out/judgment-retier.md")


if __name__ == "__main__":
    main()
