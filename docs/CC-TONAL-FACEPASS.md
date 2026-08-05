# CC-TONAL-FACEPASS — The Face Is Its Own Picture
[Received from Eric 2026-08-04, verbatim in chat; D1-D4 SIGNED. Amendment to
CC-MASTERPIECE-TONAL — that file governs outside the face, this one inside.]

The face: ~8% of canvas, ~80% of recognition, and was being processed at
global resolution. Fix = precision and economy, never more coverage.
F1 face-local nested re-posterize (3-4 bands, D1, local histogram; global
bands TERMINATE at the approved face contour; scope:face provenance; local
lightest band joins highlightBand). F2 iconic stroke vocabulary, CLOSED
(D2): eye = upper-lid + crease (never a loop); brow = one stroke; nose =
bridge-shadow-side + nostril-base (never an outline); mouth = lip-parting +
lower-lip shadow; chin = one crescent. Landmarks POSITION strokes, never
become them. F3 the smile is THE focal stroke — renders last, own small-
word budget, focal:true, and D4: export BLOCKS until the operator
explicitly touches it ("it looked fine" is not an exception). F4 clearance
zones (one config number) + total flow suppression on skin in v1. F5
hairline = the contrast boundary; monotonic density across it (per-piece
flip flag for inverted tonality, explicit only). F6 face path budget
(config) + geometry eval at export (0.5% anchor tolerance, eye-line tilt
bound, inter-feature ratios) with per-feature failure report. F7
generalization proven by a SECOND portrait passing with zero face-pass
code diff. Tool-side only; no learned models; LINE goldens hash-untouched.
Gate: Eric's face side-by-side (Done 7), THEN the combined TONAL+COLOR
session (TONAL D4), then TestFlight.

STATUS 2026-08-04: implemented in content-pipeline/wordpic/tonal.py same
session — face polygon from contour+brows, local 3-band re-posterize,
iconic strokes (shadow side auto-picked from local luminance), clearance
evictions, budget 19/28, geometry eval live (tilt -0.75deg, eyeSpacing/
faceWidth 0.379, mouth/jaw 0.296), focalTouched=false pending Eric's D4
touch. Export-block lints + second-portrait proof are the remaining Done
items alongside his review session.

## AMENDMENT F0 — de-light pre-pass (SIGNED: Eric, 2026-08-04, "sign the
de-light pass"). Face-local bands are computed on REFLECTANCE (single-
scale retinex: log I minus log of the face-scale Gaussian illumination
field), not raw luminance — bands follow anatomy through shadow.
Deterministic, no ML, nothing generated. Evidence: the four-panel demo
(bands-lit vs bands-de-lit) in the session review artifacts — her eyes
surface as their own regions through the socket shadows.

## STATUS 2026-08-04 (later): EXPORT MACHINERY STAGED + F7 PROVEN
- tonal_export.py (content-pipeline/wordpic/): builds mona's atomic
  LINE->TONAL registry flip (89 hosts = 78 band flows + the 11 iconic
  strokes; lipParting focal at max order; 144 boundary guides; ~96 est.
  slots inside her 12..=110 law). --flip REFUSES until the face-session
  verdicts file records ALL of: contourApproved, landmarksConfirmed,
  perFeaturePass, handsLasso, smileTouched (D4 as written), and
  twelfthFeatureNamed — the ledger decrees twelve features, the trace
  carries eleven named vocabulary strokes; Eric names or waives the
  twelfth at the session. Staged entry + gate checklist in the session
  scratchpad (staged-mona-entry.json).
- F7 SECOND-PORTRAIT PROOF — DONE. Vermeer's Girl with a Pearl Earring
  (PD, Wikimedia scan, c.1665) through the UNCHANGED pipeline: the same
  eleven vocabulary strokes name-for-name, facePaths 22/28, clearance
  drops 3, face polygon found on her tilted pose. Zero face-pass code
  diff (git-verified). Finding for the session: her honest eye-line
  tilt is +11.36deg vs mona's -0.75deg — the F6 export tilt bound must
  be per-piece (pose-relative), not a global constant.

## PER-FEATURE VERDICT (Eric, 2026-08-04, verbatim: "1-6 fail chin
passes" — the per-feature review page's card order): leftEye FAIL,
rightEye FAIL, leftBrow FAIL, rightBrow FAIL, nose FAIL, mouth FAIL,
chin PASS. perFeaturePass gate = false; export stays blocked. The
failure report's consequence: the six failing vocabulary strokes get
REWORKED (landmark->stroke mapping precision at zoom — the strokes are
3-7 points each and sit visibly off the painted anatomy at card scale)
and re-reviewed on the same page before the gate can flip. The chin
crescent's placement law is the one that survived his eye.

