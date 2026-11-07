# CC-SOUNDALIKE-LADDER v1: zh and hi design note (F8, D4)

Design only. No code, no table, no flag, no change to either language's `soundalikes.json` (both stay `status: missing`, zero sets).

## zh (Mandarin, cmn-CN-Wavenet-A)

**Where the lane already lives.** The zh bank stores `pinyin|hanzi` rows and grades typed toned pinyin through `PinyinKey` (src/pinyin.rs). The ortho census
found 626 rows in 254 groups that share a toned-pinyin key but have different hanzi (10.13% of rows; groups of 2 to 9; two identical-hanzi groups were
quarantined). That is the pinyin homophone lane: it is exempt from I1 by design, because the grader compares pinyin and cannot tell those rows apart.

**Rule (from F8).**
- Same syllables and same tones (`hu2li2` vs `hu2li2`) = **true** homophones. Typing the pinyin is a correct answer for every member, so these are
  accept-any by construction; the sound-alike table would only record what the grader already does.
- Same syllables, different tones (`ma1` / `ma3`) = **accent**. A tone difference is a different word with a different sound; the table may list it as an
  accent set for cues and exclusion, never as accept-any.
- Neutral tone and 3-3 sandhi: no sandhi applied (the census applied none). The key is the written toned pinyin; `yi1`, `bu4` sandhi variants are not merged.

**Key (when built).** `key_zh` = the existing `PinyinKey` of each row, nothing new: pinyin letters, tone digit kept, ü = v. No new phonetic key is needed, and none is
safe to invent (no tone-sandhi or erhua rules without a native check).

**Ladder placement.**
- machine_checked: key + tests (must-not-merge: tone pairs, zh/z, ch/c, sh/s, n/ng, l/n) + loopback. The loopback hears spectra, not pitch (vi taught this: a pure
  tone pair looks identical), so it **cannot** support or refute a tone difference. For zh it checks only that members of one toned-pinyin set sound alike, and its
  controls must be segmental (never a tone pair as a "different" control).
- provisional needs kaikki IPA agreement. Mandarin kaikki entries use `[...]` transcriptions with tone letters, so a Mandarin check is possible later; the leak
  rules apply (check only, never stored).
- The pinyin homophone lane stays outside the ladder for grading: those rows are accepted today by the pinyin grader and the ladder must not change that.
- Any later zh table is exclusion and cue only until Gig B.

**Open for Eric before any code:** whether zh sets enter the ladder at all (the grader already collapses them), and whether tone-pair accent sets are wanted for
sense cues. Voice cmn-CN-Wavenet-A is Beijing-standard; not checked by a listener.

## hi (Hindi, hi-IN-Neural2-A)

**v1 scope (F8): seed-only plus loopback, capped at machine_checked.** No key function.
- Seeds: a short curated list of true Hindi homophone pairs where the written forms differ but the sound does not, from sources already in the repo's accepted
  variants (data/hi/accepted-variants.tsv) and a native-confirmable list: chandrabindu/anusvara variants (अँधेरा / अंधेरा), nukta and non-nukta spellings (ज़/ज, फ़/फ in
  careful vs casual speech is an *accent* case, not true), and ङ/ञ/ण/न/म before a stop (anusvara/nasal-conjunct equivalence: गंगा / गङ्गा).
- Loopback: controls from the seed list itself (same: अँधेरा/अंधेरा-type; different: unrelated words). Spectral comparison only, flag-only.
- Cap: **machine_checked**. Hindi has no bank-wide IPA path (kaikki Hindi IPA is partial and often phonemic with schwa deleted inconsistently), so provisional is not
  reachable in v1, same stance as sw.

**Parked: the schwa-deletion key.** Hindi drops the inherent schwa in many positions (कमल = kamal, not kamala), so two spellings can sound alike or differ depending
on schwa rules that need a morphological and syllable-weight model to apply correctly. A wrong key here would merge different words, which is exactly the accept-any
risk the ladder exists to avoid. No code, no sketch beyond this paragraph. Reopen only with a native auditor and a Gig B round.

**Open for Eric:** who confirms the seed list (no native auditor is assigned), and whether the nukta pairs count as true or accent.

## What this does not change
Production grading, en, de, the build-56 path, the ortho flags and every other language. No file other than this note is added.
