# Urdu Nastaliq per-letter feedback — spike findings

**Bottom line: per-letter Urdu feedback IS achievable — by COLOURING the letters,
not by positioning markers under them.** Two spikes:

1. **Positioned markers via geometry — NEGATIVE.** SVG `getExtentOfChar` (the
   "promising, untested" idea) cannot locate Nastaliq glyphs; no browser text API
   can. So the P0 §5 "markers beneath, positioned from geometry" approach does not
   extend to Urdu. Detail below.
2. **Per-letter colouring — POSITIVE, and it's the better answer.** Wrapping each
   letter in an inline `<span>` that sets only `color`/`background` recolours it in
   place with the Nastaliq join fully intact, in **both Chromium and WebKit**. This
   needs no geometry at all. `coloring-webkit.png` shows پاکستان correctly joined
   and per-letter coloured. This is how Urdu feedback should work.

Run them: `node spike/urdu-nastaliq/run.mjs` (geometry) and
`node spike/urdu-nastaliq/coloring-run.mjs` (colouring). Needs Playwright chromium
+ webkit. Evidence: `boxes-*.png`, `coloring-*.png`.

---

## Part 2 (the good news): per-letter colouring preserves the join

The feedback question was framed as "position a marker under each letter", which
needs per-glyph geometry Nastaliq won't give. But feedback doesn't *need* a
position — it needs to mark which letters are right/wrong. Colouring the letter
itself does that, and the browser handles the positioning.

Measured on پاکستان (width = join integrity; a shattered word is ~2.2× wider):

| technique | Chromium | WebKit (production) |
|---|---|---|
| `<span>` + `color` | preserved (100%) | **preserved (100%)** |
| `<span>` + `background-color` | preserved (100%) | **preserved (100%)** |
| `<span>` + `transform` (the F4 "pop") | shattered (224%) | shattered (224%) |
| `<span>` + `display:inline-block` | shattered (224%) | shattered (224%) |
| SVG `<tspan fill>` | preserved (100%) | shattered (224%) — unusable |

**This reconciles with F4, and corrects what F4 seemed to say.** F4 concluded
per-letter `<span>`s shatter cursive words. The real cause is not the span — it is
a **box-forming style** on it. F4's spans carried the `pop` animation
(`transform`), which makes each letter a box and stops the shaper. A span with
*only* `color` or `background` stays inline and the shaper runs straight through.

**Rules for safe per-letter feedback on cursive scripts:**
- Recolour with `color` or `background-color` only. Both are safe in WebKit.
- **Never** `transform`, `display:inline-block`, `margin`, or anything that forms
  a box — those shatter the join (confirmed, both engines).
- Use **HTML spans**, not SVG `<tspan>` — WebKit shatters tspans.
- Wrap by **grapheme cluster / akshara**, not code point, so a combining mark
  stays with its base (the akshara segmenter on `feature/hindi-akshara` already
  does this). Cost: the `pop` reveal animation is unavailable for cursive — but it
  was already gone in F4's joined path, and colour is arguably clearer feedback.

This is script-agnostic and needs no geometry, so it is a stronger answer than the
P0 §5 positioned-marker approach for **every** script, cursive or not.

---

## Part 1 (the negative): positioned markers via geometry

## The question

The per-letter feedback mechanism places a marker under each letter to show
correct/incorrect. It needs, per letter, where that letter's ink actually sits.
For Naskh (ar/fa) that's tractable — letters sit in horizontal sequence on one
baseline. **Nastaliq (Urdu) cascades**: each letter in a ligature steps down and
to the left, so letters overlap horizontally and sit at different heights.

The canvas approach failed on this (advance widths don't describe a cascade;
11–29px error). The hypothesis under test: SVG `<text>.getExtentOfChar(i)` queries
the *shaped* run, so maybe it returns each glyph's real 2D box including the
cascade.

## Method

- **Real Nastaliq, not a fallback.** Noto Nastaliq Urdu ships on macOS
  (`NotoNastaliq.ttc`). Both engines confirmed rendering it (`document.fonts.check`
  = true; word width 93px vs 154px monospace — a fallback would match mono).
- **Both engines.** Chromium *and* WebKit, because `getExtentOfChar` on complex
  scripts is engine-defined and production is WebKit. A Chromium-only result would
  be worthless here.
