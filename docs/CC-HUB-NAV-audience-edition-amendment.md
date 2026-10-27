# CC-HUB-NAV — Amendment AE-1: adult-only rows, edition rows, gated deep links

**Status:** DECISIONS SIGNED. D-AE1 to D-AE4 were signed as recommended (Eric, 2026-10-26: "I like the recs they seem reasonable", then "record D-AE1 to D-AE4 and D-R1, D-R4, D-R6 to D-R10 as signed as recommended"). **BUILT 2026-10-26** (see "As built" below).
**Raised by:** CC-GUARDIAN-REPORT v1 §0 C3 (HALT). Base: origin/main `5ffeb19d`.
**Amends:** the CC-HUB-NAV row registry (`config/modes.json`, `src/modes.rs`), `build.rs`'s registry staging, and the hash router (`src/lib.rs:363`).
**Consumes as-is:** CC-ONBOARD-JR's `experience::audience()`. The resolver doesn't change.
**Amends CC-GUARDIAN-REPORT v1 (D-AE4):** F7's deep link is `spellgame://guardian-report?week=<monday>`, not `spell://`. The GUARDIAN-REPORT file is not in the repo yet.
**Out of scope:** any change to how existing rows behave. Every new field is opt-in.

---

## Why

GUARDIAN-REPORT F4 needs a drawer row that is **absent**, not locked, in three cases: under Spell Jr, under unknown age, and in the Education Edition. I4 adds that the route behind it must be unreachable by deep link too. The tree can do only the first of those:

| Need | Today | Gap |
|---|---|---|
| Absent under Jr | `ctx.kid && (!kid_safe \|\| junior_policy == "hidden")` (`modes.rs:356`) | none |
| Absent under unknown age | `HubCtx` carries `kid: bool` only (`modes.rs:296`), so an unanswered gate reaches `permitted` as Standard. `experience::audience(kid, gate_answered)` already returns `Unknown` (`experience.rs:161`), but nothing wires it in. | **missing** |
| Absent in Education | No edition on `HubCtx`, and no edition clause in `permitted`. `build.rs` strips rows only for `web` (`build.rs:61`, keyed on `platforms`). | **missing** |
| Deep link refused for Jr / unknown | The router knows `#daily` and `#practice` only, and checks no audience (`lib.rs:370`) | **missing** for any gated route |

---

## Changes

### AE-F1 — `adultOnly` rows

- **Registry:** a new optional boolean, `"adultOnly"`, defaulting to false.
- **`HubCtx`:** gains `audience: experience::Audience`. `play_hub::ctx` fills it from `experience::audience(s.kid, agegate::stored().is_some())`. `kid` stays, because every existing clause reads it.
- **`permitted`:** one clause, placed after the Jr clause:

  ```rust
  // AE-1: an adult-only row exists for a known Standard player only. Unknown
  // age is the stricter answer (experience::audience), so it hides too.
  if m.adult_only && ctx.audience != Audience::Standard {
      return false;
  }
  ```
- **`modes-check.mjs` adds two rules:**
  - `adultOnly` must be a boolean when present.
  - `adultOnly: true` requires `kidSafe: false` and `juniorPolicy: "hidden"`, so the fields can never disagree in the permissive direction. That's the same rule already applied to `kidSafe`.

### AE-F2 — `editions` rows, stripped at build time

- **Registry:** a new optional array, `"editions"`, holding `"consumer"` and/or `"education"`. Absent means both.
- **`build.rs`:** gains `strip_edition_modes` when `CARGO_FEATURE_EDUCATION` is set. It drops every row whose `editions` excludes `"education"`. It's the same shape as `strip_app_only_modes`: the row is absent from the bundled registry, so it is not merely filtered at runtime.
- **The module itself** stays out of Edu with `#[cfg(not(feature = "education"))]`, as `COMPLETE_PRODUCT_ID` already is (`entitlements.rs:48`).
- **`modes-check.mjs`:** `editions` values must come from that closed set.
- **Why build time and not a `HubCtx` clause:** O2's default is "absent until rostering exists", and code that's missing is a stronger promise than a guard. `web` is the precedent. `scripts/build-web-edu.sh` already runs an edition-specific check (`edu-no-purchase-check.mjs`), and that's where AE-A4 goes.

### AE-F3 — gated deep links

- The router gains a small table of gated routes. Each entry is a route hash, the registry id it opens, and an action. A route opens only if `modes::catalog(...)` currently contains its row. Otherwise it is consumed and ignored, with no toast, so a child learns nothing about what's behind it.
- That makes the row and the route one rule: a deep link can never reach what the drawer would hide.
- **Gated routes still pass the adult gate** (`parent_gate_then`) before rendering. Visibility and the gate are separate questions.
- **The scheme stays `spellgame://`.** GUARDIAN-REPORT F7's `spell://guardian-report?week=` becomes `spellgame://guardian-report?week=`. The iOS handler forwards only `url.host` (`AppDelegate.swift:45-61`), so that handler also gains forwarding of the query string. The change is native: a peer owns iOS surfaces, so check before you edit.

### AE-F4 — one adult gate (D-AE3)

- `guardian_dash.rs` keeps its own copy of the gate (`GATE_ANSWER`, `gdashGate`, `:254-262`). It moves onto `parent_gate_then`, so there is one gate with one behaviour.
- The challenge stays as it is in v1: a worded multiplication with operands 3–8.
- The spec names the gate for what it is, **a deterrent that keeps kids out, not adult verification.** Strengthening it, with a wider range, a lockout and a fresh problem each time, is a separate change. The CC-CONTRIBUTE census already asked for it, and it would serve both files.

