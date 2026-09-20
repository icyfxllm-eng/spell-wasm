# CC-SPELLDOKU v1.3 §0 census — Tier Mode

Run 2026-09-20 against `main` (a5e59245), before any feature work. C1 and C2 are
measured by `cargo test --lib spelldoku_tier_census -- --ignored`
(`src/spelldoku_tier_census.rs`), which reads the banks through `words::tier_for`
— the accessor the game serves from — and applies the per-word gates Word Mode
already applies at draw time. C3–C7 are read from the code and cited by line.

**Three HALTs fired: C1, C4, and a C3 finding that is not the one C3 asks about.**
F1–F9 stay blocked.

---

## C1 — Bank depth per (language, tier) — **HALT**

**There is no tier field, and no audited flag.** A word's tier is which of the
four bank lists it appears in — `words::tier_for(lang, tier)` (`src/words.rs:6288`)
over `assets/words/<lang>/<tier>.txt`, compiled into `src/word_data.rs`. Nothing
stores a tier on a row. The only per-word tier derivation in the codebase is a
linear scan of all four lists (`src/spelldoku/wordmode.rs:200` `band_of`, and
`src/translate_screen.rs:97`), and the only persisted per-word tier is on a miss
record (`src/model.rs:111`), written from whatever the caller knew.

The expert-tier calibration reads **the same lists**: `tools/difficulty-score/score.py:35`
loads `assets/words/<lang>/<tier>.txt`, and `tools/tierhealth/diagnose.py` consumes
a dump of `tier_for` itself. So there is **one** notion of tier, not two — but it
is a list identity, not a field, and it is not queryable per word at runtime
without an O(bank) scan.

**Why this is a HALT as written:** C1 says halt if the tier field is absent. It is
absent. The question for you is whether "the digit indexes a tier" may be built on
list identity (cheap: the ladder picks a list and draws from it, which is exactly
what F2 needs) or whether it requires a real per-row tier first.

**"Audited" also does not exist.** No bank row carries an audit flag; the only
`Status::Audited` in the tree is on number-word rows (`src/spelldoku/table.rs:8`).
v1.2 already hit this and reinterpreted its gate 2 at the language level
(`docs/CC-SPELLDOKU-v1.2.md:67`, R1). The counts below are therefore
**game-eligible**, not audited: 2..=12 graphemes, keyboard-typeable, not blocked
by the profanity list, and a legal initial glyph — the gates in
`wordmode::candidate` (`src/spelldoku/wordmode.rs:189`).

| lang | easy | medium | hard | expert |
|---|---:|---:|---:|---:|
| en | 764 | 800 | 798 | 741 |
| es | 264 | 873 | 1920 | 2800 |
| fr | 278 | 886 | 1928 | 2828 |
| de | 213 | 926 | 1861 | 2650 |
| pt | 269 | 902 | 1932 | 2825 |
| pl | 267 | 957 | 1933 | 2808 |
| vi | 244 | 755 | 1159 | 1839 |
| ko | 215 | 932 | 1891 | 2867 |
| ja | 208 | 957 | 1838 | 2752 |
| fil | 244 | 701 | 1154 | 1889 |
| zh | 295 | 932 | 1891 | 2806 |
| ru | 267 | 965 | 1935 | 2805 |
| ar | 290 | 968 | 1987 | 2990 |
| sw | 261 | 702 | 1171 | 671 |
| hi | 290 | 781 | 800 | 800 |

What the gates removed, worth knowing before F5 is trusted: German Easy loses 59
rows to keyboard typeability (213 of 272); Korean Easy and Japanese Easy lose 77
and 43 to the 12-grapheme cap; Japanese Expert loses 214 to the initial-glyph rule
(no display form). The full per-gate table is in the test output.

## C2 — Span feasibility — pass

F5 needs ≥ 34 eligible rows (27 × 1.25, rounded up) in **every** tier of a board's
span; Reading B needs ≥ 5 in each of the four.

**All 60 (language, board tier) pairs survive, and English survives every pair.**
The binding tier is Easy everywhere (the shallowest list): the worst span minimum
is Japanese and German at 208 and 213, six times the requirement. Reading B
likewise passes for all 15 languages.

