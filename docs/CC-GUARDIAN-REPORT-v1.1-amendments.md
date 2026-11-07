# CC-GUARDIAN-REPORT v1.1 (Report Days): §0 recon amendments

Status: SIGNED by Eric 2026-11-07 ("both"). The v1.1 spec body (G-D1a, G-D23..G-D29, F1..F8, A1..A13) is Eric's paste of 2026-11-07; where this file and it conflict, this file wins.

## Recon result (origin/main bf2342c0)

- Report date math lives in `reports_share.rs` (`week_monday`, `Which`), `guardian_report.rs` (`compose`, `next_ping_monday`, `profile_week`, `seed`, `route_for`, `list_key`, `parse_monday`) and `reports_share_ui.rs`. All of it routes through `window()` in Phase A. Routes and list keys are keyed on a Monday today; they re-key to the window start, and old `?week=` links keep working.
- Attempts stored UTC seconds (`ts`) with no tz offset. G-D28 is satisfied by `Attempt.tz_off_s` (option a, signed): built and pushed as e00c7e06, additive, no schema bump. Entries without it (and `ts = None`) read in the device's current zone.
- Ping: one-shot LocalNotification, id 3, shared with the DASH digest. F8 needs up to 7 pending pings, so Phase D needs an id range or a rolling horizon that keeps id 3 clear. Reschedule today fires on visibilitychange and screen init only; schedule edit and tz change are new triggers.
- LOG_CAP is 512 attempts per language. A heavy 7-day window can lose attempts, which would break invariant 1. Measure before relying on it (Phase A).
- No stored golden file exists for A2. Capture one from the current Guardian `fixture()` under `tests/golden/` before any Phase A change.

## Amendments

- **G-D27 reference.** "G-D12's value" means v1 **G-D18**: the practice list stays at 10 words per language. (v1 G-D12 is the empty-week no-ping rule, which F6 keeps.)
- **A13 scope.** The static date-math check covers the Guardian report path only. The CC-GUARDIAN-DASH files (`reports::week_over_week`, `guardian_dash.rs`) are excluded by name: they are Sunday-based, count journal mastery, and rerouting them would change DASH output and break A2. The ping's timing and empty-window check still come from `window()`; only the digest body's mastered count keeps its own math.
- **G-D26 vs MIN_GROUP.** G-D26 (at least 4 attempts and at least 2 misses per type) applies **on top of** v1's `MIN_GROUP = 3` distinct missed words per group (G-D17). Both must hold for a type to show. G-D26 does not replace the floor, so default-schedule output stays byte-identical to v1 (A2).

## Phase A (490fb34a) decisions, SIGNED by Eric 2026-11-07 ("Approve both")

- **G-D25 is kept by a forced report day, not a clamp.** When an edit would leave more than 7 days between report days, `window()` adds a boundary on the 7th day after the previous one. A clamped start would drop attempts and break invariant 1 (A1).
- **F4 fallback shows attempt count and accuracy**, not "session count": the learner does not track sessions, and F4 may add no new signal. The words for it, "Not enough practice yet to spot patterns." and "No practice this period.", land with the screen (Phase B/D) as keys in all 15 locales, translated, not English placeholders. The model already carries the facts (`Report::days`).
- A13 allows `Date::now` (a clock read) and `Date::new` (formatting) in the UI; all other date arithmetic on the report path lives in `report_days.rs`.
