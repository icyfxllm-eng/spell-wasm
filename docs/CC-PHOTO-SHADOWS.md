# CC-PHOTO-SHADOWS — Line/Density Law for the Photo Flow
[Received from Eric 2026-08-04, verbatim in chat; D1-D4 SIGNED.
DESIGN-AHEAD / EXECUTION-BLOCKED like its parent — UNBLOCKS NOTHING;
never citable to begin photo-flow execution. Amends CC-PHOTO-PICTURE v2;
inherits CC-TONAL-FACEPASS's law by signature.]

THE LAW (verbatim inheritance): "In the face, the only lines are the
vocabulary. Everything else is density." Every FACEPASS human-judgment
step is replaced on-device by an automated check or a graceful degrade —
never a shipped guess. Under uncertainty: a face with NO lines beats a
face with BAD lines; degrade to density, never invent line work.

F1 law lives INSIDE the existing v6 legalization gate (face scope allows
only vocabulary|bandFill; a boundary path = illegal, regenerate with the
extraction suppressed; rejection reasons stay reserved for truly-illegal
photos). F2 vocabulary auto-placed from landmarks under a confidence
floor, PER-FEATURE where the API allows (confident mouth + shaky eye =
mouth stroke, eye bands-only); the D4 focal-touch has NO runtime
equivalent — the floor replaces it. F3 face-local re-posterize automatic
(3-4 bands, deterministic from histogram); shadows demote to fills
structurally. F4 animals: NO human vocabulary ever; landmark presence IS
the classifier (human path vs density path); husky criterion named: no
shadow-rim line work, eyes protected, shading as density.

D1 confidence floor: calibrated in Phase B report-only mode against
Eric's grades on the 50-photo corpus, frozen at ship, re-review to
change. D2 multi-face: per-face independence. D3 animals as F4; future
animal-landmark APIs = recorded re-review. D4 degrade-never-reject;
bands-only faces are legitimate output ("moodier, softer portrait"),
not an error.

## Done #1 — design acceptance review (run 2026-08-04, texts in hand)
Reviewed against docs/CC-PHOTO-PICTURE-V2-PLAN.md (the 20-line plan;
full v2 text was chat-era and is not in the repo) + the LR references
in the ledger. ZERO CONTRADICTIONS FOUND in available text. One
WATCH-ITEM recorded, not a contradiction: LR6 "edge emphasis" must
operate in the density domain inside face scope (word weight/opacity
emphasis along edges — never new boundary paths), which is the natural
reading of the LR block (luminance->weight/opacity) and is what F1
makes structurally impossible anyway. Full-text re-review owed when
the v2 full text is re-pasted or Phase B design acceptance runs —
if the full text contradicts, STOP AND ASK per the standing rule.

## AMENDMENT F0 (inherited, SIGNED same signature 2026-08-04): when the
photo flow's era opens, face-local banding runs on reflectance (the
de-light pre-pass) and landmark detection is fed the de-lit face —
raising the confidence floor's pass rate on shadowed photos, so fewer
faces degrade to bands-only. Same determinism law: same photo, same
seed, same bands.
