CC-BUY-DRIVERS — Batch Orchestrator: Why the App, Not the Website
Status: REVIEW-GATED. Five subordinate files. Subordinate files stay authoritative on their own scope; this file governs order, shared law, and cross-file conflicts. Conflict implying a strategy reversal → stop and ask.
Intent
The web/app split is a capability gap, not a policy: web = try the game, app = your family lives here — your photos, your voices, your kid's learner model, your keepsakes. Every feature in this batch must be something spellgame.net physically cannot replicate, or it doesn't belong in the batch.
Scheduling law
Photo Picture unblocking (P1: v6/v8.x layout engine proven + fish golden holding; P2: D10 tool shipping a pack's expert piece) outranks everything here. No BD file may consume effort that delays P1/P2. If a BD task and a layout-engine task compete, the layout engine wins — silently deprioritizing Photo Picture is the batch's forbidden failure mode.
Build order
1. BD-1 CC-IOS-SURFACES — cheapest, ships nearest build 56 era
2. BD-2 CC-OFFLINE-PACKS — infrastructure other files reuse
3. BD-3 CC-GUARDIAN-DASH — blocked on CC-LEARNING-ENGINE L1 surfaces existing
4. BD-4 CC-FAMILY-VOICES — V1 executable; V2 design-ahead
5. BD-5 CC-YEARBOOK — blocked on CC-FINALE export path shipping
Stop and ask before starting each file (per-file greenlight, arcade-batch precedent).
Shared law (inherited by all five, build-failing)
* S1: All privacy posture from CC-LEARNING-ENGINE D5 is global: everything on-device, zero telemetry, no engagement optimization, no server-side accounts.
* S2: Little Speller zero-purchase-surface invariant applies everywhere, including widgets, notifications, and exports.
* S3: App-only features declare `platforms: ["app"]` in the registry (CC-PICTURE-PLATFORM law); web symbol scan denylist auto-extends.
* S4: No new displayed strings outside hard-capped audited pools; anything user-facing in 15 languages rides the registry/audit machinery.
* S5: One codebase, edition profiles only. Education edition inherits each feature only where its file says so.
Cross-file decisions awaiting Eric
* BD-D1: Climb Live Activity — include or cut from BD-1 v1?
* BD-D2: Offline pack granularity — per-language or per-language-per-tier?
* BD-D3: Guardian Dash free teaser — English dashboard free, Complete unlocks all languages?
* BD-D4: Family Voices V2 training path — where Piper training runs is undecided; V2 executes nothing until this is signed.
* BD-D5: Yearbook default period — school year (Sep–Jun) vs calendar year vs Climb season.

---
DECISION LEDGER (Eric, 2026-08-02 — recorded at greenlight)
* All five files greenlit 2026-08-02, in order BD-1..BD-5.
* BD-D1: Climb Live Activity CUT from BD-1 v1 — ActivityAttributes stubbed behind a disabled flag, nothing else built.
* BD-D2: per-language pack granularity. PACKS-D2: 150 MB budget signed. PACKS-D3: auto-suggest signed.
* BD-D3: signed — full English dashboard free, Complete unlocks all languages.
* BD-D4: OPEN — V2 Family Voices executes nothing until signed.
* BD-D5: CALENDAR YEAR default (overrides the school-year proposal; all four periods selectable).
* IOS-D4: last-active profile, no per-widget pinning v1. VOICES-D4: celebrations allowed from gate-failed voices — signed. VOICES-D5: multi-voice + grandma copy — signed. YEARBOOK-D4: photo pieces excluded by default — signed. YEARBOOK-D5: opt-in ready notification — signed.
* CC-REPORTS decisions (Eric, 2026-08-02): D3 SIGNED — new on-device per-keystroke timing capture approved (the batch's only new data collection; D3 recon evidence: no timestamps existed — input_provenance counts only, misses/wordstats word-level). Hesitation Map ships in v1. D4: cross-language floor = >=2 languages with >=100 scored attempts each (stricter than the 50 proposal). D6 SIGNED: kid tab may ship before Guardian Dash. CC-REPORTS itself awaits its per-file greenlight.
* CC-CALENDAR v2 decisions (Eric, 2026-08-02): D1 final per v2 (kid-origination, I7 CI-enforced). D2 SIGNED (recap+goals free; multi-week planner Complete). D3 SIGNED — the daily journal is the batch's second sanctioned new-data store (date-granularity, outcomes-only, 730-day cap, ReportsQuery sole reader). D4: 5 new words per WEEK (adjustable 3-15). D5 SIGNED: goal ring joins the widget snapshot. File awaits per-file greenlight.
* FOR THE RECORD (Eric, 2026-08-02): "all these read mes are app exclusives" — every file in this batch and its amendments (BD-1..BD-5, CC-REPORTS, CC-CALENDAR) is app-only: platforms:["app"], web symbol-scan denylist auto-extends (S3), zero web mention/symbol/asset.
* CC-CALENDAR greenlit (Eric, 2026-08-02) — all eight files now live. Slots after the 121-123 ladder (depends on ReportsQuery from 121, snapshot fields from 120); journal + planner + dealt hand + cheer flow as one wave on Eric's next ship number.
* CC-TRANSLATE-TOOLS decisions (Eric, 2026-08-02): D5 — WHOLE SUITE APP-ONLY (blanket app-exclusive ruling stands; tools 1-4 join the wall, denylist auto-extends). D1 SIGNED (core+daily+Passport free; depth tools Complete). D2 SIGNED (Kid Mode: full core+Passport+daily, no camera; Little Speller: no translator). D9 decided (word-level only, no phrase path even as stub). Wave-3 gates D3/D4/D6/D7/D8 UNSIGNED — design docs only, zero executable paths, CI-checked. Slots after CC-CALENDAR. NOTE: CC-BANK-TRANSLATE (authoritative parent on gloss pivot) not yet in hand; Phase A gloss column is deadline-critical and untouchable. File awaits per-file greenlight.
* CC-TRANSLATE-TOOLS GREENLIT (Eric, 2026-08-02) — ninth file live. Slots after CC-CALENDAR in the build ladder. Wave-3 gates remain unsigned. Parent file CC-BANK-TRANSLATE still not in hand; gloss-pivot data availability governs when the core can light up (schema-first build, dark until glossAudited languages exist).
