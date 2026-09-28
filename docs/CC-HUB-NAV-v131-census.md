# CC-HUB-NAV v1.3.1 §0 census — Phase A, report only

Run 2026-09-27 against `main` at `fb0a5a72` (shipped build 252). Nothing built,
nothing moved.

**Three HALTs.** All three are the same shape: removing a hub entry point
leaves a mode with no door, which is the exact failure §0 C1's HALT language
exists to prevent. Two of them are already live in build 252 and are not
caused by this spec.

A note on the baseline: this file repeatedly calls build 219 the baseline. The
shipped build is **252**, and v1.1 Phases A–C are merged, so several of the
things described as present (the utility row, the hub Misses chip's old
position) have already moved. Where the two disagree I measured what ships.

---

## C1 — drawer landed: YES, no halt

CC-HUB-NAV v1.1 Phase A (drawer), Phase B (Account + Your Words) and Phase C
(the cut-over) are all merged and shipped in build 252 (`22c7e662`). The
drawer is the navigation: the three round icons, the utility row and the
gamepad sheet are gone, and every Play row routes.

## C2 — burger baseline: it is unstyled, and that breaks F4's own arithmetic

Measured at 375pt and 430pt on the dev build:

| | 375pt | 430pt |
|---|---|---|
| burger frame `(x, y, w, h)` | `(152, 83, 16, 48)` | `(396, 24, 16, 48)` |
| position | **inline, beside the Misses chip** | **inline, beside the Misses chip** |

Neither is v1.1 D-N2's top-right. At 430pt the meta-corner shares the brand's
row and the burger's trailing edge lands 18pt from the viewport edge; at 375pt
the whole corner **wraps onto a second line below the wordmark**, which is why
it reads as a left-aligned strip in the screenshot rather than a corner.

**`W_burger_baseline` = 16pt, and that number should not be doubled.** The
burger is created by `drawer.rs` with `class="icon-btn"` — and `.icon-btn`
**does not exist in the stylesheet**. It matches nothing, so the only
navigation control in the app is an unstyled bare `<button>`: no circle, no
border, 16pt wide. The icon family it was meant to join is `.meta-icon`, which
is 44×44 (52×52 in Kid Mode).

So F4's two clauses contradict each other on the measured baseline:
`2 × 16 = 32pt`, and the same feature requires a hit target `≥44×44pt`. **This
needs your word before A3 can be written**, because A3 asserts the width to
±1pt in CI:

- **(a) 88 × 44** — treat the baseline as the 44pt the icon family uses and
  the 16pt as the bug it is. A wide, obvious pill. *My recommendation*, and
  it is the only reading where "2×" and "≥44×44" are both true.
- **(b) 44 × 44** — "2× the measured 16" rounded up to the minimum. The
  smallest change that is still legal.
- **(c) 32 × 44** — literal 2× width with padding to reach the tap minimum.
  Satisfies the letter of both clauses; the visible control is still narrow.

Either way the burger needs a real class. Fixing that is in scope for F4 and
is not a new token — `.meta-icon` already exists.

## C3 — drawer Your Words contents: **HALT**, one row is missing and one is conditional

| Row | In the drawer? | |
|---|---|---|
| Misses | **conditional** | Rendered from the registry, but filtered out entirely when `missesBtn` is disabled — which is when total misses AND tone-drill words are both 0. |
| My Words | yes | Routes by proxying a click to `importBtn`. |
| From a photo | **NO** | `photo_list` is `status: hidden` in the registry, and `permitted()` rejects Hidden first and unconditionally. There is no row on any platform. |

**From a photo is unreachable in build 252.** Its button, `photoBtn`, lives
inside the `.data-group` that Phase C retired (`display:none !important`), and
it has no drawer row to replace it. On iOS `photo_list::reflect_visibility`
still un-hides the button, but its parent is display:none, so nothing shows.
This is a regression I shipped in the Phase C cut-over, not something this
spec introduced — and it is exactly the dead end I-N3 forbids.

Misses is not a HALT: the row exists. But F3 says the row stays at a count of
0 with the badge hidden, and today the row is *removed* at 0 instead. See the
F3 note below.

## C4 — references to remove

**Hub Misses chip (F1).**
- `index.html:1798` — the chip itself, inside `.meta-corner`.
- `src/game.rs` `refresh_mode_buttons` — renders its icon, label, badge,
  tooltip and disabled state. This is also C5's store, so it moves rather
  than dies.
- `src/lib.rs:892` — its click handler, which enters and exits review mode.
  Must survive as the drawer row's proxy target.
- `tests/e2e/specs/review-queue.mjs:92` — clicks it programmatically. Works on
  a hidden element, so it survives, but it should go through the drawer.

**Quick-play tile row (F2).**
- `index.html` — `.home-group.modes-row` and its **six** buttons, not four:
  `vsBtn`, `climbBtn`, `dailyBtn`, `wordPicTile`, `sayItBtn` (flag-hidden),
  `soBtn` (flag-hidden).
- `tests/e2e/specs/hub-row.mjs` — the whole file (2 tests) is about this row.
- `tests/e2e/specs/modes.mjs:9,30`, `submit-advance.mjs:45` — click `#dailyBtn`
  and `#vsBtn`.
