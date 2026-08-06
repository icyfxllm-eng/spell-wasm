# CC-PHOTO-IMPORT — phased implementation plan

**Status: REVIEW-GATED — PLAN ONLY.** Nothing here enters the submission pipeline.
This is a proposal for how Photo Import would be built if approved. Per the spec's
"decide or push back, don't infer" instruction, every open decision below is either
DECIDED with a recommendation or flagged as a gate for Eric — none is silently
assumed. File paths are current-tree accurate.

**The most important fact up front:** a shipped-dark seed of this feature ALREADY
EXISTS — `flags::photo_list` (default ON in the integration build), `src/photo_list.rs`,
`src/native_lang.rs`, `ios/App/App/NativeLanguageKitPlugin+PhotoList.swift`,
`ios/NativeLanguageKit/Sources/NativeLanguageKitCore/WordListRecognizer.swift`, the
`photo_list` entry in `config/modes.json`, and the `photo_ocr` parent-premium boolean
in `src/entitlements.rs`. CC-PHOTO-IMPORT is the **hardening and completion** of that
seed to the spec, not a greenfield build. Two of the spec's cited companions
(CC-SPELL-READER, a `capturePage()` VisionKit path) **do not exist as code** — where
the spec assumes them, this plan makes building them a phase and says so.

---

## Gate before anything: three decisions that block the start

| Gate | Status | Why it blocks | Needs from Eric |
|---|---|---|---|
| **G-A · Custom-word audio egress** | ✅ RULED 2026-07-24 — **on-device AVSpeech, zero egress** | Out-of-dictionary imports have no pre-rendered `/api/speak` clip. The spec PROPOSES server Pi TTS at import (word text leaves device, with an explicit review-sheet line) OR on-device `AVSpeech` if zero egress is wanted. This collides with the COPPA on-device-only posture (Invariants) unless resolved. | **Recommendation: on-device `AVSpeech`** via the existing `native_lang::speak` path (`src/native_lang.rs:52`) — zero egress, consistent with "no image or text bytes in any network call". Server Pi TTS only if Eric wants server-quality voices for custom words and accepts word-text egress with the review-sheet disclosure. Pick one before Phase 5. |
| **G-B · Vision language matrix** | ✅ RULED 2026-07-24 — **Claude to script the on-device measurement** (data still to collect) | The registry field in Phase 0 needs REAL data: `VNRecognizeTextRequest.supportedRecognitionLanguages` measured on the iOS floor device across all 13 launch languages. Expected gaps (Filipino, Swahili almost certainly; Arabic OS-dependent) decide which languages get the English-recognizer fallback vs. are hidden. Guessing the matrix would ship a wrong gate. | Run the measurement on the floor device (or approve me scripting it under `xcodebuild test`) and sign off the resulting per-language classification (`Native` / `EnglishFallback` / `Unsupported`) before Phase 0 lands its values. |
| **G-C · "Dictionary" source for classification** | ✅ RULED 2026-07-24 — **word banks authoritative**, UITextChecker secondary (promote-only) | Spec F3 requires the core to classify every token as **in-dictionary / custom / filtered**. The codebase has TWO candidate authorities: the app's own word banks (`words::tier_for`, `src/words.rs`) — deterministic, fully on-device — and the native `UITextChecker` bridge (`native_lang::check_word_await`, `src/native_lang.rs:152`), which is async and iOS-only. | **Recommendation: the word banks are authoritative** (deterministic, offline, testable in the Rust core; keeps classification pure per the "logic lives in the CORE" rule), with `UITextChecker` as an OPTIONAL secondary signal that can only reclassify custom→in-dictionary, never the reverse. Confirm, or name a different dictionary. |

**✅ All three gates ruled 2026-07-24 — the plan is unblocked to start.** G-A =
on-device AVSpeech (Phase 5); G-B = Claude scripts the `supportedRecognitionLanguages`
measurement, Eric signs off the resulting per-language classification before Phase 0
lands its values; G-C = word banks (`words::tier_for`) are the classification authority,
`UITextChecker` a promote-only secondary. Phase 0 (registry field) begins with the
measurement; Phase 1 (core extraction/classification) can proceed in parallel.

