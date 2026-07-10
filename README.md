# SpellGame — frontend

Rust→WASM core (`src/`) compiled with `wasm-bindgen`, served as a static PWA
(`index.html` + `dist/`), wrapped by Capacitor for iOS. UI is Rust-driven DOM;
static chrome lives in `index.html`.

## Build

```bash
npm run build        # i18n + viewport + word-list gates, then wasm → dist/
npm run sync         # build + cap sync (native)
```

`npm run build` runs three CI gates before compiling — all fail the build:

- **`i18n:check`** — locale key parity across all locales.
- **`viewport:check`** — iOS zoom-bug guards (see below).
- **`words:check`** — word-list charset/exclusion/balance/determinism.

Plus Rust `cargo test` (`norm`, `hangul`, `pinyin`, `viet`, keyboard charset +
layout-drift), and `kb:check` for on-screen-keyboard layout.

## ⚠️ iOS zoom bug — do not delete `scripts/viewport-check.mjs`

**iOS WKWebView auto-zooms the page when a focused `<input>`/`<textarea>`/
`<select>` has a computed `font-size` below 16px, and frequently fails to zoom
back out on blur.** This shipped as a real bug (screen stuck zoomed after using a
form field). The fix, and the CI lint that keeps it fixed, must both stay:

- **Every focusable field is ≥ 16px** (global floor `input,textarea,select,
  [contenteditable]{font-size:max(16px,1em)}` plus per-rule sizes). If you need
  smaller-looking text, scale with a `transform`, never a sub-16px `font-size`.
- **Viewport is locked**: `maximum-scale=1, user-scalable=no, viewport-fit=cover`
  (pinch-zoom off is fine — this is an app shell with its own Big Text mode).
- **`touch-action: manipulation`** on keys/buttons kills double-tap-to-zoom.
- A tiny **zoom-recovery guard** (iOS-only, in `index.html`) re-asserts the
  viewport meta if scale ever gets stuck > 1, and `console.warn`s when it fires.

`scripts/viewport-check.mjs` (run in `npm run build`) asserts all three. **The
lint exists because of a real bug — do not remove it.** These are global (CSS +
viewport), so the fix applies in every language automatically.
