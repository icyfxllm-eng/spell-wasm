# CC-LAUNCH-EN Phase C: English quality (F2, F7, F8, F9)

Branch `launch-en/phase-c` (worktree ~/repos/spell-launch-en-c), 2026-11-08.

| Item | Acceptance | State |
|---|---|---|
| F2 British spelling | A2.1, A2.2, A2.3, I2.4 | Green. `cargo test --lib en_quality` |
| F7 Repetition | A7.1, A7.2 (+ Spell Jr pool) | Green. `cargo test --lib repetition_sim` |
| F8 Homophones | A8.1 | Green. `cargo test --lib en_quality_a8_1` |
| F8 Near-sounds | A8.2 | Whisper 34/34; Phase B DTW bar 24/34. **Needs Eric: which gate counts.** `python3 tools/near_sound_loopback.py` |
| F9 Definitions | A9.1 | Sheet built, not rated. Kid Def Match fix done. **Needs Eric: who rates; what to do about 1,878 uncached words.** |

## F2 British spelling

- Merged `en-variants-live` (fe9d3687, c7530bf0, Eric 2026-11-04): 115 SCOWL pairs live, "pedophilia" quarantined, feces, gonorrhea, hemorrhoid and womanizer out of Kid Mode and Spell Jr. The English table now holds 335 pairs.
- `tests/fixtures/en-uk-variants.json`: the 335 pairs, frozen. It includes all 15 spec examples (colour, organise, centre, grey, programme, tyre, aluminium, jewellery, mould, plough, defence, licence, practise, catalogue, cheque).
- Normal, Daily, Climb and Spell Jr grade through one path, `game::submit_guess`. For English outside Impossible that path is `en_route` (other sense or variant, local), else `en_grade_local`. Tier and mode never enter it. The tests drive those two real functions for a standard player and for Spell Jr.
  - A2.1: 335/335 pairs correct in both directions, for both player types.
  - A2.2: 100 US-only bank words with a one-letter corruption, 0 accepted.
  - A2.3: the result card (`variants::display` with the British switch off, the default) shows the US form for every pair. Accent and compound rows are accepted answers only and are never a display pair, by design (`accepted::pairs`).
  - I2.3: Climb score reads only the true/false verdict. The other-sense marker feeds the learner tag, never the score.
  - I2.4: a UK form is never accepted against an unrelated row.
- Not covered by a screen snapshot: there is no e2e hook to force a specific word, so A2.3 is a unit test on the card's display function.
- Still true from the census (`reports/en-variants-census.md`): Word-cross uses the language-blind matcher, so a British spelling is graded wrong there. Spelluzzle and Practice compare letters. These are outside A2.1's four modes.

## F7 Repetition

- Built in Phase A: `repetition_sim` covers A7.1 (30 Daily dates, standard and Little Speller, no word twice) and A7.2 (200 Normal draws, Easy to Hard, legacy and re-tier banks).
- Added: the Spell Jr pool (the legacy bank through the kid filter, stricter after the variants merge) still holds 200+ words per tier, with no repeat in 200 draws.

## F8 Homophones and near-sounds

- The SENSE-CUE Phase B fairness work is on main (R5). English cueMode is off, so a true homophone takes the accept-all path: `en_route` returns OtherSense and the answer is correct.
- A8.1: `tests/fixtures/en-homophones.json` holds 50 true homophones (their/there, made/maid, main/mane, sea/see, ...). All 50 take the other-sense path in both directions and grade correct, for both player types.
  - read/red is left out: read is also said "reed", so its set accepts one way only, by design.
- The shipped sets include some generated pairs that are not true homophones. These are accepted as correct today: capped/kept, lass/last, marc/mark, dos/dose, car/carr, pic/pick, mass/masse, caller/color, carry/kerry, aim/ame, mae/may, walk/wok, are/our, sot/sought. They were not changed here; that table is SENSE-CUE's (tools/build_collisions.py). **Worth a look before launch.**
- A8.2: `tests/fixtures/en-near-sounds.json` holds 34 near-sound pairs: worm/were, main/made and 32 more. Both words in each pair are in the bank, and no pair is a true homophone. `tools/near_sound_loopback.py` renders each word with the app's voice (local /api/speak) and measures two ways:
  - **Phase B DTW bar (the spec's literal "Phase B gate"):** 24/34 at or above the bar (0.6584 this run). Under it: worm/were, main/made, bit/beat, sit/seat, men/man, much/match, pick/peak, three/tree, bird/board, farm/form.
  - **Whisper (the spec's "whisper loopback"):** 34/34. Each clip is heard as itself, or a true homophone of itself, and never as its partner. It uses the small model, with large-v3 as a fallback.
  - The two disagree because the DTW measure compares coarse band energies. It does not resolve vowel-only differences: men/man scores closer than the same-sound control gray/grey. It also reads the shared "The word is..." lead-in on worm and were as similarity. Phase B used it only to flag sets for a human, never as a gate.
  - The tool currently exits 1 because it gates on both measures.
  - Report: `reports/launch-en-near-sound-loopback.json`.

## F9 Definitions

- Where English definitions come from (all served, none bundled):
  - Definition Match reads `def_pools/en.json` on the server.
  - The play hint, the result card, Spelldoku, Spell Search and the Sentence button read `/api/meaning`. This is dictionaryapi.dev's first sense, cached on the server forever.
  - Kid Mode shows both; the Definition and Sentence buttons are not kid-gated.
- **Fixed:** Kid Definition Match trusted the server's mechanical `kid_register` (short text, easy or medium), so it served dead, die, kill and dies. Kid Mode excludes those everywhere else. `defmatch::kid_register` now also requires `kid_filter::kid_allowed`, for targets and distractors.
- **Built:** the locked-sheet path did not exist for definitions. `tools/defs/en_sheet.py` exports every English definition that can render, after the audio sheet's design:
  - Each row is one word, source, field and text, with `kid_visible`, a mechanical keyword hint, and blank `severity` / `fix` columns.
  - A stamp covers the locked columns.
  - Eight decoy rows are invented, plainly not-for-kids definitions; their key is in `keys/` (gitignored).
  - The severity scale is 0-3.
- `tools/defs/en_ingest.py` checks three gates: the stamp, that every row is rated, and that the decoys were rated above 0 (one miss allowed). Then, with `--apply`:
  - it appends every word with a kid-visible row above 0 to `assets/words/kid-exclude/en.txt`;
  - it writes a patched `def_pools.en.json` for the content-only server deploy;
  - it writes `reports/en-definitions-pass.md`.
- Tested on synthetic fills: a complete sheet is accepted; a sheet that misses two decoys is rejected; a sheet with one altered text is rejected.
- **Sheet en-defs-20261108:** 4,838 rows (4,830 real plus 8 decoys), 2,660 of them kid-visible, 139 with a keyword hint. File: `build/defs-audit/en-defs-20261108/sheet.csv`.
- **Gap: 1,878 of 3,157 served English words have no cached meaning.** They are fetched live from dictionaryapi.dev the first time anyone asks, so nobody can review them in advance. 1,870 of them are in Spell Jr's pool. The list is in `uncached.txt`, beside the sheet.
- Drift: the served `def_pools/en.json` still has the "coup" row that 9174c05e removed from the repo copy. The sheet exports the served copy.
