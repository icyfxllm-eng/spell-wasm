# CC-SPELLDOKU-RULES v1 — §0 census (read-only)

Ran 2026-09-22 against main at ab47295c (live build 239). Read-only: no
behaviour changed. The spec targets "build 220"; the tree is 19 builds past
that, and 12x12 was cut on 2026-09-21, which voids v1.2 D17/D18/D19 and
removes one of F5's "absent" cases.

**Three HALT conditions fired: C1, C2, C7.** Two further findings change the
shape of the work: C5 (F6 has nothing to migrate) and C9 (there is no streak).

## C1 — Placement paths — HALT

Every path that writes a symbol into a cell, in `src/spelldoku_ui.rs`:

| # | Path | Call site | Writes |
|---|---|---|---|
| 1 | Spelled commit | `submit_typed` -> `commit_value(v, true, …)` :834 | `entries[i]` |
| 2 | Tier Mode spelled commit | `submit_typed` -> `commit_value(v, true, …)` :757 | `entries[i]` |
| 3 | Tier Mode earned digit (no spelling) | `tier_digit` -> `commit_value(v, false, …)` :703 | `entries[i]` |
| 4 | Unlocked chip tap (D1) | click handler -> `commit_value(v, false, …)` :1073 | `entries[i]` |
| 5 | Pencil mark | click handler :1080 | `pencil[i]` only — exempt by D-R6 |

Not placement paths, contrary to the spec's assumed list: **hint** selects a
cell and writes nothing (:838); **undo/redo do not exist**; **saved-board
restore does not exist** (see C5); the **test seam is read-only**.

HALT as written — no validator exists anywhere, so all four paths write
unchecked. The mitigating structure: all four already funnel through ONE
function, `commit_value` at :616, so F1 needs a single insertion point in the
UI plus the pure `validatePlacement` in the core (I-R7 wants it symbol-blind,
so it belongs in `src/spelldoku/`, beside `geo::Geo`, which already has the
row/column/box peer sets the check needs).

**The exploit reproduces, and it is tier-scoped.** `commit_value` writes on
`Verdict::Correct` always, and on `Verdict::WrongValue` only when
`!play::logic_errors_shown_at_once(tier)` — that is, Hard and Expert. On Easy
and Medium a wrong value is refused with a message and never written. So
spamming one letter into a box is possible on Hard and Expert only, which
matches the build 219 report (9x9 Hard).

## C2 — Input order — HALT and ask

**The typed word determines the symbol**, on the primary path. `submit_typed`
(:803) reads the cell, then `values_for(&g.typed)` resolves which symbols that
spelling could be, and the verdict picks the value. Nothing knows which symbol
is being placed until the word is complete.

Two secondary paths do pick the symbol first: the unlocked chip tap (path 4)
and Tier Mode (tap a digit, then spell its word).

So I-R2 — "the conflict check runs before the composer opens" — cannot hold
for the primary path as built. There is nothing to check before spelling.

## C3 — Prompt strings

Six hardcoded call sites, all in `src/spelldoku_ui.rs`: :508, :510, :511
(composer prompt), :636, :646 (rejections), :799. All 15 locales carry every
key; only ONE of them varies by mode today (:646 picks `sd.misspelledWord` over
`sd.misspelled` when the board is words).

English strings that say "number" and are reachable on a Letters board:

| Key | English | Mode-aware? |
|---|---|---|
| `sd.spell` | Spell the number, then ✓ | no — **this is the reported bug** |
| `sd.wrongValue` | That number doesn't go here | no |
| `sd.misspelled` | That isn't a number word — check the spelling | yes, swaps to `sd.misspelledWord` |
| `sd.tech.single` | a single — no other number fits here | no (hint line) |
| `sd.desc` | Sudoku you solve by spelling numbers | no (mode catalogue) |
| `sd.tierPick` | Pick a digit — its tier is on the badge | n/a, Tier Mode is digits |

## C4 — Mode selection

