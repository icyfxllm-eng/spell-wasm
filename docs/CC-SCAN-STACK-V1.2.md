# CC-SCAN-STACK v1.2 — Perception Enhancers Amendment

**Status: REVIEW-GATED** amendment to CC-SCAN-STACK (+v1.1). Inherits all
gating: Phase T0 (tool-side, fixtures only) executable on Eric's sign-off; app
integration EXECUTION-BLOCKED on P1 + P2. **Module 12 (path suggestion) is
tracer-tool work and is the priority item** — it feeds CC-PICTURE-BANK wave-1
authoring directly.

Parent files: CC-SCAN-STACK v1 (modules 1–7) + v1.1 (Rust TonalKit, frozen
module-1 data contract, D8 pinning), tracer v7.x, CC-PHOTO-PICTURE v2 (consumer
+ privacy posture). Authority order, inherited:
**trust > layout law > lifelikeness > speed.**

## Intent

Two additions that make the stack read a photo better, closing out Bucket A.

- **Module 11, depth.** Renders currently guess z-order from containment; a
  depth map makes near/far real — foreground pops, layers stack correctly, the
  3D feel arrives with zero generative pixels.
- **Module 12, path suggestion.** Hand-tracing is the bottleneck holding the
  re-author queue (horse, peacock, dragon) and pacing CC-PICTURE-BANK wave 1. A
  model that *proposes* paths for Eric to approve in tracer v7.x multiplies
  authoring speed while keeping every shipped path human-approved.

Both are **readers, never inventors**.

## Features

### 11. Depth module (perception layer, behind the frozen contract)

Near strokes heavy, far strokes light, real z-order.

- **Source:** Apple's on-device depth — `VNGeneratePersonSegmentation`-era APIs
  where applicable, ARKit/AVFoundation captured depth when the photo carries it,
  and Vision's monocular depth request (iOS 18+) for flat photos. The exact API
  set is resolved at implementation against the D2 iOS floor and recorded in the
  file. If OS-provided depth is unavailable for a photo, the render proceeds
  **without** depth: graceful absence. Depth is an enhancer, never a dependency.
