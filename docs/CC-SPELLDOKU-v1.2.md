# CC-SPELLDOKU v1.2 — Word Mode: status and census rulings

**Status:** REVIEW-GATED. The §0 census ran on 2026-09-19
(`docs/census/spelldoku-v1.2-census.md`). Eric, 2026-09-19: "use your
recommendations" — the rulings below are recorded from that. Phase A2 code
was started by Eric on 2026-09-19 (see Phase A2 below).

The spec text Eric sent follows the rulings, unchanged.

## Phase A2 — built (Eric, 2026-09-19: "start Word Mode phase A2")

- **I14 first.** The core (`gen`, `solve`, `canon`, `geo`, the new `symbols`)
  sees each symbol only as sequences of opaque glyph ids; `bind` turns number
  words or bank words into ids and back. A scan test fails the build if the
  core names a word, table, grapheme, spelling or language. The Phase A golden
  digest is recomputed through `bind` in Phase A's own JSON shape and is
  unchanged (host and WebAssembly), so the refactor moved no board.
- **Word Mode** (`wordmode`): 9×9 Medium, Hard and Expert (D12); Jr and Easy
  stay Number Mode (D16). F11 ladder, F12 mixes, F13 gates as ruled (R1–R6),
  and two gates the spec implies: no two words that sound alike on one board
  (the legend is an audio orb), and every word typeable on the in-app keyboard
  (D10). A set that cannot be drawn serves Number Mode (F13 fallback).
- **Screen.** Cells, chips and pencil marks show glyphs; a Number Mode
  "spelled" given shows its glyph in Word Mode (I12). The legend is glyph +
  audio orb in every language (D15 default; no definition cards yet). A
  source badge counts the player's words (D14). A Word Mode misspelling joins
  the existing missed-words queue (D13).
- **Fragments.** Unrelated words rarely line up, so for Word Mode the
  generator cuts fragments where another symbol of the same length shares a
  glyph (`Symbols::aimed`); Number Mode keeps Phase A's search.
- **I12 guard.** A board is redrawn if any fragment would spell another
  symbol's whole word.
- **Not in A2:** 12×12 (Phase A3), definition-card legend (D15 gate).

## Census rulings (Eric, 2026-09-19: "use your recommendations")

