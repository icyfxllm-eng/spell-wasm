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

**RESULT, 2026-08-23, Eric's hand on device:**

    zh  21/25   (84%)      F0 proxy said 9/10
    ja  18/25   (72%)      F0 proxy said 8/10

The proxy overstated by 6 and 8 points, which is close enough to say the F0
method was honest about its own weakness rather than useless. The drop is not a
collapse and the tripwire does not fire.

**But the absolute numbers are the thing, not the delta.** 84% means roughly one
correctly written character in six is not recognized; 72% means one in four. The
question that decides the feature is not "is Vision good" but WHAT A MISS COSTS,
and those are different answers:

  * as a GRADE, this is not usable. A player who writes 猫 correctly and is
    marked wrong one time in four learns that the app is broken, not that their
    character was. A false rejection in a spelling test is far more damaging
    than in a keyboard, where you simply try again with no penalty.
  * as PRACTICE with candidates shown (F3) and no penalty for a failed read, it
    is plausibly fine: the player writes, sees what came back, and tries again.
    The recogniser becomes a mirror rather than a judge.

**WHICH characters failed, which matters more than the total.** Seven were
reported: 蓝 語 餐 曜 験 警 議. Every one is thirteen strokes or more. (The
totals imply eleven misses and seven were listed, so four are unaccounted for --
three ja, one zh. The pattern below rests on the seven that are known.)

What did NOT fail is as informative:

  * all ten kana, 10/10 -- which MEASURES D5's kana-first sequencing rather
    than assuming it;
  * every simple character in both languages: 日 本 山 川 水 火 花 犬 猫 空,
    八 人 大 小 口 月 白 六 百;
  * the middle of the range too -- 搬, 熊, 楼, 電 sit around thirteen strokes
    and came back fine.

So this is not a stroke-count cliff, it is a dense TAIL, exactly what F0
predicted when 語 failed in both proxy conditions while 山 and 川 sailed
through.

**D7 turns out to solve this without having been chosen for it.** Tiering by
stroke count was signed as a PEDAGOGICAL call -- writing difficulty is strokes,
reading difficulty is frequency. It happens to track the recogniser's
reliability curve almost exactly: the early tiers are where Vision is solid and
the dense tail is where it is not. The limit becomes a ceiling on the Climb
rather than frustration scattered at random through the game.

**Done.** COMPLETE — the number exists and Eric has seen it.

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

**COMPLETE 2026-08-23.** All three landed, and F1's fifty-character session
exercised them on device rather than in theory: capture, undo, clear, commit and
palm rejection (primary pointer only, one active pointer) all held for fifty
drawings.

**One pad, two modes.** The pad was welded to the probe screen, which would have
forced F4 to duplicate the markup and the pointer handlers -- a second canvas and
a second wiring, which is the same mistake as a second recogniser path only
further from the recogniser. It now takes a Mode: `Probe` runs the fixed set and
scores itself, `Practice` writes the character for the word in play and scores
NOTHING. They share the surface, the strokes and the commit, and differ only in
what happens after the read.

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

**COMPLETE 2026-08-23.** Candidates render as tappable chips with their
confidence, and **"None of these" is a first-class option rather than a
fallback** — at F1's 84% and 72% it is the honest answer roughly one time in
five, and burying it would push players toward claiming a character they did not
write.

**"Airplane mode changes nothing" is now checked, not promised.** Done 4 asked
for it and nothing enforced it, so the gate scans the whole ink path — the Swift
plugin, the pad, the probe — for any network symbol, the same scan BD-D4 uses to
keep household voice recordings at home. A child's handwriting deserves the same
treatment. Full-line comments are stripped first so the files stay free to
DISCUSS the ban, since this invariant is written inside them; an inline URL in
real code still trips it. Verified by planting one.

**Done.** Done 4.

---

## F4 — What is graded

**Mechanism.** A drawn round is a SEPARATE round type from a typed one. The
typed round asks for the sound; this asks for the shape. A word can be served
either way, and the two are scored as different studies — the same doctrine that
keeps the tone drill out of the general misses queue.

**AMENDED after F1 (Eric, 2026-08-23): a drawn round is PRACTICE, not a grade.**

F1 measured 84% and 72%. Read as a grade that is unusable, and the reason is
asymmetry rather than the percentage: a player who writes 猫 correctly and is
marked wrong one time in four learns that the app is broken, not that their
character was. A false rejection costs far more in a spelling test than in a
keyboard, where an unoffered character just means you try again with no penalty.

So the recogniser is a MIRROR, NOT A JUDGE:

1. A drawn round shows the candidates and never marks a correctly written
   character wrong. A failed read costs a retry, not a miss.
2. Nothing from a drawn round enters the missed-words queue or the tone drill.
   It is practice, and practice that punishes a working hand is worse than no
   practice.
3. The player confirms which candidate they meant. That is the only place a
   drawn round produces a verdict, and it is the player's verdict.

This is what makes 84% and 72% acceptable numbers rather than blocking ones,
and it is why the feature can ship before the recogniser improves.

