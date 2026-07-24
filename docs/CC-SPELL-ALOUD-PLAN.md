# CC-SPELL-ALOUD — phased implementation plan

**Status: REVIEW-GATED — PLAN ONLY.** Nothing here enters the submission pipeline.
This is a proposal for how the revised Spell-It-Aloud *mode* would be built if
approved. Per the spec's "decide or push back" rule, every open decision below is
either DECIDED with a recommendation or FLAGGED for Eric — never silently assumed.
Any reversal of shipped, tested behaviour is STOP-AND-ASK; the gates block phase 0.

Companion to the spec (`CC-SPELL-ALOUD`, a revision of the shipped input-method
draft). File paths are current-tree accurate.

> **The important context up front:** this is *not* a greenfield feature. The
> letter-by-letter voice path already ships as an **input method** — a mic beside
> the answer field — and its Rust core, lexicons, native plugin surface, and mode
> registry entry are all live. `src/spell_aloud.rs` (429 lines, 19 tests) already
> parses spoken letter names, rejects whole words, and streams live echo. This plan
> is about **completing and re-shaping** that seed into the mode the revised spec
> describes (push-and-hold turn-taking, slot filling, spoken commands, confusable
> chips, a CI loopback oracle) — not rebuilding it. Where the spec assumes something
> that is *not* in the tree (the whisper harness; localized permission strings;
> per-token confidence), that assumption is called out and its construction is a phase.

---

## Gate before anything: three decisions that block the start

| Gate | Status | Why it blocks | Needs from Eric |
|---|---|---|---|
| **G-A · Answer-leak vs the shipped whole-word rule** | ✅ RULED 2026-07-24 — **drop the target from the matcher** (rewrite the 2 shipped tests to the new invariant) | The spec's invariant is absolute: *the target word is NEVER provided to the recognizer or matcher.* But the shipped `spell_aloud::interpret(lang, transcript, **target**)` (`src/spell_aloud.rs:245`) rejects whole words by measuring Levenshtein `similarity(transcript, target)` — it feeds the answer to the matcher. Two tests depend on this (`en_whole_word_said_is_rejected`, `genuine_spelling_is_never_rejected_even_when_it_resembles_word`). | **Recommendation: DROP the target from the matcher.** In a slot-filling mode each accepted token fills one slot; a spoken whole word yields ~0 letter-names so it *structurally* cannot fill slots correctly — the yield-ratio guard (`WHOLE_WORD_YIELD`, already present) is sufficient without the target. This reverses shipped, tested behaviour, so it needs your sign-off before Phase 0. |
| **G-B · Spanish accents in v1 (spec-mandated stop-and-ask)** | ✅ RULED 2026-07-24 — **accept the "con tilde" phrasing**, measure in loopback, do NOT restrict es lists | The spec: accept "a con tilde / con acento"; *if unreliable*, restrict this mode's es word lists to diacritic-unambiguous words — but **"Stop and ask before restricting."** The diacritic phrases already exist in the lexicon (`lexicons/letters/es.json`, `diacritics` block: `a con tilde/con acento/acentuada` → `á`, etc.). Restricting the es lists is a **content change** with cross-mode blast radius (the shared word banks feed every mode). | Decide: (a) ship the accept-the-phrasing path and measure it in the loopback suite (Phase 5) — my recommendation — or (b) restrict this mode's es lists. I will NOT touch a word list without your explicit yes. |
| **G-C · Identity: input-method → standalone mode** | ✅ RULED 2026-07-24 — **build the new push-and-hold mode, KEEP the existing input method** | The shipped feature is an *input method* (tap-to-toggle mic that **appends** to the answer field; `spell_aloud::mic_tap`, `reflect`, `wire`). `config/modes.json` already registers `spell_aloud` as a **live** mode routing into that flow. The revised spec wants a **mode**: push-and-hold turn-taking, per-slot fill, spoken echo, commands, chips — a different interaction model on a new surface. | Confirm the reshape: build the mode as a **new surface** that reuses the parser/plugin, and decide the fate of the existing tap-to-toggle mic-beside-the-field (keep both, or retire the input method once the mode ships). Until confirmed, the mode stays `hidden` and the shipped input method is untouched. |

