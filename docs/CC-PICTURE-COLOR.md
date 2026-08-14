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

---

## Done #6 CALLED, 2026-08-13 — neither faithful nor floored

Eric asked for the verdict; it is **neither**, and the measurement says the
question was framed around the wrong variable.

**Faithful is unreadable on the current ground.** Mona's regions sampled from
`ref/mona-lisa.jpg` and scored against `#0e1420`, which is what the glyph colour
sits on:

```
face 4.20 (large only)   hands 2.52   hair/veil 2.79   dress 5.20 (passes)
sleeve 1.11   landscape L 2.26   landscape R 2.70   dark surround 1.01
                                            → 7 of 8 fail the 4.5 small-glyph floor
```

Even the face misses. Words in the sleeve or surround colour are invisible, not
subtle.

**Floored is a different painting.** Lifting each failing colour to 4.5:1 costs
+10 to +53 points of lightness, and because the shadows are near-black with a
faint violet cast, flooring amplifies it:

```
#1c1223 → #966db4      #120a1e → #916acd      #331e24 → #a96c7e
```

Leonardo's shadows become lavender and dusty pink. That is not the Mona Lisa in
any register, "dark Rembrandt" included.

**Colouring only the paths that pass** gives a floating gold bodice. Worse than
either.

### The variable was the ground, not the palette

All three fail for one reason: the painting's luminance sits almost entirely
below what a near-black canvas can carry. Same regions, same faithful colours,
scored against candidate light grounds:

```
ground              fails of 8 (4.5:1)   the two that fail
#0e1420 current              7           (everything but the dress)
#f4efe4 warm off-white       2           face 3.83, dress 3.09
#e8e0d0 gallery linen        2           face 3.35, dress 2.70
#ffffff plain white          2           face 4.39, dress 3.54
#faf6ec paper cream          2           face 4.07, dress 3.28
```

The only regions that struggle on a light ground are the MID-tones, which is
expected — a mid-tone is the hardest value against either extreme. And both
clear the 3.0:1 **large-glyph** threshold on warm off-white, plain white and
paper cream. So:

> **On a light ground at large-glyph size, the faithful Mona palette passes
> completely.** No flooring, no lavender, no compromise.

`#e8e0d0` is the one to avoid: the dress lands at 2.70 and fails both thresholds.

### What this decides and what it leaves open

- **Mona stays monochrome today.** Not pending a verdict any more — pending a
  ground. Recorded so nobody re-runs this analysis.
- **The colour system is not implicated.** dog and eiffel are bright subjects
  clearing 4.5:1 on the dark ground; D5's rollout order was right and stands.
- **D2 picked the wrong pilot.** Making the darkest painting in the bank the
  mandatory first colour case guaranteed this outcome. That is a lesson about
  test-case selection, not about colour.
- **Still Eric's:** whether masterpieces render on a light ground, and at what
  glyph size. That is a renderer change, and PICTURE-COLOR F3's zero-layout-
  impact invariant covers the SOLVER — a background swap does not feed it — but
  the claim needs proving, not assuming.
- **Worth knowing before choosing a different pilot:** Great Wave and Red Fuji
  have no colour to sample, since their references are 1-bit pictograms. The
  only masterpieces with continuous-tone references are Mona and the Dürer
  rhino, and a woodcut is ink-on-paper — also a light-ground register. The
  source-scan gap blocks masterpiece colour exactly as it blocks re-tracing.

