# DigitalInkPlugin (staged)

A local Capacitor plugin bridging the webview to **ML Kit Digital Ink
Recognition** (on-device, offline after a one-time per-language model download).
It is the recognizer for 中文/日本語-kanji drawn answers (听写 / 書き取り).

**Status: STAGED, deliberately outside the build.** These files are NOT
referenced by the Xcode/Gradle projects yet, so they can't break the shipping
build. Nothing drawing-related activates until `consts::DRAW_MLKIT_READY` is
flipped `true` — and that flip requires everything below plus physical-device
recognition testing (simulator ML Kit behaves differently).

## Architecture / boundary
```
 pad strokes (Rust drawing.rs)
   → strokes_to_json (digital_ink.rs)          ── Rust emits strokes
   → JS reads them, calls SpellInk.recognize    ── digital-ink.js → native plugin
   → native ML Kit → { candidates:[{text,score}] }
   → JS calls the Rust scorer digital_ink::score_drawn(expected, accepted, json, tier, kid)
   → verdict → game applies it (same Misses/SRS path as typing)
```
The Rust/WASM core **never calls the plugin** — JS owns the plugin call; Rust
owns stroke serialization and the expected-answer top-N scorer. Keep it clean.

## Contract
```ts
downloadModel({ languageTag }): Promise<{ status: "downloaded" | "already" }>
isModelDownloaded({ languageTag }): Promise<{ downloaded: boolean }>
deleteModel({ languageTag }): Promise<void>
recognize({ strokes, writingArea, expected?, preContext?, languageTag })
  : Promise<{ candidates: { text: string; score: number }[] }>
```
- `languageTag` is a **full BCP-47 tag** — `zh-Hani-CN`, `ja-JP`, `ko-KR`,
  `th-TH`. Bare `zh`/`ja` fail identifier lookup. `digital-ink.js` maps the
  app's language code → tag.
- `strokes`: `[{ points: [{ x, y, t }] }]` in **CSS-pixel space** (do NOT
  pre-scale by devicePixelRatio — DPR is rendering-only). `t` = ms from stroke-
  session start. `writingArea` = `{ width, height }` in that same space.
- iOS candidates have **no native score** → the plugin synthesizes a descending
  rank score. Android exposes `candidate.score`.

## iOS activation
1. **Add the ML Kit dependency.** This project uses **Capacitor SPM**
   (`ios/App/CapApp-SPM/Package.swift`), so there is no `Podfile`. Two options:
   - **SPM (preferred, matches the project):** add ML Kit's Swift Package
     (`MLKitDigitalInkRecognition`) to `Package.swift` and the `CapApp-SPM`
     target's dependencies. Verify ML Kit's current SPM package name + min iOS
     target; raise the app's deployment target if required.
   - **CocoaPods:** reintroduce a `Podfile` with
     `pod 'GoogleMLKit/DigitalInkRecognition'`, then `npx cap sync ios`. Note the
     min-iOS bump in the PR. Cache `Pods/` on `Podfile.lock` in the fastlane lane.
2. Copy `ios/DigitalInkPlugin.swift` + `ios/DigitalInkPlugin.m` into
   `ios/App/App/` and add both to the App target in Xcode. The `CAP_PLUGIN`
   macro auto-registers the plugin under the name `DigitalInk`.
3. **Re-verify ML Kit signatures** against the current docs (VERIFY-FIRST rule) —
   the Swift was written against
   developers.google.com/ml-kit/vision/digital-ink-recognition/ios but ML Kit
   APIs shift between releases.

## Android activation (secondary)
1. `android/app/build.gradle`:
   `implementation 'com.google.mlkit:digital-ink-recognition:<current>'`
2. Copy `android/DigitalInkPlugin.kt` into
   `android/app/src/main/java/net/spellgame/app/`.
3. Register in `MainActivity`:
   ```java
   public class MainActivity extends BridgeActivity {
     @Override public void onCreate(Bundle s) {
       registerPlugin(DigitalInkPlugin.class);
       super.onCreate(s);
     }
   }
   ```

## Webview wiring
- `digital-ink.js` (repo root) exposes `window.SpellInk`. Add it to
  `index.html` (next to `audio-native.js`) and to the `cp` list in
  `scripts/build-web.sh` when activating. It's a harmless dormant stub until the
  native plugin exists (`available()` → false → UI hides draw mode).

## Still TODO before this recognizer reaches players (per drawing spec)
- Stroke-capture canvas: per-character cells + cultural grids (田字格 / genkō /
  hangul block / Thai band), brush rendering, undo/clear, auto-advance.
- Model-download UX (progress on first zh/ja selection; hide draw if offline).
- `inputModes` locale config + `answerScript` on ja words (kanji→draw, kana→
  type); remove the keyboard for zh; availability-gate zh/ja until relaunch.
- Ghost-outline trace hint after N misses; wire drawn results into Misses/SRS.
- Localized draw microcopy (写在格子里 / マスに書いてください / …), auditor-reviewed.
- Physical-device acceptance testing (≥20 chars/language) — the merge gate.
