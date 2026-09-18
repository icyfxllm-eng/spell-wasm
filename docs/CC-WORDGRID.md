# CC-WORDGRID v1 — Spell Cross & Spell Search

**Status:** REVIEW-GATED. Execute phase by phase. Stop-and-ask conditions are binding.
**Owner:** Eric. **Target:** first build after sign-off.

> **Phase 0 census run 2026-09-18 — see `docs/census/wordgrid-census.md`.**
> Stop conditions fired for en, es and ru (E3, E4, E5, E6, E7) and for >5% E2
> failure (vi, hi). Nothing past Phase 0 is built.

---

## 0. Intent (read first)

A normal crossword or word search is a **reading** game. SpellGame is a **hearing → writing** game. If we copy the classics, players can finish by matching shapes on screen without spelling anything.

**Design rule for everything in this file:**

> Every letter a player commits must be earned by the ear, not copied from the screen.

Two more rules from Eric that sit next to the design rule:

- **No repeats.** Players should not keep seeing the same words across puzzles, and a word must never appear twice in the same puzzle.
- **Hints never give the answer away.** A hint may help the player *hear* or *understand* the word. It may never show its spelling, its stem, or a recognizable chunk of it.

When this spec is silent, choose the option that best protects these three rules. If two rules conflict, stop and ask.

---

## 1. Ownership boundaries (do not duplicate these specs)

| Concern | Owner | This file does |
|---|---|---|
| Audio playback | CC-BUILD219-FIXES (one audio resolver) | Calls the resolver only. No new audio path. |
| Homophone collisions, sense cues | CC-SENSE-CUE | Reads the collision table and requests cues. |
| Russian ambiguous positions | CC-RU-ORTHO (`ambiguous_positions`) | Consumes it if available (see F-C2 fallback). |
| Keyboards, ё/е equivalence, audio preconditions | CC-PLAYER-CONTRACT | Applies them per cell. |
| Jr resolver | CC-ONBOARD-JR | Reads the resolved tier only. |
| My Words dated lists | CC-MYWORDS-LISTS | Adds one "Make a puzzle" entry point. |
| Confusion matrix, `TRAP_MISS` | Base-game stats | Writes to them. No parallel stats. |

---

## 2. Signed decisions

- **D1 (signed):** Spell Cross is a **freeform criss-cross** layout, not a dense American-style grid. It uses only the source list's own words. There are no filler words.
- **D2 (signed):** If fewer than 5 words interlock, offer Spell Search for that list instead of showing a weak crossword.
- **D3 (signed):** Neither mode ever consumes shields. Rewards are stars only.
- **D4 (signed):** Launch languages are **en, es, ru**, plus any other bank language that passes the eligibility gate in §3. **Korean and Chinese are excluded**, regardless of the gate result.
- **D5 (signed):** Mixed-language grids are deferred to v2, and will cover same-script pairs only. They are not in this file.
- **D6 (signed):** Decoys come **only from the language confusion matrix**. The player's personal miss history is **never** a decoy source. This removes the earlier "your old mistakes in the grid" feature.
- **D7 (signed):** The player-facing names are **Spell Cross** and **Spell Search**, exactly.
- **D8 (signed):** The Daily Puzzle is pre-generated server-side. Puzzles from My Words and bank tiers are generated on device.

---

## 3. §0 Census (blocking — run before any feature work)

Produce `wordgrid_census.md` with one row per bank language. For each language, report the following gates.

| Gate | Pass condition |
|---|---|
| E1 Script | Left-to-right, alphabetic. Not ko, not zh. |
| E2 Cells | 100% of bank words segment into NFC grapheme clusters, and every cluster can be typed on that language's in-app keyboard. |
| E3 Audio | ≥ 98% of bank words have resolver-served audio. Words without audio are listed and excluded. |
| E4 Pool size | ≥ 400 audio-served words per tier that will be offered. This is required for the no-repeat window in F-X3. |
| E5 Decoys | The confusion matrix has ≥ 15 substitution, insertion, or deletion pairs with frequency above the threshold. |
| E6 Validity dictionary | A word list exists that is large enough to prove a decoy is **not** a real word. It must be the bank plus an external wordlist of at least 20k entries. |
| E7 Blocklist | A profanity blocklist exists for the language. |

