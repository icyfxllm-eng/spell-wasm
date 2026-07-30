# CC-WORD-PICTURE v7 — Round-2 TestFlight fixes: enforcement + legibility

STATUS: REVIEW-GATED. Amends v6 (v6 layout law stays authoritative).
Source: Eric's round-2 notes + 6 annotated screenshots, July 2026.
North star: every picture looks like what it's intended to be; no word is
ever cut off or overlaps another — ON DEVICE, not just in CI.

## Round-2 verdict
Concept landed. Fish exactly right; star/snowman/mona "great steps forward".
Failed: (1) star shipped 3 overlaps + cutoffs despite a green sweep — the
law exists on paper, not on device; (2) players circled strokes they
couldn't name (fish top blob, snowman bottom layers, tower sides, mona
background); dragon fails entirely.

## Features
1. **Layout law enforced at runtime, not CI theater.** Root-cause the star
   escape (CI glyph bounds vs device: estimated widths / DPR / font
   fallback) — write the finding into the PR. Replace estimated text
   measurement with measured glyph boxes from the real font stack at device
   metrics. Runtime legality check after every solve: intersection or
   out-of-canvas → deterministic re-solve (next seed, cap 5) → deterministic
   shorter-word substitution → an illegal frame is NEVER displayed (D1).
   Fixtures: `star-overlap-r2`; star legal 25 seeds × 15 langs; CI=device
   glyph boxes within 1px.
2. **Stroke attribution lint.** Manifest gains required `feature` label per
   path; "decorative/filler/background texture" rejected. Author pass over
   all starters; deletions per D2/D3/D6.
3. **Feature-scale word budgeting.** Per-path scale class dot/small/medium/
   long (derived default, author may override class, never layout). Dot
   paths: char budget 2–4; too-long words ineligible, never squeezed below
   the v6 floor. Empty dot pool → adjacent-tier borrow (deterministic,
   logged); still empty → manifest invalid for that language, stop and ask
   (D4). CI reports per lang × tier word counts at lengths 2–5. Fixture:
   `snowman-eye-length` (15 langs × 5 seeds).
4. **Recognizability gate + dragon re-author.** outline outline gate: Eric
   must name the subject from outline strokes alone BEFORE word-layout work.
   Dragon → serpentine East-Asian silhouette (D5; head-portrait fallback;
   both fail → stop and ask). Classifier top-5 check = advisory only.
5. **Mona fidelity pass.** v5 posterize-then-trace on the PD scan; dense
   form-following paths (face/hands/hair/folds); background REMOVED (D6);
   smile authored last; verify expert wiring live (150–200 paths, pan/zoom,
   n/N resume). Side-by-side vs reference in re-review.
6. **UI chrome unmistakably UI.** Identify star's side rails + bottom line;
   UI → consolidate into the single anchored active-path indicator; artifact
   → delete; neither → stop and ask (D7). Reserved chrome styling class
   picture strokes may never use. Fixture: `star-side-rails` extends
   `detached-slots`.
7. **Input discipline.** System dictation dead on every spelling surface
   app-wide (suppress; where unsuppressable, reject at the single submit
   path via input provenance — D8, no exceptions without stop-and-ask).
   Word Picture wires the existing per-language keyboard + typing-unit
   layer (no parallel input stack). "Spell It" mic = CC-SPELL-ALOUD capture
   component wholesale, `voiceSpell` registry flag (en/es v1), no dead mic
   button on unflagged languages (D9). Evals: Maestro dictation sweep;
   whisper.cpp loopback (whole word rejected, letter-by-letter accepted,
   en+es); keyboard-parity screenshot diff ko/vi/zh/ru.

## Constraints
No v6 solver-core changes beyond the runtime check + fallback. No per-
picture layout hacks (standing ban). Fish body frozen; only the stray blob.
No new displayed strings without audit. No CC-PHOTO-PICTURE work — P1 not
proven until THIS file's acceptance passes. No scoring/entitlement changes.
No runtime art generation. No new speech pipeline.

## Done means
All 7 new fixtures + 4 v6 fixtures pass; device-metric sweep zero
violations incl. unlabeled strokes + dot budgets; root-cause writeup in PR;
Eric IDs all 10 starters from outline s; input-discipline sweep passes;
re-review renders delivered BEFORE any round-3 build (blocking gate).