So F5 as written never excludes anything today. If the intent was that Tier Mode
should be *harder* to qualify for than Word Mode, the threshold is not doing that
work: 27 × 1.25 is a floor for one board's 27 cells, not for variety across
sessions.

## C3 — D1 gate location — one table, two commit paths

The threshold itself is single-source: `play::unlock_after` /
`unlock_after_on` (`src/spelldoku/play.rs:38-55`), consumed only by
`Unlocks::new`/`for_board` (`:65-71`), built once per board at
`src/spelldoku_ui.rs:179`.

The gate **fires** structurally, through two commit entry points into one
function — `commit_value(v, spelled, verdict)` (`src/spelldoku_ui.rs:487`):

- `submit_typed()` (`:538`) → `commit_value(v, true, ...)` (`:570`) — the spelled
  path, the only one that advances the counter (`:498-500`).
- the chip path (`:779-791`) → `commit_value(v, false, ...)`, refused unless
  `g.unlocks.chip(v)` (`:784`).

That is one code path in the sense C3 asks about, so **no HALT on C3 as written**.
But the census turned up a defect next door, which C5 details and which F8 would
otherwise inherit.

## C4 — Symbol-agnostic boundary — **HALT**

What the generator, solver and grader can observe:

| Component | Signature | Sees |
|---|---|---|
| `gen::generate` | `(seed, cfg, symbols: &Symbols)` (`src/spelldoku/gen.rs:181`) | digits `u8`, `Tier`, `Size`, **and `Symbols`** |
| `solve::count_solutions` / `grade` / `next_step` | `(&Geo, &[u8], &[u16])` (`src/spelldoku/solve.rs:38,352,383`) | digits and bitmasks only |
| `canon::hash` | `(&Puzzle)` (`src/spelldoku/canon.rs:92`) | digits only |
| `Symbols` | `forms: Vec<Vec<Vec<u32>>>` (`src/spelldoku/symbols.rs:11`) | **glyph ids** |

`Clue::Fragment(Vec<Option<u32>>)` (`gen.rs:59`) carries glyph ids into the
puzzle, and the generator consults `Symbols::fragment_set` / `agreements` while
digging. So **a glyph identifier is already reachable from the generator** —
C4's halt condition, read literally.

It is reachable by design: the ids are opaque `u32` with no text, language or
tier attached, which is what v1.2's I14 intends (`src/spelldoku/symbols.rs:1-9`).
Your call is whether I-T1 means "no glyph identifier at all" — which would
require unpicking fragment clues — or "nothing interpretable", which is today's
state and needs only the scan widened.

**The existing scan has a hole either way.** `the_core_is_symbol_agnostic`
(`src/spelldoku/wordmode_tests.rs:258`) covers `gen.rs`, `solve.rs`, `canon.rs`,
`symbols.rs`, `geo.rs`. It does **not** cover `pack.rs`, `play.rs`, `table.rs` or
`bind.rs`. A tier field added to `pack.rs` or `play.rs` would not trip it, and
Tier Mode's ladder is exactly the kind of thing that would land there.

## C5 — Commit / erase semantics — **defect found**

**There is no erase.** The click router (`src/spelldoku_ui.rs:739`) handles cell,
key, backspace, go, chip, pencil and say; backspace edits the typed buffer
(`:770`), not the board. There is no clear-cell control in `index.html:2836-2841`.

What exists instead: re-selecting a non-clue cell clears only the typed buffer
(`:751-760`), and committing again **overwrites** `g.entries[i]` (`:495`). So
today, re-spelling a cell that is already correct:

- re-fires the spell gate, and
- **increments the unlock counter again** (`:498-500`).

A player could therefore farm chips by spelling the same easy cell repeatedly.
**Fixed 2026-09-20** (Eric): `Unlocks` now credits a (cell, value) once, so
re-spelling a cell that is already right earns nothing, while a different value
in the same cell or the same value elsewhere still counts. Build 233 and earlier
carry the defect. F8 assumes this hole is closed;
it is not, and D-T7 should be decided knowing F8 is a **fix**, not a refinement.

There is no per-cell "already spelled" memory to build on: `Unlocks`
(`src/spelldoku/play.rs:57`) counts per symbol value, is created fresh per board
(`spelldoku_ui.rs:179`) and is never persisted.

## C6 — Palette surface — feasible, but the surface does not exist yet