`wordmode::is_word_mode(kid, n, tier)` — a pure tier rule (9x9 Medium and up,
never Spell Jr), consulted at generation time inside
`wordmode::board_or_number`. It is **not** stored per board and there is no
player preference anywhere; `TIER_KEY` ("spell_sd_tier") stores only the Tier
Mode reading. What IS per board is `bind::Board.mode` (`Mode::Number(Table)` or
`Mode::Words(Vec<WordSymbol>)`), which is what the header chip reads as of
build 239. F5's resolver replaces the `is_word_mode` call inside
`board_or_number`.

## C5 — Saved boards: there are none

SpellDoku persists exactly three things: `spell_spelldoku_seen_v1` (board
hashes for the repeat window), `spell_spelldoku_stats_v1` (counters), and
`spell_sd_tier`. **An in-progress board is never saved** — it lives in a
thread-local `GAME` and dies when the screen closes.

So the count F6 asks for is zero, on every device, and it cannot be otherwise.
A conflicted board cannot survive a session, so F6 has nothing to migrate.

## C6 — Localization

Confirmed: `src/i18n/locales/<lang>.json`, 15 locales, reached through
`i18n::t` / `i18n::tp`, with `node scripts/i18n-check.mjs` in the gate
enforcing key parity across all 15 and that every referenced key exists (812
keys today). Strings no speaker has reviewed are queued in
`docs/review/ui-strings-unaudited.md`. A string went through this path on
2026-09-21 (`sd.tier.letters`), so it is live and working.

## C7 — Tier Mode audio — HALT

**There is no ▶ in Tier Mode.** The digit strip renders `.sd-tierkey` buttons
that carry the digit and its band label and nothing else (:485). Tapping a
digit draws a word and speaks it once; tapping the same digit again calls
`tier::Session::draw`, which draws a **different** word (:183). The current
word cannot be replayed.

The ▶ orbs (`data-sd-say`) live only in the Word Mode legend, which is not
rendered on a numbers board — and Tier Mode boards are numbers boards.

F4's Tier Mode row, "Tap ▶ to hear the word, spell it, then ✓", would be a lie.

## C8 — Composer sheet

GEOMETRY D-G2 has **not** landed. There is no composer sheet in the codebase
(zero matches for "composer"); the keyboard is a persistent part of the screen,
and the 2026-09-21 phone-fit work deliberately keeps it that way. So I-R2 reads
in its alternative form: "the conflict check runs before any spelling input is
accepted" — which is exactly what C2 makes impossible on the primary path.

## C9 — Daily path — no HALT

`serve(app, n, tier, daily = true)` seeds from
`play::daily_seed(ymd, lang) = fnv("spelldoku-daily|{ymd}|{lang}")`, generated
on device. No server fetch, no cache, no seed pack — and no tie to 12x12, which
is cut. Same date + language + configuration gives every player the same board,
and a replay gives the same board again.

There is **no "already played today" gating** to remove, and — the part D-R11
assumes — **SpellDoku has no streak at all.** No streak is read, written, or
displayed anywhere in the mode. D-R11 has nothing to count toward.

One more: the Daily deliberately bypasses the player's repeat ledger
(`if daily || !seen…`, :202), so today a Daily can repeat a board the player
has already met. D-R4 makes that moot.

---

## Eric's rulings on the halts, 2026-09-22

- **C2 — invert to pick-then-spell.** The player taps the symbol first, the
  conflict check fires there, and spelling confirms it. I-R2 holds literally.
  I had recommended checking at commit instead, on the grounds that the
  inversion collides with D1; Eric chose the inversion, and on a second look it
  does not collide — it **generalises the flow Tier Mode already uses**
  (`tier_digit`: tap a digit, and if the ladder is satisfied it commits, else it
  draws a word to spell). D1 survives unchanged, read as "an unlocked symbol no
  longer has to be spelled". Consequence: the legend and the unlock chip tray
  become one row of tappable symbols, and a Number Mode board needs that row
  too — today it has none until D1 unlocks a chip.
- **C7 — add a replay ▶.** The pending digit gets a ▶ that re-speaks the
  current draw without drawing a new word. F4's Tier Mode copy becomes true.
- **C5 / F6 — dropped.** Nothing can be migrated, so the feature and Done #9
  are cut, and I-R1 loses its exception clause, which strengthens it.
- **D-R11 — streak clause dropped.** D-R4's fresh-per-play Daily and unlimited
  replays stand; SpellDoku says nothing about streaks.
