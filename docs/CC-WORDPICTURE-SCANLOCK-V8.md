# CC-WORDPICTURE-SCANLOCK (v8) — scan-locked word typesetting

REVIEW-GATED. Supersedes v6/v7 word-layout behavior; does NOT touch the
tracer, pin hashes, or scan files. Authority: pinned scan > layout law >
legibility > aesthetics > speed.

INTENT: the tracer round is done and graded (12 subjects byte-identical,
100% precision). The failure is downstream — two geometry systems. The
fix: the scan is the ONLY geometry. Words are text-on-path along the
pinned path's arc length; glyph rotation = local tangent. Erase the
letters and you recover the pinned scan byte-for-byte. The typesetter
may choose the word and the size — never the geometry.

F1 scan-locked baselines: scan coordinates verbatim; layout never
resamples/simplifies/smooths/offsets/re-fits; no label floats; the
nudge pass is DELETED, not flagged off.
F2 fit solver, exact order: size = arc/chars clamped [floor, band max],
spacing justifies to fill exactly -> split ONLY at pre-marked segment
boundaries carried in the scan file -> INVALID + redraw. Forbidden:
squeeze, glyph scaling, path bending/stretching, endpoint overflow,
wrapping to a neighbor.
F3 collisions: shrink toward floor -> redraw shorter -> re-seed; >=3
consecutive seed failures on one path pair = subject fails
typesettability (F4 report). Baselines are immovable.
F4 typesettability gate at scan acceptance: arc >= floor x
MIN_WORD_CHARS; curvature cap per segment; inter-path clearance >= one
floor glyph height. Per-subject gate report; any failing path BLOCKS
the subject (peacock + dragon expected blocked — D-B).
F5 shape-fidelity eval: renderer EXPORTS baselines; residual_px == 0;
recall == 100% (decorative-thin excluded); precision == 100%; raster
skeleton check <= RASTER_RESIDUAL_MAX. Calibration first: every render
Eric graded crap must FAIL the eval before it is trusted.

Decided: D-A sub-floor paths merge into a parent at scan time; D-B
peacock/dragon blocked until F4-clean via scan re-authoring; D-C an
F5-failing render never ships; D-D splitting beats squeezing.
Stop-and-ask: D1 LEGIBILITY_FLOOR per band (rendered samples), D2
MIN_WORD_CHARS (propose 3), D3 curvature/raster maxima (measured on
fish+snowman goldens), D4 justify cap 2x tracking vs split.

Invariants: I1 scan hash unchanged post-render; I2 layout geometry
access read-only; I3 zero nudge fields; I4 every glyph on exactly one
path, residual 0 by construction; I5 zero word overlap at ship; I6
references/scans never in the bundle.

## Burn-down (2026-08-01)
Phase 1 begins: scan library from the pinned batch (verbatim paths +
segment marks + D-A tags) -> scanlock core (new pure crate; typesets +
exports baselines) -> tools/typeset_gate.py, render_eval.py (with the
broken-render calibration corpus), run_fixtures.py, invariant_check.py
-> 12-golden render set + D1-D4 ask to Eric.