**Stroke count gates how far it goes.** Drawn rounds are offered on the tiers
where F1 says recognition is solid, and the dense tail is simply not served yet.
That is D7's ladder doing double duty rather than a new rule.

Stroke order is still not graded in v1 (D4).

**COMPLETE 2026-08-23, less the stroke gate.** A "Write it" affordance sits
beside a zh or ja round, absent everywhere else rather than disabled, and absent
in Little Speller per D6. It opens the pad on the word's characters ONE AT A
TIME -- asking for 帮助 on a single pad would make the recogniser read a phrase,
which is not what F1 measured. Tapping a candidate is the player's verdict and
the only verdict a drawn round produces; "None of these" is a retry.

The rule is enforced rather than described. Two symbol scans in
`ink_probe.rs` fail the build if the pad ever reaches a record
(`misses::add_miss`, `tone_drill::add`, `stats::record`, `wordstats::record`,
`note_attempt`) or the typed path (`submit_guess`, `s.answer`, `type_char`).
They are scans rather than behavioural tests because the pad runs on wasm and
the queues live behind DOM state -- naming the forbidden calls is what can be
checked, and it is the same shape of guard the zh grading law uses.

**THE STROKE GATE IS NOT BUILT, and deferring it is honest rather than an
oversight.** Gating by stroke count needs stroke counts, and the repo has none:
Unicode does not carry them and no table ships here. That data is exactly what
F5 introduces, so the gate lands with F5 rather than being faked now with a
proxy for complexity. Until then a drawn round can be offered on a dense
character the recogniser will likely miss -- which costs a retry and nothing
else, because of the amendment above.

**Done.** Done 5.

---

## F5 — The Climb

**Mechanism.** Writing has its own difficulty ladder, and it is not the typing
ladder: 一 is trivial to write and 憂 is not, regardless of how common either
word is. Tier by STROKE COUNT, which is the honest measure of writing
difficulty and is derivable per character rather than authored.

**COMPLETE 2026-08-23.** `src/stroke_counts.rs`, generated, 2316 characters
covering everything either bank can ask for.

**Two sources, because the bank holds two kinds of character.** Han counts come
from Unihan's kTotalStrokes -- downloaded rather than hand-typed, since 2234
hand counts would be 2234 chances to be wrong. Kana are DERIVED from the 46 base
forms: が is か plus a dakuten and ぱ is は plus a handakuten, so NFD plus a
two-stroke and one-stroke rule covers the voiced forms, and the ten small kana
map to their full-size counterparts because writing a small ゃ is the same three
strokes as や. Only 46 numbers were typed by hand instead of 82.

**The tier boundaries are measured, not chosen.** F1 found every miss at
thirteen strokes or more and nothing at twelve or under. So the ladder is cut
where the recogniser's reliability actually changes:

    1   1-4     kana and the simplest characters
    2   5-8     the bulk of everyday hanzi
    3   9-12    still solid
    4   13+     the dense tail — every F1 miss lived here

A word is as hard as its WORST character, not its average: 帮助 is a 9 because
帮 is, even though 助 is a 7.

**This closes the gate F4 had to leave open.** `writing_ready` holds tier 4 back,
and the "Write it" offer no longer appears on a word the recogniser will
probably miss. A test pins the boundary to F1's actual results -- the seven
characters that failed must not be offered, and the ones that read fine must be.
If the recogniser improves, that test is what has to be changed deliberately.

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
4. **Recognition** — PASSING on the parts a machine can check: top-3 candidates
   with confidences render as choices, and the gate refuses any network symbol
   in the ink path (verified by planting a URL). The literal airplane-mode run
   is still Eric's, on device.
5. **Round separation and the practice rule** — PASSING. Three tests: the pad
   reaches no record, the pad cannot reach the typed path, and practice walks a
   word one character at a time and stops. The deliberate case is covered by
   the first: a correctly drawn character the recogniser misses writes nothing,
   because the pad has no call that could.
6. **The Climb** — PASSING. Four tests: the table is sorted and covers every
   bank character, tiers are monotone by construction, a word takes its worst
   character, and the tier boundary matches the seven characters F1 actually
   missed.
7. **Deliberate-failure piece** — PASSING, both halves, each verified by
   actually breaking it rather than by reasoning that it would break.

   *A network call in the ink path* fails the gate's ink scan, which covers the
   Swift plugin, the pad and the probe. Verified by planting a URL.

   *A drawn answer accepted for Spanish* fails `ink_allowed`, which is a pure
   function precisely so it can be tested. Verified by widening it to `true`
   and watching es, en, fr and ten others fail. A second test pins the CALLER
   too: game.rs must consult the guard rather than re-implement the condition
   inline, because a test that only exercises the function while the caller
   duplicates the rule would pass while the rule leaked. Verified by inlining
   it and watching that test fail.

   Korean is excluded on purpose and has its own test saying so: it is
   CJK-adjacent with its own script, but hangul composes from jamo the keyboard
   already provides, so there is nothing there that typing does not teach.
8. **Eric's device pass.** Draw ten characters badly and see what happens. This
   gate closes the file; the tests do not.