- **Output** joins the frozen module-1 data contract as one more pure-data
  field: normalised depth map + per-part median depth. No Vision/ARKit type
  escapes the module (v1.1 #10 rule).
- **Consumption:** per-part median depth (a) orders the containment tree's
  z-stack, replacing containment-guessed order where depth is present, and (b)
  modulates TypeKit weight/opacity within existing legibility floors. Depth
  **never** changes glyph size, band assignment or word selection — styling and
  z-order only (D9).
- **Determinism:** model/API version pinned per D8; per-platform goldens; depth
  maps held to a tolerance metric vs goldens (mean absolute error bound, D10),
  not byte identity, since OS depth models may vary.

### 12. ML path suggestion in tracer v7.x (tool-side, human-in-loop)

Kill the authoring bottleneck without diluting *every shipped path is
Eric-approved*.

- **Pipeline:** edge/line extraction (Rust TonalKit Sobel/tensor output) →
  vectorise to candidate stroke paths (fit polylines/Béziers along ridge lines,
  ordered by structural prominence) → present in the tracer as a **suggestion
  layer**. Eric accepts, rejects or edits each candidate; accepted paths become
  ordinary v7.x traced paths, indistinguishable downstream.
- The classical pipeline above is **attempt #1** — it may be sufficient with
  zero learned components, which is the best possible pinning story. If it
  under-suggests on the fixture set, a pinned on-device line-drawing extraction
  model may be added behind the same suggestion interface (D11). Either way:
  suggestions never bypass review, never ship raw, and **no suggestion code
  compiles into the app target** — tracer tool only, enforced by a
  target-membership check in CI.
- **Provenance:** manifests gain an authoring-note field recording
  suggested-then-approved vs hand-traced per path. Invisible to players,
  invaluable for debugging quality drift.
- **UX contract in the tracer:** bulk accept/reject by region, edit-in-place,
  and a hard rule that an unreviewed suggestion cannot be exported — export
  blocks while any candidate is pending.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D9 | Depth boundary | **DECIDED**: depth influences z-order and stroke weight/opacity only. Any path where depth reaches glyph size, band assignment, layout-law parameters or word selection is a **build-failing violation** (property-tested like LEARNING-ENGINE D7). |
| D10 | Depth golden tolerance | **DECIDED (Eric, 2026-07-31)**: mean absolute error ≤ 0.05 (normalised depth) vs stored goldens per platform, and per-part z-**order** must match goldens exactly — ordering is what players see. |
| D11 | Learned suggester escalation | **DECIDED**: classical pipeline first. A learned model is added only if the classical path fails the module-12 eval on the fixture set, and then must be on-device, bundled and version-pinned (D8), behind the identical suggestion interface. Escalation is its own recorded decision, never a silent swap. |
| D12 | Priority | **DECIDED (Eric, 2026-07-31)**: module 12 executes first within Phase T0; module 11 follows. Note the rationale has shifted -- the horse/peacock/dragon queue was cleared by hand on 2026-07-31, so module 12's justification is now PICTURE-BANK wave 1's ~23 new pictures, which D1 signed off the same day. That makes the sequencing more load-bearing, not less. |

## Constraints and non-goals

- No generative models, no cloud, no new data flows; depth and suggestions both
  derive from the photo/reference on-device. CC-PHOTO-PICTURE privacy posture
  untouched.
- Solver supremacy unchanged: suggested paths enter the same v6/v8.1/v8.2 layout
  law as hand-traced paths, with zero special treatment.
- No suggestion code in the app target (CI-enforced). No depth dependency:
  absence degrades gracefully to today's behaviour.
- Do not modify tracer golden fixtures, pinned traces, or CC-PICTURE-BANK
  manifests beyond the additive authoring-note field.
- Do not begin module 11 before module 12's eval is green if D12 is signed —
  sequencing is scope, not preference.

## Done when

1. **Module 12 eval (the headline).** Re-author the horse from its reference via
   suggestions: total authoring time recorded and materially below the
   hand-trace baseline Eric reports; the final horse passes the full v8.2 gate
   set (junction margins, OBB overlap, decorative arc budget ≤20%) with zero
   violations; every exported path carries approved status; an export-block test
   proves a pending candidate prevents export.
2. **Suggestion determinism.** Same reference + seed → identical candidate set,
   both runs, both arches (classical path). If D11 ever escalates, the same bar
   against the pinned model.
3. **CI target-membership check** fails the build if any module-12 symbol
   appears in the app target.
4. **Module 11 eval.** The busy-background husky fixture renders with
   foreground/background weight separation visible and z-order matching golden
   exactly; a depth-absent photo renders byte-identical to the pre-v1.2 pipeline
   (graceful-absence proof); D9 property tests (10k seeded runs) show zero
   reaches into size/band/selection; depth MAE within the D10 bound on the
   50-photo corpus.
5. **Peacock and dragon re-authored** via module 12 and passing v8.2 gates — the
   re-author queue is empty.
6. **Eric's sign-offs recorded**: D10, D12 (and D11 escalation if ever invoked)
   before the respective module's code is written.

---

## Status against this file (2026-07-31)

Nothing here is started; D10 and D12 are unsigned.

Three notes that bear on the plan as written:

- **The re-author queue that motivates module 12 is now empty, by hand.** The
  horse was re-authored today (standing, head up), and the peacock and dragon
  were re-authored earlier under v8.2.1. Done #1 uses the horse as the timing
  benchmark and Done #5 asks for peacock and dragon — all three now have a
  hand-traced baseline to beat, which is *better* for the eval than a pending
  queue. Module 12's value is the pictures still to come in CC-PICTURE-BANK wave
  1, not the three named here.
- **Module 12's input already exists in Swift, not Rust.** `Flow.field` (Sobel →
  structure tensor) is built and tested in `ios/ScanStack/Sources/TonalKit/`.
  This file specifies the input as *Rust* TonalKit output, which is v1.1 section
  A — still gated on D6. So module 12 either waits on D6 or begins against the
  Swift implementation and ports later. **That sequencing question is unresolved
  and worth deciding alongside D12.**
- **A relevant field lesson from today's hand-authoring**, for whoever builds
  the suggester: the layout law refuses candidates that a human eye reads as
  fine. The rhino's tail, drawn where a tail belongs, put two baselines inside
  one corridor width and blocked the whole picture; armour creases narrow enough
  to read as creases did the same. A suggester ranking candidates by structural
  prominence alone will keep proposing paths the solver then refuses. Candidate
  scoring should include corridor clearance, or the tracer should mark refused
  candidates as such before Eric ever sees them.
