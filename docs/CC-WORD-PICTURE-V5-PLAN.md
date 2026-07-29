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
  fidelity bar: recognizably the notebook sketch) + outline renders.
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

## v6 layout engine — burn-down state (2026-07-28, mid-flight)

Engine landed (src/wordpic_layout.rs): flatten M/L/Q/A → L5 normalize → L4
corner split → pool-median auto-segmentation → L1 solver (fill-the-line,
band-clamped; stacks fill-the-column) → junction-aware distance collisions →
L9 sweep test (10 × 15 × 5 seeds) + fish golden + star-pentagon + mona
band/density pins. Sweep burned 12,771 → 2,749 failures.

Remaining, by design intent (the sweep is DOING ITS JOB — it is rejecting
v5-era stroke maps whose art-strokes don't leave text clearance):
- overlap class (cat 987 worst; all pictures some): strokes drawn as ART
  lines sit closer than ~0.8×(sizeA+sizeB); fix = RE-AUTHOR manifests with
  clearance-aware geometry (L3: structural overlap must fail content build —
  it now does). Cat needs the full redo (Eric: "crammed").
- unfilled class (mona/eiffel 75 each, dragon 70…): short strokes whose
  budget can't host any pool word; fix = lengthen/merge or per-language
  budget overrides (I1 suggestions).
- mona 146 slots vs 150 target: add 2 paths or one more seg pair.

NEXT (in order): re-author manifests against the sweep → screen switches to
wordpic_layout (textLength justification, solved sizes, docked indicator
L6, expert pan/zoom L7) → L10 review artifact set for Eric → NO TestFlight
until his sign-off. CC-PHOTO-PICTURE v2 filed design-ahead, blocked on
P1/P2 (docs/CC-PHOTO-PICTURE-V2-PLAN.md).

## Animals pack (D13, 2026-07-29)

Second pack authored + sweep green (20 pics x 15 langs x 5 seeds, zero
failures): dog/butterfly/snail/duck (easy), owl/turtle/elephant (medium),
horse/peacock (hard), rhino = Durer 1515 expert headline (PD ref in
content-pipeline, never ships). Expert path band relaxed 50-90 -> 38-200
(v7 mona flagship needs 150-200). Review artifact (L10 + v7 outline-ID gate)
sent; NO ship until Eric approves. v7 spec filed: CC-WORD-PICTURE-V7-PLAN.