---

## Reuse (the seed) — what already exists

The seed is substantial and correct in shape; most phases EXTEND it rather than
replace it.

- **Core parse + advisory gate** — `native_lang::parse_candidates` (`src/native_lang.rs:329`):
  per-line split, NFC-normalize, strip numbering/bullets/edge punctuation, drop
  digit-bearing tokens and stray single letters, case-insensitive dedupe. Already
  NFC-correct (test `nfc_normalizes_accented_words`). `native_lang::gate_reason` +
  `GateFail{Blocked,Charset}` (`src/native_lang.rs:377`) pre-flag candidates for the
  review UI. **Missing: the in-dictionary / custom classification (G-C).**
- **The typed-importer save gate** — `importer::extract_words` (charset,
  `src/importer.rs:14`) → `profanity::filter_allowed` (`src/profanity.rs:182`) →
  `apply_saved_words` (`src/lib.rs:685`) → `importer::save_words` (`src/importer.rs:58`).
  Photo confirm ALREADY routes through this exact path (`photo_list::confirm`,
  `src/photo_list.rs:215`) — no duplicated gate. **Missing: batch ID + undo, custom marking.**
- **Profanity screen** — `profanity::is_blocked` (`src/profanity.rs:155`): English/leet
  layer + per-language union (17 language lists) + loose leet/separator pass, NFC
  throughout. Satisfies the "profanity seed/custom/overlay pass" invariant as-is.
- **NFC normalization** — `norm::fold_strict`/`fold_lenient` (`src/norm.rs`), plus
  `.nfc()` in the parser and profanity. "cigüeña byte-identical" is already achievable.
- **On-device recognizer** — `WordListRecognizer.recognizeLines`
  (`ios/NativeLanguageKit/.../WordListRecognizer.swift:31`): `VNRecognizeTextRequest`,
  `.accurate`, `recognitionLanguages`, on-device, never uploads. The plugin shim
  (`NativeLanguageKitPlugin+PhotoList.swift`) wires camera (`UIImagePickerController`)
  + library (`PHPickerViewController`) → Vision → `{supported, lines}`. **Missing: the
  VisionKit `VNDocumentCameraViewController` document-scanner path and multi-page.**
- **Bridge + capability gate** — `native_lang::recognize_word_list`/`supported`
  (`src/native_lang.rs:281,296`); `photo_list::reflect_visibility` shows the affordance
  only where `supported()` and not Kid Mode (`src/photo_list.rs:25`). Bridge stub
  returns all-false off-iOS, so callers keep one code path.
- **Entitlement + registry gating** — `EntitlementSet.photo_ocr` (Complete
  parent-premium, `src/entitlements.rs:104`); `modes.json` `photo_list` entry with
  `requiresPremium:"photo_ocr"`, `kidSafe:false`, `platforms:["ios"]`; the pure
  `modes::visible` rule (`src/modes.rs:165`) already ANDs premium + kid + platform.
- **Localized permission string** — `NSCameraUsageDescription` exists in
  `ios/App/App/Info.plist:33` (English only today).
- **Parent gate primitive** — `agegate::parent_problem` (`src/agegate.rs:137`), the
  worded-math challenge, and the Kid-Mode lock (`agegate::is_kid_locked`).
- **Test fixture beachhead** — `WordListRecognizerTests.swift` +
  `Tests/.../Resources/printed-words.png` already exercise the recognizer end-to-end
  under `xcodebuild test`.

## What's missing (build)

Vision language-support **registry field** + accessor; in-dictionary/custom/filtered
**classification in the core**; the VisionKit `VNDocumentCameraViewController`
**document-scanner capture path** + multi-page aggregation (the absent "CC-SPELL-READER"
path); review-sheet **low-confidence surfacing** (nothing is dropped) + per-chip toggle
state; **batch ID + one-tap undo** and the **custom-word marking** (the CC-MODE-HUB
"stub" referenced by the spec does not exist as a field — see Open items); custom-word
**audio** (G-A); **parental gate** before camera in kid-managed profiles; the **OCR
fixture set** + CI precision/recall **eval + merge gate**; a **Maestro** flow; localized
camera Info.plist strings.