- Five Urdu words exercising the cascade (اردو, کتاب, محبت, پاکستان, لھ). For each
  character: `getExtentOfChar`, `getStart/EndPositionOfChar`, `getRotationOfChar`,
  plus DOM `Range.getBoundingClientRect` as the control, plus a screenshot with
  every extent box drawn over the ink.

## What came back

**The cascade is invisible in both engines.** For every word, `getExtentOfChar`
returned boxes with **identical `y` and identical `height` (160px, the line box)**
for every character — `y`-stagger = 0. The extent is measured against the line
baseline, not the glyph. The defining feature of Nastaliq — letters at different
heights — is simply not in the data. `Range` gave the same: `y`-stagger = 0, line
boxes. This is the same wall the earlier Range attempt hit.

**The horizontal boxes disagree between engines, and WebKit's are unusable.**

- *Chromium* returned tidy, non-overlapping, right-to-left-sequential x-cells —
  which look clean but are essentially advance cells laid out linearly. They do
  **not** track where the cascaded ink is (see `boxes-chromium.png`: the boxes are
  tall vertical strips; the final ب/ن tails sit well below and left of their box).
- *WebKit* (production) returned **overlapping, non-monotonic** boxes: for
  کتاب the four character extents overlap 3 ways and are not in x-order; for
  پاکستان, 5 overlaps. A single character's extent spans much of the connected
  ligature, so you cannot map character *i* to a distinct horizontal slot.

**The screenshots are the evidence.** In both overlays the coloured rectangles are
full-line-height vertical strips that do not hug individual letters. The ink
cascades diagonally through them. No rectangle corresponds to one glyph's position.

## Why — and why no text-geometry API can fix it

The DOM/SVG text model is a horizontal run on a single baseline per line. A
per-character *extent* is that character's horizontal span against the line box —
by construction it carries no per-glyph vertical position. Nastaliq's whole
complexity is vertical and diagonal, applied by the shaper *below* that model. So
all three text-geometry APIs — canvas advances, `Range` rects, `getExtentOfChar` —
expose at most horizontal advance and never the glyph's true 2D ink box. This is
an engine/model limitation, not a bug to work around.

## What "Urdu at its best" actually takes

Colouring (Part 2) removes the reason Urdu looked blocked. The path to great Urdu
is now ordinary work, no HarfBuzz, no cultural compromise:

1. **Render in Nastaliq — bundle the font.** The app does **not** yet bundle a
   Nastaliq face (Naskh was bundled for ar/fa only, `d7fcbcf`), so Urdu currently
   depends on a device font and would fall back to Naskh or tofu where absent. Ship
   Noto Nastaliq Urdu self-hosted (woff2 subset, `unicode-range`-lazy like the
   Naskh face) so Urdu always renders in proper Nastaliq. The F4 joined-run path
   already stops the word shattering. **Biggest single win; buildable now.**
2. **Per-letter feedback by colouring** (Part 2). Recolour each akshara in place
   with `color`/`background`; join stays intact in WebKit. No geometry, no Naskh
   fallback, no HarfBuzz. This is the feedback mechanism for cursive scripts.
3. **Input + content.** F5 RTL input handling, and a native-audited Urdu word bank
   (its charset is already declared). Unchanged by this spike.

**Superseded** — the earlier framing had these as the only options; colouring beats
all three, but they are recorded because they were the pre-colouring analysis:
- ~~Word-level-only feedback for Urdu~~ — unnecessary; per-letter works via colour.
- ~~Render Urdu in Naskh~~ — unnecessary; Nastaliq keeps its per-letter feedback.
- ~~HarfBuzz in wasm~~ — only needed if a future feature genuinely requires a glyph's
  2D position (a *positioned* overlay, not colour). Feedback does not.

This does not touch ar/fa (Naskh, horizontal baseline), and the colouring approach
applies to them and to non-cursive scripts too — so it is a candidate to *unify*
the P0 §5 feedback mechanism rather than keep a positioned-marker path at all.

**Bottom line:** per-letter feedback positioned under Nastaliq ink is not
achievable with browser text APIs. If the product needs it for Urdu, that means
HarfBuzz (C); otherwise Urdu takes word-level feedback (A). Either way this is a
decision, not an implementation task — which is why it stays a stop-and-ask.