**Outcome per language:**

- **Eligible for both modes:** passes E1–E7.
- **Eligible for Spell Search with no decoys, and Spell Cross:** passes E1–E4 and E7, but fails E5 or E6. This case is open decision D11.
- **Not eligible:** fails E1, E2, E3, E4, or E7.

**Stop and ask if:**

- en, es, or ru fails any gate.
- More than 5% of any language's bank fails E2.
- The census finds a language where one on-screen character needs more than one keyboard press to type, other than the diacritics already handled by CC-PLAYER-CONTRACT.

---

## 4. Features — Spell Search (Phase A)

**F-S1. Hear & Find**
- *Intent:* A visible word list turns a word search into shape-matching with no spelling.
- *Behavior:*
  - Each list slot shows only the word length (`_ _ _ _ _ _`) and a ▶ button.
  - The player hears the word, finds it, and drags across it.
  - A correct find reveals that slot's word.

**F-S2. Trap decoys (matrix only, per D6)**
- *Intent:* Make the player tell the right spelling apart from the plausible wrong one. That is real spelling work.
- *Behavior:*
  - The generator plants misspellings of targets, built by applying **one** confusion-matrix edit to the target.
  - Selecting a decoy shows "That's the trap spelling," logs a `TRAP_MISS`, and costs 1 star-point. It never costs a shield.

**F-S3. Difficulty table**

| Tier | Directions | Decoys | Clue | Slow replay |
|---|---|---|---|---|
| Jr | → ↓ | 0 | audio + picture | yes |
| Easy | → ↓ ↘ | 1 per 4 targets | audio | yes |
| Medium | → ↓ ↘ ↗ | 1 per 2 targets | audio | yes |
| Hard | all 8 directions, including backwards | 1 per target | audio | yes |
| Expert | all 8 directions | 2 per target, filler weighted toward target letters | audio | no |

**F-S4. Lock It In**
- *Intent:* Finding a word is weaker practice than writing it, so the session ends on writing.
- *Behavior:*
  - After the grid is cleared, the found words replay in random order and the player types each one with the grid hidden.
  - This step is optional below Hard and required for Expert stars.
  - Results go through the normal base-game scoring path.

**F-S5. Filler**
- Filler letters are sampled from the language's letter frequency.
- Filler must satisfy I3 (no accidental words).

---

## 5. Features — Spell Cross (Phase B)

**F-C1. Audio clues**
- Tapping a clue number plays the word through the resolver.
- Jr also shows a picture.
- There are no text clues except the paid definition hint in F-X4, which is filtered.

**F-C2. Crossings are the hint system**
- *Intent:* When a player is stuck on a hard word, the way forward is to solve an easier word that crosses it.
- *Behavior:* The generator scores candidate layouts by where crossings land. A **trap position** is the position from `ambiguous_positions` (ru, once CC-RU-ORTHO ships) or any position the confusion matrix edits.
  - **Jr, Easy, Medium:** prefer crossings **on** trap positions.
  - **Hard, Expert:** prefer crossings **off** trap positions.
- *Fallback:* If `ambiguous_positions` is not available for ru, use confusion-matrix positions only. Do not wait for CC-RU-ORTHO.

**F-C3. Homophone answerability**
- A clue word that is in a collision group must be handled in this order:
  1. Give it a crossing at a position that tells the two spellings apart.
  2. If that isn't possible, attach a sense cue (CC-SENSE-CUE).
  3. If neither is possible, exclude the word from this grid.
- Never accept both spellings in a grid cell.

**F-C4. Cells**
- One NFC grapheme per cell.
- An accented letter is distinct from its base letter, so the generator never crosses *é* with *e*.
- The ё/е equivalence rule applies per cell at the tiers where CC-PLAYER-CONTRACT applies it.

**F-C5. Check and feedback**
- A word checks when its last cell is filled.
- If it's wrong, only the wrong cells flash, and the result is written to the confusion matrix and `TRAP_MISS` exactly as the same misspelling would be in the base game.
- There is no letter-by-letter auto-check at Hard or Expert.