**✅ All three gates ruled 2026-07-24 — the plan is unblocked to start.** G-A = drop
the target from the matcher (Phase 0 hardens the parser to the answer-leak invariant
and rewrites the two shipped tests); G-B = accept the "con tilde/con acento" phrasing
already in the lexicon and measure it in the loopback suite — the es word lists are
NOT restricted; G-C = the new push-and-hold mode is a new surface reusing the
parser/plugin/lexicons, and the shipped mic-beside-the-field input method is KEPT
(two entry points to the same core). The parser, lexicons, plugin, and registry entry
are the seed — the first build is Phase 0 (parser hardening), not new UI.

---

## Reuse (the seed) — what already exists, leverage don't rebuild

- **The Rust letter parser** — `src/spell_aloud.rs` (429 lines, 19 host tests in
  `src/spell_aloud/tests.rs`). Already does: greedy longest-phrase match (`parse`),
  whole-word rejection (`interpret` → `Insert`/`WholeWord`/`Nothing`), NFC-exact
  output equal to keyboard input (Invariant I1), `contextual_strings()` for
  recognizer biasing, per-language leaked-once lexicon cache. **This is ~80% of the
  linguistic core.** It is written as an input method, not a mode; the parsing stays,
  the UI layer (`reflect`/`wire`/`mic_tap`/`on_partial`/`on_final`) is replaced.
- **The letter lexicons — the single source of record per language.**
  `lexicons/letters/en.json` and `es.json` (Invariant I4: no letter-name literal
  lives anywhere else). Already near-complete against the spec's checklist:
  - en: `zee` AND `zed` (both, line 30), `double u / double-u / double you` → `w`,
    full homophone set (`sea/see`→c, `why`→y, `are`→r…).
  - es: `eñe` as a **first-class precomposed** letter (→ `ñ`, accent-preserving),
    `be`/`ve`, `be larga/be grande/be alta`, `ve corta/ve chica/ve baja`,
    `uve`/`uve doble`/`ve doble`/`doble ve`, `i griega`/`ye`, `che`→ch, `elle`→ll,
    plus the full `con tilde/con acento/acentuada` diacritic block.
  - **What's missing in the lexicon:** the `commands` (delete/borrar, clear,
    done/listo) and the `X as in Y` / confusable-clarifier phrases (Phases 2–3).
- **The native capture surface — already built and on-device-only.**
  `ios/App/App/NativeLanguageKitPlugin.swift` exposes `startLetterCapture` /
  `stopLetterCapture` (lines 171–199), a *second recognition profile over the ONE
  mic*. `SpeechListener.begin` sets `requiresOnDeviceRecognition = true`
  (line 319), biases with `contextualStrings`, streams `letterToken`/`letterFinal`/
  `letterError` events, and refuses to start unless `SpeechCapabilities.available`
  is true (fail-closed, never server fallback). **The `listenForLetters()` method
  the spec asks for effectively exists** — it is named `startLetterCapture`.
- **The JS/Rust bridge** — `src/native_lang.rs:493 start_letter_capture` /
  `539 stop_letter_capture` / `217 speech_capabilities`, and
  `ios/App/App/public/native-language-kit.js`.
- **Locale resolution** — `LocaleResolver.swift` (en→en-US, es→es-ES) and
  `SpeechCapabilities.swift` (`available` gates strictly on
  `supportsOnDeviceRecognition`).
- **Mode registry + gating machinery** — `config/modes.json` already has the
  `spell_aloud` entry (ios-only, `languages:["en","es"]`, `entitlementLevel:preview`,
  `kidSafe:false`); `src/consts.rs:271 VOICE_SPELL_LANGS = [EN, ES]` +
  `voice_spell(lang)` is the language gate; `src/flags.rs:95 spell_aloud()` is the
  master flag; `src/entitlements.rs:332 preview_allows` is the depth resolver.
- **i18n pipeline** — `voiceSpell.*` and `tools.spellaloud.*` keys already exist in
  all **11** locales (`src/i18n/locales/*.json`).

### A correction to the brief's assumption
`SpeechMatcher.swift` is **not** the letter matcher. It is the **Say-It whole-word**
match rule (exact token match after NFC+fold; `SpeechMatcher.matches(transcript,
target)`), mirroring `norm::spoken_matches`. **All letter/speech matching for this
feature lives in Rust** (`spell_aloud.rs`), by design — the plugin does "ZERO
parsing" (its own comment, line 169). So the confusable/lexicon work is Rust work,
not Swift work; the Swift changes are confined to *surfacing per-token confidence*
(Phase 3) and *localized permission strings* (Phase 6).

