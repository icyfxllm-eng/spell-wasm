# CC-LAUNCH-EN: store metadata claims, verified 2026-11-08

Each claim in the store text was checked against main at cbd6f0a9 and the release flag table (config/flags.release.json). A claim that failed was cut.

| Claim | Result | Evidence |
|---|---|---|
| Handwriting input in English ("write the letters by hand") | **FAIL, cut** | No English drawn-answer input. Quick Bee · Draw is zh/ja only, and its flags (quickbee_draw_*) ship off. |
| Big Text option | PASS | index.html bigTextToggle, wired at src/lib.rs:987 |
| Head-to-head pass-and-play | PASS, **renamed to "Spell Off"** to match the button | `vsBtn` "Spell Off" (index.html:3480); versus is a core mode, always on (src/flags.rs `is_on`). A free player gets Easy only (D21). |
| spellgame.net/app and /challenge Universal Links | **FAIL** (was not claimed in the new text) | ios/App/App/App.entitlements has no associated-domains entitlement, so no link can open the app. |
| Daily reminder notification | PASS (not claimed in the new text) | settings.remind, settings::apply_reminder (src/lib.rs:380) |
| British or American both right | PASS | data/en/accepted-variants.tsv (215 pairs incl. colour, centre, organise, grey), graded both ways (src/accepted.rs:41, src/norm.rs:167) |
| Meaning and example sentence after each answer | PASS (needs a connection) | btn.sentence, meaning.noExample |
| Spaced repetition for misses | PASS | Misses review |
| "Little Speller" | **renamed to "Spell Jr"** | The app's name for it (settings.kid). |
| Weekly parent report | **cut** | The Guardian Report rides Complete. Complete is held (D4/O4), and progress_report ships off. |
| Two purchases on the sheet, upgrade in Settings > Purchases | **reworded** | Complete and the upgrade are held from sale. The sheet only lists products the store returns. |
| Adult gate wording | filled in | parent.title "Ask a grown-up"; parent.question "What is {a} times {b}?" with spelled-out numbers 3–8 (src/agegate.rs:178) |
| Mic used by "Say It" | **renamed to "Spell It"** | tools.spellaloud.name "Spell It", under Tools & Features, spell_aloud ships off. |
| Offline Easy audio | **cut** | Easy audio is not bundled yet (F6). |
| Release notes | "First release" kept | App Store Connect has only version 1.1, REJECTED. No version has been live. |

**Met by Phase B (6697a93d and cccfc7b4).** "No account", "Data Not Collected", "nothing leaves the device" and "speech recognized on the device" rest on the Phase B compile-outs: the Climb front door and leaderboard upload, /api/check, telemetry and the Say-It server step are absent from both the App Store build and the spellgame.net build. scripts/seam-absence-check.mjs proves it on each dist. This metadata ships together with Phase B.

The Fastfile beta texts (beta_app_description and the beta review notes) dropped "Fully playable offline after first load", which is false because uncached audio needs the server, and now describe the paywall.