- `tests/e2e/specs/sayit.mjs:15-39` — asserts `#sayItBtn` visibility.
- `tests/e2e/specs/front-door.mjs:127,209` — asserts and clicks `#climbBtn`.
- `tests/e2e/specs/settings-effects.mjs:409,420` — clicks `#sayItBtn`.
- `src/hub_tiles.rs` — renders the row's membership from the registry.
- `.maestro/tool-spellaloud-effect.yaml` — mentions `sayItBtn` in a comment.

**The row's buttons cannot be deleted, only hidden.** Every one of them is a
proxy target: the drawer routes by clicking the mode's existing element
(`climbBtn` is also how `ghost.rs` reaches the board, and `climb.rs` has a
test asserting it still owns `open_leaderboard`). This is the precedent the
`.retired` class was created for in Phase C. I-N2's "hub children" test must
therefore count **rendered** children, or it will fail on markup that is
correctly invisible.

The leaderboard survives the row's removal: `acctClimb`, inside the Account
sheet, is its second entrance, and the Account sheet is a drawer row.

## C5 — Misses count source, and a discrepancy with A6

One store: `AppState.misses`, via `misses::due_misses(&s)`.

The chip's badge shows **`due` — the count due for review now**, not the total
saved. `refresh_mode_buttons` renders `due` in the badge and puts both numbers
in the tooltip (`"{total} saved · {due} due now"`), and disables the chip when
`total == 0 && tone_total == 0`.

**A6 reads as though the badge shows the total**: "Seed 181 misses → drawer
badge `181`". With today's semantics, seeding 181 misses whose review dates are
in the future shows `0`. Tell me which you want:

- **(a) due now** — what ships today; the number equals what tapping the row
  will actually serve you. *My recommendation.*
- **(b) total saved** — what A6's wording implies; a truer "size of your pile",
  but the row then offers 181 and serves 3.

There is also **no Misses screen**. `missesBtn` toggles *review mode*, which
serves the due set as a session. I-N5's "badge = Misses screen count" has to be
read as "badge = the number the store exposes = what review will serve", which
(a) satisfies by construction and (b) does not.

F6's watermark is unaffected either way: "new misses since last drawer open"
is a change in the **total**, and that is unambiguous.

## C6 — vertical baseline

375pt (iPhone SE class), then 430pt:

| | 375pt | 430pt |
|---|---|---|
| wordmark | 24 | 24 |
| tagline | 54 | 54 |
| meta-corner (Misses chip + burger) | 83 | 24, inline with the brand |
| quick-play tile row | 143 | 84 |
| session pill | 217 | 158 |
| **orb top** | **287** | **228** |
| Replay/Slow row | 495 | 436 |
| field label ("YOUR SPELLING") | 568 | 509 |
| spellbox (the input) | 593 | 534 |
| keyboard top | 693 | 634 |

Removing the tile row (62pt + 16pt gap) and folding the corner into the brand
row (48pt + gap at 375pt) frees roughly **140pt at 375pt** and **78pt at
430pt**. I-N6 holds everything from Replay/Slow down to within ±2pt, so that
space becomes F5's flexible spacer rather than moving anything below the orb.

---

## The three HALTs, together

**H1 — Spell Off has no drawer row, and cannot get one without touching the
registry.** `vsBtn` (local head-to-head, `top.headToHead` = "Spell Off") is
**not in `config/modes.json` at all**. I-N3 requires
`Spell Off ∈ drawer.Play.rows`; the constraints say "do not touch the mode
registry". Both cannot hold. Deleting the tile row as written makes Spell Off
unreachable.

**H2 — Spell It has no drawer row either.** `say_it` is `status: hidden`, and
Hidden is rejected first in `permitted()`, so it never becomes a row. It is
reachable today *only* through `sayItBtn` in the tile row F2 deletes. Same
conflict: the fix is a registry status change.

(For completeness: of I-N3's four Play modes, **Daily** and **Spell Picture**
do have drawer rows and are fine. `online_spelloff` — "Challenge a friend" —
is also `hidden` and also has no row, but it is not named in I-N3.)

**H3 — From a photo has no drawer row**, per C3, and is already unreachable in
the shipped build.

All three resolve the same way, and it is a three-line change I did not make
because the constraints forbid it: give `say_it` and `photo_list` a status the
catalog admits, and add a registry entry for the local head-to-head mode. Every
one of them keeps its existing gate — `say_it` stays behind its flag and iOS,
`photo_list` behind `photo_ocr` and VisionKit — so nothing becomes visible that
is not visible today. Say the word and the three HALTs clear together.

## One more thing worth your verdict before Phase B

F3 says the Misses row stays at a count of 0 with the badge hidden. Today the
row is removed at 0, because `missesBtn` is *disabled* at 0 and a disabled
button fires no click — a row that stayed would be a dead row. Keeping F3 as
written means either enabling review with nothing due (it has nothing to
serve), or a row that looks live and does nothing. The alternative is the
current behaviour, which is v1.1 I3's "absent, not greyed".
