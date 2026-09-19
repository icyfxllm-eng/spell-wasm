# CC-SPELLDOKU v1 — SpellDoku game mode

**Status:** REVIEW-GATED. Approved by Eric, 2026-09-18: "implement english
immediately", then "All languages have to be audited so they all have to be
playable if the game mode fits the languages".
**Surface:** App only (iOS/Android). Never spellgame.net.
**Blast radius:** New mode. Reads shared systems (language registry, audio resolver, in-app keyboard, Jr resolver). Modifies none of them.

> **§0 census run 2026-09-18 — see `docs/census/spelldoku-census.md`.**

## Phase A build notes (2026-09-18, English)

Each is a place where the repo and this file met something the file did not
say; flagged, not decided silently.

- **F2's example does not match its own display.** `_ _ _ e` is four letters,
  which in English fits five and nine; the example's {1, 3, 5, 9} is the set of
  words that END in e, at any length. Built as displayed: a fragment shows the
  word's length and some of its letters, and its constraint is the numbers
  matching letter for letter. Every fragment reveals at least one letter -- an
  all-hidden pattern would test length, not spelling.
- **The English table has 0–20, 30 and 40.** D3 forbids composing number words
  by rule and needs a citation per row. Compound numbers (21–29, 31–39, 41–45)
  each need their own entry, and the English keyboard has no hyphen key, so they
  are Phase B. Authority: Merriam-Webster, pending your confirmation.
