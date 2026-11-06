#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL Phase 1 follow-up cleanups (Eric, 2026-11-06: "Yes, do them").
  es  periodo and video quarantined; the accented rows (periodo-accented, video-accented) are kept, the bare forms become accepted variants
  fil lalakeng quarantined (inflected sibling of the resolved lalake/lalaki pair); lalaking kept, lalakeng an accepted variant
Rows are moved, never deleted. Run once from a clean checkout of this branch."""
import os, sys, unicodedata
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO, TAG = "2026-11-06", "Eric 2026-11-06", "CC-ORTHO-SWEEP-ALL: "
cuts = [("es", "periodo", "two rows for one word; período kept (RAE preferred), periodo an accepted variant"),
        ("es", "video", "two rows for one word; vídeo kept (RAE preferred), video an accepted variant"),
        ("fil", "lalakeng", "legacy vowel spelling of lalaking (inflected lalake/lalaki); lalaking kept, lalakeng an accepted variant")]
import csv
overrides = []
for lang, w, why in cuts:
    ev = TAG + why + "; " + WHO
    assert ps.apply_cuts(lang, {w}, "variant_duplicate", ev, D), (lang, w)
    overrides.append([lang, w, "variant_duplicate", WHO, ev])
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
    csv.writer(f, lineterminator="\n").writerows(overrides)
ps.rebuild_and_rehash()
RAE = "RAE, Diccionario de la lengua española: both forms are valid"
with open(f"{REPO}/data/es/accepted-variants.tsv", "a", encoding="utf-8") as f:
    f.write(f"período\tperiodo\t{RAE} (período is the standard form; periodo is the accepted alternative)\tdual\n")
    f.write(f"vídeo\tvideo\t{RAE} (vídeo is the Spain form, video is used in the Americas)\tdual\n")
with open(f"{REPO}/data/fil/accepted-variants.tsv", "a", encoding="utf-8") as f:
    f.write("lalaking\tlalakeng\tKWF Ortograpiyang Pambansa 2014: e/i in final syllables (legacy lalake, with the -ng ligature)\tlegacy_vowel\n")
print("done")