---

## Phases

Each phase is independently reviewable and lands green (tests + i18n + build). Later
phases depend only on earlier ones. The mode stays behind `flags::photo_list` (and,
for the App Store, that flag reverts to OFF per PR) until every acceptance test passes
and Eric approves activation.

### Phase 0 — Vision language-support as a registry field  *(gated on G-B)*
**Goal:** the Vision support matrix is ONE registry field, not scattered checks.
- `src/consts.rs`: extend the language registry (`LANGS_BASE` / `BUILTIN_LANGS`,
  currently `(code, name, LangStatus, Direction)`) with an OCR-support axis — a new
  `enum OcrSupport { Native, EnglishFallback, Unsupported }` and an accessor
  `ocr_support(lang) -> OcrSupport`, exactly the way `direction`/`rtl_required` derive
  from the registry today. `Native` = Vision recognizes the study language directly;
  `EnglishFallback` = Latin-script language Vision doesn't cover, run the English
  recognizer with **correction OFF** and let the dictionary validate (G-C);
  `Unsupported` = non-Latin/no coverage, feature HIDDEN for that language.
- Values come from the G-B measurement (Filipino/Swahili expected `EnglishFallback`;
  Arabic OS-dependent but already RTL-blocked → moot).
- **Covers acceptance:** foundation for #6 (hidden on unsupported-language builds).
- **Risk:** touches the shared registry tuple. Additive; add a snapshot test pinning
  each language's `OcrSupport`, mirroring `registry_is_the_swapped_lineup_of_14`.

### Phase 1 — Candidate extraction & classification in the core  *(gated on G-C)*
**Goal:** raw lines → tokenize → NFC → dedupe → classify {in-dictionary, custom,
filtered}, ALL in Rust.
- Extend the existing core (`src/native_lang.rs`, or lift the photo-specific logic
  into a new `src/photo_import.rs` core module to keep `native_lang` a pure bridge):
  keep `parse_candidates` for shape cleanup, add a `classify(lang, tokens) ->
  Vec<Candidate>` where `Candidate { word, class, confidence_low }` and
  `class ∈ {InDictionary, Custom, Filtered}`.
  - `Filtered` = `profanity::is_blocked` OR empty after `importer::extract_words`
    (reuse `gate_reason`).
  - `InDictionary` vs `Custom` = membership in `words::tier_for` for the study
    language (NFC-folded compare via `norm::fold_strict`), per G-C. Low-confidence
    tokens are NOT dropped — they are carried with `confidence_low = true`.
- `confidence` must flow from Vision (`VNRecognizedTextObservation` confidence) through
  the bridge into the candidate — the current bridge returns only `lines: string[]`,
  so extend the recognizer result to `{ text, confidence }[]` (Phase 2 plumbs it).
- **Covers acceptance:** #1 (printed sheet → correct chips, zero junk after filtering),
  #3 (Spanish diacritics round-trip byte-identical — pure NFC test in the core),
  #4 (a profanity token classifies `Filtered`).

### Phase 2 — VisionKit document capture path + multi-page  *(the absent "CC-SPELL-READER")*
**Goal:** `VNDocumentCameraViewController` scanning, multi-page, library fallback.
- `ios/App/App/NativeLanguageKitPlugin+PhotoList.swift`: replace the
  `UIImagePickerController` camera branch with `VNDocumentCameraViewController`
  (`import VisionKit`) — the spec's primary capture. Keep `PHPickerViewController` as
  the photo-library fallback. **No live-scanner UI in v1.** Aggregate every scanned
  page's recognized lines into one ordered list before returning.
- `ios/App/App/NativeLanguageKitPlugin.swift`: expose the capture as the single
  `capturePage()`-style entry the spec names (today it is `recognizeWordList`); keep
  one Swift plugin. Set `recognitionLanguages = [current study language]` from the
  Rust caller; drive `usesLanguageCorrection` from the registry (`Native` → on,
  `EnglishFallback` → OFF).
