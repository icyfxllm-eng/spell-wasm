# CC-HUB-NAV §0 census

Run 2026-09-26 against `cc-sd-fit`.

**C1 halted, and has since been resolved** in the same branch — the two
registries are now one. Eric asked for that reconciliation directly; it is its
own amendment, as §0 requires, not something folded into a later phase. The C1
section below records both the finding and what was done, including two facts
the first draft of this census got wrong.

Everything else is a finding, and most of them are good news.

---

## C1 — Mode registry — HALT, now **RESOLVED**

> HALT if tiles and sheet are hand-maintained separately — reconcile to one
> registry first (own amendment, do not silently merge here).

**The halt was correct; two of the details I first wrote were not.** Corrected
here because Phase B reads this file.

What was true: two registries with no join. The Play hub sheet rendered from
`config/modes.json`, keyed by MODE id. The home row was gated by
`config/hub-tiles.json`, keyed by DOM ELEMENT id. No shared vocabulary, so
"resolve tiles and sheet through the registry" had no single registry to mean.

What I got wrong:

* I wrote that the registry has no route field and therefore no destinations.
  A mode → element table already existed: `LAUNCH` in `src/play_hub.rs`. It is
  a Rust const rather than a JSON field, and deliberately so (below).
* I wrote that `climbBtn` and `dailyBtn` had no registry entry. They do. The
  registry carries `climb` and `daily` with `status: "core"`, which its own
  `$fields` defines as a surface the registry governs but the SHEET never
  tiles — CC-ONBOARD-JR option (a), signed 2026-09-11. The home row showing
  Daily and the sheet not showing it is that decision working, not a gap.

**The resolution.** The row's membership AND its launcher id are one field on
the mode entry — `hubTile: { element, member }`. `config/hub-tiles.json` is
deleted. One registry, and a tile is described in the same place as everything
else about its mode.

**The first attempt was wrong and an existing test caught it.** I initially
resolved mode → element through `play_hub::launch_for`, adding `climb` and
`daily` rows to its `LAUNCH` table. `launch_table_covers_every_registered_mode`
failed, and its doc comment says why: core modes are *"excluded rather than
bound to None, which would claim they are in-round aids."* `LAUNCH` answers
where a SHEET tile goes, and the Climb and the Daily have no sheet tile and a
home-row launcher each. Making my join work by reversing that decision was the
wrong direction, so the element moved onto the row's own field instead and
`play_hub` is untouched.

The registry is compiled into the SITE build, so no `hubTile` may name an
app-only launcher — its symbols would reach the site bundle and breach the
picture wall (CC-PICTURE-BANK I1), which is how the first cut of F1 broke.
That rule is carried over as a test rather than left as a comment.

`vsBtn` (Challenge a friend) has no registry entry and does not need one: the
old file's own `$wall` note states the registry **only ever suppresses**, so
absence costs nothing and behaviour is unchanged. Head-to-Head stays out of the
catalog until it has a reviewed spec, exactly as the file requires.

Carried over from the deleted file, now keyed by mode: an element id that is
not real markup fails the build, so a typo cannot silently suppress nothing and
leave the tile looking like the feature broke.

**Not bundled: the `group` field.** My first read of this census recommended
doing `route`, `group` and the row in one amendment. That was wrong, and the
data says so: D-N6 names 14 modes, but **11 of the 23 registry entries have no
D-N6 group**, and two D-N6 names (`spell_it`, `spell_off`) are not registry ids
at all. Adding a required field would have meant inventing 13 assignments.
`group` belongs to Phase B, where the drawer that reads it is decided.

## C2 — Route table

Corrected: destinations exist, as `LAUNCH` in `src/play_hub.rs` — a
`(mode id, Option<element id>)` table where `None` marks an in-round aid with
no destination. The registry entry itself carries no route, and after this
amendment it still should not, for the picture-wall reason in C1.

Registry fields: `id`, `nameKey`, `descKey`, `icon`, `status`, `kidSafe`,
`platforms`, `entitlementLevel`, `requiresPremium`, `languages`, `exitStyle`,
`juniorPolicy`, and now `hubTile`.

F3's "tapping navigates to the mode's route" resolves through `launch_for`
today. Note `descKey` already exists, so D-N3's move of the tagline to the mode
start screen adds no translation surface, as the file says.

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

1. **The `group` field**, which this amendment deliberately left out: D-N6
   covers 12 of 23 entries, so 11 need a decision and two D-N6 names are not
   registry ids. That is Phase B's first task and it needs Eric, not a guess.
2. **The tile row is still static HTML.** The registry now decides membership,
   but the row itself is hand-written launchers. F6's "quick-play row = fixed
   four" can be met by a data edit today; making the row registry-RENDERED is a
   separate question nobody has asked yet.
3. Everything else is buildable as written. C4 is merged, C6 has double the
   headroom A5 needs, C7's hook is five lines, and C9 is a move rather than a
   build.
