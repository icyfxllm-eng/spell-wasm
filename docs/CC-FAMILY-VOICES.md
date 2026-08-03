CC-FAMILY-VOICES — The Kid Hears Mom Read Every Word (BD-4)
Status: REVIEW-GATED. V1 executable on greenlight; V2 is DESIGN-AHEAD / EXECUTION-BLOCKED(blockers below). Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
A child spelling words read in their parent's or grandparent's voice is emotionally unmatchable and structurally app-only. This is a trust-critical feature: it involves a family's voice data and a child's ears. Therefore the governing order, fixed: trust > voice quality > feature scope. Any implementation choice that sends voice data off-device, or lets an unverified voice read words to a kid, is wrong even if it works.
Features
V1 — Apple Personal Voice (executable)
1. Personal Voice as orb voice. iOS 17+ `AVSpeechSynthesizer` with the device's Personal Voice(s), behind the system permission prompt. The parent records their voice once in iOS Settings (Apple's flow, ~15 min); SpellGame requests access and offers it as a selectable orb voice per profile.
2. Honest availability. Personal Voice supports a limited language set (English-centric). The voice picker shows a Personal Voice option only for languages Apple supports it in, verified at runtime — never a degraded cross-language readout. Unsupported languages keep their standard voice with no badge, no tease.
3. Selection model. Parent (behind gate) approves which Personal Voices are usable; the kid picks among approved voices in their profile. Device-local by nature (Personal Voice never leaves the device — Apple's guarantee is our guarantee).
4. Pronunciation sanity gate. Before a Personal Voice is enabled for gameplay, it must pass a lightweight on-device check: synthesize a fixed audited calibration word list, run the existing whisper.cpp loopback, require distinctness/recognizability at the minimal-pair QA harness threshold (CC-LEARNING-ENGINE's harness, reused). Fail → voice stays selectable for menus/celebrations but not for word readout, with one honest parent-facing explanation.
V2 — Voice Studio Piper family voices (design-ahead only)
5. Custom family voices. Parent records a scripted prompt set in-app (script drawn from audited pools); a Piper voice model is trained; the voice ships as an offline voice pack (CC-OFFLINE-PACKS manifest schema, voice ID field). Blocked on: (a) Voice Studio pipeline shipping, (b) BD-D4 — the training-location decision. Piper training cannot run on-phone; every candidate path (Eric's backend, local Mac companion app, on-device future) has different privacy meaning. Record candidate analysis; execute nothing. If any instruction elsewhere implies uploading family recordings, stop and ask — that is a strategy-level privacy decision only Eric makes.
6. Custom-voice QA gate (binding on V2 now). No custom voice ever reaches a child's gameplay without passing the minimal-pair QA harness per language (standing Voice Studio rule; Tagalog → Paul). This clause is law even before V2 executes.
Decisions
* D1 (decided): V1 = Apple Personal Voice only. No third-party voice SDKs, ever.
* D2 (decided): Complete feature; parent-gated setup; kid-side selection free once approved.
* D3 = BD-D4 (open, blocking V2): training location/privacy path.
* D4 (proposed): Celebration lines ("Great job!") in a Personal Voice are allowed even when the voice fails the word-readout gate — the gate protects spelling audio fidelity, not warmth. Sign off.
* D5 (proposed): Grandparent angle: any Personal Voice on the device is offerable (Apple allows multiple); we add no cross-device voice transfer. Marketing copy may say "record grandma's voice on her visit."
Invariants
* I1: No voice audio or voice model ever leaves the device under V1; V2 may not change this without BD-D4 signed.
* I2: No voice reads spelling words to a kid without passing the readout gate (Personal Voice) or the minimal-pair harness (custom).
* I3: Voice selection is per-profile state; Little Speller and Kid Mode cannot enter recording/setup flows (S2 + parental gate).
* I4: Spoken-input discipline untouched: voices are output only; nothing here accepts spoken words as answers.
* I5: App-only (`platforms: ["app"]`); no web mention, symbol, or asset.
Non-goals
No celebrity voices (separate future idea, not this file). No voice sharing/export/cloud backup. No modification of TTS backend, Voice Studio pipeline, or the QA harness itself. No Android.
Acceptance tests / Done (V1)
1. Device with a Personal Voice: permission flow → approval → selection → full English session read entirely in Personal Voice (Maestro, manual audio spot-check by Eric).
2. Permission denied path: one honest state, standard voice continues, no re-prompt nagging (system rules respected).
3. Readout gate: a deliberately poor fixture voice fails the harness → blocked from word readout, allowed for celebrations (per D4 if signed), parent explanation renders.
4. Language honesty: Spanish profile shows no Personal Voice option when unsupported; zero fallback cross-language synthesis (CI + manual).
5. Network assertion: entire feature under proxy shows zero voice-related requests.
6. Kid Mode: recording/setup unreachable from any kid-facing route.
7. V2 check: repo contains V2 design docs + manifest schema affordance and zero V2 executable code paths (CI symbol check on training/upload symbols).