## Burn-down state (2026-07-29)
Not started — queued behind Animals pack review. Note: Animals pack was
authored under v6 law; its strokes will need `feature` labels + scale
classes when F2/F3 land (lint will catch).


## v7 amendment (Eric, 2026-07-30) — operative additions

TERMINOLOGY (D11, binding): strokes rendered without words = "outline
render"; sign-off = "Eric's outline review". The prior term is RETIRED —
banned from code, identifiers, comments, PR text, and messages (renamed on
contact 2026-07-30: manifest field -> `column`, CSS -> `.wp-outline`,
render files -> `-outline.svg`).

F8 — photo-traced outlines (exactness by measurement):
- Every representational picture's outline traced from a licensed
  reference (provenance CI gate; references never ship). Geometric
  subjects (star) exempt.
- Offline tracer with a runtime-portable PURE core (image in -> outline
  paths + scale classes + fidelity metrics out; zero tool/filesystem deps;
  isolated-compile CI target). Player photo upload stays in
  CC-PHOTO-PICTURE, execution-blocked (D12).
- Fidelity gates (D10, Eric confirms numbers after first contour
  overlays): max path deviation <= 1.5% of canvas width (7.7px @ 512);
  traced silhouette covers >= 95% of reference perimeter. Recorded in the
  manifest; missing reference/license/deviation = build failure. No
  freehand-only outline ships for a representational subject.
- Re-trace this round: dragon (D5 serpentine), fish (validation-only —
  body frozen), snowman, tower, mona (feeds F5), and every animal that
  failed outline review.

ROUND-1 ANIMALS VERDICT (2026-07-30): NONE of the 10 hand-authored
animals passed Eric's outline review. All ten go through the F8 trace
pipeline with references before re-review. The pack's sweep-green
geometry work stands as engine validation only.

Done-means additions: tracer fixture (deviation + coverage on a stored
reference); contour overlay in every re-review packet; zero occurrences
of the retired term scoped to the feature; `dragon-outline-id`,
`fish-stray-blob`, `tower-extra-regions`, `monalisa-background`,
`snowman-eye-length`, `star-side-rails` fixtures.

## v7 burn-down (2026-07-30)
- F1 COMPLETE: textLength span forcing; runtime legality ladder
  (measure -> re-solve cap 5 -> shorter-word substitution); TWO root
  causes fixed (device metrics spill; feed identity bug that misfired
  same-path/seam exemptions + deleted the accept-a-collider branch).
  star-overlap-r2 + star-25-seeds fixtures green.
- F2 COMPLETE: required `feature` labels on all paths (gen + mjs lint,
  banned labels rejected); D2 fish blob deleted; D3 tower sides deleted.
  D6 mona background rides the F5 rebuild.
- NEXT: F8 tracer core (pure module + fixture) -> references (Eric
  approving downloads) -> re-trace all animals + dragon/snowman/tower/
  mona -> outline review -> F3 dot budgets -> F6 chrome -> F7 input
  discipline -> full re-review packet -> build 106.

## v7.1 / v7.2 amendments (Eric, 2026-07-30) — F8 corrected

- MASK FIRST: segment subject as a binary mask (not a luminance
  threshold); mask is human-editable and approved before tracing; gates
  measure against the APPROVED MASK (deviation vs a luminance isocontour
  is meaningless — the first run scored perfectly against the wrong
  target). Manifest records reference, license, mask hash, deviation.
- LASSO SEEDING (v7.2, from the peacock failure): the core accepts an
  optional region prompt — a rough hand-drawn enclosure; outside = hard
  background, subject found within. Lasso-first is the DEFAULT for
  low-contrast subjects. A failed unseeded auto-trace is a ROUTING event
  (to the lasso step), never an output. The lasso lives in the core's
  interface — it is the future player finger-circle gesture.
- Contour classes (only legal sources): silhouette (mask boundary),
  interior identity lines (inside mask only, ranked, budget-capped),
  one frame rectangle where the subject is a framed work.
- Smoothness lint: min segment length + max per-vertex turn angle;
  stair-step artifacts fail the build; values in the manifest schema.
- Mask containment invariant: any path in the background region is
  illegal by construction (frame path excepted).
