# CC-SOUNDALIKE-LADDER: Polish (pl) status

Voice: pl-PL-Wavenet-B. Key: `key_pl@1` in tools/soundalike_keys.py. ladderStatus: **machine_checked** (lowest set).

## Key
ó=u, rz=ż (rz after a voiceless consonant is sz: trzy, krzak), ch=h, and word-final devoicing (kot=kod, grób=grup, tuż=tusz).
Not merged: ą, ę, ł, ń, ć, ś, ź, ż/z, sz/s, cz/c, mid-word voicing. Tests: 7 must-merge and 19 must-not-merge pairs (rok/róg, żar/zar, ból/bol,
ten tests included in the selftest). Two pairs I first listed as must-not-merge (mur/mór, tuż/tusz) are real homophones, so they moved to must-merge.

## Result
The Polish bank holds almost no homophone pairs: only **3 sets**. The classic pairs (może/morze, kot/kod, grób/grup) are not both banked.
- pad/pat — **provisional** (key, kaikki IPA check and loopback agree)
- czynny/czyny — machine_checked, reason `loopback_flag` (0.644 against a bar of 0.643, a hair over)
- rodzina/rodzinna — machine_checked, reason `ipa_missing`

Loopback controls: same kot/kod, może/morze, grób/grup, hart/chart; different kot/mleko, dom/okno. Floor 0.609, ceiling 0.780, bar 0.643.
Disagreement rate under F3: 2 of 3 sets, but the sample is far too small to read as a key problem (the stop condition is not meaningful at n=3).

## Effects
No grading change: nothing reads a ladder set for production except verified ones. Impossible pool for pl is unchanged (fresh build matches).
Gig B sheet: reports/soundalike-gigb-pl.csv (3 rows, not sent).
