# CC-SOUNDALIKE-LADDER: Swahili (sw) status

Voice: sw-TZ-RehemaNeural. Key: `key_sw@1`. Cap: **machine_checked** for v1 (decision 2026-11-07; 92% of the sw bank has no kaikki IPA, so
a set could never pass the F3 IPA check). A set that would otherwise be provisional carries reason `no_ipa_coverage`.

## Key
Normalized spelling plus the Arabic-loan digraph spellings dh=z, th=s, kh=h, gh=g. ng' (velar nasal) stays apart from ng. No squeeze of
doubled letters: mm is a syllabic nasal and aa is a long vowel (mmoja is not moja). Never merged: l/r, b/mb, ch/sh, ng'/ng, m/mm.
Tests: 4 must-merge, 11 must-not-merge; selftest passes.

## Result
The key finds **0 sets** in the 2,826-word sw bank. Swahili spelling is near-phonetic, so the bank holds no two words that sound the same
and are spelled differently. ladderStatus stays `missing` (no sets), `status` missing, and nothing changes in grading, exclusion or the
Impossible pool. No loopback was run (no sets, and no authentic same-sound control pair exists for this language: the Arabic-loan
digraph pairs are read digraph-by-digraph by the voice, so they would set a bad floor). No Gig B sheet (0 rows).