---

## Invariants

- **AE-I1.** A row with `adultOnly: true` is absent whenever `audience` is Junior or Unknown, on every surface.
- **AE-I2.** A row whose `editions` excludes `education` is absent from the bundled Edu registry. A source scan finds the module unreferenced in an Edu build.
- **AE-I3.** A gated route opens only if `catalog` contains its row in the same context.
- **AE-I4.** Rows without the new fields behave exactly as before, which a golden of `catalog()` across the existing contexts confirms.
- **AE-I5.** There is one adult gate implementation in the crate.

## Acceptance (CI)

1. **Unknown age:** `permitted` hides an `adultOnly` row for Unknown, even with `kid` false. It also hides it for Junior and shows it for Standard.
2. **`modes-check`:** it rejects `adultOnly: true` alongside `kidSafe: true`, or alongside any `juniorPolicy` other than `hidden`. It also rejects an unknown `editions` value.
3. **No regression:** the `catalog()` golden over the existing contexts (kid/standard × app/web × preview/full) is byte-identical before and after.
4. **Edu strip:** in `build-web-edu.sh`, the staged registry has no row whose `editions` excludes `education`. `edu-no-purchase-check` grows that assertion.
5. **Deep link:** under Junior and under Unknown, `#guardian-report` is consumed, nothing renders, and no gate opens. Under Standard, the gate opens first. This is e2e, through `__spelltest`.
6. **One gate:** a source scan finds `parent_problem()` called only from `lib.rs`, and DASH reaches it through `parent_gate_then`.

## Decisions (signed as recommended, Eric 2026-10-26)

| # | Decision | Signed |
|---|---|---|
| **D-AE1** | How unknown-age rows are hidden | **An opt-in `adultOnly` field** that hides for Junior and Unknown. The alternative, treating Unknown as Jr in every row, changes every kid-gated row at once. |
| **D-AE2** | How rows are kept out of Education | **Strip at build time,** following the `web` precedent. The alternative is an `edition` field on `HubCtx` checked at runtime. |
| **D-AE3** | The adult gate for v1 | **Keep the existing challenge, merge DASH's copy into `parent_gate_then`, and name it a deterrent.** Strengthening it is a separate change. |
| **D-AE4** | GUARDIAN-REPORT's `spell://` scheme | **Use the shipped `spellgame://`,** and amend GUARDIAN F7. |

## As built (2026-10-26)

**F1:**
- `Mode` gains `adult_only` (`adultOnly`) and `editions`.
- `HubCtx` gains `audience`, which `play_hub::ctx` fills from `experience::audience(kid, agegate::stored().is_some())`.
- `permitted` gains the AE-I1 clause right after the Jr clause.
- `modes-check` validates both fields, and its selftest grows from 19 to 24 malformed fixtures, plus a valid adult-only row that must pass.

**F2:**
- The registry strips moved into `src/modes_strip.rs`. `build.rs` pulls it in with `include!`, and the lib's tests compile it (`#[cfg(test)] mod modes_strip`).
- The web strip's logic is unchanged, and a test pins that it keeps exactly the rows listing "web".
- `CARGO_FEATURE_EDUCATION` strips rows whose `editions` excludes "education".
- `edu-no-purchase-check` adds an `"id": "<id>"` needle for every excluded row.
- No row sets `editions` yet, so the Edu registry is unchanged today.

**F3:**
- `modes::GATED_ROUTES` (empty), `RouteVerdict` and the pure `gated_route` are new.
- The hash router consults them before its ordinary routes. A Refuse is consumed silently; an Open clicks the row's `launch_for` target.
- **Changed with REPORTS-SHARE (2026-10-27):** the first gated row's launcher owns the parent gate, so the drawer and the link pass the same gate once, rather than twice on a deep link. `ae1_tests::every_gated_routes_launcher_opens_the_parent_gate_first` holds every gated launcher to opening the gate before it renders or shares anything.
- The query string is ignored when matching, so `#guardian-report?week=…` matches its route.
- **Not done:** the iOS `AppDelegate` change to forward the query. Nothing needs it until GUARDIAN-REPORT F7, and the native side wants checking with whichever session owns it.

**F4:**
- `guardian_dash.rs` dropped `GATE_ANSWER` and the `gdashGate` markup, and its opener now calls `parent_gate_then`.
- `learner-surfaces.mjs` drives `parentQ` / `parentAnswer` / `parentSubmit` instead.
- The `gdash.check` and `gdash.wrong` strings are now unused, but are left in the 16 locales for a later sweep.

**Tests:**
- `modes::ae1_golden` (A3) was pinned from the untouched tree before any change. It covers 24 contexts: junior, standard and unknown × app and web × preview and full × premium none and all.
- `modes::ae1_strip_tests` covers A4, and adds an Education-only test when run with `--features education`.
- `modes::ae1_tests` covers A1, A5's logic and wiring, and A6.
- A mutation run (the Unknown clause, Refuse, the Education keep and the modes-check rule) failed a test each time.

**Not covered:** A5 as an e2e. No gated route exists to drive yet, so the router is checked through `gated_route` and a source check. The first real e2e comes with GUARDIAN-REPORT F4.

## Non-goals

- No change to `kidSafe`, `juniorPolicy` or any existing row.
- No change to how `experience::audience` resolves.
- No Android custom scheme.
- No stronger gate.
