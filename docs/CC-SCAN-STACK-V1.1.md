# CC-SCAN-STACK v1.1 — CV Enhancers Amendment

**Status: REVIEW-GATED.** Inherits all of CC-SCAN-STACK's gating. Phase T0
(tool-side, fixtures only) is executable on Eric's sign-off; app integration
stays EXECUTION-BLOCKED on P1 + P2. **Section C is DESIGN-AHEAD ONLY** —
Android is not greenlit scope. Authority order, inherited:
**trust > layout law > lifelikeness > speed.**

## Intent

Two upgrades, both aimed at the same thing: make the photo-upload math more
deterministic and less platform-bound, without adding a single cloud call or
generative model.

- **A.** TonalKit moves from Apple's vImage to a shared Rust implementation,
  putting the tonal/flow math in the same language as the solver.
- **B.** The vision layer is formalised as the *only* per-platform seam, with
  a written substitution map, so a future Android build changes one module
  rather than the architecture.

---

## A. TonalKit in Rust (kornia-rs) — supersedes module 2's vImage plan

**#8 Rust TonalKit.** Byte-identical math everywhere, one language boundary,
solver and image math in one crate. Reimplement module 2 — luminance,
posterisation to 8–12 bands, Sobel → structure tensor → flow field, resize,
colour conversion — in the existing Rust core, using kornia-rs for the
standard ops.

The seeded median-cut palette sampler stays **hand-written Rust** (~100
lines), not delegated to any library: hand-rolled *is* the determinism
guarantee. Core Image and vImage are **removed** from the plan, not wrapped
alongside it — module 6's one-implementation rule. Metal (module 5) is
re-scoped: justified only if the Rust path misses the scan-time budget, and
under the same identical-output CI requirement if it is ever built.

**#9 Qualification spike — kornia-rs must earn its place.** Never bet the
stack on a young crate untested. Before A is adopted:

1. Implement posterize + Sobel + structure tensor via kornia-rs.
2. Run the 50-photo corpus on x86 and ARM Macs.
3. Assert byte-identical bands and flow across arch, across runs, and across
   thread counts.
4. Pin the crate version.
5. Record unsupported ops; anything missing is hand-written and logged in the
   file.

**PASS** → A is adopted, D6 flips to decided. **FAIL** (nondeterminism,
missing core ops, unacceptable perf) → the vImage plan stands unchanged and
this section is struck. The spike is Phase T0 work: tool-side, fixtures only.

## B. Vision-layer seam — module 1 hardening (iOS, unchanged scope)

**#10 Segmenter-agnostic module 1 interface.** Platform vision APIs are
pluggable; everything downstream is platform-blind. Module 1's output
contract is frozen as pure data — named instance masks, landmark point sets,
saliency region — and **no Vision type crosses the boundary**. Vision remains
the sole iOS implementation. Goldens are per-platform by definition; the
IoU ≥ 0.98 discipline (D5) is unchanged. This is an interface refactor only:
no behaviour change, verified by the existing fixture suite passing untouched.

## C. Android substitution map — DESIGN-AHEAD, ZERO EXECUTION

Recorded so the knowledge isn't lost. **No Android work is authorised by this
or any file.**

| Need | Android substitution |
| --- | --- |
| Instance masks | ML Kit Subject Segmentation — per-subject masks, the one capability MediaPipe's category segmenters lack |
| Face landmarks | MediaPipe Face Landmarker — 478-pt mesh, multi-face, on-device; richer than Vision's ~76 pts |
| Saliency | Derived: union of subject masks = salient region. No separate model. |
| Modules 2–3 math | The shared Rust crate from section A, unchanged |

Deployment constraint carried into any future greenlight: models ship
**bundled and version-pinned** (MediaPipe bundles; ML Kit's Play Services
delivery needs the bundled variant or an explicit pin). Auto-updating models
silently break the determinism eval and violate the nothing-phones-home
posture.

Cross-platform acceptance shape: the same portrait through both stacks — each
passes *name-everyone-without-hesitation* independently, each platform holds
IoU against its **own** goldens. Cross-platform byte comparison is explicitly
**not** a test.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D6 | TonalKit language | **DECIDED (Eric, 2026-07-31)**: run the kornia-rs qualification spike (#9). Adoption is NOT decided here -- the spike result decides it mechanically. PASS adopts section A; FAIL leaves the vImage plan standing and strikes the section. |
| D7 | Android status | **DECIDED**: design-ahead only. Section C confers no execution authority. Any executor finding themselves writing Android code under this file must stop. |
| D8 | Crate pinning | **DECIDED**: kornia-rs — and every model file in any future section-C build — is version-pinned. Upgrades require re-running the full determinism eval and are their own reviewed change, never a drive-by bump. |

## Constraints and non-goals

- **No OpenCV, anywhere**, including the desktop tracer. It duplicates
  Vision/Rust capability, adds a 30–100MB C++ dependency, and forking the
  math between tool and app is the exact divergence module 6 forbids.
- No generative models, no cloud, no new data flows. CC-PHOTO-PICTURE's
  privacy posture is untouched by every line above.
- Solver supremacy unchanged: this amendment produces *inputs* to the
  v6/v8.1/v8.2 layout law, never bypasses it.
- Do not touch CC-PICTURE-BANK content work, scoring, or word banks.

## Done when

1. Spike #9 report: pass/fail per criterion, corpus hashes from both arches
   attached, crate version pinned in `Cargo.lock`.
2. If adopted: the module 2 fixture suite — grayscale ramp, husky palette +
   fur-flow + glint, 4-person portrait bands — passes through the Rust path
   with results meeting CC-SCAN-STACK Done #2–#5 unchanged.
3. Interface freeze #10: module 1 emits only the frozen data contract; grep
   confirms no Vision framework type escapes the module; full fixture suite
   green with **zero** golden changes.
4. Scan-time budget re-measured on Eric's oldest supported device; Metal
   go/no-go recorded as a number, not a vibe.
5. Eric's sign-off on D6 recorded **before** the spike runs; sections A/B
   execution begins only under Phase T0's existing authorisation.

---

## Status against this file (2026-07-31)

**Nothing in section A has been started, and nothing will be until D6 is
signed off** — that is the file's own rule and it names the spike as the
gate.

What already exists that this amendment touches:

- **Module 2 (TonalKit)** is built in Swift and green
  (`ios/ScanStack/Sources/TonalKit/`, 8 tests). Under A it is a reference
  implementation to port and then delete, not a wrapper to keep.
- **Module 5** is built as an interface, not as shaders
  (`TonalBackend` / `Backends.firstDisagreement`). A's re-scoping is
  compatible with what is there: the equivalence contract stays, the GPU
  backend simply may never be written if Rust makes the budget.
- **Module 1 (VisionKitScan)** already keeps Vision types inside the module —
  `instanceMasks` returns masks, `faceLandmarks` returns point sets keyed by
  name. #10 is therefore a small formalisation plus a grep-level CI check,
  not a rewrite. It is authorised under Phase T0 and does not wait on D6.

Open question for Eric, unchanged from CC-SCAN-STACK: if the tonal math moves
to Rust, the Swift package's remaining job is Vision plus typography, which
strengthens the case for the recommended architecture — a Swift CLI the
Python authoring driver shells out to, with the Rust crate shared by both.