## SESSION VERDICTS (Eric, 2026-08-04, all verbatim)
- "1-6 fail chin passes" — machine strokes rejected 6-of-7; rework
  followed (ridge-refine + chin-smooth fit), then SUPERSEDED by:
- Operator drawing: Eric drew 8 strokes himself on the close-up draw
  page (lids as loops -> upper envelope; nose as outline -> shadow-side
  edge; doubled lip line -> median path; all chin-smooth fitted).
- "they pass drop the brows for mona" — perFeaturePass = TRUE with
  OPERATOR-DRAWN strokes (the strongest form of the gate), AND the brow
  amendment: leftBrow/rightBrow dropped from mona's requiredFeatures
  and vocabulary — the painting has no eyebrows; the twelve become TEN
  for mona; other portraits keep brows. Exporter + checklist updated.
- Hands lasso: recorded (left 6pts, right 5pts, verified on-overlay);
  handsLasso = TRUE. All TEN required features now evidence-checked.
- REMAINING GATES: contourApproved, landmarksConfirmed, smileTouched
  (D4 — the smile is now a stroke Eric drew himself).

## THE FLIP (2026-08-04): MONA IS TONAL
All five session gates green, all verbatim above ("contour approved
landmarks confirmed" closed the set; D4 "smile touched" on the
operator-drawn lipParting). tonal_export.py --flip applied atomically:
extractionClass TONAL, requiredFeatures = the amended TEN, 21 host
paths (12 tonal flows + Eric's 9 drawn strokes; lipParting focal at
max order 21) + 200 guide strokes carrying the full painting ink.
Engine battery GREEN: mona_bands_and_density, playable_everywhere,
feed determinism, focal-order CI, extraction lint, and the scoped L9
sweep across ALL languages x seeds. Exporter lessons ledgered in the
selection law: words-as-shading band INVERSION (light=small band 3,
dark=heavy band 1), min-separation between hosted lines, corner
pre-split + sub-60px crumb demotion (CJK dot law).
HONEST NOTE for Eric's eye: the legal selection is SPARSER than the
96-slot vision (34ish engine slots; density lives in guide ink). The
SEP knobs trade density vs CJK fill — his on-device verdict decides
whether to revisit.
CONSEQUENCE: the masterpiece lane (greatwave/sunflowers/scream tonal
re-authors) is UNBLOCKED; the ship-131 streak-feed is LIVE on mona's
focal smile the moment this ships.

SHIP RECORD: ship 132 = build 139 (2026-08-04 late). The flip's final
lessons, ledgered for the lane: (1) frame pieces must hug the edge for
their WHOLE length — an any-point test admitted a boundary that
wandered to 1.3px from the right hand; (2) structural separation uses
MIN distance on densified lines — median let crossings through, and
2-point straight twins dodged sparse vertex sampling entirely; (3) her
right hand lies over the left wrist: abutting lassos resolve by
placing the smaller stroke first and trimming the larger within 18px;
(4) hands host as OPEN ARCS chosen by openness (chord/arc), never
closed loops. All baked into tonal_export.py; masterpiece_stage.py
carries the same law for the lane (staged, awaiting Eric's per-piece
class verdicts + feature signatures).

## REWORK INTEGRATED (2026-08-05): _ridge_refine_smooth in tonal.py
The chin lesson is now the pipeline's own law: every iconic stroke is
ridge-refined against the de-lit reflectance (F0) then fit as ONE
confident cubic — generic, zero per-subject numbers. F7 RE-PROVEN:
the girl re-ran through the updated shared code — identical 11-stroke
vocabulary, 22/28 budget, 3 clearance drops, geometry eval unchanged
(it measures landmarks, not strokes — construction-invariant by
design). Operator-drawn strokes still outrank machine strokes at
export (mona's shipped face is untouched by this). LINE goldens
untouched (tool-side only).

## 139 FIELD BUG (Eric, 2026-08-05, verbatim: "139 has the old mona
lisa whats going on here?") — ROOT CAUSE: the registry flip shipped,
but mona is ALSO in the v8.2 scan bundle (scans.json, 366 subjects),
and every play-path gate checks spellpic::has() FIRST — scan-locked
subjects render their scan plan and never read registry paths. The
TONAL mona was aboard 139 but unreachable.
FIX (v8.3 precedence, in tree): scan_locked(p) = spellpic::has AND
extractionClass != TONAL — the operator-gated TONAL declaration
outranks the scan bundle at all five gates (feed, campaign length,
empty-state, canvas render, yearbook render; yearbook returns the
honest placeholder for TONAL until a layout export path exists).
Verified: wordpic battery, density law, streak test, full npm build
green. Awaits a ship number.
