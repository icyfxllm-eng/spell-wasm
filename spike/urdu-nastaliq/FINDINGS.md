# Urdu Nastaliq per-glyph geometry — spike findings

**Verdict: NEGATIVE. SVG `getExtentOfChar` does not solve Urdu.** The approach I
flagged as "promising, untested" is now tested, and it does not give per-letter
positions for Nastaliq in either engine — decisively not in WebKit, which is the
production engine (iOS WKWebView). Urdu per-letter feedback remains unsolved and
is genuinely a different problem from ar/fa. The stop-and-ask stands, now with
evidence for *why*.

Run it yourself: `node spike/urdu-nastaliq/run.mjs` (needs Playwright chromium +
webkit). Overlays saved as `boxes-chromium.png` / `boxes-webkit.png`.

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

## Options for Eric (Urdu only — ar/fa are unaffected)

This does **not** touch Arabic or Persian: they render in Naskh, letters keep a
horizontal baseline, and the earlier P0 spike covered per-letter feedback there.
This is specifically Nastaliq/Urdu.

- **A — Word-level feedback for Urdu.** Keep per-letter markers for ar/fa; for
  Urdu, show correctness on the whole word (or per akshara-as-typed, not
  positioned under the ink). Cheapest, ships, loses per-letter precision only for
  Urdu. Recommended unless per-letter Urdu feedback is a hard requirement.
- **B — Render Urdu in Naskh instead of Nastaliq.** Naskh is horizontally
  separable, so per-letter works, and Noto Naskh Arabic is already bundled. But
  Urdu readers strongly prefer Nastaliq; Naskh Urdu reads as foreign/wrong to many.
  This is a product/cultural call, not a technical one.
- **C — Real glyph geometry via HarfBuzz in wasm.** Shape the text ourselves and
  read actual glyph positions (x, y, advance) from the shaper. This is the only
  path that yields true per-glyph 2D boxes and thus real per-letter Nastaliq
  markers. HarfBuzz compiles to wasm and the app is already wasm — feasible, but a
  real project (bundle size, wiring the shaper's output to the render), not a tweak.

**Bottom line:** per-letter feedback positioned under Nastaliq ink is not
achievable with browser text APIs. If the product needs it for Urdu, that means
HarfBuzz (C); otherwise Urdu takes word-level feedback (A). Either way this is a
decision, not an implementation task — which is why it stays a stop-and-ask.
