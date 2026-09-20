# CC-SPELLDOKU v1.3 — Tier Mode

**Status:** REVIEW-GATED
**Type:** Amendment atop CC-SPELLDOKU v1.2. Does not supersede it.
**Target:** Phase A4 (after A2/A3 Word Mode). Phase B unaffected.
**Execution:** §0 census executable now. F1–F9 blocked on Eric's census review.

---

## Intent

An Expert 9×9 today always-spells but drills the same nine number words as an
Easy board. Word Mode (v1.2) fixed vocabulary *variety*. It did not fix
vocabulary *progression by board tier*, and it does not apply to Number Mode.

Tier Mode makes the digit an index into a difficulty band. Placing a 4 on a Hard
board means spelling a word drawn from Expert. One board delivers up to 81
distinct words instead of 9 repeated ones, and the player can sequence their own
difficulty — solve the cells you can spell first, which turns spelling ability
into a legitimate Sudoku-solving resource.

If you disagree with any signed decision below, stop and ask. Do not infer a
reversal.

---

## Ownership boundaries

Stated up front so this file does not duplicate or contradict existing specs.

| Owned elsewhere | Owner | Tier Mode's relationship |
|---|---|---|
| Generator, solver, grader, uniqueness tests | CC-SPELLDOKU v1.0 | Read-only. Never modified. |
| Word Mode, glyph legend, F11 sourcing ladder, F13 eligibility gates | CC-SPELLDOKU v1.2 | Mutually exclusive with Tier Mode (D-T4). |
| Spell-to-unlock threshold (D1) | CC-SPELLDOKU v1.0 | **Stacks unchanged** (D-T3). Not superseded. |
| Confusion matrix | CC-WORDGRID D6 | Read-only, and only via v1.2 F13's existing call. |
| Audio resolver | CC-BUILD219-FIXES | Consumed as-is. No per-mode audio path. |
| Jr resolver, Jr = Easy+Medium | CC-ONBOARD-JR | Consumed as-is. Tier Mode never widens Jr's span. |
| Keyboards, audio preconditions, servability | CC-PLAYER-CONTRACT | Consumed as-is. |
| Per-language tier calibration | CC-MASTER-PARITY / existing expert-tier calibration | Read-only. Tier Mode never defines a tier band. |
| `LearnerQuery`, missed-words queue | CC-LEARNING-ENGINE-L0 | **Not read.** See I-T5. |

---

## §0 Blocking census (executable now)

Produce a written census before any feature work. Stop and ask on any result
marked **HALT**.

**C1 — Bank depth per (language, tier).** Count audited, game-eligible bank rows
per `(language, tier)` for all 15 languages × {Easy, Medium, Hard, Expert}.
Emit as a table. **HALT** if the tier field is absent, unpopulated, or not the
same field the existing expert-tier calibration uses — Tier Mode must not invent
a second notion of tier.

**C2 — Span feasibility.** For each (language, board tier), evaluate the F5 depth
rule against C1 and emit the availability matrix. Report how many of the 60
(language, board tier) pairs survive. **HALT** if English fails any pair.

**C3 — D1 gate location.** Identify the single code path where the v1.2
spell-to-unlock threshold fires on commit. **HALT** if more than one exists —
that is a single-source-of-truth violation and must be fixed before Tier Mode
stacks onto it.

