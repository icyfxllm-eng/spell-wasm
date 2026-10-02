# CC-HUB-DEADROWS v1.1 §0 census

Run 2026-10-02 against `main` at c91f3086. Read-only; nothing was modified.

**Outcome: HALT on C2.** The "Spell It" row is not help content and is not
broken — it is the base game, and its do-nothing behaviour is deliberate and
documented. D2 as written would delete the base game's drawer row. Nothing
past §0 is built.

C3 also resolves D4 against removal: Say It is **not** the sole microphone
consumer, so the mic permission must stay.

---

## C2 — what the Spell It row is wired to (HALT)

The row is registry id **`standard`**, and `config/modes.json` says what it is:

```json
{ "id": "standard", "group": "spell_it", "nameKey": "top.spellIt",
  "icon": "🔊", "status": "core",
  "$note": "CC-ONBOARD-JR option (a): the base game." }
```

Its route is `Route::Home`, and `follow()` implements that as `{}` — nothing,
on purpose (`src/drawer.rs:310-316`):

> `Home` — Already there. The base game IS the surface the drawer opens over,
> and the burger only exists on it, so closing the drawer has arrived.

The tap handler is `close(); follow(&route)` (`src/drawer.rs:667-671`), so
tapping "Spell It" **closes the drawer and leaves you on the base game, which
is correct**. To a tester it reads as a dead row because the drawer closing is
the entire visible effect — the screen underneath was already the destination.

Three consequences for D2:

1. **"Spell It is help content, not a mode" is false.** It is the base game, status `core`. There is no help page to move; `descKey` is `share.tagline`.
2. **Removing the row contradicts a signed decision.** `src/modes.rs:586` pins D-N6 membership as Eric signed it, `standard` included:
   `assert_eq!(of(Group::SpellIt), ["bee_sim","climb","daily","online_spelloff","practice","say_it","standard","versus"])`.
   v1.3.1 deliberately *gave* the base game a row, and v1.3.2 renamed the group header to "Spelling" specifically to stop the header colliding with it (`src/drawer.rs:961-971`).
3. The spec's own HALT wording does not quite fire — it says HALT "if Spell It launches a playable session **distinct from** the main orb game", and this launches the main orb game itself. The substance is the same: ask before removing it.

**Ruling needed.** Three readings, all defensible:
- (a) Leave the row. It works; the complaint is that arrival is invisible.
- (b) Keep the row but make arrival legible (a brief highlight, or close-with-feedback). This is the smallest fix to the actual complaint and touches no other decision.
- (c) Remove the row as D2 says, on navigation grounds — not because it is help content — and amend the D-N6 test. The base game stays reachable by closing the drawer, as it was before v1.3.1.

D3 ("How to play: Spell It" page) is moot until this is settled: no such help content exists in code today.

## C1 — Say It footprint

Say It is real, shipped, and not dark. `flags::say_it()` defaults **true**
(`src/flags.rs:66`); the comment at `src/lib.rs:372` saying it "ships dark" is
stale. iOS-only (`platforms: ["ios"]`), `juniorPolicy: hidden`, `kidSafe: false`.

| category | count | notes |
|---|--:|---|
| Rust source files | 13 | `src/say_it.rs` is 378 lines; the others are references |
| Registry entry | 1 | `say_it`, group `spell_it`, `hubTile.element: sayItBtn`, `member: true` |
| Feature flag | 1 | `flags::say_it()`, storage key `spell_flag_say_it`, default ON |
| `index.html` | 39 lines | the `sayItScrim` modal with 5 sub-views, plus CSS and `sayItBtn` |
| Locale keys | **19 keys × 15 locales = 285 entries** | `sayit.*` (16) + `tools.sayit.{name,desc,avail}` |
| e2e specs | 9 files | `tests/e2e/specs/sayit.mjs` (55 lines) is dedicated; 8 others reference it |
| Native JS bridge | 2 files | `native-language-kit.js` + its copy under `ios/App/App/public/` |
| Swift | 0 files | no Swift names it; the bridge is generic speech capability |
| Docs | 6 files | CC specs naming Say It |
| Telemetry | 2 | see C4 |

**Worth knowing before D1 deletes the strings:** all 285 locale entries are
genuinely translated — 0 of the 266 non-English ones are English fallbacks.
Deleting them discards real translation work that cannot be recovered from the
repo if Say It ever returns.

## C3 — microphone consumers: D4 resolves to KEEP

Say It is **not** the sole consumer, so F1's mic-removal clause must not run.

- **`spell_aloud` — "Spell It Out Loud"** (`src/flags.rs:120-129`): voice spelling INPUT, the player speaks letter names. Flag default **ON**, and it is a real mode with its own launcher `spellAloudEnter` (`src/play_hub.rs`, "promoted to a real mode (G-INT-1)").
- `ios/App/App/NativeLanguageKitPlugin+FamilyVoices.swift`, `src/c3_probe.rs`, `src/norm.rs`, `src/native_lang.rs`, `ios/NativeLanguageKit/.../SpeechCapabilities.swift`.
- The shipped permission string itself covers both features, in 15 languages:
  "Spell listens so you can **say or spell** a word out loud."

Removing `NSMicrophoneUsageDescription` would break Spell It Out Loud and
invalidate a translated permission string in every locale. **Leave the mic
untouched**, and note the plist text needs no change either — it already reads
correctly with Say It gone.

## C4 — external references

- **Telemetry wire enum**: `src/telemetry/schema.rs:110` (`SayIt => "say_it"`) and the generated `workers/telemetry/schema.json:146`. The D1 Worker's database already holds rows whose mode value is `say_it`.
- No persisted user-data key names Say It. `src/settings.rs:257` is a call to `reflect_gating`, not storage. **C4's literal HALT does not fire** — no saved progress or streaks are at stake.
- 6 CC docs reference it (expected; D1 exempts docs).

