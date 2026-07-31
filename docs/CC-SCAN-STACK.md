# CC-SCAN-STACK — On-Device Vision & Typography Toolbelt

STATUS: REVIEW-GATED, **NO CODE WRITTEN**. This file's own Done-when #8
requires Eric's review and D2/D3/D4 sign-offs *before any Phase T0 code
is written*, so filing this document is the whole of the work until he
answers. Parents: CC-PHOTO-PICTURE v2 (consumer), CC-WORD-PICTURE
v6/v8.1/v8.2 (layout law), tracer v7.x (authoring tool).
Authority order (inherited): trust > layout law > lifelikeness > speed.

INTENT: reach the realism ceiling with classical on-device vision plus
variable-font typography. No generative AI, no cloud, no invented
pixels. A capability layer consumed by the tracer tool (now) and the
player photo flow (blocked) — wired once so the same math never gets two
implementations.

## Features (as specced)
1. VisionKit — instance masks + ~76 face landmarks + saliency, expressed
   as v7.4 containment-tree parts; landmarks map to protected micro
   features (the snowman-eyes rule).
2. TonalKit — pure luminance/posterize/Sobel-flow/palette math: photo +
   seed in, identical output out.
3. TypeKit — Core Text variable weight: band -> weight + opacity, never
   size. The v6/v8.1 solver stays the only authority on size and place.
4. Per-face guarantees — minimum landmark path budget, protected eye and
   mouth paths, per-face palette region.
5. MetalKit — perf only; CI asserts CPU and GPU agree on the fixtures.
6. Runtime-portable boundary — same modules for tool and (blocked) app;
   the v7.2 lasso polygon is part of the API (the future finger-circle).

## Decisions
- D1 execution split — READ AS WRITTEN: Phase T0 (modules 1-3 + 5 inside
  the offline tool, fixtures only) is executable, app-target integration
  stays blocked on P1+P2. Note the ordering: Done-when #8 gates even T0
  behind Eric's sign-off, so nothing is written yet.
- D2 iOS 17 feature gate for instance masks — AWAITING ERIC.
- D3 Inter Variable (OFL) as the shipped variable font — AWAITING ERIC.
- D4 per-face floors (>=8 landmark paths; faces under 60px trigger the
  audited rejection) — AWAITING ERIC.
- D5 determinism — DECIDED: seed + photo bytes fix bands/palette/flow;
  Vision masks pinned by IoU >= 0.98 against goldens, and an IoU
  regression fails the build rather than silently re-goldening.

## Field note that bears on D4
The 20-subject inventory just measured a related constant: v8.2's
proposed MICRO_MAX_FRACTION of 0.10 was too tight — the subjects that
exceeded it were exactly the ones carrying features Eric asked to SEE
(rhino legs and eye 0.236, smiley eyes 0.167, snowman face 0.131). It is
now 0.25, set from the inventory as v8.2 D4 instructed. Expect the same
to be true of per-face floors: measure before hard-coding.


## Phase T0 progress (2026-07-31)

STARTED on Eric's "start when ready". Built the piece that needs NONE of
the pending sign-offs, so no decision gets pre-empted by code:

- `ios/ScanStack/` Swift package, macOS/iOS, offline-tool only. No
  app-target wiring exists (P1+P2 unchanged).
- **TonalKit (module 2)** written and green — 7 tests:
  * posterize by EQUAL-POPULATION quantiles, not equal luminance. A
    clustered photo (most portraits) otherwise collapses into two used
    bands and eight empty ones; a test asserts every band carries pixels
    and that a darker pixel can never land in a lighter band.
  * Sobel -> structure tensor -> principal direction flow field, with a
    test that vertical stripes yield flow along the stripes. This is what
    makes fur read as fur rather than hatching.
  * seeded median-cut palette, ordered dark->light, plus the contrast
    floor (drops swatches the page would swallow, never returns empty).
  * determinism assertions on all three (D5).
- vImage/Accelerate and the Metal path (module 5) can replace these
  loops later WITHOUT changing results; these fixtures are what will
  hold that promise.

BLOCKED ON ERIC, and deliberately unwritten:
- module 1 VisionKit — needs **D2** (iOS 17 feature gate).
- module 3 TypeKit — needs **D3** (which variable font ships).
- module 4 per-face floors — needs **D4** (path minimum, size cutoff).

ARCHITECTURE QUESTION FOR ERIC: the spec says "swift test green ... in
the authoring-tool target", but this project's authoring tool is Python
+ Rust (tools/, tracer-core/), not Swift. Vision/vImage/Core Text are
Apple-only, so the stack must live in Swift regardless. Options: (a) the
Swift package becomes a second authoring tool used for photo subjects
while the Python tracer keeps the clip-art ones; (b) the Python tracer
is retired and everything moves to Swift; (c) the Swift package exposes
a CLI the Python driver shells out to. Recommend (c): one implementation
of the math, the existing pipeline and its gates keep working.
