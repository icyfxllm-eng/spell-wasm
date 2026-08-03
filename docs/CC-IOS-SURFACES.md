CC-IOS-SURFACES — Widgets, Live Activity, App Intents (BD-1)
Status: REVIEW-GATED. Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
Every home-screen and lock-screen surface is a daily, zero-effort reminder that SpellGame is a platform citizen the website can never be. These surfaces exist to make the free daily loop (Daily Challenge, streak, Climb) ambient — they are goodwill and habit, not upsell. A widget that nags or sells would poison exactly the trust that makes parents buy later.
Features
1. Daily Challenge widget (small + medium)
Shows: today's date-seeded challenge language + a "done/not done" state + current streak count. Tapping deep-links straight into the Daily Challenge. Content is Kid-Mode-safe by construction: language name, streak number, orb art — no words, no definitions (a lock screen is a public surface; a spelling word is an answer leak for a shared-device family).
2. Streak & shields lock-screen widget (accessory circular + rectangular)
Streak count + earned shield segments (Climb shield state, read-only). Shield art reuses existing assets; no new drawn strings.
3. Climb Live Activity — BD-D1, pending sign-off
During an active Climb run: current segment, shield forging state, per-run progress. Ends with the run; never persists after. If BD-D1 is cut, stub the ActivityAttributes behind a disabled flag (season-2-config-stub precedent) and build nothing else.
4. App Intents / Siri Shortcuts
Intents: "Start Daily Challenge", "Practice spelling", "Practice [language]" (parameterized over the registry's entitled+enabled language list only). Intents open the app to the mode; no headless gameplay, no spoken answers through Siri (voice input discipline: a spoken full word is never accepted anywhere).
5. Spotlight indexing
Registered modes (from `modes.json`, platform- and edition-filtered) indexed as app entities so "Spell Racing" in Spotlight opens the hub tile.
Decisions
* D1 = BD-D1 (Live Activity in/out).
* D2 (decided): Widget extension gets no network entitlement. State flows one way: app writes a snapshot (streak, done-flag, shield state, entitled language list) to the App Group container on session end; widgets only read it.
* D3 (decided): Widgets render nothing entitlement-gated as locked/teased — a surface either shows what the player owns or omits it. No 🔒 icons on the home screen.
* D4 (proposed): Widget follows the profile last active on device; multi-kid households see the last player. If you think per-widget profile pinning is needed for v1, stop and ask.
Invariants
* I1: Zero purchase surfaces, prices, or Complete mentions in any extension, intent phrase, or notification (S2 extended to all profiles, not just Little Speller).
* I2: No spelling words or definitions ever render outside the app process.
* I3: Extensions are read-only over the App Group snapshot; they never write game state.
* I4: All widget/intent strings come from a hard-capped audited pool; adding a string = registry + audit event.
Non-goals
Do not touch scoring, Daily Challenge seeding, streak logic, or shield rules. No push notifications of any kind (local or remote) in this file. No watchOS/macOS targets v1.
Acceptance tests / Done
1. WidgetKit previews render all sizes/families for fixture snapshots (fresh install, streak 0; streak 12 done; streak 12 not-done; shields 3/5).
2. Widget extension binary links no networking symbols (CI symbol scan, same machinery as the web wall).
3. Complete a fixture Daily Challenge → App Group snapshot updates → widget timeline reflects "done" without app relaunch.
4. Every intent appears in Shortcuts; "Practice French" with French unentitled resolves to a graceful in-app landing, not an error.
5. Maestro: tap widget from lock screen → lands in Daily Challenge in ≤1 navigation step.
6. String-pool audit: every extension-visible string traceable to an audited pool entry, 15/15 languages or explicitly English-only with registry badge.
