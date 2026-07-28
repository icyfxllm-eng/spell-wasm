# CC-WORD-PICTURE v5 — implementation ledger

Spec lineage: v2 (semantic mapping — BANNED), v3 (fill-mask — BANNED),
v4 (calligram core), **v5 = operative** (v4 + tonal bands, authoring tool,
library growth). Origin: Eric's notebook smiley — "Smile"/"Happy" stacked as
eyes, "Terrific Excellent cool" arcing as the mouth. The words ARE the art.

## Rulings on file

- **D4 curve shaping (Eric, 2026-07-28)**: complex-shaping scripts (ar, hi)
  render curved strokes as STRAIGHT words rotated to the stroke's chord
  angle — the spike (WebKit render, scratchpad ar-spike) showed textPath
  breaking Arabic joins and glyph order, exactly the tripwire D4 names.
  Latin-class scripts keep full textPath flow; stacks are script-universal;
  CJK/hangul use per-unit placement.
- **PD reference handling**: the approved Mona Lisa JPEG moved to
  `content-pipeline/wordpic/ref/` (build-time tracing reference only). The
  bundle scan (I4) asserts it never reaches dist/ or the iOS bundle.

## Carried over from the v3 build (still valid under v5)

- Bank growth (+186 basic nouns, gates green) — independent of this mode.
- `src/wordpic.rs` campaign/State machinery (grow-only fills → becomes
  placed-words; scoped restart; LRU picker; shuffle-bag + 10k-draw property
  test) — being refit from pieces to paths + recorded words.

## Phases

- P0 ✅ v3 surfaces cleared; PD ref relocated; this ledger.
- P1 stroke-map manifests + generator (10 starter pictures; smiley = #1,
  fidelity bar: recognizably the notebook sketch) + ghost renders.
- P2 engine refit (budget-filtered seeded feed, ≥3-candidate I1, D14
  recency bias, recorded words + "Spell it again" replays) + test suite.
- P3 calligram screen (textPath/stack/angled/per-unit placement, flow-in,
  milestones, ceremony, picker, gallery, how-to, share, expert pan/zoom),
  i18n ×15, modes.json registration (menu grows to 4 — CC-HUB-CLEANUP D5
  reconciliation noted in that spec's own terms).
- P4 wordpic-check.mjs (bands, subject-tier uniqueness, archetype lint,
  provenance, bundle scan, I1 + override suggestions, legibility floor) +
  authoring tool package (`content-pipeline/wordpic/`) + README.
- P5 gates, build, live web verify (smiley start-to-finish), ship.

## Standing debts (same class as other modes)

- Playwright suites + videos: no local browsers (ledgered CI debt).
- Native-speaker audit rides the standing round for the new i18n strings.
- I5 golden render fixtures: approximated by the WebKit spike + live web
  verification until a render harness exists.