- `ios/App/App/public/native-language-kit.js` + `src/native_lang.rs`: carry
  per-line confidence through to the core (Phase 1).
- **Single-source-of-truth doctrine:** the study language is passed IN; the existing
  `LanguageDetector`/`detect_language` path is NOT used here (no auto-detect).
- **Covers acceptance:** #1/#2 (real capture into the pipeline), #5 (fully on-device —
  no network in the Vision path), #7 (capture leg of the Maestro flow).

### Phase 3 — Review sheet hardening
**Goal:** every candidate a toggleable, editable chip; low-confidence pre-shown; nothing
imports without an explicit confirm.
- `src/photo_list.rs` + `ios/App/App/public/index.html` (`#photoScrim`/`#photoChips`):
  each chip carries `class` (in-dictionary / custom / filtered) and a `confidence_low`
  style; low-confidence tokens render pre-shown (dimmed, editable), never absent.
  Per-chip on/off toggle in addition to the existing edit input and remove button
  (`pchip`/`pchip-x`); live re-classify on edit (`reflag` already exists,
  `src/photo_list.rs:183`) so fixing a misread updates its class before save.
- Import remains behind `photoConfirm` — nothing persists without the sheet confirm.
- i18n: new keys via the existing pipeline (chip class labels, low-confidence hint,
  the G-A audio disclosure line), all launch locales.
- **Covers acceptance:** #2 (handwritten misses appear as editable low-confidence
  chips, not absent), #4 (profanity flagged in adult review).

### Phase 4 — Import: batch ID + one-tap undo + custom marking
**Goal:** a confirmed import is one undoable unit; custom words are marked.
- `src/model.rs` (`CustomSet`): add an import **batch id** per word (side map keyed
  like the existing `word_lang: HashMap<String,String>`, so `words: Vec<String>` and
  its call sites stay unchanged) and a **custom marker** set (out-of-dictionary words).
  Serde `#[serde(default)]` so old stored lists migrate untouched.
- `src/importer.rs` (`save_words`): accept the batch id + per-word class; keep the
  additive-merge semantics. Add `undo_batch(state, batch_id)` removing exactly that
  batch's words (and only words that batch introduced).
- `src/photo_list.rs` / `src/lib.rs` (`apply_saved_words`): thread the batch id through;
  surface a one-tap "Undo import" affordance after a successful import.
- **Import writes to My Words ONLY** (`CUSTOM_KEY`) — never to a built-in bank, the
  Daily list, or any audited source-of-record.
- **Covers acceptance:** #7 (import → word playable in My Words), undo foundation.

### Phase 5 — Custom-word audio  *(gated on G-A)*
**Goal:** imported custom words are playable.
- Per G-A recommendation: on-device `AVSpeech` via `native_lang::speak`
  (`src/native_lang.rs:52`) using the batch's "Speak in" language — no egress. If Eric
  chooses server Pi TTS instead, synthesize at import via `/api/speak` (`src/api.rs:111`)
  with the explicit review-sheet disclosure line (Phase 3) and the word-text-only
  egress boundary.
- **Covers acceptance:** #5 (airplane-mode full flow succeeds; only server TTS, IF
  chosen, is absent offline — the on-device recommendation makes even audio work offline).

### Phase 6 — Gating: entitlement + parental gate + registry hide
**Goal:** the feature is reachable only where it's allowed.
- Entitlement: reuse `EntitlementSet.photo_ocr` (Complete parent-premium) +
  `modes.json` `requiresPremium:"photo_ocr"`. A **free account sees no camera**
  (absence, not a lock, per the mode-hub doctrine) — and the paywall for the mode is
  the Complete upsell surface. Wire `photo_list::reflect_visibility` to also hide on
  `ocr_support(lang) == Unsupported` (Phase 0).