| Item | Ruling |
|---|---|
| **R1 Audited bank rows (F13 gate 2, I8, I13)** | Bank words carry no per-row audited flag, so read literally gate 2 admits no word at all. **Ruling:** a bank row counts as eligible when its language is enabled in the registry (`consts::BUILTIN_LANGS`), the language-level status every mode already serves from. When a per-row audit flag lands, gate 2 reads it instead. I13 is unchanged: every symbol is a real bank row and nothing is composed, derived or drafted. |
| **R2 12×12 source (D18; census item 12, the stop trigger)** | No server-side puzzle generation or seed-pack delivery exists anywhere (CC-WORDGRID's Daily is generated on device too; its server Phase C is unbuilt). **Ruling:** 12×12 boards generate on device, like 4×4 to 9×9. I5 makes a device board byte-identical to what a server would build from the same `(seed, config)`, so nothing in I1, I2 or I6 changes. Seed packs move to CC-WORDGRID Phase C and can be adopted later without touching the core. |
| **R3 Initial-glyph rule (F13 gate 1, I11)** | Korean: the first **syllable block** (200–605 distinct per band; by first jamo only 18–19, too few for 12). Chinese: the first **Hanzi** (272–1,393 distinct; pinyin initials give only 25–26). Arabic: the first letter, with the article ال **not** skipped — the glyph shown is the glyph the word starts with; the ال-heavy bands simply redraw more (R5). Japanese: the first kana; rows that begin with a small kana are malformed bank data and are ineligible until the bank is fixed (a separate task). Every other script: the first NFC grapheme cluster, lowercased; Cyrillic ы/ь/ъ never word-initial, as the spec says. |
| **R4 Non-confusable (F13 gate 4)** | CC-WORDGRID has no language-level confusion matrix; its signed E5 ruling is a hand-authored confusion list per language, and only English has one (`config/confusions/en.json`). **Ruling:** gate 4 applies where a list exists — English today — and each new list switches it on for its language. Its word-to-word check reuses `wordsearch::confusion::edits`, made available outside tests. |
| **R5 Drawing a set** | The generator redraws only the word that broke a gate, not the whole set. In the census simulation that policy found a valid set in 100% of 1,000 attempts for every language, size and tier; redrawing the whole set fell below 95% at 12×12 in most languages. |
| **R6 Length cap (F13 gate 3)** | No fragment renderer cap exists today. **Ruling:** 12 graphemes (pinyin measured on the typed form), as the census assumed; the Word Mode legend and fragment rendering are built to that cap. |
| **D11–D16 defaults** | Kept as the spec applies them: tone marks required in Jr (D11, one config value); Word Mode from 9×9 Medium up (D12); misses feed the existing missed-words queue (D13); silent top-up with a source badge (D14); audio-orb legend in every language, definition cards behind the per-language definitions flag (D15); Jr Number Mode only (D16). |

---

# CC-SPELLDOKU v1.2 — SpellDoku game mode

*v1.1: D3 rewritten so every range is playable in DEV_PREVIEW/TestFlight for the game audit; production release gated by range.*
*v1.2: Word Mode added (F10–F13, I11–I14, Phase A2) — a board's nine or twelve symbols may be audited bank words instead of numbers, which is where vocabulary progression comes from. 12×12 Expert confirmed as Word Mode only, server-generated, with a scoped unlock-threshold exception.*

**Status:** REVIEW-GATED. Do not execute until Eric approves this file.
**Surface:** App only (iOS/Android). Never spellgame.net.
**Blast radius:** New mode. Reads shared systems (language registry, audio resolver, in-app keyboard, Jr resolver). Modifies none of them.

---

## Intent

SpellDoku is Sudoku where every cell is committed by spelling a number word in the player's study language.

The design problem: a naive version is a normal Sudoku with a typing tax. The player solves in numerals, then types "seven" fifty times. That is tedious by board two and teaches nothing after the first nine words.

This mode therefore follows one rule: **spelling knowledge must be load-bearing in the solve, not a cost on input.** Three mechanisms make that true:

- **Fragment clues** (F2): partially revealed number words act as logic constraints.
- **Echo givens** (F3): givens are heard, not read.
- **Spelled sums** (F4): cage and sandwich totals push vocabulary from 1–9 up to 45, where number words get genuinely hard (German inversion, French 40s, Spanish accents).
- **Word Mode** (F10–F13): a board's symbols may be audited bank words instead of numbers. This is where vocabulary progression comes from.

**Why Word Mode exists.** A Sudoku does not care what its symbols mean; the constraint is only that they are N distinct things. With numbers as symbols, an Expert board drills the same nine words as an Easy board — the logic hardens but the spelling never does, and one through nine is exhausted in minutes. Making the symbols real bank words gives every board a fresh vocabulary at the player's actual level, strengthens fragment clues (a pattern across nine real words is a far richer deduction than across nine number words), and turns My Words into a puzzle source. It costs nothing in the solver, because of I14.

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
8. **Number-word tables and DEV_PREVIEW.** Report whether any number-word data already exists for any language, and locate the DEV_PREVIEW flag and how builds read it. Do not populate table rows yourself beyond transcribing cited entries from the dictionary authority list (D3); if a language has no cited source for a row, leave the row absent and list it in the report.

9. **Bank depth (Word Mode).** For each enabled language and each difficulty band, report how many audited bank rows exist, the distribution of word-initial glyphs, and — by simulation — the success rate of drawing a valid 9-word set and a valid 12-word set under F13's four gates (1,000 attempts each). Name every (language, size, tier) combination that falls below 95%.
10. **Confusion matrix.** Locate the confusion matrix owned by CC-WORDGRID D6 and report its read API. SpellDoku reads it and never writes it.
11. **My Words and missed-words queue.** Locate both stores and report their read APIs and whether rows carry a difficulty band.
12. **Server generation.** Locate how CC-WORDGRID's server-generated Daily Puzzle is built and delivered. SpellDoku's 12×12 seed packs (D18) reuse that path.

**Stop-and-ask triggers:** item 1 finds a collision; item 3 has no "unavailable" state; item 7 has no reusable app-only mechanism; item 8 finds no DEV_PREVIEW flag a build can read at runtime; item 9 shows no language clearing 95% at 9×9 Word Mode; item 12 finds no reusable server-generation path.

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
- **D17 12×12 Expert unlock threshold (signed).** 12×12 Expert uses N=3 (the Hard threshold) rather than always-spell. This is a **scoped exception to D1 at this size only** — 9×9 Expert still always spells. Rationale: a 12×12 board has ~110 commits; always-spell turns the last stretch into transcription, which is the tedium failure this spec exists to prevent. At N=3 the player still performs 36 full expert-band spellings, more than a 9×9 Expert board's total.
- **D18 12×12 board source (signed).** 12×12 boards are **generated server-side** and delivered as seed packs, reusing CC-WORDGRID D8's server-generated Daily path. Devices reconstruct boards from `(seed, config)`; I5 makes this byte-identical, so nothing in I1/I2/I6 changes. 4×4, 6×6 and 9×9 stay on-device.
- **D19 12×12 is Word Mode only (signed).** Number Mode is **cut** at 12×12. It existed only to drill 10–12, it required the audit-gated 13–45 range's neighbours, and Word Mode covers the size better. A 12×12 board always draws 12 bank words.

### Word Mode decisions (OPEN — defaults applied, all reversible by config)

- **D12 Coexistence (open, default applied).** Word Mode sits **alongside** Number Mode, not replacing it: Jr and Easy are Number Mode, Medium and above are Word Mode, 12×12 is Word Mode only (D19). Number Mode remains the teaching on-ramp.
- **D13 Missed words (open, default applied).** A `MISSPELLED` verdict in Word Mode feeds the **existing** missed-words review queue through its existing path. Do not create a SpellDoku-specific queue.
- **D14 Short lists (open, default applied).** Top-up from the ladder (F11) is **silent**. The board shows a source badge ("4 of your words") and is never blocked or degraded by an empty My Words list.
- **D15 Legend content (open, default applied — flag this one to Eric).** The legend identifies symbols by **audio orb at launch in every language**; the definition-card legend is gated per language by the same registry flag that unlocks definitions under the definitions-dark decision. This lets non-English Word Mode ship with the 15-language release instead of trailing the Fiverr definitions round. **If this reading of definitions-dark is wrong, stop and ask** — it decides whether Word Mode ships with build 56's successor or after the audit.
- **D16 Jr (open, default applied).** Spell Jr is Number Mode only in v1.

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
- Every total produced by the generator, in every enabled language, maps to a cited table row. In a production build, a board needing a non-`audited` row in 13–45 is never served; in a DEV_PREVIEW build it is (D3).

### F5 Twin Systems
**Intent:** Drill languages that have two counting systems.
**Behavior:**
- Korean: boxes alternate between Sino-Korean and native Korean numbers in a checkerboard pattern. Each box shows a system badge. Cells must be spelled in their box's system.
- Japanese: may use the same box pattern for readings where the table marks systems. If the table does not tag systems for Japanese, Twin mode is not offered in Japanese.
- A correct value in the wrong system is `WRONG_SYSTEM`.
**Acceptance:**
- Fixture test: correct value, wrong system → `WRONG_SYSTEM`; correct value, correct system → accepted.

### F6 (removed, see D5)

### F7 Sizes and tiers
| Size | Box | Spell Jr | Standard tiers | Symbol source | Generated |
|---|---|---|---|---|---|
| 4×4 | 2×2 | Easy | Easy | Numbers 1–4 | On device |
| 6×6 | 2×3 | Medium | Easy, Medium | Numbers 1–6 | On device |
| 9×9 | 3×3 | — | Easy, Medium, Hard, Expert | Numbers 1–9 at Easy; bank words at Medium+ (D12) | On device |
| 12×12 | 3×4 | — | Expert | Bank words only (D19) | Server (D18) |

**Behavior:** Jr profiles (resolved via the Jr resolver) see only the Jr column, in Number Mode (D16). No 16×16.
**Acceptance:** A Jr profile is never served a board outside the Jr column (test across 10,000 requests).

### F10 Word Mode
**Intent:** Vocabulary should scale with tier instead of staying frozen at one through nine.
**Behavior:**
- A board has N **symbols** (N = 4, 6, 9 or 12). In Number Mode they are the numbers 1..N. In Word Mode they are N audited bank words.
- The grid renders each symbol as its **initial glyph** — one large character — so the board stays scannable at phone width. A full word is never printed in a cell.
- The **legend** identifies each symbol by audio orb, or by definition card where D15's gate allows. **The legend never shows a spelling**; spelling is what the player owes. Solving a cell means deducing which symbol belongs there, then spelling that symbol's word.
- Fragment clues (F2), echo givens (F3) and hints (F9) all operate on the symbol's word exactly as they do on a number word. In Word Mode a fragment's constraint set is the symbols on this board whose spelling matches the pattern.
- Spell-to-Unlock (F1) counts per symbol, unchanged.
**Acceptance:**
- No surface renders the full spelling of an unsolved symbol (I12).
- The N initial glyphs on a board are pairwise distinct (I11).
- A fragment's candidate set equals a brute-force match of the pattern against this board's N words.

### F11 Word sourcing ladder
**Intent:** Answer "what if the player hasn't saved many words" without ever generating a word.
**Behavior:** Draw eligible words in this order, stopping when N are found:
1. **My Words**, filtered to the tier's difficulty band
2. **Missed-words queue**, same filter
3. **Audited bank**, same filter

Eligibility is F13. The board shows a quiet source badge naming how many symbols came from the player's own words. Top-up is silent (D14).
**Acceptance:**
- A profile with 0 saved words generates valid boards at every size and tier.
- A profile with ≥N eligible saved words gets at least ⌈2N/3⌉ symbols from My Words.

### F12 Mixed-band boards
**Intent:** A tier should set a vocabulary *mix*, not a floor — this is what makes a hard board feel hard without becoming unplayable.
**Behavior:** Each symbol occupies N cells, so the mix is felt across the whole board while leaving the player footholds.

| Tier | Mode | Band mix (9×9) | Band mix (12×12) |
|---|---|---|---|
| Jr | Number | — | — |
| Easy | Number | — | — |
| Medium | Word | 3 medium, 6 easy | — |
| Hard | Word | 5 hard, 4 medium | — |
| Expert | Word | 6 expert, 3 hard | 8 expert, 4 hard |

**Acceptance:** 10,000 generated boards per (size, tier) match the stated mix exactly.

### F13 Eligibility gate
**Intent:** Not every word can serve as a Sudoku symbol.
**Behavior:** A candidate word set of size N is valid only if **all four** hold:
1. **Distinct initial glyphs** — pairwise distinct under the language's glyph rule. For pinyin, the digraphs zh/ch/sh count as distinct from z/c/s and never collide with them. For Cyrillic, ы/ь/ъ never appear word-initially.
2. **Audited** — every word is an audited bank row in a language the registry enables for this player (I8, I13).
3. **Length cap** — no word exceeds the fragment renderer's cap.
4. **Non-confusable** — no pair is confusable under CC-WORDGRID D6's confusion matrix, read-only. For N=12 this is 66 pairwise checks.

**Fallback:** if a (language, size, tier) cannot produce a valid set, serve **Number Mode** there. Never serve a degraded or partial board. At 12×12, where Number Mode is cut (D19), that combination is simply not offered, and the census (§0 item 9) must have named it.
**Acceptance:** 10,000 sets per (language, size, tier), zero gate violations; every fallback path exercised by fixture.

### F8 Generator, grader, and Daily
**Intent:** Variety has to come from logic, not from relabeling the same puzzle.
**Behavior:**
- Lives in the Rust core. Pure function of `(seed, config)` where config = size, tier, rule set, clue-mode mix, language, symbol mode, and (in Word Mode) the ordered symbol set. The core itself is symbol-agnostic (I14).
- 4×4, 6×6 and 9×9 generate on device. 12×12 boards come from server-generated seed packs (D18), reconstructed on device from `(seed, config)` under I5.
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
- **I8 Gating.** A board never contains a word in a language the registry gates off for that player, and never uses a table row the D3 release rule forbids in that build.
- **I9 No leaderboard contact.** SpellDoku code never calls the leaderboard or score-submission service.
- **I10 App only.** The SpellDoku mode is absent from the spellgame.net build.
- **I11 Distinct glyphs.** The N displayed symbol glyphs on a board are pairwise distinct.
- **I12 No free spelling.** No legend, given, badge, or hint level ever renders the full spelling of an unsolved symbol.
- **I13 No generated symbols.** Symbols come only from audited bank rows or the D3 number table. Nothing is composed by code, derived by rule, or LLM-drafted — D3's rule, extended to words.
- **I14 Symbol-agnostic core.** The generator, solver, technique grader and canonical hasher operate on symbol indices 1..N and never see a word, a number word, or a language. Word Mode is a presentation layer bound after generation. This is what keeps Done items 1–4 valid without re-running them.

---

## Phases

**Phase A — Core (executable after approval and §0)**
- Generator, solver, technique grader, canonical-form hashing (F8)
- Sizes 4×4, 6×6, 9×9 (F7)
- Classic rules; clue modes: numeral, word, fragment (F2)
- Spell-to-Unlock input (F1), verdicts (I4), error timing (D2)
- Hints levels 1–2 (F9); level 3 only if the resolver is available
- Daily, unranked, per language (F8)
- Sourced number-word tables per D3, with the `status` field and release rule. Unit-test fixtures stay in tests and are never loadable by any app build.

**Phase A2 — Word Mode (executable after Phase A lands; §0 item 9 must have run)**
- Symbol abstraction and the I14 symbol scan (do this first; it is cheaper before Phase A code settles than after)
- F10 Word Mode rendering, legend, fragment binding
- F11 sourcing ladder, F12 band mixes, F13 eligibility gate and Number Mode fallback
- 9×9 Medium/Hard/Expert switch to Word Mode (D12)

**Phase A3 — 12×12 Expert (executable after A2)**
- Server-side generation and seed-pack delivery (D18), reusing CC-WORDGRID D8's path
- N=3 unlock threshold at this size only (D17)
- Word Mode only; no 12×12 Number Mode (D19)

**Phase B — Depth (buildable after Phase A; production release per language follows D3)**
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
- No generated, composed, or LLM-drafted number words or symbol words (D3, I13).
- No 12×12 Number Mode (D19).
- SpellDoku reads the confusion matrix, My Words, and the missed-words queue. It writes none of them except through the existing missed-words path (D13).
- Word Mode adds no new definition surface; the definition-card legend rides the existing per-language definitions gate (D15).
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
13. **Table gating (D3, I8).**
    - A production build never serves a non-`audited` row in 13–45 for any non-English language, and never serves an English row before Eric's sign-off.
    - A DEV_PREVIEW build serves every cited row in every range and rule set.
    - A row with no source citation never loads in any build.
    - No app build can load unit-test fixtures.
14. **Symbol-agnostic core (I14).** A symbol scan fails the build if any word type, number-word type, or language identifier appears in the generator, solver, technique grader, or canonical hasher.
15. **Eligibility (F13).** 10,000 word sets per (language, size, tier): zero violations of the four gates, including all 66 pairwise confusion checks at N=12.
16. **No free spelling (I12).** Extends Done #9: across every hint level, legend state, given type, and source badge, no surface renders the spelling of an unsolved symbol. 1,000 boards per language.
17. **Cold start (F11).** A profile with 0 saved words generates valid boards at every offered size and tier.
18. **Fallback (F13).** A (language, size, tier) failing the gate serves Number Mode there and never a partial board; at 12×12 that combination is not offered at all. Every fallback branch is covered by fixture.
19. **12×12 viability.** For every enabled language, F13 produces a valid 12-word Expert set in ≥95% of 1,000 attempts, or that language does not offer 12×12 and appears in the census report.
20. **12×12 determinism (D18, I5).** Server-generated seed packs reconstruct byte-identically on iOS and Android, and pass Done #1's uniqueness and grading checks unchanged.
21. **12×12 spelling volume (D17).** A completed 12×12 Expert board requires exactly 36 full spellings (12 symbols × N=3), and a 9×9 Expert board still requires a full spelling on every commit.
22. **Band mixes (F12).** 10,000 boards per (size, tier) match the stated mix exactly.
