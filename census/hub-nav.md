# CC-HUB-NAV §0 census

Run 2026-09-26 against `cc-sd-fit`. Phase A only: nothing built, nothing moved.

**C1 HALTs.** Everything else is a finding, and most of them are good news.

---

## C1 — Mode registry — **HALT**

> HALT if tiles and sheet are hand-maintained separately — reconcile to one
> registry first (own amendment, do not silently merge here).

They are separate, and further apart than "separate" suggests: the two sources
do not share an id vocabulary.

| | source | ids | count |
|---|---|---|---|
| Gamepad sheet (`src/play_hub.rs`, CC-MODE-HUB F2) | `config/modes.json` | mode ids: `practice`, `spelldoku`, `def_match` … | 23 |
| Hub tile row (`src/hub_tiles.rs`, CC-BUILD219-FIXES F1) | `config/hub-tiles.json` | **DOM button ids**: `vsBtn`, `climbBtn`, `dailyBtn`, `sayItBtn`, `soBtn` | 5 |

**Overlap: zero.** Not "some tiles missing from the registry" — no tile id is a
registry id at all, because one file names modes and the other names buttons.

And the row is not rendered from its file either. `hub_tiles.rs` says so in its
own header: *"The 'Ways to play' row is static markup: six launchers, each with
its own id, i18n key and feature gate. This module does not render it."* It only
suppresses members by toggling a class. So the tile row is hand-written HTML in
`index.html`, gated by a five-row JSON of element ids.

**What this blocks.** I2 ("Drawer rows and hub tiles both resolve through the C1
registry. No second list.") has no single registry to resolve through. A3 ("for
every hub tile route `r`: `r ∈ drawerRoutes`") is not merely failing — it is
unsatisfiable, because tiles have no routes anywhere and the registry has no
route field (see C2).

Per §0 this reconciliation is its own amendment. I have not merged them.

## C2 — Route table

`config/modes.json` entries carry: `id`, `nameKey`, `descKey`, `icon`, `status`,
`kidSafe`, `platforms`, `entitlementLevel`, `requiresPremium`, `languages`,
`exitStyle`, `juniorPolicy`.

**There is no `route` field.** `play_hub.rs` resolves a mode to a destination by
kind — a *launcher* renders a button that clicks an existing DOM button
(`sayItBtn`, `soBtn`), an *info* mode has no destination at all. So F3's "tapping
navigates to the mode's route" has nothing to read today; routes would be added
alongside C1's `group` field, which makes that one amendment rather than two.

Note the registry already has `descKey` — D-N3 moves that string to the mode's
start screen, so no translation surface is added or lost, exactly as F3 says.

## C3 — Auth state

One predicate: `climb::is_logged_in()`. `climb::reflect_auth()` is the reflector
that toggles `btn-hide` on `climbBtn`, `accountBtn`, `setAccountRow`, and calls
`online_spelloff::reflect_gate()`. No second source and no disagreement found.

One wrinkle for F2: Kid Mode is read from the DOM — `body.class_list().contains("kid")`
— not from the Jr resolver. Two spellings of the same idea in one function.
Worth unifying when F2 touches it, but it is consistent today, so it is a finding
rather than the C3 HALT.

## C4 — Jr resolver — merged

`src/experience.rs` resolves `Experience::Junior` / `Standard` and returns
`Resolved { experience, locked, source }`, including an `AgeGate` source that
locks it. I7 has something real to read.

## C5 — Entitlement source

`entitlementLevel` and `requiresPremium` on each registry entry, deserialized in
`src/modes.rs` and filtered by `modes::visible(&all, &ctx)`. Read-only for this
file, as specified.

## C6 — Baseline geometry

Measured at 375 × 667 on the dev web build (hub markup is shared with the app;
a simulator run would confirm the safe-area inset, which the browser does not
model).

**Orb top edge: 341px.** Five rows of chrome above it, which is exactly the
count the Intent describes:

| row | top | height |
|---|---|---|
| `.brand` — wordmark + tagline | 24 | 45 |
| `.meta-corner` — the three round icons | 83 | 44 |
| `.home-group.data-group` — the utility row | 139 | 48 |
| `.home-group.modes-row` — the four mode tiles | 197 | 62 |
| `#setupChip` — the session pill | 271 | 44 |

**A5 wants ≤ 301.** Removing the utility row (48) and folding `.meta-corner`
(44) into the header line frees ~92px against a 40pt requirement, so A5 has
roughly double the headroom it needs. No other row has to move.

## C7 — Timer hook — one finding, and it is small

Only the base game has a countdown. `defmatch_screen.rs` states it has no
visible countdown by its own D2, and nothing else holds a deadline.

**There is no pause.** `TIMER` stores `total` and an **absolute** `deadline`.
`stop_timer(reset)` clears the interval but leaves the deadline in wall-clock
terms, so time keeps passing while the tick is stopped; `start_timer` always
assigns `deadline = now + total`, a full restart rather than a resume.

So D-N7's "architecturally un-pausable" case does not arise. The hook is: on
pause store `remaining = deadline − now`; on resume set `deadline = now + remaining`.
Roughly five lines in one file, and I6 becomes testable.

## C8 — Component inventory

26 elements already use `class="scrim"`, so the scrim + panel idiom is
well-worn and F1 should reuse it rather than introduce a drawer primitive.

**No focus trap exists anywhere** — the `inert` matches in `index.html` are
unrelated prose. F1's trap, Escape handling and first-focus-on-X are new work,
and they are the part of F1 least served by copying the existing sheets.

## C9 — Misses count

`game.rs` renders `missesBtn` as `↻` + label + `<b class="dc-badge">{due}</b>`,
where `due` is the review queue's due count, and disables the button when both
it and the tone-drill count are zero.

That markup is already F4a's chip in all but placement: same glyph, same count,
same source. F4a is a move and a restyle of an element that exists, not a new
component — and "one source, two views" is satisfied by construction if the
drawer row reads the same `due`.

## C10 — Utility-row consumers

One: `tests/e2e/specs/review-queue.mjs:92` clicks `#missesBtn` by id. Since
F4a keeps a `missesBtn`-shaped chip on the header, this may survive the
cut-over untouched — worth confirming rather than assuming, since the spec asks
for the list so Phase C can update it.

No Maestro flows or onboarding coach marks reference `+ My words` or
`From a photo`.

---

## What I would decide before Phase B

1. **C1 is the whole gate.** The reconciliation wants three things in one
   amendment, not three: a `route` per entry (C2), a `group` per entry (F3),
   and the tile row rendering from the registry instead of static markup. Doing
   them separately means touching the same file three times.
2. **The tile row is static HTML.** F6's "quick-play row = fixed four" is
   currently five hand-written launchers gated by a JSON of button ids. Deciding
   whether the row becomes registry-rendered, or stays static and simply drops a
   member, changes how much of C1 has to be solved now.
3. Everything else is buildable as written. C4 is merged, C6 has double the
   headroom A5 needs, C7's hook is small, and C9 is a move rather than a build.
