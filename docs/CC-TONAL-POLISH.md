# CC-TONAL-POLISH — the tonal lane's "even better" rollup
[Received from Eric 2026-08-04, verbatim in chat. REVIEW-GATED: five ideas
approved in principle ("roll them all up"); D2/D3/D4 parameters UNSIGNED —
"Do not begin until those are signed." Nothing executed at receipt.]

F0 gate sequencing (no new authority): 0a = PICTURE-COLOR goldens + the
mandatory Mona dark-painting color side-by-side FIRST; 0b = FACEPASS v1.1
verification (vocabulary-only debug view, squint beats round 4, play-scale
legibility). Features 1-5 judged ONLY on 0a+0b-passed renders.
F1 continuous weight within bands (LR luminance->weight math pulled
forward TOOL-SIDE via the shared module; layout byte-identical with
weights stripped). F2 feathered band boundaries (seeded interleave in a
feather zone; solver owns every placement; face scope stays Line/Density-
lawful). F3 tracking as a fourth tone axis (legibility floor re-verified
at resolved size+weight+tracking; report-only first render). F4 the smile
is the last word (focal stroke's word scheduled last; FINALE replay
inherits the climax; D2 extension = streak words feed focal strokes).
F5 multi-scale SSIM eval (three scales, REPORT-ONLY until it reproduces
Eric's recorded grades — the number earns authority, D5/D6 restated law).

OPEN SIGNATURES (file blocked until Eric signs):
  D2 — streak-words-feed-focal-strokes: include flagged default ON, or strike?
  D3 — tracking range: proposed ±8%, report-only first render, frozen after his pass.
  D4 — feather zone width: proposed 15% of local band width, single default, tool-tunable.

## SIGNATURES (Eric, 2026-08-04, verbatim: "How can we get rolling on d2
d3 and d4 and fix the failures") — recorded as signing the proposals:
D2 streak-words-feed-focal-strokes: INCLUDED, flagged, default ON.
D3 tracking range: ±8%, report-only first render, frozen after his pass.
D4 feather zone: 15% of local band width, single default, tool-tunable.
Any parameter amendable by a word. Execution order per F0/D1: 0a color
(Mona side-by-side + dog/eiffel goldens) -> 0b face verification -> F1-F5.

## 0a VERDICT (Eric, 2026-08-04, verbatim: "cascade passes lets roll with
it") — CASCADE is Mona's palette. Faithful recorded structurally-best-
but-band-0-illegal; floored recorded drifted. D5 calibration point #1:
his grade ranks cascade acceptable at SSIM .132/.143/.139 with all lints
green. Rolling F1 (continuous weight) + F3 (tracking, report-only) on
cascade renders; F2 feather folds into F1's local-luminance sampling at
preview level (the 15% interleave lands with the layout-engine render);
F4 focal-last is app-side and queues for the next tree window.

## F1+F3 VERDICT (Eric, 2026-08-04, verbatim: "the polished cascade
passes lets roll with it") — F1 continuous weight APPROVED on her.
D3 tracking range FROZEN at ±8% per its own signature ("frozen after
Eric's pass"); observed distribution at freeze: n=1314, 0.923-1.052,
mean 0.961. Post-freeze changes = recorded re-review.
D5 calibration point #2: his grade ranks polished > flat cascade,
AGREEING with the scorer's direction (.208/.173/.175 > .132/.143/.139)
— two-for-two; the number is earning its authority the specified way.
Remaining in the rollup: F2 true 15% interleave + F4 focal-last (layout-
engine side, next tree window, CI sweep as specced); 0b face session
pieces (contour approval, per-feature pass, hands lasso, D4 smile
touch); final gate = his side-by-side of fully-polished vs current.

## F2 VERDICT (Eric, 2026-08-04, verbatim: "feather passes lets roll
with it") — graded feather ON at the signed 15% for mona. The measured
trade (boundary melt 302->294, quarter-scale SSIM up, thumbnail punch
.208->.157) is ACCEPTED by his eye — D5 calibration point #3, and the
first where his grade overrides the thumbnail metric: recorded so the
scorer learns that mid-scale smoothness outranks thumb-scale contrast
when sfumato is the subject. Rollup state: 0a cascade / F1 approved /
F2 approved / F3 frozen / F4 data-landed (streak flag + CI sweep queue
for the tree window) / F5 calibrating 3-for-3 files-signed. Remaining:
his 0b face session + D4 smile touch + the closing side-by-side.

## D2 STREAK-FEED DELIVERED (2026-08-04, tree window after ship 130)
Eric's assignments verbatim: "carry on with the streak-feed apply when
130 lands"; "ship 131 for the streak-feed when gates pass".
Shipped shape (word CHOICE only — D7-legal by construction):
- WordPath.focal (serde default false): no shipped path carries it; the
  flag is dormant-by-data until mona's tonal export sets it on
  lipParting. The feature arms itself at her gate.
- flags::streak_focal — DEFAULT ON per the signed D2; override off via
  localStorage['spell_flag_streak_focal']='off'.
- game.rs pushes each validated-correct word into the session streak
  store (wordpic_layout::streak_push) and clears it at all three
  chain-break/reset sites — the words of the current unbroken chain,
  session-only, no telemetry.
- layout_feed_opt: on a FOCAL slot with the flag on, streak members
  rank ahead of the existing (fresh, rotation/fit) order. Every
  candidate still faces the same solve/overlap/frame law — an unfit
  streak word simply loses to the next legal one.
CI landed with it:
- focal_order_is_max_registry_wide (F4 sweep: focal = max export order,
  <=1 per picture, negative fixture proves the law bites) — GREEN.
- streak_words_feed_focal_slot (streak word lands on the focal slot;
  flag OFF reproduces the baseline feed byte-for-byte; focal-less
  pictures ignore the streak entirely) — GREEN.
Celebration framing rides mona's tonal export surface (the smile is
made of the words you conquered), not this engine change.