## What's missing (build)

Push-and-hold turn-taking + slot UI; spoken-command vocabulary; per-segment
**confidence** surfacing from `SFSpeechRecognizer` (the plugin currently discards it
— only `formattedString` is sent) and the confusable two-choice chip; the
`X as in Y` clarifier grammar; the b/v-always-ambiguous rule; the whisper.cpp
loopback CI oracle (**does not exist anywhere in the tree** — the spec's "already in
your harness" is inaccurate); **localized** `NSMicrophoneUsageDescription` /
`NSSpeechRecognitionUsageDescription` (today English-only in `Info.plist`, no
`InfoPlist.strings` localizations — review-critical given the 2.3.6 history).

---

## Prerequisites / resources — acquire or verify FIRST

| Resource | State in tree | Action |
|---|---|---|
| Apple `SFSpeechRecognizer`, `requiresOnDeviceRecognition = true` | ✅ present (`SpeechListener.beginCapture`, plugin line 319) | Verify still enforced after the mode reshape; add a Swift assertion/test seam. |
| AVFoundation audio session (record + restore) | ✅ present (`.playAndRecord`/`.measurement`; restores `.playback` on finish) | Reuse as-is; confirm push-and-hold press/release maps cleanly onto start/stop. |
| Capacitor plugin `listenForLetters()` | ✅ present **as `startLetterCapture`** | No new method needed; extend its event payload to carry confidence (Phase 3). |
| Letter-name lexicon files (en + es) as authored registry assets | ✅ present and near-complete (`lexicons/letters/*.json`) | Add the `commands` + `X as in Y` clarifier sections (Phases 2–3); **checklist line, not a new gig.** en already has zee/zed + double-u; es already has be larga/be grande, ve corta/uve, i griega/ye, uve doble/doble ve, eñe, con acento/con tilde. |
| Per-segment confidence from the recognizer | ❌ discarded (plugin sends only `formattedString`) | **Build:** surface `SFTranscriptionSegment.confidence` through the events (Phase 3) — precondition for the confusable chip. |
| Localized Info.plist permission strings (mic + speech) | ❌ English-only, no `*.lproj/InfoPlist.strings` | **Build:** per-language `InfoPlist.strings` (Phase 6) — review-critical. |
| whisper.cpp loopback TTS oracle | ❌ not in `tests/`, `tools/`, or `scripts/` | **Build:** the CI oracle for the audio mode (Phase 5). |

---

## Phases

Each phase is independently reviewable and lands green (tests + i18n + build). Later
phases depend only on earlier ones. The mode stays `hidden` in `modes.json` until it
passes every acceptance test and Eric approves activation.

### Phase 0 — Answer-leak hardening of the parser  *(gated on G-A)*
**Goal:** make the core obey the invariant *the target never reaches the matcher.*
- `src/spell_aloud.rs`: change `interpret(lang, transcript, target)` →
  `interpret(lang, transcript)`; drop `similarity(...)`/`levenshtein(...)` and the
  `WHOLE_WORD_SIM` constant; whole-word rejection becomes yield-ratio-only
  (`yield_ratio() < WHOLE_WORD_YIELD` ⇒ reject). Introduce the slot model
  (`Vec<Slot>` where each accepted letter is one slot) as the return shape.
- `src/spell_aloud/tests.rs`: rewrite the two target-dependent tests; add a
  **CI invariant test** that `interpret`/`parse` take no target parameter (a
  compile-time signature guard) and that a whole-word utterance yields zero slots.
- **Covers acceptance:** the *reject* half of #1 ("cat" → rejected, attempt not
  consumed); the invariant "a whole-word utterance can NEVER score as a correct
  spelling."
- **Risk:** load-bearing reversal of shipped code — do not start before G-A.

### Phase 1 — Mode surface: push-and-hold + slots + echo  *(gated on G-C)*
**Goal:** the feature becomes a *place* with kid-simple turn-taking.
- `src/spell_aloud.rs` (replace the input-method UI layer) or a new
  `src/spell_aloud/screen.rs`: **push-and-hold** mic (press = `start_letter_capture`,
  release = `stop_letter_capture`) instead of tap-to-toggle; render one slot per
  accepted letter with optional spoken echo (reuse `native_lang`/`Speaker.speak`);
  no auto-submit.