**C4 — Symbol-agnostic boundary.** Enumerate every symbol the generator, solver
and grader can observe. **HALT** if any word, tier, glyph or audio identifier is
already reachable from them (would mean v1.2's I14 is already breached).

**C5 — Commit/erase semantics.** Document current behaviour when a committed
cell is erased and re-entered: does the v1.2 spell gate re-fire? Needed to
resolve D-T7.

**C6 — Palette surface.** Identify the digit-entry palette view and confirm it
can carry a persistent per-digit badge without reflowing the board (I-T3).

**C7 — Jr board sizes.** Confirm which board sizes the Jr resolver currently
permits. Needed for D-T5.

---

## Features

### F1 — Tier ladder

A board carries a **span** (an ordered set of tiers) determined by its board
tier, and a **ladder** mapping each of the 9 digits to a tier within that span.

| Board tier | Span | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | Words/board |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Easy | E–M | E | M | E | **M** | E | E | M | **M** | **M** | 54 E / 27 M |
| Medium | E–H | E | M | M | **H** | E | E | M | **H** | **H** | 27 / 27 / 27 |
| Hard | E–X | E | M | H | **X** | E | M | H | **X** | **X** | 18 / 18 / 18 / 27 |
| Expert | M–X | M | H | H | **X** | M | M | H | **X** | **X** | 27 M / 27 H / 27 X |

The ladder is a **static table**, not computed. It is the single source of truth
and lives in one registry entry per board tier. Hard is exactly D-T2 as signed.
Expert raises the floor: no Easy word appears on an Expert board.

### F2 — Tier-indexed draw

On commit of digit *d* into a cell, where the v1.2 D1 gate fires, the word served
is drawn from `ladder[boardTier][d]` rather than from the number-word row for
*d*. Draw is uniform over eligible rows in that band, excluding words already
drawn on this board (I-T4).

### F3 — Ladder visibility

Each digit in the entry palette carries a persistent tier badge (colour + label).
The badge is present from board load, before any cell is attempted. The player
can always read the cost of a placement before committing to it.

### F4 — Failure handling

A failed spelling never destroys the logic answer. On failure the cell returns to
empty, the digit remains available, and the next attempt at that digit draws a
**new** word from the same tier. Spelling errors surface immediately in all board
tiers (v1.2 D2 unchanged); logic-error surfacing is unchanged.

### F5 — Availability gate

Tier Mode is offered for a (language, board tier) pair only if, for every tier
*t* in that board's span:

```
bank_depth(language, t) >= 27 * 1.25
```

A pair that fails is not offered — it is absent from the picker, never locked
(CC-TRANSLATE-SCREEN F4 precedent). A language may therefore offer Tier Mode at
Easy and Medium but not Hard or Expert. Number Mode remains the fallback for any
absent pair (v1.2 F13 precedent).

### F6 — Reading B (tier-as-symbol), standard only

A 4×4 board (2×2 boxes) whose four symbols are the four tier badges E/M/H/X.
The cell fills with a badge, not a digit. Uses the Hard span by construction.
Standard game only — see D-T5. Depth rule for B: `bank_depth(language, t) >= 4 * 1.25`
for each of the four tiers.

### F7 — Jr behaviour

Jr uses Reading A on a 4×4 board with the Easy span (E–M) and the Easy row of the
F1 ladder truncated to digits 1–4. Jr never receives Hard or Expert words through
any path in this file.

### F8 — Re-commit accounting

Erasing a correctly-spelled cell and re-entering **the same digit** does not
re-charge a spelling gate for the remainder of that board. Entering a
**different** digit charges normally. Prevents erase-grinding for an easier draw.
Implementation: a per-board `(cell, digit) -> satisfied` set, discarded on board
completion or abandon.

### F9 — Mode selection

`tierMode: off | numbersIndexed | tierSymbols` on the board configuration.
`numbersIndexed` = Reading A. `tierSymbols` = Reading B. Default `off` —
existing SpellDoku behaviour is unchanged unless explicitly selected.

---

## Invariants

- **I-T1 — Symbol-agnostic core.** The generator, solver and grader never observe
  a word, a tier, or a ladder. Extends v1.2 I14. Enforced by symbol scan (A4).
- **I-T2 — No unspellable lock.** A cell whose digit is logically determined is
  never permanently blocked by spelling failure. A board reachable under pure
  Sudoku logic remains completable.
- **I-T3 — Tier legible before commitment.** Every digit's tier is visible from
  board load. No tier is revealed only at the moment of commit.
- **I-T4 — No intra-board repeats.** All words drawn on one board are distinct,
  and no word violates the CC-WORDGRID D9 repeat window.
- **I-T5 — No personal history.** Tier Mode draws from the audited bank only.
  It never reads `LearnerQuery`, the missed-words queue, or My Words. (Extends
  CC-WORDGRID D6's principle: board content is never a mirror of the player's
  failures.)
- **I-T6 — Monotonic difficulty.** For board tiers A < B: `floor(span(B)) >=
  floor(span(A))` and `ceiling(span(B)) >= ceiling(span(A))`. Asserted as a table
  test over the F1 registry, not enforced by prose.
- **I-T7 — Pencil marks are free.** Candidate marks never fire a spelling gate.
  Only commit does.
- **I-T8 — No generated words.** Every served word is an existing audited bank
  row. Nothing is composed, inflected, or LLM-sourced. (v1.2 I13 carried forward.)
- **I-T9 — Single ladder source.** Exactly one ladder table exists in the
  codebase. No `if (boardTier === ...)` tier logic anywhere else.

---

## Decisions

### Signed

- **D-T1 — Both readings ship under one flag.** Reading A (`numbersIndexed`) and
  Reading B (`tierSymbols`), not two hub tiles. No new hub entry: Tier Mode is a
  setting inside SpellDoku.
- **D-T2 — Digits 5–9 cycle.** The ladder assigns 5=E, 6=M, 7=H, 8=X, 9=X at
  Hard; other board tiers follow the F1 table, with the span's top tier always
  occupying digits 4, 8, 9.
- **D-T3 — Tier Mode stacks with D1; it does not supersede it.** D1 governs *how
  often* a gate fires (board tier). Tier Mode governs *which word* it serves
  (digit). Orthogonal axes. Expert therefore gets harder on both: always-spells,
  and never serves an Easy word.
- **D-T4 — Tier Mode and Word Mode are mutually exclusive.** In Word Mode the
  symbols are themselves words; there is nothing to index. Number Mode only.
- **D-T5 — Reading B is standard-only.** Four distinct symbols require four
  distinct tiers; Jr's span is E–M. Jr gets Reading A on a 4×4 (F7). *Applied as
  recommended; config-reversible.*

### Open

- **D-T6 — My Words in Tier Mode.** My Words rows carry no calibrated tier, so
  they cannot be tier-indexed. *Rec: exclude — Tier Mode is bank-only. My Words
  stays served by Word Mode's F11 ladder.*
- **D-T7 — Re-commit accounting scope.** F8 as written keeps the satisfied-set
  per board. *Rec: per board, discarded on completion. Alternative (per session)
  is more forgiving but lets a player farm one easy word across boards.*
- **D-T8 — Badge vocabulary.** Tier badges need player-facing labels that are not
  the internal enum. *Rec: reuse the existing difficulty-picker strings verbatim;
  do not invent a second naming.*
- **D-T9 — Daily Puzzle.** Whether the server-generated Daily Puzzle
  (CC-WORDGRID D8 path, reused by v1.2 D18) ever ships in Tier Mode. *Rec: defer
  to Phase B; A4 is local boards only.*

---

## Done

Phase A4 is complete when all of the following pass.

1. `spelldoku::tier::ladder_table` — table test asserts I-T6 across all four
   board tiers and the exact F1 values.
2. `spelldoku::tier::no_repeats` — property test, 1,000 generated boards per
   (language, board tier) available pair: zero duplicate words within a board,
   zero words outside their declared band.
3. `spelldoku::tier::failure_redraw` — fail one cell 5×, assert 5 distinct words,
   same tier each time, cell still accepts the correct digit (I-T2).
4. `spelldoku::tier::symbol_scan` — scan generator/solver/grader for word, tier,
   ladder and glyph symbols. Must return empty (I-T1).
5. `spelldoku::tier::no_learner_read` — scan the Tier Mode draw path for
   `LearnerQuery`, missed-words and My Words symbols. Must return empty (I-T5).
6. `spelldoku::tier::availability` — generated matrix of 60 (language, board
   tier) pairs matches C2; every absent pair is absent from the picker, none is
   rendered locked.
7. `spelldoku::tier::recommit` — erase and re-enter the same digit: no second
   gate. Different digit: gate fires (F8).
8. `spelldoku::tier::pencil` — candidate marks fire no gate (I-T7).
9. `spelldoku::tier::jr_span` — no Jr path can produce a Hard or Expert word
   (F7). Exhaustive over the Jr ladder.
10. Existing v1.0 uniqueness and grading suites pass unmodified.
11. DEV_PREVIEW: one playable Tier Mode board at each board tier in English,
    plus one in each language passing F5, for the Gig B audit.

---

## Non-goals

- No SpellDoku leaderboard (v1.2 D2 stands).
- No 12×12 Tier Mode (v1.2 D19's spirit; 12×12 stays seed-packed Word Mode).
- No bilingual or mixed-language boards.
- No change to the generator, solver, grader, or any v1.0 uniqueness test.
- No change to D1's thresholds.
- No new audio path — the CC-BUILD219-FIXES resolver is used as-is.
- No definitions surface — Tier Mode serves audio and spelling only, so it is
  unaffected by the definitions-dark gate.
- No spellgame.net build (app only, v1.2 stands).
---

## As built (2026-09-20)

Phase A4 was built after the §0 census
(`docs/census/spelldoku_tier_census.md`). Its two HALTs were **not** signed
off first — Eric said "build tier mode", so they were read as: the ladder
indexes a tier by **bank list identity** (C1), and I-T1 means **nothing
interpretable**, so today's opaque glyph ids may stay (C4). Both readings are
the tool's, and both are still open for him.

| Piece | Where |
|---|---|
| The ladder, spans, availability, the per-board session | `src/spelldoku/tier.rs` — the only ladder in the tree (I-T9) |
| Draw from a band | `spelldoku::wordmode::draw_from` / `depth`, shared with Word Mode so "servable row" has one definition |
| Badges, prompt, commit, redraw, Reading B labels | `src/spelldoku_ui.rs` (`tier_digit`, `speak_prompt`, `submit_typed`, `cell_label`) |
| The setting (F9) and the strip | `index.html` — `#sdTier`, `#sdTiers`, `--band-*` tokens |
| Done #1–#9 | `src/spelldoku/tier.rs` tests + `the_core_is_symbol_agnostic` in `wordmode_tests.rs` |
| Done #10 | the existing suites, unmodified |
| Done #11 | `docs/census/spelldoku_tier_preview.md`, regenerated by `src/spelldoku_tier_preview.rs` |
| On the screen | `tests/e2e/specs/spelldoku-tier.mjs` |

**I-T4 is wired in both halves.** No word repeats within a board, and a board's
words enter the player's CC-WORDGRID D9 repeat window — the ledger Word Search
and Spell Cross already share (`src/wordsearch/ledger.rs`,
`spell_wordgrid_seen_v1`), keyed `lang:band`. One board counts as one puzzle
however many bands it spans (`Ledger::record_many`), and the words are recorded
when the board ends, not as they are drawn, so an abandoned board does not
spend them. A band with nothing outside the window serves its longest-ago word
and counts a relaxation, as the ledger's own selector does (F-X3).

One consequence to know about: an English **Easy** board spends 54 Easy words,
so about 14 of them fill the 764-word Easy bank, and from there every mode
sharing that window — Word Search and Spell Cross included — is drawing on the
relax path, which is least-recently-served rotation rather than an error. The
other board tiers spend 27 of a band at a time and saturate in ~27 boards.
**Eric kept the window shared (2026-09-20)**, told what it costs: D9 says a word
served in either mode counts, and the relax path is least-recently-served
rotation, not an error. Namespacing SpellDoku's key (`sd:en:easy`) would undo
the sharing in one line if that ever needs revisiting.

Three things worth knowing:

- **F1's Easy row is inconsistent in the spec.** The table prints
  `E M E M E E M M M` (4 Easy digits, 5 Medium) while the same row's totals say
  "54 E / 27 M" (6 Easy, 3 Medium), which is also what D-T2 describes — the
  span's top tier on digits 4, 8 and 9. The totals and D-T2 agree with each
  other, so the code follows them: Medium on 4, 8, 9 and Easy elsewhere. One
  line in `LADDER` reverses it if the printed row was meant literally.
- **Typing a number word commits nothing while a ladder is on the board.** F2
  says the ladder sets the price of a digit; leaving the old path open would
  have let a player pay "three" for a digit the badge priced at Expert.
- **Reading B degrades to Reading A** on any board that is not a standard 4×4
  — including every Jr board (D-T5, enforced in `Session::new`) — rather than
  refusing the setting.

Still open, unchanged by this work: D-T6, D-T7 (built as recommended: per
board), D-T9 — plus the C1 and C4 readings above and the F1 Easy row.