- New fixtures: peacock-lasso (unseeded MUST fail on the stored
  reference, tool routes to lasso, seeded trace passes with the mask
  confined to the bird); monalisa-target-annotation (Eric's blue-line
  annotation stored as the acceptance exemplar — awaiting the actual
  annotation file; descriptor stored meanwhile).

## v7.3 amendment (Eric, 2026-07-30) — correction loop + pinning (D13)

Human marks are HARD constraints, never hints: green scribble = subject
(every scribbled pixel in the mask), red scribble = background, boundary
redraw splices verbatim and LOCKS (re-solves may not move it), landmark
anchors get 0.5%-canvas deviation tolerance. Iterations versioned;
Eric's acceptance hash-pins the golden trace — CI fails if a shipped
manifest's trace hash differs from its pin; changing one requires an
explicit unpin (reviewable event). Primitives live in the tool UI; the
portable core consumes them as constraint inputs (player flow may later
expose a subset). Fixture `correction-loop` covers all four authorities.

## v7.4 amendment (Eric, 2026-07-31) — structured contours: the trace is a TREE

- Part decomposition: named parts with their own closed silhouettes
  (peacock -> body + fan), defined by author part-divider strokes or
  per-part lassos (same D13 hard-constraint authority). One blob for a
  visibly multi-part subject = review failure by definition.
- Containment hierarchy: frame ⊃ figure/parts ⊃ interior features; every
  child inside its parent; frame-clip the mask BEFORE tracing (kills the
  Mona balloon-past-rectangle bug structurally, not as cleanup).
- Required-features list per subject in the manifest (Mona: face set +
  hands; peacock: rays + crest) — a missing required feature fails lint.
- New fixtures: peacock-parts (two labeled closed silhouettes partition
  the mask; rays confined to fan, crest to body; zero boundary
  crossings) and monalisa-containment (mask clipped pre-trace; required
  features present; the v7.3 balloon case reproduces as a FAILURE on the
  unclipped path).

## CC-TRACER-V7.5 — INK LOCK (Eric, 2026-07-31)

ALL 13 batch subjects FAIL Eric's hand grading while the tool printed
PASS/100% — two defects: (1) the eval was SELF-GRADING (truth derived
from the pipeline's own mask; a metric that cannot disagree with the
pipeline is not a metric); (2) ink art was treated like photographs —
for clip-art the drawn black line IS the ground truth.

- F1 class detection: ink-art vs photo, printed per subject, author
  override, misroutes report-visible.
- F2 ink pipeline: adaptive ink threshold -> morphological skeleton ->
  junction-preserving centerline vector paths (D3: words ride the
  centerline; stop-and-ask before mixing conventions). Containment tree
  unchanged; interior ink is interior-feature ink, traced not skipped.
- F3 structural completeness: every ink component with skeleton length
  >= L_min (D2: 2% bbox diagonal) must have a matching path; residual
  ink renders red in the report and fails lint; sub-L_min ink listed,
  never silently dropped.
- F4 eval rewrite: ink_recall / path_precision / component_coverage
  against the reference's ink skeleton (photo branch: against Eric's
  hash-pinned red/blue annotation). D4 gates: recall >= 0.97, precision
  >= 0.97, coverage = 100%. Legacy coverage/deviation = diagnostics
  only; the tool may never print PASS from them (D5: badge suppressed
  until calibration holds).
- F5 required-features extended (horse: 4 legs+tail+mane+head; owl:
  face disc+wings+beak+talons+branch; fish: silhouette+tail fin+dorsal
  fin+eye; elephant: trunk+ears+4 legs+tail; snowman: hat+eyes+buttons+
  arms+3 circles; duck: bill+wing+feet; rest to the same standard).
- F6 calibration gate: the 13 FAILing renders are FROZEN as the
  calibration corpus (do not delete/regenerate); tune tau_path (D1: 1%
  bbox diag) / L_min / gates until ALL 13 grade FAIL to match the
  notebook; only then may the eval grade new renders. If calibration
  can't also pass a hand-verified correct trace: stop and ask.
- F7 Mona annotation pinned: red = outer frame + full figure silhouette
  + hands/arms; blue = hair + face contour. Digitized to
  ref/mona-exemplar.json (Eric's images of 2026-07-31); his re-grade
  stays the final gate — metric PASS is necessary, never sufficient.