- `index.html`: `#spellAloud` mode surface (slots row, hold-mic, status line).
- Reuses the existing plugin events and `parse()` unchanged.
- **Covers acceptance:** the *accept* half of #1 ("c, a, t, done" → CAT accepted),
  pending the `done` command in Phase 2.

### Phase 2 — Spoken commands (delete/borrar, clear, done/listo)
**Goal:** hands-free editing and turn completion.
- `lexicons/letters/en.json` + `es.json`: new `commands` block
  (`delete`/`backspace` & `borrar`, `clear`/`start over` & `borrar todo`,
  `done`/`finished` & `listo`/`ya`) — kept in the **same single-source-of-record
  file**, never a scattered map.
- `src/spell_aloud.rs`: parser distinguishes command tokens from letter tokens and
  emits a `Command` variant; the mode applies it to the slot buffer. `done` triggers
  the submit the mode was withholding.
- **Covers acceptance:** completes #1 (the `done` that accepts CAT).

### Phase 3 — Confusable handling + confidence surfacing
**Goal:** below-threshold letters offer a two-choice chip instead of guessing.
- **Native (Swift):** `SpeechListener` reads `bestTranscription.segments[].confidence`
  and includes it in the `letterToken`/`letterFinal` event payload;
  `NativeLanguageKitPlugin.startLetterCapture` marshals it; `native_lang.rs` bridge
  carries it into Rust. (New data, no new method.)
- **Rust:** `src/spell_aloud.rs` — the English **E-set** (B C D E G P T V Z) and
  **M/N** are flagged as a confusable class; when a token in that class arrives below
  the confidence threshold, emit a `Disambiguate{a, b}` outcome → two-choice chip
  rather than a guess. Accept **`X as in Y`** clarifiers ("B as in ball") — a new
  lexicon grammar (`clarifiers` phrases) that resolves the chip verbally.
- **Threshold is a calibration value** (like the racing pace bands): I will ship a
  PROPOSED default and calibrate against the Phase-5 loopback suite; final value
  needs sign-off (see Open items).
- **Covers acceptance:** foundation for #2's "bare `be` → chip" (the es b/v case in
  Phase 4 rides this).

### Phase 4 — Spanish lexicon completion + b/v rule  *(gated on G-B)*
**Goal:** full Spanish variant set; the answer-safe ambiguity rule.
- `lexicons/letters/es.json`: confirm/complete the variant set (already substantial);
  add es command + clarifier phrases (Phases 2–3 counterparts).
- `src/spell_aloud.rs`: **an unqualified "be" is ALWAYS ambiguous → chip** (never
  auto-resolved — auto-resolving would leak the answer). `eñe` stays first-class.
  Accents per G-B decision.
- **Covers acceptance:** #2 (spell "niño" incl. eñe; bare "be" always → chip).

### Phase 5 — whisper.cpp loopback harness (the CI oracle)  *(es suite gated on G-B)*
**Goal:** an automated oracle for the audio mode — the spec's "already in your
harness" built for real.
- New `tools/spell-aloud-loopback/` (or `scripts/`): TTS speaks letter sequences at
  **three speeds**; a **50-word suite per language × three synthetic voices**;
  transcribe via whisper.cpp, run tokens through the Rust `parse`/`interpret`, assert
  **≥95% letter accuracy**. Wire into CI (offline; the mode itself is device-only, so
  the oracle exercises the *parser* over synthetic ASR, documented as such).
- **Covers acceptance:** #3 (50-word suite, 3 voices × 3 speeds, ≥95%); the loopback
  form of #1 ("c, a, t, done" → CAT; "cat" → rejected).

### Phase 6 — Permission strings, airplane mode, denial handling
**Goal:** review-safe permissions and graceful failure.
- `ios/App/App/*.lproj/InfoPlist.strings` (new, per shipped language): localize
  `NSMicrophoneUsageDescription` + `NSSpeechRecognitionUsageDescription` — today
  English-only; **review-critical given the 2.3.6 rejection history.**
- Verify the on-device path works in **airplane mode** (should, by construction —
  `requiresOnDeviceRecognition = true`); **mic/speech permission denied** →
  graceful explain-and-exit (reuse the `PERMISSION_DENIED` → explainer path already
  in `on_error`), **no crash**.
- **Covers acceptance:** #4 (airplane-mode full round; permission-denied graceful).