**F-C6. Keystone word**
- Shaded cells hold the letters of a hidden word.
- When the grid is done, the keystone's audio plays and the player spells it with the grid hidden.
- The keystone word must itself obey F-X3 (no repeats).

**F-C7. D2 fallback**
- If the best layout for a list connects fewer than 5 words, show "This list makes a better Spell Search," with one button.
- Never render a disconnected crossword.

---

## 6. Shared features

**F-X1. Sources**
- The Daily Puzzle (one per mode per language), identical for all players.
- Bank tier puzzles.
- A "Make a puzzle" button on any dated My Words list.

**F-X2. Deterministic generation**
- The seed is built as `seed = hash(source_id, player_puzzle_counter, date_for_daily)`.
- The same seed and inputs always produce a byte-identical grid on every platform.

**F-X3. No-repeat engine**

- *Intent:* Players should not keep meeting the same words, and nothing should appear twice in one puzzle.
- *Behavior:*
  - **Within a puzzle:**
    - Words are deduplicated after NFC and casefolding.
    - In Spell Search, if one target is contained in another (forward or reversed, such as *car* in *cart*), the shorter one is moved to a later puzzle.
    - In Spell Cross, no two entries may share a lemma where lemma data exists.
    - No decoy is used twice in the same puzzle.
  - **Across bank and Daily puzzles:**
    - A per-player **seen-word ledger** is stored on the device, and synced if the player has an account.
    - Players can play without an account (CC-ONBOARD-JR D1), so the ledger must work locally.
    - A word served in either mode is excluded from bank puzzles for the next **20 puzzles or 14 days, whichever is longer**, in that language and tier. These numbers are open decision D9.
    - The Daily Puzzle keeps a global no-repeat window of **90 days** per language.
  - **My Words puzzles:**
    - The player chose the words, so repeating them is intended.
    - The layout, decoys, and keystone must still differ from the player's previous puzzle made from the same list. `player_puzzle_counter` in the seed guarantees this.
  - **Pool exhaustion:**
    - If a tier can't supply enough unseen words, relax the window for the least-recently-seen words first. Never show an error.
    - Log each relaxation as a metric.

**F-X4. Hints never give the answer away**

*Allowed hints:*
- Replay the audio.
- Slow replay, per tier.
- Picture (Jr).
- Sense cue.
- Paid definition text (Easy and Medium only).

*Forbidden in any hint, and verified at generation time:*
- The target word itself, in any case or diacritic variant.
- Its lemma or any inflected form.
- Any substring of 4 or more graphemes shared with the target. For targets shorter than 5 graphemes, the limit is 3 or more.
- Text inside a picture.
- A picture file name or alt-text visible to the player that contains the word.

*If no definition passes the filter,* that word offers no definition hint. The generator does not rewrite definitions.

**F-X5. Jr**
- Both modes read the resolved tier from CC-ONBOARD-JR.
- Under-13 profiles only ever get Jr puzzles.

**F-X6. Stats**
- All outcomes write to the base-game confusion matrix and drill queue.
- There are no mode-specific stats tables.

---

## 7. Invariants (every generated puzzle; failure = generator bug)

- **I1 Uniqueness:** each Spell Search target appears in the grid exactly once, counting all 8 directions.
- **I2 Decoy validity:** a decoy is not a word in the E6 dictionary, not any target or its reverse, not on the blocklist, and is exactly one matrix edit away from its target.
- **I3 Clean filler:** no run of 3 or more graphemes in any direction forms a bank word or a blocklisted word, except where placed on purpose. Checking bank words of length 4 or more is enough for bank words; blocklisted words are checked at every length.
- **I4 Answerability:** every Spell Cross collision-group clue satisfies F-C3.
- **I5 Crossing consistency:** each crossing cell holds the same NFC grapheme in both words.
- **I6 Audio:** every target and keystone has resolver audio before generation.
- **I7 Connectivity:** Spell Cross has one connected component with 5 or more words.
- **I8 Determinism:** the same seed and inputs give a byte-identical grid on iOS, Android, and web.
- **I9 No in-puzzle repeats:** targets, decoys, and the keystone are all distinct within a puzzle.
- **I10 No giveaway:** every hint shown passes the F-X4 filter.
- **I11 No shield use:** nothing in either mode reads or writes the shield count.

