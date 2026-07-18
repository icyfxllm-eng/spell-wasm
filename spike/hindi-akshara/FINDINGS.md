# CC-HINDI-PHASE0 F2 — akshara render prototype: PASS

**Result: one tile per akshara renders correctly, in both engines. Hindi's
rendering foundation holds — the akshara is a tile.** Continues the phase-0
sequence from `377ec59` (F1 segmentation + F3 nuqta gate); F2 is the render
prototype that commit names, run against its own ground-truth word set.

Run it: `node spike/hindi-akshara/run.mjs` (Playwright chromium + webkit).
Evidence: `render-chromium.png`, `render-webkit.png`.

## What it checked, and what came back

The F2 word set (क्षमा, प्रश्न, स्त्री, हिन्दी, ज़रूरी, किताब, अँधेरा), each rendered
whole and then as one tile per akshara using the **Rust core's** segmentation
(`akshara.rs` ground truth — the app consumes those boundaries; it never segments
JS-side, per D3).

**1. Conjuncts survive a lone tile — the core F2 question. PASS.** क्ष renders as
one conjunct shape in its own tile, not broken into क् + ष at the virama. स्त्री (a
three-consonant conjunct + matra) holds as one tile. The reordering i-matra कि
renders ि *before* क within its tile — logical order in, visual order out, the
shaper doing the swap. The nuqta ज़ (stored decomposed as ज + U+093C per D4) and
the candrabindu अँ both render as one shape. See `render-webkit.png`.

Why it works, and why it differs from Arabic: an akshara IS one extended grapheme
cluster, so every conjunct/matra interaction happens **inside** one tile, and
aksharas do not shape across tile boundaries. `display:inline-block` tiles are
therefore safe here — the opposite of cursive Arabic, where per-letter boxes
shatter the join (spike/urdu-nastaliq). **Devanagari can use the English-style per-unit
tile model; Arabic cannot.** That is the load-bearing difference between the two
scripts' render/feedback paths.

**2. The browser's own segmenter agrees with the Rust core — on all 7 words, in
both Chromium and WebKit.** `Intl.Segmenter('hi','grapheme')` produces exactly the
core's aksharas, including grouping every conjunct (क्ष = one). So both engines
implement UAX #29 GB9c (the Unicode 15.1 Indic-conjunct rule) that F1 relied on in
`unicode-segmentation`. This does NOT license JS-side segmentation (D3 still
stands — one source of truth), but it means the platform's Unicode is current, and
a future device-level cross-check has no surprise waiting.

**3. Normalization stability (F4 preview). PASS.** Every word renders to the same
width in NFC and NFD, in both engines — the decomposed and precomposed forms are
visually identical. This is what F4 will assert at the device level; the prototype
shows it holds.

## Where this leaves Hindi

Phase 0's question was "can Hindi work at all?" The answer across F1–F4 is now yes,
demonstrated not asserted: segmentation groups conjuncts (Rust + both browsers
agree), the akshara renders as one correct tile, and normalization is stable.

Still ahead, and all gated exactly like the other new languages (not a phase-0
concern): adding Hindi to the registry, a Devanagari keyboard + charset, a bundled
Devanagari font (as Nastaliq was bundled for Urdu — this prototype used the system
face), and native-audited word content. None of it is blocked by a rendering
unknown any more.
