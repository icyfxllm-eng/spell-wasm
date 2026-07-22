# Per-letter feedback on cursive scripts — standing rendering constraint

**Status: SETTLED — this is a constraint, not a proposal.** It is implemented in
`src/game.rs` (the per-akshara reveal path) and governs any future change to how
letter-level feedback is rendered. Breaking these rules shatters cursive words in
production WebKit, silently and only for Arabic-script languages.

Applies to **Arabic** (`docs/CC-MASTER-PARITY.md`, Track A) and to every other
script the feedback mechanism touches. Read this before touching CC-RTL F4/F5
render paths.

## The rule

Feedback marks *which letters are right or wrong*. It does **not** need to know
where a letter sits. Colour the letter in place and the browser shapes the ink.

- Recolour with **`color` or `background-color` only**. Both are safe in WebKit.
- **Never** `transform`, `display:inline-block`, `margin`, or anything else that
  forms a box — those shatter the join. Confirmed in both engines.
- Use **HTML `<span>`**, not SVG `<tspan>` — WebKit shatters tspans.
- Wrap by **grapheme cluster / akshara**, not code point, so a combining mark
  stays with its base.

Cost: the `pop` reveal animation is unavailable for cursive scripts, because it
works by `transform`. It was already absent from the joined path, and colour is
arguably the clearer feedback anyway.

## The measurement

Join integrity measured as rendered width — a shattered word is ~2.2× wider than
a correctly joined one.

| technique | Chromium | WebKit (production) |
|---|---|---|
| `<span>` + `color` | preserved (100%) | **preserved (100%)** |
| `<span>` + `background-color` | preserved (100%) | **preserved (100%)** |
| `<span>` + `transform` (the F4 "pop") | shattered (224%) | shattered (224%) |
| `<span>` + `display:inline-block` | shattered (224%) | shattered (224%) |
| SVG `<tspan fill>` | preserved (100%) | shattered (224%) — unusable |

Evidence: `docs/img/cursive-coloring-webkit.png`,
`docs/img/cursive-coloring-chromium.png`.

## This corrects CC-RTL F4

F4 concluded that per-letter `<span>`s shatter cursive words. **That diagnosis was
wrong, and the correction is the whole reason this document exists.** The cause is
not the span — it is a **box-forming style on it**. F4's spans carried the `pop`
animation (`transform`), which makes each letter a box and stops the shaper. A
span carrying *only* `color` or `background` stays inline and the shaper runs
straight through it.

The practical consequence: per-letter feedback on cursive scripts is **not**
blocked, and never was. Anything downstream still treating it as blocked on F4's
finding is working from a superseded premise.

## Scope and one open item

The measurements were taken on **Nastaliq**, the hardest case — a cascading script
where letters sit at different heights. The spike concluded the technique is
script-agnostic and applies to Naskh (Arabic) and to non-cursive scripts as well.

**Open:** that conclusion was reasoned, not separately re-measured on Arabic.
Re-running the width check on an Arabic word under WebKit is cheap and belongs in
CC-RTL's rendering half before `rtlSupported` flips. The rules above are expected
to hold a fortiori — Naskh sits on one horizontal baseline and is strictly easier
to shape than Nastaliq — but "expected" is not "verified".

## Provenance

Extracted from `spike/urdu-nastaliq/FINDINGS.md` (Part 2), commit `2eb0318`. The
spike also carried a **negative** result — SVG `getExtentOfChar` cannot locate
Nastaliq glyphs, so positioned markers beneath cascading ink are unachievable with
browser text APIs. That half concerned Urdu only and died with the language cut
(`docs/CC-MASTER-PARITY.md`, Phase A); it is not reproduced here.

The colouring repro harness (`coloring-run.mjs`, `coloring.html`, Playwright
chromium + webkit) was deleted with the spike. To re-verify against a future
WebKit, restore it:

```bash
git checkout 2eb0318 -- spike/urdu-nastaliq/coloring-run.mjs spike/urdu-nastaliq/coloring.html
```
