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
4. **Recognizability gate + dragon re-author.** Ghost-outline gate: Eric
   must name the subject from ghost strokes alone BEFORE word-layout work.
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
Eric IDs all 10 starters from ghosts; input-discipline sweep passes;
re-review renders delivered BEFORE any round-3 build (blocking gate).

## Burn-down state (2026-07-29)
Not started — queued behind Animals pack review. Note: Animals pack was
authored under v6 law; its strokes will need `feature` labels + scale
classes when F2/F3 land (lint will catch).
