# CC-CJK-INK

**Status:** D1–D7 SIGNED by Eric 2026-08-23. F0 complete. F1 is the next gate
and it needs a human hand: nothing else in this file proceeds until real ink has
been measured.

**Depends on:** CC-ZH-TONE, all features complete. The orthographic stage
consumes the phonetic stage's output: a player is asked to WRITE a character
only after the sound of that word has been graded, so tone has to be settled
first. It is.

**Blocks:** nothing yet. This is the leaf.

---

## Intent

Typing pinyin teaches the sound. It does not teach the character, and for
Chinese and Japanese the character IS the writing system — a learner who can
type 帮助 from its sound and cannot write 帮 has learned half a word. The base
game already asks "spell what you hear"; this asks "now write it".

The model is Apple's Chinese handwriting keyboard: a pad you draw on, one
character at a time, recognized as you go. That interaction is familiar,
one-handed, and needs no keyboard at all — which matters, because the reason
this app builds its own on-screen keyboard is to keep the system keyboard, its
dictation and its autocorrect out of a spelling test.

---

## Non-goals

- Do not touch tone grading. CC-ZH-TONE is closed; this file consumes its
  output and changes none of it.
- Do not build a general handwriting input for other languages. Latin
  handwriting is a different problem with no pedagogical payoff here — the
  alphabet is already on the keyboard.
- Do not teach calligraphy. Brush quality, stroke weight and aesthetics are not
  graded. The question is whether the character is right.
- Do not send ink anywhere. See Invariant 1; this is not negotiable and not a
  performance trade.
- Do not reimplement stroke capture. `src/drawing.rs` already has pointer
  events, pressure, smoothing, undo, eraser and palm rejection. It is orphaned,
  not missing.

---

## F0 — Step 0: ALREADY RUN (2026-08-23)

**Intent.** Every fix in this project that skipped step 0 tuned a dead code
path. The one thing that decides this file's whole shape is whether the
recognizer that already ships can read a drawn character.

**What was measured.** The app runs `VNRecognizeTextRequest` on device today for
photo-to-word-list, seeding both Chinese scripts and normalizing to Simplified.
Twenty characters — ten zh, ten ja — were rendered clean and again degraded
toward a pen trace (strokes thinned toward constant width, a smooth random
wobble, a few degrees of rotation), then put through that same recognizer with
`recognitionLanguages` set per language.

    zh printed   10/10        zh pen-like   9/10
    ja printed    9/10        ja pen-like   8/10

**What this establishes.** Vision reads ISOLATED CJK characters at all, which
was not obvious: `VNRecognizeTextRequest` is built for text lines and a lone
glyph could easily have fallen below its notice. It also barely degrades under
distortion. Both languages, on device, no network.

**What it does NOT establish, and this matters.** No handwriting-style CJK font
is installed on the build machine, so the "pen" condition is a degraded PRINTED
glyph, and it still reads as thinned Songti rather than handwriting. Real
writing differs in ways a font cannot fake: variable stroke width, connected
strokes, running-script forms, wrong proportions. **The test is asymmetric.**
Failure would have been decisive and it did not fail, so "Vision cannot do CJK"
is ruled out. Success is suggestive, not proof.

The two failures are informative rather than random. 語 failed in BOTH
conditions — a dense 14-stroke character, so complexity is the ceiling, not
degradation. 白 failed only when penified, and 白 thinned toward 日 is a
genuinely ambiguous shape rather than a recognizer error.

Artifacts: `tools/cjk-ink-probe/` (renderer, Swift probe, images, results).

**Done.** Complete. F1 may not start until D1–D7 are signed regardless.

---

## F1 — Real ink, before anything is built

**Intent.** F0 used a proxy. This uses a finger.

**Mechanism.** The retired pad is re-enabled behind a dev flag, on device, for
one session. Fifty characters drawn by hand — a mix of simple and dense, zh and
ja — are saved as images and put through the SAME probe F0 used. Nothing else
in this file proceeds until that number exists.

**Stop-and-ask tripwire.** If real-ink accuracy lands materially below F0's
proxy figures, **stop and ask before continuing.** The answer is then D2's
fallback, not a tuning exercise on the Vision path.

**Done.** The 50-character real-ink figure is recorded here, per language.

---

## F2 — The pad

**Mechanism.** `src/drawing.rs` returns to the module tree. It already has what
is needed and is unwired rather than unwritten. Changes required:

1. The old OCR hook called out to `window.spellOcr`, a JS global that no longer
   exists. It is replaced by a Capacitor call into the native recognizer,
   alongside the photo-list one.
2. One character at a time, with an explicit commit. Apple's keyboard commits
   on a pause; a spelling test should not guess when you have finished, so the
   commit is a tap.
3. Undo removes the last STROKE. Clear removes the character.

**Done.** Done 3.

---

## F3 — Recognition

**Mechanism.** A new plugin method beside the photo-list one: strokes are
rasterized to an image, run through `VNRecognizeTextRequest` with
`recognitionLanguages` for the study language, and the top three candidates come
back with confidences.