---

## 8. Phases

- **Phase 0:** §3 census. Wait for Eric's review before Phase A.
- **Phase A:** Spell Search, plus F-X1–F-X6.
  - Spell Search goes first: it's simpler to generate, and its decoy and filler logic is reused later.
- **Phase B:** Spell Cross. It reuses the F-X engine.
- **Phase C:** Daily Puzzle server generation and the global 90-day window.

---

## 9. Acceptance tests (done = all pass in CI)

Wire these into the repo's existing test runner. **If the repo has no property-test harness, stop and ask before adding a dependency.**

1. `wordgrid_property`: 10,000 seeds × each eligible language × each tier. I1–I11 hold at 100%. Any failure prints its seed and inputs.
2. `wordgrid_determinism`: 500 fixed seeds produce identical grid hashes on iOS, Android, and web.
3. `wordgrid_no_repeat`: simulate 60 consecutive bank puzzles per tier for one player. With the pool at or above E4, **zero** words repeat inside the window. With a pool deliberately shrunk, relaxation is logged and no error is shown.
4. `wordgrid_mywords_variation`: the same 10-word list generated 5 times in a row gives 5 distinct layout hashes and 5 distinct keystones (when there are at least 5 eligible keystones).
5. `wordgrid_hint_filter`: a fixture of definitions that deliberately contain targets, stems, and 4-grapheme overlaps, in en/es/ru. **100% are blocked**, and 0 false blocks occur on a clean fixture.
6. `wordgrid_decoy_audit`: across test 1, zero decoys appear in the E6 dictionary.
7. `wordcross_yield`: random 8-word lists from bank tiers. At least 80% produce 5 or more connected words. The rest show the F-C7 fallback, and 0 disconnected grids are rendered.
8. `wordcross_trap_crossings`: at Easy, ≥ 60% of crossings land on trap positions. At Hard, ≤ 10% do.
9. `wordcross_homophones`: every grid containing a known collision pair (en *their/there*, ru *замок* forms) satisfies I4, or the pair is absent.
10. `wordgrid_stats_parity`: a given misspelling produces identical confusion-matrix rows from Spell Cross, from Spell Search Lock It In, and from the base game.
11. `wordgrid_perf`: on-device generation of a 12-word list takes ≤ 300 ms at p95 on the oldest supported device.
12. `wordgrid_jr`: an under-13 profile can reach only Jr puzzles: → ↓ directions, zero decoys, picture clues.
13. `wordgrid_shields`: the shield count is unchanged after 100 simulated sessions containing mistakes.
14. `wordgrid_langs`: ko and zh never appear in the mode language picker, whatever the gate results.

---

## 10. Non-goals (v1)

- No dense American-style grids, and no words that aren't in the source list.
- No text clues except the filtered paid definition hint.
- No personal miss history as a decoy source (D6).
- No mixed-language grids (D5, v2).
- No Korean or Chinese. No right-to-left scripts.
- No new audio path. No changes to base-game scoring, shields, the Climb, or leaderboards.
- No multiplayer, no timed races, no user-authored grids.
- Do not modify CC-SENSE-CUE, CC-RU-ORTHO, or CC-PLAYER-CONTRACT. If one of them needs a change, stop and ask.

---

## 11. Open decisions (Eric)

- **D9:** Repeat window of 20 puzzles or 14 days (whichever is longer), plus 90 days for the Daily Puzzle. Recommend as stated. Change the numbers freely.
- **D10:** Confirm the reading of the new requirement: "same words don't show up" = F-X3, and "hints don't show up" = F-X4 (a hint never reveals the spelling). If you meant *no hints at all*, F-X4 collapses to audio replay only.
- **D11:** Languages that pass E1–E4 and E7 but fail E5 or E6: ship Spell Search with no decoys, or hold them back? Recommend shipping with no decoys, capped at Easy.
- **D12:** Where the modes live in the hub (new tiles, or inside an existing mode row).
