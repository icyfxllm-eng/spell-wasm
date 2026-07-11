# "Hear It, Write It" — HanziWriter writing mode (adapted plan)

Stroke-order writing mode for Chinese (and Japanese kanji, v2). Player hears the
word → writes each character on a canvas → HanziWriter grades each stroke against
the known character → score feeds the existing game loop.

## Why HanziWriter (and why it supersedes the ML Kit path for writing)
For "hear it, write it" the app knows the target character, so we need
stroke-order **grading against a known glyph**, not free recognition. HanziWriter
does exactly that, and beats the ML Kit `DigitalInkPlugin` here on every axis that
matters for this mode: **no model download**, **offline immediately**, **mainland-
China reachable** (data bundled locally, no CDN), and it **teaches stroke order**.
The ML Kit scaffold (`native-plugins/digital-ink/`) stays parked; HanziWriter is
the zh/ja writing engine.

## Three deviations from the work order (the order assumed a different repo)
1. **No bundler.** This project has no Vite/webpack (classic `<script>` + native
   ESM + wasm-bindgen). The order's `import 'hanzi-writer'` and dynamic
   `import('node_modules/.../char.json')` cannot work. → We **vendor**
   `hanzi-writer.min.js` (37 KB) and **fetch** per-char JSON from a bundled
   `hanzi-data/` dir. The writing module is plain JS exposing `window.SpellWrite`
   (mirrors `audio-native.js`), not a TS module.
2. **Not zero-Rust.** The game loop is Rust-driven (Rust owns the DOM). So Rust
   must launch the quiz and receive the score, via a thin bridge (like
   `drawing.rs` → `window.spellOcr`). Small, but not none. Gating reuses
   `consts::draw_available` / `expert_requires_drawing`.
3. **Japanese has no kanji words yet.** The ja pool is 100% hiragana, so ja
   writing has nothing to grade → **v1 is zh-only**. ja-kanji writing waits on
   kanji words being added (native review) + `hanzi-writer-data-jp`.

## Proven (foundation, done)
- `npm install hanzi-writer hanzi-writer-data` (devDependencies — build-time only).
- `scripts/extract-hanzi-data.mjs`: extracts ONLY the characters in the zh word
  bank → `hanzi-data/`. Result: **99 chars, 232 KB** (vs the 40 MB full set).
  Both `hanzi-data/` and the vendored lib are gitignored (regenerated at build).
- Bundle impact ≈ **270 KB, fully offline, no network calls**.

## Remaining build (v1, zh)
1. `writing.js` (web): wrap HanziWriter lifecycle — `create` + `quiz` with
   per-difficulty config (kid: outline + hint@2 + leniency 1.6 … expert: no
   outline, no hint, leniency 1.0), `charDataLoader` = fetch from `hanzi-data/`.
   Expose `window.SpellWrite.run(char, lang, difficulty) → Promise<{mistakes,strokes}>`.
2. `src/writing.rs` (Rust): thin bridge (available / run) + per-char score
   `max(0, 1 - mistakes/strokes)`, word score = mean, mapped onto the existing
   keyboard-mode scale so leaderboards stay comparable.
3. `game.rs`: for a zh word, drive char-by-char (TTS → mount canvas → quiz →
   collect) → existing answer-submission + Misses/SRS (feed per-char mistakes so
   hard characters resurface). Progress indicator (2/4).
4. UI: full-viewport modal, `touch-action:none` + `preventDefault` on touchmove,
   canvas ≥ 260 pt, safe-area insets, undo/clear, reduced-motion-gated success
   animation.
5. Build wiring: `build-web.sh` runs the extractor + copies `hanzi-data/` +
   `vendor-hanzi-writer.min.js` into `dist/`; `index.html` loads the lib + module.
6. Tests: loader resolves 森/水/猫, fails gracefully on missing char (→ keyboard
   fallback for that word). Device test: finger input per difficulty, no scroll
   mid-stroke, quiz-cancel on Head-to-Head exit, airplane-mode offline.

## Hard gate (from the order)
Do NOT touch App Store privacy metadata autonomously. Stroke data never leaves
the device; no new analytics; no new privacy-label categories.