- **Parental gate:** before the camera opens in a kid-managed profile, challenge with
  `agegate::parent_problem`. See Open items — today `photo_list` is `kidSafe:false`,
  so it is ABSENT in Little Speller entirely; the parent gate is the answer only if
  Eric wants the mode reachable inside a Complete parent's child profile.
- **Covers acceptance:** #6 (free account: paywall shown, camera never opens).

### Phase 7 — Fixtures, CI eval + merge gate, Maestro
**Goal:** recognition quality is measured and regressions are blocked.
- OCR fixture set: 3 printed fonts + 3 handwriting (print-style) samples per launch
  language, each with a ground-truth word list, extending
  `Tests/NativeLanguageKitCoreTests/Resources/`.
- CI eval: a scoring script over the fixture set reporting per-language
  precision/recall; **merge gate = no regression below the recorded baseline**.
- Maestro flow: inject a fixture image → capture → review → import → assert the word
  is playable in My Words.
- **Covers acceptance:** #1, #2 (scored on fixtures), #8 (the eval + merge gate),
  #7 (the Maestro flow).

### Cross-cutting (every phase)
- **#5 On-device / offline:** the Vision path already makes no network call; add a CI
  test running the full extract→classify→import flow with networking disabled.
- **COPPA:** the image and OCR text are discarded after extraction; assert no image or
  OCR-text bytes reach any network call (grep-gate the plugin + a runtime test), the
  same shape as `scripts/entitlement-core-purity-check.mjs`.

---

## Acceptance-test → phase map

| # | Test | Lands in |
|---|---|---|
| 1 | Printed 20-word English: ≥19 chips, zero junk after filtering | Phase 1 + Phase 7 |
| 2 | Handwritten 10-word: ≥8, misses as editable low-confidence chips | Phase 3 + Phase 7 |
| 3 | Spanish diacritics round-trip byte-identical | Phase 1 |
| 4 | Profanity word flagged in adult review, absent in Kid Mode | Phase 3 / Phase 6 |
| 5 | Airplane mode: full flow (minus server TTS if G-A lands server-side) | Phase 5 / cross-cutting |
| 6 | Free account: paywall shown, camera never opens | Phase 6 |
| 7 | Maestro: capture → review → import → word playable in My Words | Phase 4 + Phase 7 |
| 8 | CI eval: per-language precision/recall, merge gate = no regression | Phase 7 |

## Guardrails / invariants — enforced, not aspirational

- **On-device only.** Image and OCR text discarded after extraction; NO image or OCR-text
  bytes in any network call (COPPA posture). Enforced by a plugin grep-gate + a
  networking-disabled flow test. The Vision path (`WordListRecognizer`) already uploads
  nothing.
- **Every persisted word passes the profanity seed/custom/overlay pass** — via the
  shared `profanity::filter_allowed` save gate (`src/importer.rs`/`src/lib.rs:685`); no
  photo-specific shortcut. Filtered words never render in Kid Mode review (the mode is
  Kid-absent today; if the parent-gate path lands, filtered chips are suppressed there).
- **Import writes to My Words ONLY** (`CUSTOM_KEY`) — never a built-in bank, Daily, or
  any audited source-of-record.
- **NFC everywhere.** Parser, profanity, and comparison all normalize to NFC;
  "cigüeña" imports byte-identical to the dictionary form (Phase 1 test).
- **Language = current study language, no auto-detect** (single-source-of-truth
  doctrine). `LanguageDetector` is not used in this path.
- **Classification lives in the CORE (Rust)**, not the plugin. The Swift
  `WordListRecognizer.parseCandidates` mirror stays a test-only convenience
  (documented as such at `WordListRecognizer.swift:14`); production truth is the WASM core.
- **The Vision support matrix is ONE registry field** (`consts::ocr_support`), never a
  scattered `lang == …` check — mirroring how `direction`/`rtl_required` already work.
- **Nothing imports without the review-sheet confirm.**

## Prerequisites / resources — acquire/verify first

- **Apple frameworks:** VisionKit `VNDocumentCameraViewController` (new to this repo —
  no `import VisionKit` exists today), Vision `VNRecognizeTextRequest` `.accurate`
  (already in use).
