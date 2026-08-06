# CC-PICTURE-COLOR — Subject-Appropriate Color for Every Spell Picture
[Received from Eric 2026-08-04, verbatim. Queued: "after all the read mes are completed". D1–D5 SIGNED as written; deviations = STOP AND ASK.]

Status: REVIEW-GATED on Eric's on-device pass of the color goldens. Decisions D1–D5 are signed as written; if any proves wrong in practice, STOP AND ASK — do not silently do the other thing. Scope owner: Spell Picture rendering (curated bank) + path-authoring tool. Standing rule applies: conflicts with other instruction files that imply a strategy reversal → STOP AND ASK. Do not infer.

Intent: A peacock spelled in monochrome wastes the peacock. Every picture should render in its subject's true colors — teal-and-gold tail feathers, blue morpho wings, Mona Lisa's actual earth tones — while Numbers & Alphabets stay deliberately neutral. Governing principle: color is data curated at authoring time, never generated at runtime; everything is looked up.

Features:
1. Registry palette block — palette: [{id, hex, role}] (8–12 max) per subject; per-path paletteRef; authoring in the path tool (masterpieces: sampled from PD reference then Eric-approved; natural subjects: photo-sampled or hand-picked; flags: exact official values under the WAVE2 palette-lock CI). Build-time registry content, no network.
2. Region-true assignment via the v7.4 containment tree — part-node defaults, per-path override, unresolvable color = lint failure, no default-gray escape hatch.
3. Zero layout impact — HARD INVARIANT. Color is fill only; solver never receives palette data. CI: full solve color-on vs color-off byte-identical, every subject.
4. Legibility floor — 3.0:1 contrast at/above large-glyph threshold, 4.5:1 below; CI lint; live contrast in tool; D2 tripwire (faithful-vs-floored side-by-side, Eric decides — dark Rembrandt-register expected).
5. State separation stays monochrome — outlines/active-path neutral unchanged; a word takes its path's color when it lands; micro scan strokes stay plain.
6. Category-level exclusion as data — colorEnabled default false for Numbers & Alphabets, true elsewhere, per-subject override; `if (category === ...)` banned.
7. Downstream inheritance — FINALE exports + gallery re-render pick color up free (D3 retroactive signed); photo-flow out of scope (LR block in CC-PHOTO-PICTURE).

Decisions (SIGNED): D1 per-path flat fill v1 (gradients/shading stay in photo-flow LR). D2 sampled-then-approved with tripwire; Mona Lisa is the MANDATORY side-by-side approval case first. D3 retroactive gallery color yes. D4 no player toggle v1 (classic-monochrome = disabled v2 stub). D5 rollout: dog + eiffel first as color goldens; rest follows v8.2 re-author queue; horse/peacock/dragon wait for scan re-author exit.

Constraints: don't touch layout solver/capacity/junctions/scan pipeline/scoring/FINALE animation/active-path/In Progress strip/picker. No runtime generation, no variation, deterministic renders. Schema must not preclude multiple named palettes later (v1 reads index 0). App-only (PLATFORM wall). No new subjects/packs/art. No telemetry.

Done: (1) byte-identical layout CI, (2) resolution lint w/ deliberately-unresolvable test entry, (3) contrast lint both thresholds, (4) Numbers & Alphabets pixel-identical image diff, (5) dog+eiffel goldens hash-pinned + existing regression suite green, (6) Mona side-by-side approved IN THE TOOL before any other masterpiece, (7) gallery retroactive test, (8) Eric on-device pass of color goldens — final gate.

## BUILT 2026-08-05 (Eric: "lets build all the unbuilt code please")
The renderer seam and every automatable Done item now exist:
- Resolvers in wordpic.rs: `path_color` (F2, pure lookup) and
  `color_enabled` (F6 — category exclusion AS DATA; the learn shelf is
  neutral by default, per-subject override wins, and no `if category ==`
  exists at any call site).
- The seam: scan paths carry an optional palette ref (`RawPath.c`),
  the Plan carries the subject + index-aligned refs, and scanlock_svg
  fills LANDED word glyphs only. Outlines, pinned strokes and micro scan
  stay monochrome (F5) — asserted byte-for-byte by test.
- Goldens authored (D5 order): dog (6 colors) + eiffel (4), every path
  refed, all values clearing the 4.5:1 small-glyph floor vs #0e1420.
- Done #1 byte-identical layout CI, #2 resolution lint, #3 contrast
  lint (both thresholds), #4 Numbers & Alphabets cannot take color even
  with a planted ref, #5 goldens render their palette, #7 gallery
  re-render is retroactive (runs store words, never colors) — ALL GREEN.
STILL ERIC'S: #6 the Mona faithful-vs-floored verdict (artifact
a30c2f42) — no masterpiece palette is authored until he calls it — and
#8 his on-device pass of the goldens.