The candidates are shown. A recognizer that silently picks for the player turns
a wrong answer into a mystery, and the top candidate was right but low-confidence
often enough in F0 (0.30 on several hits) that hiding the alternatives would be
a worse experience than showing them.

**Done.** Done 4.

---

## F4 — What is graded

**Mechanism.** A drawn round is a SEPARATE round type from a typed one. The
typed round asks for the sound; this asks for the shape. A word can be served
either way, and the two are scored as different studies — the same doctrine that
keeps the tone drill out of the general misses queue.

Grading compares the recognized character to the expected character. Stroke
order is not graded in v1 (D4).

**Done.** Done 5.

---

## F5 — The Climb

**Mechanism.** Writing has its own difficulty ladder, and it is not the typing
ladder: 一 is trivial to write and 憂 is not, regardless of how common either
word is. Tier by STROKE COUNT, which is the honest measure of writing
difficulty and is derivable per character rather than authored.

**Done.** Done 6.

---

## Decisions

Sign or amend each. If you disagree with any, stop and ask before executing.

- **D1. Reverse the drawing retirement, for zh and ja only.**
  `REVIEW_zh_metadata.md` records drawing as retired app-wide and the App Store
  copy was rewritten to drop handwriting, stroke, 书写 and hanzi drawing.
  `src/drawing.rs` is not in the module tree — orphaned, not dormant. Reviving
  it means reversing a shipped positioning decision and re-editing that copy.
  *This is the decision the rest of the file hangs on.*

- **D2. Vision first, stroke-matching as the fallback.** F0 says Vision is
  plausible and F1 will say whether it is real. The alternative is a
  stroke-based recognizer matched against a character database, which is a
  materially bigger build and needs stroke data the repo does not have. The
  recommendation is to try the cheap path properly before committing to the
  expensive one — but the fallback is named now so it is not a surprise later.

- **D3. A drawn answer is a separate round, not a substitute for typing.**
  Typing teaches the sound, drawing teaches the shape, and collapsing them
  would let a player finish a Mandarin word without ever producing its tone.
  *Note:* CC-WORDPICTURE D8 says a spoken whole word is never accepted as a
  spelling answer, because saying it is not spelling it. Drawing is the
  opposite case — for CJK the character IS the spelling — so the guard needs an
  explicit carve-out rather than being quietly bypassed.

- **D4. Stroke order is not graded in v1.** It is the real pedagogy and it is
  also a second database and a second failure mode. Recommendation: ship
  recognition first, revisit stroke order once the pad is real.

- **D5. Japanese starts with kana, then kanji.** Kana is small, regular and
  what a learner writes first. Kanji is where the recognizer is weakest (語
  failed in both F0 conditions). Sequencing it this way puts the easy win first
  and the hard case behind a measurement.

- **D6. Little Speller does not get the pad.** Drawing a character is a
  fine-motor task well above the age this mode is for, and the mode's doctrine
  is ABSENCE rather than a locked control.

- **D7. Tier by stroke count, not by word frequency.** Writing difficulty is
  strokes; reading difficulty is frequency. They are different ladders and
  reusing the typing tiers would put 一 and 憂 in the same place.

---

## Invariants

1. **Ink never leaves the device.** No stroke, no rasterization, no recognition
   request touches the network. The photo-import path already holds this line
   and the review sheet disclosed it; this inherits the same posture.
2. **Strokes are never persisted.** The pad holds them for undo and drops them
   on commit. Nothing about a child's handwriting is stored.
3. **One recognizer path.** As with zh grading, a second path is a build
   failure, not a variation.
4. **Drawing is never the only way to answer.** A player who cannot draw, or
   will not, can always type.
5. **A drawn answer is never accepted for a non-CJK language.** The carve-out in
   D3 is exactly as wide as CJK and no wider.
6. **The pad is absent where it does not apply**, not disabled — no dead
   control in Little Speller or in Spanish.

---

## Done

1. **F0 step-0 probe** — COMPLETE, recorded above.
2. **Real ink** — 50 hand-drawn characters through the same probe, per language.
   **Pass = a number exists and Eric has seen it.** There is no threshold here
   on purpose: what counts as good enough is a product call, and inventing a
   bar before seeing real data is how a gate becomes theatre.
3. **The pad** — capture, undo, clear, commit, palm rejection, on device.
4. **Recognition** — top-3 candidates with confidences, on device, offline.
   **Pass = airplane mode changes nothing.**
5. **Round separation** — a drawn round and a typed round of the same word score
   as different studies, and a drawn answer cannot satisfy a typed round.
6. **The Climb** — stroke-count tiers, monotone by construction, verified in CI
   the way D5 monotonicity already is.
7. **Deliberate-failure piece** — a network call in the ink path, and a drawn
   answer accepted for Spanish. **Pass = both fail CI.**
8. **Eric's device pass.** Draw ten characters badly and see what happens. This
   gate closes the file; the tests do not.