- **The Vision `supportedRecognitionLanguages` matrix** measured on the iOS floor
  device across all launch languages → becomes `consts::ocr_support` (G-B). Expect
  gaps: Filipino/Swahili almost certainly `EnglishFallback`; Arabic OS-dependent (and
  already RTL-blocked, so moot in production).
- **The single Swift plugin exposing `capturePage()`** — the spec says "reuse the
  CC-SPELL-READER VisionKit path." **That path does not exist**; Phase 2 builds it on
  the existing `NativeLanguageKitPlugin+PhotoList.swift` recognizer rather than reusing
  a companion that was never written.
- **OCR fixture set for CI** — 3 printed fonts + 3 handwriting samples per launch
  language with ground-truth lists (Phase 7). One printed English fixture already
  exists (`Resources/printed-words.png`).
- **Localized Info.plist permission strings** — `NSCameraUsageDescription` +
  `NSPhotoLibraryUsageDescription` exist in English only (`Info.plist:33`); localize
  for all UI locales. **Review-critical given the 2.3.6 rejection history.**

## Open items still needing your call (beyond the three gates)

- **Custom-word marking "stub"** — the spec says custom words "carry the custom-word
  marking from the CC-MODE-HUB stub." **No such field exists** — `CC-MODE-HUB` in this
  tree is the mode registry (`src/modes.rs` / `config/modes.json`) and has no
  custom-word marker. Phase 4 PROPOSES adding a custom marker set to `CustomSet`
  (`src/model.rs`); confirm this is the intended home, or point me at the stub if it
  lives elsewhere.
- **Parental gate vs. Kid-absence** — `photo_list` is `kidSafe:false`, so it is
  ABSENT in Little Speller entirely (the mode-hub absence-not-locks doctrine). The
  spec's "parental gate before camera in kid profiles" only bites if the mode should
  be REACHABLE inside a Complete parent's child profile. **Recommendation:** keep it
  Kid-absent; apply `agegate::parent_problem` as the camera pre-gate ONLY where a
  parent-managed child profile (Complete `multiple_profiles`) could reach it. Confirm
  which behavior you want.
- **Activation (do LAST):** flip `flags::photo_list` semantics for the App Store PR
  and confirm the `modes.json` copy/entitlement once all eight acceptance tests pass.
- **Confidence threshold** for "low-confidence" pre-shown chips — a first value in
  Phase 3, calibrated against the Phase 7 handwriting fixtures; bring the number to
  you rather than hard-code it silently.

## G-B MEASURED (2026-08-05) — the data, not a guess
Run: `VNRecognizeTextRequest.supportedRecognitionLanguages()` (accurate
level) on **iOS 26.5 (23F77)**, iPhone 17 Pro simulator. The probe is
`ios/NativeLanguageKit/Tests/.../VisionLanguageMatrixTests.swift`; it was
compiled standalone for the simulator because the SPM package is
UIKit-bound and has no macOS scheme (recorded so the run is repeatable).

Vision reports 30 recognition languages:
en-US, fr-FR, it-IT, de-DE, es-ES, pt-BR, zh-Hans, zh-Hant, yue-Hans,
yue-Hant, ko-KR, ja-JP, ru-RU, uk-UA, th-TH, vi-VT, ar-SA, ars-SA,
tr-TR, id-ID, cs-CZ, da-DK, nl-NL, no-NO, nn-NO, nb-NO, ms-MY, pl-PL,
ro-RO, sv-SE.

CLASSIFICATION for the app's 14 registry languages — AWAITING ERIC'S
SIGN-OFF (the plan's own rule: Phase 0 lands these values only after he
signs):
  Native (12): en es fr de pt pl vi ko ja zh ru ar
  EnglishFallback (2, both Latin-script): fil sw
Both predictions in the plan held: Filipino and Swahili are the gaps,
and Arabic turned out SUPPORTED on this OS (the plan flagged it as
OS-dependent). Nothing is Unsupported — every gap language is Latin, so
the English recognizer is a sound fallback for both.
