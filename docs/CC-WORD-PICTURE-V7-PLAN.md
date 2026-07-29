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
