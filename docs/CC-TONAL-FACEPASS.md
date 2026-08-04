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