There is **no persistent digit palette** to hang a badge on:

- `#sdChips` (`index.html:2833`) renders only **unlocked** values
  (`spelldoku_ui.rs:399-408`), so before the first unlock it is empty.
- Digit buttons `1..=n` exist **only in pencil mode** (`spelldoku_ui.rs:415-419`);
  otherwise `#sdKeys` is the letter keyboard (`:420-451`).

So I-T3 ("tier legible from board load") needs a **new always-on element**, not a
badge on an existing one.

Adding it will not reflow the board: `.sd-grid` sizes off `--sd-n` with
`aspect-ratio:1` inside `max-width:520px` (`index.html:1327-1331`), and the tray
is a separate flex column below it (`.sd-tray`, `:1343`). A badge inside
`.sd-chip` would need `position:relative` added (`:1345` has none).

## C7 — Jr board sizes — 4×4 Easy and 6×6 Medium, declared twice

`gen::jr_allowed` (`src/spelldoku/gen.rs:98`) permits exactly `(4, Easy)` and
`(6, Medium)`, dispatched by `play::allowed` (`play.rs:106`) and applied at
`spelldoku_ui.rs:109`. A 10,000-case test holds the line
(`src/spelldoku/tests.rs:231`). The Jr Daily is hardcoded to 6×6 Medium
(`spelldoku_ui.rs:719`).

F7 (Jr = Reading A, 4×4, Easy span) fits inside that, and F6's Reading B is
already excluded from Jr by D-T5.

**But the Jr ceiling is stated in two places that do not agree in shape.**
`config/modes.json:342` declares `"juniorPolicy": "ceiling:medium"`, read by
`experience::allowed_tiers` (`src/experience.rs:86`) — which would permit 9×9
Medium. SpellDoku never reads it: `grep experience:: src/spelldoku*` returns
nothing. Two sources, no test tying them together. I-T9 forbids exactly this
shape for the ladder; the Jr ceiling already has it.

---

## What this census recommends you decide

1. **C1 (HALT):** may the ladder index a tier by *bank list identity*, given no
   per-row tier field exists and the calibration uses the same lists? If yes, F2
   is cheap. If no, a per-row tier must land first, and that is a bank-data
   project, not a SpellDoku one.
2. **C4 (HALT):** does I-T1 forbid opaque glyph ids reaching the generator (they
   do today, via `Clue::Fragment`), or only interpretable identifiers? Either
   way, widen the I14 scan to `pack.rs`, `play.rs`, `table.rs`, `bind.rs` before
   the ladder is written.
3. **C5 (defect):** the unlock counter can be farmed by re-spelling one cell.
   Fix it independently of Tier Mode, or fold it into F8 and say so.
4. **C2 (no-op gate):** F5 excludes nothing at 27 × 1.25. Raise it, or accept
   that Tier Mode is offered everywhere Word Mode is.
5. **C6:** I-T3 needs a new always-on tier strip, not a badge on the chips.
6. **C7:** the Jr ceiling lives in both `gen::jr_allowed` and `modes.json`.
   Fix before adding a third tier table.

## Not measured

Nothing in F1–F9 was built. No generator, solver, grader or v1.0 test was touched.
The census test is `#[ignore]`d and reads only.

---

## Built on two ASSUMED readings — both still need Eric (2026-09-20)

Phase A4 was built on "build tier mode" without the HALTs being answered, so
the two readings below are the tool's, NOT Eric's. Both are one-line reversals
if he reads them the other way:

- **C1 — assumed yes:** the ladder indexes a tier by bank list identity, so no
  per-row tier field is required. Reversing it makes Tier Mode wait on a
  bank-data project.
- **C4 — assumed "nothing interpretable":** the opaque glyph ids that reach
  the generator today may stay, and the I14 scan is widened instead (it now
  also covers
  `pack.rs` and `play.rs`, for the ladder's own vocabulary; `band` stays legal
  because `canon.rs` means Sudoku row bands by it). Reversing it means
  unpicking fragment clues, which touches the generator.

**C5**'s chip-farming defect was fixed first, on its own, and F8 builds on the fix.

Phase A4 is in `docs/CC-SPELLDOKU-v1.3.md` ("As built"), with the F1 Easy-row
contradiction flagged there for Eric.

