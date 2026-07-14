# PR: NativeLanguageKit — iOS native language capabilities plugin

Branch `feature/native-language-kit`. **REVIEW-GATED: nothing merged, no version/build
bump (CURRENT_PROJECT_VERSION stays 47), no fastlane/TestFlight touched — build 48 is in
Apple review.**

One Swift Capacitor plugin exposing iOS's on-device linguistics (offline AVSpeech TTS,
UITextChecker word validation, NLLanguageRecognizer detection) to the Rust/WASM core, as
a **capability provider** — the web core asks "what can this platform do?" and decides;
the plugin never picks voices/languages/fallbacks. Web/PWA/Tauri get `available:false`
from the same interface, so behavior off iOS is byte-identical.

## What's built (Features 1–6)

| # | Feature | Where |
|--|--|--|
| 1 | Capability discovery `capabilities(lang)` | `NativeLanguageKitCore/Capabilities.swift`; JS `native-language-kit.js` |
| 2 | Offline TTS `speak/stop` (AVSpeech) | `Speaker.swift`; `.playback` session matches audio-native.js (silent-switch parity) |
| 3 | Audio source router | `src/api.rs` — ONE routing point, config-ordered `["server-cache","native-tts"]` |
| 4 | Real-word gate (UITextChecker) | `src/lib.rs` `saveWords` → between charset and profanity |
| 5 | Language-detect hint (NLLanguageRecognizer) | `src/lib.rs` `lang_hint`, non-blocking |
| 6 | Tests | package XCTest + Rust router/gate tests + e2e |

Architecture: the testable logic lives in a standalone SwiftPM package
`ios/NativeLanguageKit/` (iOS-targeted so `xcodebuild test` runs it on the simulator).
The thin CAPPlugin wrapper (`ios/App/App/NativeLanguageKitPlugin.swift`) marshals
Capacitor calls to it. The Rust bridge `src/native_lang.rs` reflects on
`window.SpellNativeLang` exactly like `window.SpellAudio`.

## Decisions

**D1 — Audio source order (YOUR CALL).** Shipped default is **server-primary /
native-fallback**: `["server-cache","native-tts"]` (current behavior preserved; native
AVSpeech rescues offline). Flip with localStorage `spell_audio_src` =
`native-first | server-only | native-only` (`src/api.rs::parse_source_order`,
unit-tested).
- *native-primary* → zero-latency offline audio, less Pi/tunnel load, BUT the iOS voice
  differs from the Google TTS voice on web/PWA — the same word sounds different across
  platforms, and Daily Challenge players on different platforms hear different renditions.
- *server-primary* (default) → voice consistency everywhere; native only rescues offline.

**D2 — UITextChecker verdict (YOUR CALL). Config flag location:**
`src/native_lang.rs::wordcheck_policy(kid)`, reading localStorage
`spell_wordcheck_policy` = `off | warn | block`. **Default = the recommendation: WARN in
adult flows, BLOCK in Kid Mode.** Block drops non-dictionary words with a skip count;
warn keeps them. Both are one flag flip, not a rewrite. Profanity ALWAYS runs regardless
of the dictionary verdict.

**D3 — Voice selection (done).** The core picks the **highest-quality installed voice**
for the locale, **stable per session** (`native_lang::session_voice`, cached; the Swift
catalog returns voices best-quality-first). No voice-picker UI.

**D4 — es locale (done).** Both AVSpeech and UITextChecker resolve **es → es-ES** (the
Spanish audit's pipeline locale), en → en-US (`LocaleResolver`).

## Test results

- **`xcodebuild test -scheme NativeLanguageKitCore` on iPhone 17 simulator: 15/15 pass.**
  Real es/en words validate, garbage fails, unsupported locale → `supported:false`,
  NFD≡NFC verdict, es-over-en detection, voices quality-sorted, rate mapping, `speak()`
  resolves on real completion, unknown voiceId fails.
- **`xcodebuild build -scheme App -sdk iphonesimulator`: BUILD SUCCEEDED** — plugin +
  package compile into the actual app.
- **`cargo test --lib`: 81 pass** incl. 3 router-order tests (D1 default + overrides).
- **web e2e: 20/20 pass** — web behavior byte-identical (bridge is a no-op off iOS).
- **`i18n-check`: 189-key parity across 17 locales.**
- **grep doctrine check:** no raw platform/language conditionals in game logic; capability
  access only through `native_lang` (bridge) + `api` (router).

## Known gaps / deviations (please review)

1. **`xcodebuild test -scheme App`** — the Capacitor App scheme has no XCTest target, and
   adding one to the app project is heavier than the pure-logic tests warrant. The XCTest
   lives in the `NativeLanguageKitCore` package scheme (run above). The **App scheme builds
   green** with the plugin integrated. If you want tests under the App scheme, that's a
   follow-up (adds a test target + provisioning surface).
2. **Maestro offline-audio flow** — authored at `tests/ios-ui/flows/offline-native-audio.yaml`
   but **not executed here** (Maestro isn't installed in this dev shell; it's provided by
   the fastlane `:ui_tests` lane / CI). It also needs the harness to seed
   `spell_audio_src=native-only` (Maestro can't reliably write WKWebView localStorage). The
   offline-audio *logic* is covered by the router unit tests + the Swift Speaker completion
   test; this flow guards the offline UX entry.
3. **Provisioning:** none needed for this PR — AVSpeech / UITextChecker / NaturalLanguage
   require no entitlements, and this plugin does NOT use the microphone/speech-recognition
   (that's Feature Pack 2 F2). Simulator-only, no signing.

## Constraints honored
No backend/TTS/tunnel changes. No scoring/streak/SRS/Daily/leaderboard/word-list changes.
No native audio cache, no voice-picker UI, no lemmatization, no Android. Web/PWA/Tauri
byte-identical. Build/version numbers untouched. Nothing merged or uploaded.