- **Every language plays (Eric, 2026-09-18: "All languages have to be audited
  so they all have to be playable if the game mode fits the languages").**
  Auditors audit by playing, so the release rule is now: every language, English
  included, plays its `sourced` 0–12 rows in every build; 13–45 (Phase B) still
  waits for `audited` in production. This supersedes D3's "English: served once
  Eric signs off" line. A TestFlight binary is a production binary here, so this
  is what makes the audit possible at all.
- **Fourteen languages sourced from Wiktionary, never written.**
  `tools/numbers/source_numbers.py` reads English Wiktionary's cardinal-number
  translation tables, applies the selection rules in its header, and keeps a row
  only when the word's own entry has a section for that language; every row cites
  both pages. The auditors' checklist is `docs/review/spelldoku-number-sourcing.md`.
  Rules worth an auditor's eye: Japanese keeps the numeral readings the table
  itself romanizes (よん/し, なな/しち, きゅう/く -- exactly D4's examples); Korean
  accepts the native and the Sino-Korean word (Twin Systems stays Phase B);
  Russian stress marks and Arabic short vowels are dropped, since neither is
  written or typed. Korean 0 came out as 제로 (a loanword) beside 영; 0 is not
  used before Phase B.
- **"If the mode fits the language" is tested, not assumed.** For 1–9, every
  language has at least one spelling its own keyboard can type through the same
  input the screen uses: tone numbers for Mandarin, the tone row for Vietnamese,
  jamo composition for Korean, and each layout's accented alternates. All
  fifteen pass.
- **Hindi stops at Medium.** Hard and Expert need a necessary fragment, and
  Hindi's words for 1–9 share no letter in the same place, so no partly hidden
  word can fit two numbers. The tiers a language offers are computed from its
  own table.
- **11 and 12 contain spaces or hyphens** in Vietnamese, Swahili, Filipino and
  Mandarin, which the keyboards cannot type. They are needed only for 12×12,
  which is Phase B.
- **The Daily is 9×9 Medium, and 6×6 Medium for Spell Jr.** The spec leaves the
  size open.
- **Repeat detection is an invariant, not a full canonical form.** Every
  relabeled or permuted copy hashes equal (tested); a rare collision between
  genuinely different boards only makes the generator skip one. Transposition
  counts only where the boxes are square, since it is not a symmetry of 6×6.
- **Hints:** level 3 plays the word through the one resolver; if the resolver
  fails, level 3 is not offered again on that board.
- **Determinism across platforms** is proven by a digest over fixed seeds: the
  host test pins it and the browser test requires the wasm build to reproduce it.
  iOS and Android run that same wasm.

---

## Intent

SpellDoku is Sudoku where every cell is committed by spelling a number word in the player's study language.

The design problem: a naive version is a normal Sudoku with a typing tax. The player solves in numerals, then types "seven" fifty times. That is tedious by board two and teaches nothing after the first nine words.

This mode therefore follows one rule: **spelling knowledge must be load-bearing in the solve, not a cost on input.** Three mechanisms make that true:

- **Fragment clues** (F2): partially revealed number words act as logic constraints.
- **Echo givens** (F3): givens are heard, not read.
- **Spelled sums** (F4): cage and sandwich totals push vocabulary from 1–9 up to 45, where number words get genuinely hard (German inversion, French 40s, Spanish accents).

When this spec is silent, choose whatever keeps spelling load-bearing and keeps input un-tedious. If those two conflict, stop and ask.

---

## §0 Census (blocking — run before any code, report results, then stop and ask if any check fails)

Produce a short report answering each item. Do not write mode code until Eric has seen it.

1. **Name collision.** Confirm no existing mode, flag, route, or asset is named `spelldoku` / `SpellDoku` (case-insensitive search across the repo).
2. **Registry.** Locate the per-language registry and the gate that decides which languages a player may see. Report the path and the gate function.
3. **Audio resolver.** Locate the single audio resolver from CC-BUILD219-FIXES. Report whether it exposes a "no audio for this item" state that callers can branch on.
4. **Keyboard.** Locate the in-app script keyboard from CC-PLAYER-CONTRACT. Report which study languages it currently covers.
5. **Jr resolver.** Locate the Jr difficulty resolver from CC-ONBOARD-JR. Report its API.
6. **Leaderboard service.** Locate every entry point to the leaderboard/score-submission service (needed for the negative test in Done #10).
7. **Web build mode registration.** Locate how modes register (or are excluded) in the spellgame.net build, and how Spell Picture is kept app-only. SpellDoku must use the same mechanism.
8. **Number-word tables.** Report whether any audited number-word data (1–9, 0–45, 0–12) already exists for any language. Expect: none. Do not create it (see D3).

**Stop-and-ask triggers:** item 1 finds a collision; item 3 has no "unavailable" state; item 7 has no reusable app-only mechanism.

---

## Decisions (all signed by Eric unless marked)

- **D1 Unlock threshold (signed).** For each number on a board, after N correct full spellings that number becomes a tap chip for the rest of that board. N = Easy 1, Medium 2, Hard 3. On Expert, every commit is spelled and N never triggers.
- **D2 Error feedback timing (signed).**
  - Spelling errors (`MISSPELLED`, `WRONG_SYSTEM`) show immediately at every tier.
  - Logic errors (`WRONG_VALUE`): immediately on Easy and Medium. On Hard and Expert, shown only when the player uses **Check Board**, which is limited to 3 uses per board.
- **D3 Number words come from a sourced table, released by range (signed, v1.1).** Each language has one number-word table covering 0–45. Number words are **never** composed by code, generated by an LLM, or derived by rule. French 70–99 and Hindi 1–100 are irregular, and composition fails silently.
  - **Sourcing:** every row cites its entry in the per-language dictionary authority list (source name + locator). A row without a citation is invalid and never loads in any build.
  - **Cross-check:** the dictionary-matcher runs over the table as a flag-only pre-check. It never approves, edits, or removes a row.
  - **Row status:** each row carries `status: sourced | audited`. English rows become `audited` on Eric's sign-off; all other languages become `audited` only through the Fiverr audit ingest.
  - **Release rule:**

    | Range | DEV_PREVIEW / TestFlight builds | Production builds |
    |---|---|---|
    | English, all rows | Served | Served once Eric signs off |
    | Other languages, 0–12 | Served | Served when `sourced` or `audited` |
    | Other languages, 13–45 | Served | Served only when `audited` |

  - **Why:** the Gig B game audit happens by playing TestFlight, so every range must be playable in DEV_PREVIEW or it can never be audited. Production exposure of the error-prone range (13–45) still waits on the audit.
  - **Audit path:** number-word rows join the Gig A spreadsheet as ordinary rows; Gig B auditors play every SpellDoku rule set in context. Ingest uses the existing locked-spreadsheet audit ingest. Do not build a new write path.
  - A board is served in a build only if every row it needs is servable in that build under the table above. Never substitute, fall back, or partially render.
- **D4 Alternate readings (signed).** Every audited alternate spelling of a number is accepted (Japanese yon/shi, nana/shichi, kyū/ku). Exception: in Twin Systems mode (F5), the cell's assigned system is required.
- **D5 No bilingual or mixed-language boards (signed).** Every board is one language. Removed from scope, not scheduled.
- **D6 No shields (signed).** SpellDoku never consumes or awards Climb shields.
- **D7 No leaderboard (signed).** SpellDoku has no ranking, no score submission, and no leaderboard UI. The Daily SpellDoku exists but is unranked (see F8).
- **D8 Chinese tone marks required at every tier (signed).**
  - Cells take citation tones only (yī, never sandhi yí/yì).
  - A correct value with a wrong or missing tone is `MISSPELLED`.
  - Fragment patterns (F2) match on toned forms, so tones narrow candidates.
- **D9 App only (signed).** SpellDoku never registers on spellgame.net.
- **D10 Input (signed).** Input uses the in-app script keyboard from CC-PLAYER-CONTRACT. No platform IME.
- **D11 Tone marks in Spell Jr (OPEN, default applied).** Default: yes, Jr players also need tone marks, for consistency with D8. **Implement the default behind a single config value** so Eric can reverse it without code changes. Do not guess further.

If anything below appears to contradict a decision above, the decision wins. If a decision appears to contradict another CC file, stop and ask.

---

## Features

### F1 Spell-to-Unlock input
**Intent:** Spelling should be real practice without typing the same word fifty times.
**Behavior:**
- Tapping an empty cell opens the in-app keyboard. A commit is a full spelling of a number word.
- Per board and per number, count correct full spellings. When the count reaches N (D1), show that number as a tap chip in the input tray. Tapping the chip commits that number to the selected cell.
- Pencil marks use digits and never open the keyboard. Notes are not commits.
- Matching: NFC-normalize input and the table entry, then compare. Case-insensitive where the script has case.
**Acceptance:**
- On Easy 9×9, the number of full spellings required to complete a board is at most 9.
- On Expert, a chip never appears.
- Entering or clearing a pencil mark never opens the keyboard.

### F2 Fragment Clues
**Intent:** Knowing how a number word is spelled becomes a solving tool.
**Behavior:**
- A fragment given shows a number word with some letters hidden, e.g. `_ _ _ e`.
- Its constraint is the set of numbers whose accepted spellings (D4) in that language match the pattern, including tone marks for Chinese (D8). For English `_ _ _ e` that is {1, 3, 5, 9}.
- Fragments are constraints in the generator, solver, and grader, not decoration.
- Tiers: none on Easy; optional on Medium; on Hard and Expert, each board includes at least one fragment that is necessary for a unique solution.
**Acceptance:**
- For every fragment on every generated test board, the solver's candidate set equals a brute-force match of the pattern against the audited table.
- On every Hard/Expert test board, replacing at least one fragment with an unconstrained cell makes the puzzle non-unique.

### F3 Echo Givens
**Intent:** Match SpellGame's core loop, hear it and spell it.
**Behavior:**
- An echo given renders as a blank orb. Tapping it plays the number word via the single audio resolver.
- Expert: natural speed only, no slow replay.
- If the resolver reports audio unavailable for a language, echo givens are not generated for that language. Never fall back to a system voice.
**Acceptance:**
- Every echo given's audio request goes through the resolver (assert via test spy).
- For a language with audio forced unavailable, 1,000 generated boards contain zero echo givens.

### F4 Spelled Sums
**Intent:** Push vocabulary beyond 1–9, into the numbers that are actually hard to spell.
**Behavior:**
- Rule sets: Killer (cage totals), Sandwich (sum between the 1 and the 9 in a row/column), Arrow, Little Killer.
- Totals display as number words from the audited table (range 0–45).
- Optional bonus commit: the player may spell a cage total. Doing so is recorded in local stats only.
**Acceptance:**
- Every total produced by the generator, in every enabled language, maps to an audited table row. A board needing an unaudited row is never served (D3).

### F5 Twin Systems
**Intent:** Drill languages that have two counting systems.
**Behavior:**
- Korean: boxes alternate between Sino-Korean and native Korean numbers in a checkerboard pattern. Each box shows a system badge. Cells must be spelled in their box's system.
- Japanese: may use the same box pattern for readings where the audited table marks systems. If the audited table does not tag systems for Japanese, Twin mode is not offered in Japanese.
- A correct value in the wrong system is `WRONG_SYSTEM`.
**Acceptance:**
- Fixture test: correct value, wrong system → `WRONG_SYSTEM`; correct value, correct system → accepted.

### F6 (removed, see D5)

### F7 Sizes and tiers
| Size | Box | Spell Jr | Standard tiers | Needs audited rows |
|---|---|---|---|---|
| 4×4 | 2×2 | Easy | Easy | 1–4 |
| 6×6 | 2×3 | Medium | Easy, Medium | 1–6 |
| 9×9 | 3×3 | — | Easy, Medium, Hard, Expert | 1–9 (0–45 with F4) |
| 12×12 | 3×4 | — | Expert | 1–12 |

**Behavior:** Jr profiles (resolved via the Jr resolver) see only the Jr column. No 16×16.
**Acceptance:** A Jr profile is never served a board outside the Jr column (test across 10,000 requests).

### F8 Generator, grader, and Daily
**Intent:** Variety has to come from logic, not from relabeling the same puzzle.
**Behavior:**
- Lives in the Rust core. Pure function of `(seed, config)` where config = size, tier, rule set, clue-mode mix, language.
- Difficulty is the hardest human technique the solve requires, from a ladder (singles → hidden/naked pairs and triples → pointing/box-line → X-wing/swordfish → chains). Do not grade by backtracking guess count.
- Clue modes per cell: numeral, word, fragment (F2), echo (F3).
- **Daily SpellDoku:** date-seeded per language, unranked (D7). Same date and language produce the same board for everyone.
- **Repeat avoidance:** compute a canonical form for each board, invariant under digit relabeling and the standard symmetry group (row/column permutations within bands, band permutations, transposition). Store served canonical hashes per player locally.
**Acceptance:** covered by I1, I2, I5, I6.

### F9 Hints that never reveal letters
**Intent:** Hints must not give the answer away. In SpellDoku, the spelling is the answer.
**Behavior:** Three escalating levels:
1. Highlight a solvable cell.
2. Name the technique that solves it.
3. Play the number word's audio via the resolver. The player still spells it. If audio is unavailable, level 3 is not offered.
**Acceptance:** No hint level renders any letter of the target word (assert across all three levels on 1,000 boards per language).

---

## Invariants

- **I1 Uniqueness.** Every served board has exactly one solution, proven by the solver with fragments and sums treated as constraints.
- **I2 No guessing.** Every served board is solvable using only techniques at or below its tier on the ladder.
- **I3 Accept all valid spellings.** Every audited spelling of a number is accepted, except where F5 requires a system.
- **I4 Separate verdicts.** `MISSPELLED`, `WRONG_VALUE`, and `WRONG_SYSTEM` are distinct outcomes and are recorded separately. A spelling error never counts as a logic error or the reverse.
- **I5 Determinism.** The same `(seed, config)` produces a byte-identical board on iOS, Android, and any test harness.
- **I6 No isomorphic repeats.** No player is served the same canonical board twice within the repeat window. Default window: 365 days, as one config value.
- **I7 NFC.** All input and table entries are NFC-normalized before comparison.
- **I8 Gating.** A board never contains a word in a language the registry gates off for that player, and never uses a table row that is unaudited (D3).
- **I9 No leaderboard contact.** SpellDoku code never calls the leaderboard or score-submission service.
- **I10 App only.** The SpellDoku mode is absent from the spellgame.net build.

---

## Phases

**Phase A — Core (executable after approval and §0)**
- Generator, solver, technique grader, canonical-form hashing (F8)
- Sizes 4×4, 6×6, 9×9 (F7)
- Classic rules; clue modes: numeral, word, fragment (F2)
- Spell-to-Unlock input (F1), verdicts (I4), error timing (D2)
- Hints levels 1–2 (F9); level 3 only if the resolver is available
- Daily, unranked, per language (F8)
- Serves only languages with audited 1–9 (or 1–4/1–6 for smaller sizes) rows. If no language has audited rows yet, ship behind a dev flag using a clearly labeled **test fixture** table in tests only. Fixtures must never be loadable by a production build.

**Phase B — Depth (blocked on audited 0–45 rows per language)**
- Spelled Sums (F4)
- Echo givens (F3)
- Twin Systems (F5)
- 12×12 Expert (F7)

---

## Non-goals and constraints

- No bilingual or mixed-language boards (D5).
- No leaderboard, ranking, or score submission (D7, I9).
- No web availability (D9, I10).
- No 16×16.
- No generated, composed, or LLM-drafted number words (D3).
- No new audio path; use the resolver (F3, F9).
- No shield changes (D6).
- Do not modify base-game scoring, The Climb, shields, the leaderboard service, the registry, the keyboard, the audio resolver, or the Jr resolver. If a change seems necessary, stop and ask.
- No share cards for SpellDoku in v1.

---

## Done (every item must pass)

1. **Uniqueness and grading.** For 10,000 seeds per (size × rule set × tier) in Phase A: 100% unique (I1) and 100% graded tier equals requested tier (I2).
2. **Generator sanity.** Enumerating all 4×4 solution grids returns exactly 288.
3. **Determinism.** The same `(seed, config)` hashes equal across iOS, Android, and the Rust test harness (I5).
4. **No repeats.** A simulated 365-day player history has zero canonical-hash repeats (I6), including relabeled and permuted variants of served boards.
5. **Fragments.** Done #1's boards: every fragment's candidate set equals brute-force matching against the table (F2), and every Hard/Expert board has a necessary fragment.
6. **Verdicts.** Fixture tests produce all three verdicts: `MISSPELLED`, `WRONG_VALUE`, `WRONG_SYSTEM` (I4). `WRONG_SYSTEM` is Phase B.
7. **Chinese tones.** Every Chinese table entry carries a tone mark. An untoned or wrong-toned answer is `MISSPELLED` at every tier, including Jr while D11's default holds (D8).
8. **Unlock threshold.** Easy completes with at most 9 full spellings on 9×9; Expert never shows a chip (F1, D1).
9. **Hints.** No hint level renders a letter of the target word (F9).
10. **No leaderboard.** A test fails if any SpellDoku code path calls the leaderboard or score service (I9).
11. **App only.** A test fails if the web build registers the SpellDoku mode (I10).
12. **Jr gating.** A Jr profile never receives a board outside the Jr column (F7).
13. **Table gating.** A language or rule set with unaudited required rows is never served, and production builds cannot load fixture tables (D3, I8).
