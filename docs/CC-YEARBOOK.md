CC-YEARBOOK — The Spelling Yearbook (BD-5)
Status: REVIEW-GATED. Blocked on CC-FINALE's clean re-render export path shipping. Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
Parents pay for artifacts of their kid's growth, not features. The Yearbook compiles what the app already stores — Spell Picture trophy gallery (piece data + seed, re-renderable), learner-model milestones, streak/Climb history — into a printable keepsake. Everything is a re-render or a read; this file authors no new gameplay, no new data collection, and no new art. If the Yearbook needs data the app doesn't already keep, the answer is "then it's not in the Yearbook," not "let's start keeping it."
Features
1. Yearbook compiler
Per profile, per period (BD-D5 default pending: school year Sep–Jun / calendar year / Climb season / all-time — all four selectable regardless of default). Assembles, in order:
* Cover: kid's profile display name (never real-name prompts), period, one Spell Picture chosen by the parent as cover art.
* Spell Picture gallery spread: every picture completed in period, re-rendered clean via the CC-FINALE export path (never screenshots), captioned with completion date + word count.
* Milestones spread: learner-model highlights via the Guardian-Dash audited template pool (shared pool, no new strings): words mastered, skills conquered, hardest word beaten.
* Journey spread: streak history, Daily Challenge count, Climb season result, shields forged.
* First-and-favorite spread: first word spelled in period per language; most-replayed Spell Picture.
2. Output
Client-side PDF, A4 + US Letter, print-at-home margins, Noto/OFL fonts (practice-sheet precedent). Deterministic: same profile + period + template version = byte-identical PDF.
3. Export & sharing
Share sheet / Files / add-only Photos (CC-FINALE permission posture). Exports metadata-free. Kid Mode: Yearbook is viewable in-app; export/print is parent-gated.
4. Graceful sparsity
A period with little play produces a shorter honest book ("not enough yet" below a floor of content), never padded filler, never guilt copy.
Decisions
* D1 = BD-D5: default period. Proposal: school year (Sep 1 – Jun 30), matching how parents think about progress.
* D2 (decided): Complete feature. Free tier: parents can preview the current period's cover + gallery spread in-app (real data), export locked behind Complete — the artifact itself is the upsell moment, honest and visible.
* D3 (decided): Wordmark on the back cover only; interior pages are the kid's, not marketing (tighter than CC-FINALE share-card D2 because a printed book lives on a shelf).
* D4 (proposed): Photo Picture pieces (when unblocked) are excluded from Yearbook exports by default, includable per-piece by the parent — aligning with PHOTO-PICTURE's photo-derived-content caution. Sign off.
* D5 (proposed): One "Yearbook is ready" local notification when a period ends, opt-in, generated on-device. Sign off (it is the only time-based prompt in the batch).
Invariants
* I1: Read/re-render only — zero writes to gallery, learner model, or stats; zero new persistent data beyond the parent's cover/inclusion choices.
* I2: All prose from the shared audited template pool; unaudited language → numbers-and-pictures layout (definitions-dark pattern, same as Guardian Dash I4).
* I3: Deterministic output (golden-testable), metadata-free.
* I4: Entirely on-device; no print service, no cloud (a mail-order printed-book service is a recorded future idea requiring its own file).
* I5: App-only (`platforms: ["app"]`).
Non-goals
Do not touch CC-FINALE's export renderer, the trophy gallery schema, or learner-model computation. No photo uploads, no social sharing surfaces, no templates marketplace, no per-page customization v1 beyond cover choice.
Acceptance tests / Done
1. Fixture profile (rich year) → deterministic PDF golden, A4 + Letter, opens and prints with correct margins.
2. Sparse fixture → short honest book; sub-floor fixture → "not enough yet" state; zero filler pages in either.
3. Gallery spread pictures are byte-identical to CC-FINALE clean exports of the same pieces (re-render equivalence test).
4. Metadata scan on exported PDF: zero EXIF/XMP/document-info identifying fields.
5. Kid Mode: view works, every export/print path parent-gated including share-sheet deep links.
6. Network assertion: full compile + export under proxy = zero requests.
7. Free-tier fixture: preview renders, export blocked with the single Complete surface (S2-compliant placement, absent in Little Speller).
8. Unaudited-language fixture renders the I2 fallback layout.