### Phase 7 — Hub integration, entitlement depth, activation (do LAST)
**Goal:** the mode becomes discoverable at the approved depth.
- `config/modes.json`: reconcile the `spell_aloud` entry with the mode reshape
  (per G-C); keep `platforms:["ios"]`, `languages:["en","es"]`; flip `status`
  `hidden`→`live` only at activation.
- `src/entitlements.rs`: gate by **depth, not mode** — **propose PREVIEW depth is
  free** (a previewed en/es language reaches the mode); FULL adds nothing gating-wise
  in v1 unless Eric wants a paid depth split. Language gate stays `VOICE_SPELL_LANGS`
  (en/es only — the standing decision).
- i18n: all strings via the pipeline, 11 locales; `i18n-check` green.
- **Covers acceptance:** #8-style localization lint (parity across locales).

### Cross-cutting (every phase)
- **On-device only / offline:** enforced by construction (`requiresOnDeviceRecognition`);
  a test runs the parser flow with networking disabled.
- **Zero persistence:** no audio or transcript written to disk or sent anywhere;
  nothing outlives the attempt (Invariant I2/I5, already honoured in the plugin).
- **Isolation:** `src/spell_aloud.rs` imports no Climb/shield/scoring modules.

---

## Guardrails / invariants — enforced, not aspirational

- **The target word NEVER reaches the recognizer or the matcher** (G-A) — no
  answer-bias `contextualStrings`, no target parameter on `parse`/`interpret`.
  Enforced by the Phase-0 signature guard + the yield-only whole-word rule.
- **A whole-word utterance can NEVER score as a correct spelling** — structural: a
  spoken word yields ~0 letter-names, fills no slots. Test #1 (reject half).
- **Ambiguity resolves via the user chip, never via the answer** — bare "be" (es) is
  *always* a chip; the E-set/M-N chips never consult the target.
- **One letter-lexicon source-of-record per language** (Invariant I4) —
  `lexicons/letters/<lang>.json` only; commands + clarifiers go *in* those files, no
  scattered maps, no letter-name literal in Swift or elsewhere.
- **Zero audio/transcript leaves the device; nothing persisted past the attempt**
  (Invariant I2/I5).
- **On-device recognition only** — `available:false` means the mode is unavailable,
  never a cue to use server ASR.
- **en/es only in v1** — the standing decision, gated by `VOICE_SPELL_LANGS`.
- **No new authored word content** — only UI labels via i18n and letter/command
  phrases in the lexicons (G-B word-list restriction is STOP-AND-ASK).

---

## Acceptance-test → phase map

| # | Test | Lands in |
|---|---|---|
| 1a | Loopback "cat" → rejected, attempt not consumed | Phase 0 (+ oracle in 5) |
| 1b | Loopback "c, a, t, done" → CAT accepted | Phases 1 + 2 (+ oracle in 5) |
| 2 | es spells "niño" incl. eñe; bare "be" always → chip | Phases 3 + 4 |
| 3 | 50-word suite/lang, 3 voices × 3 speeds, ≥95% letter accuracy | Phase 5 |
| 4a | Airplane mode: full round works | Phase 6 |
| 4b | Mic/speech permission denied: graceful explain-and-exit, no crash | Phase 6 |
| — | Localization parity (11 locales) | Phase 7 |

## Sequencing recommendation

`0 → 1 → 2 → 3 → 4 → 5 → 6 → 7`, with the offline + isolation checks wired from
Phase 0. The English mode is playable after Phases 0–3; Spanish (4) and the loopback
es suite (5) unblock once G-B is decided. **Nothing ships to users until the whole
mode passes all acceptance tests and Eric approves activation** — the mode stays
`hidden` in `modes.json` until then, and the shipped tap-to-toggle input method is
untouched until G-C rules on its fate.

## Open items still needing your call (beyond the three gates)

- **Confusable confidence threshold** — a calibration value, not a guess I want to
  bury. I'll ship a PROPOSED default and tune it against the Phase-5 loopback suite,
  then bring the number for sign-off (same discipline as the racing pace bands).
- **Fate of the shipped input method** (part of G-C) — keep the mic-beside-the-field
  *and* the new mode, or retire the input method once the mode ships? Retiring is a
  reversal of shipped behaviour → your call.
- **Spoken echo default** — echo each accepted letter aloud by default (kid-friendly)
  vs. off by default (quieter, hands-free classrooms)? Proposing **on**, per-attempt
  toggle; confirm.
