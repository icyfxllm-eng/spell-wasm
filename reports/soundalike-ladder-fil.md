# CC-SOUNDALIKE-LADDER: Filipino (fil) status

Voice: fil-PH-Wavenet-A. Key: `key_fil@1` (tools/soundalike_keys.py). ladderStatus: **machine_checked** (lowest set).

## Key
Near-phonetic: normalized spelling (accents dropped, doubled letters squeezed) plus listed mergers c=k/s, qu=k, ph=f, x=ks, ñ=ny.
Not merged (accent or variant questions, left to accepted-variants): e/i, o/u, d/r, p/f, b/v. Tests: 8 must-merge, 13 must-not-merge; selftest passes.

## Result
The fil bank holds only **3 sets**, all spelling variants:
- barangay/baranggay: **provisional** (key, Tagalog kaikki IPA check and loopback agree)
- hangang/hanggang: machine_checked, reason `ipa_missing`
- ika-anim/ikaanim: machine_checked, reason `ipa_missing`

Loopback controls: same taksi/taxi, sine/cine, kilo/quilo, senyor/señor; different bahay/aso, tubig/kamatis. Floor 0.701, ceiling 0.974,
bar 0.756; 0 sets flagged. IPA source: kaikki Tagalog dump, check-only, not shipped (ipa-leak-check passes).
Disagreement rate: 0 of 3 by the key; 2 of 3 not promoted for missing IPA. n is too small to read as a key problem.

## Effects
No production grading change. The fil Impossible pool loses 1 word (baranggay, 1977 → 1930 available rows in the availability report), the intended F2 exclusion effect. Gig B sheet: reports/soundalike-gigb-fil.csv (3 rows, not sent).