**Ruling needed.** D1 says "zero references outside the changelog and this
file", which would delete the enum variant. That is a wire-schema change, and
the schema is single-sourced with a generated Worker binding and a blessed
snapshot. Retiring the variant also makes the historical D1 rows unreadable by
the schema's own validator. Recommend **keeping `say_it` as a historical wire
value** with a comment, and narrowing D1's "everywhere" to exclude the
telemetry schema — a shipped wire enum is an append-only record, not a
reference to a live feature.

## C5 — other dead rows, and why F3 as written would not bite

I did this statically from the registry plus the launcher table, which is
stronger than tap-testing: it covers every row, every profile and every
language at once.

**Every drawer row has a resolvable target.** The only `None` is `standard`,
and that is `Route::Home` by design. So **F3's proposed static check — "every
registry entry with a drawer group must have a resolvable route" — already
passes today, and would not have caught either row in this spec.**

The real failure mode is runtime, not static: the target element exists but is
**hidden or disabled**, and a proxied click on it does nothing.

- `sayItBtn` is `btn-hide` unless the native bridge is present AND Kid Mode is off (`index.html:1940`). On a device whose language has no on-device recognizer, the row renders and the tap lands on a hidden button.
- The drawer **already knows about this class of bug** and fixes it for one group only: the Your Words rows are filtered by `element_is_disabled` (`src/drawer.rs:604-611`, "Without this the row rendered and did nothing, because a disabled button fires no click"). **The Play rows are not filtered.** That asymmetry is the actual defect behind this spec.

So F3 should assert what the Your Words filter already asserts, extended to
every group: a row is rendered only if its target element exists and is neither
hidden nor disabled — checked at render time, in the browser, per uiLang and
per profile. A static route check is worth keeping as a cheap floor, but it is
not the gate that would have caught this.

I did not find any additional dead row. `online_spelloff` is `status: hidden`
with its flag defaulting **false** (`src/flags.rs:116`), so it should not render
at all; worth a tap-test to confirm it does not, since a hidden-status row that
renders would be the same bug class.

## F4 — Letter Forge icon: confirmed, and it is the only collision

`letter_forge` and `bee_sim` both carry 🐝 — the one duplicate among all 22
drawer-visible rows. 🔨 (U+1F528) is unused. F4 can proceed as written; the
no-duplicate-icon static check is cheap and would have caught this.

## Spec metadata that has drifted

- **Target build 220** — TestFlight is at **258**. Builds 220 through 258 have shipped since this number was written, and acceptance test 3 compares row order against "build 219".
- **Acceptance test 3** expects the Spelling group to be exactly Practice, Spelling Bee, The Climb, Daily Challenge, Spell Off. That omits **`versus`** (⚡, "Head to Head", status core), which is a signed D-N6 member of the group. The expected list needs `versus` added, or an explicit decision to remove it.

---

## What I need from Eric before Phase A

1. **C2**: which reading of the Spell It row — (a) leave, (b) make arrival legible, or (c) remove on navigation grounds and amend the signed D-N6 test? D3 depends on this.
2. **C4**: may the telemetry enum keep `say_it` as a historical wire value, narrowing D1's "everywhere"?
3. **D4 is resolved by the census**: mic stays. Confirming only.
4. **Acceptance 3**: add `versus` to the expected row list, or decide its fate separately.

D1 (remove Say It), F3 (dead-row gate, rewritten per C5) and F4 (🔨) are
unblocked and can proceed once 1 and 2 are answered. D1's deletion will also
need `src/modes.rs:586` updated, which is the recorded form of an earlier
signature — expected under D1, flagged so it is not mistaken for tampering.

---

## Correction, 2026-10-02: what the code actually did

Implementing D1 turned up two things the census got wrong, and one that
explains both reported symptoms better than anything above.

**Say It had no opener at all.** `say_it::wire()` bound only the modal's own
buttons — begin, cancel, mic, next, exit, close. Nothing bound a handler that
*opened* `sayItScrim`. The module said so itself: "the pronunciation mode is
DORMANT (no entry points; code retained)". So D1 was a dead-code deletion, not
the removal of a working mode.

**The element named `sayItBtn` was Spell It's front door.** CC-HUB-CLEANUP D1
handed the button to Spell It Out Loud and left the old name. So:

- `spell_aloud::wire()` bound the click → the Spell It guide, then play.
- `say_it::reflect_gating()` still governed its visibility — but already by `spell_aloud() && native_lang::available()`, not by Say It's own availability.

That is the whole explanation for the TestFlight report. The drawer row
labelled **Say It proxied a click to Spell It's button**, so it either opened
the wrong mode's guide or, where the native bridge was absent, did nothing at
all. A row that reads one thing and launches another is worse than a dead row,
and no check could see it because both halves were individually valid.

The census said this gating was worth checking as a possible cross-gate bug. It
is not a bug — the comment and the code agree, and Kid Mode is deliberately
allowed through because `spell_aloud` is kidSafe. That part of C1 was wrong.

The element is `spellItBtn` now, and the gate moved to
`spell_aloud::reflect_tile()`, where it describes the feature it belongs to.

**Two deliberate exemptions from acceptance test 1** (zero references outside
the changelog and the spec):

- `docs/` — six historical CC specs name Say It. Rewriting shipped history to make a grep pass would be falsifying the record; they stay.
- `archive/locales/tr.json` — the retired Turkish locale, kept as an archive of work that was cut. It ships nowhere.
