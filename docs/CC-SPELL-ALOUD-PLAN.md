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

### Phase 0 — Answer-leak hardening of the parser  ✅ **LANDED** *(G-A ruled)*
**Goal:** make the core obey the invariant *the target never reaches the matcher.*
- `src/spell_aloud.rs`: `interpret(lang, transcript, target)` → **`interpret(lang,
  transcript)`**; removed `similarity(...)`, `levenshtein(...)`, and `WHOLE_WORD_SIM`.
  Whole-word rejection is now **yield-ratio-only** (`total_words > 0 && yield_ratio()
  < WHOLE_WORD_YIELD` ⇒ `WholeWord`; `total_words == 0` ⇒ `Nothing`). Added the **slot
  model**: `pub struct Slot { letters }` + `Parsed.slots: Vec<Slot>`, one entry per
  matched letter-name token (a multigraph like "elle"→"ll" is one slot). Removed the
  `TARGET` thread-local and the `s.word` read in `mic_tap` — the answer is no longer
  even loaded into the module.
- **Design note (worth Eric knowing):** without the target, "spoke a whole word" and
  "made babble" are the *same* low-yield signal, so they now share the `WholeWord`
  nudge; only a truly empty utterance is `Nothing`. This merged the old
  `noise_yields_nothing` case — the shipped input method's UX is unchanged for the
  common paths (genuine spelling inserts; whole word nudges).
- `src/spell_aloud/tests.rs`: rewrote the target-dependent tests; added a
  compile-time **signature guard** (`let _: fn(&str,&str)->SpellOutcome = interpret;`)
  and a slot test (whole word → 0 slots; "see ay tee" → `[c,a,t]`; slots concat == letters).
- **Covers acceptance:** the *reject* half of #1 ("cat" → rejected, attempt not
  consumed); the invariant "a whole-word utterance can NEVER score as a correct spelling."
- **Result:** `spell_aloud` 20/20; full lib suite **257/0**; wasm release builds clean.

### Phase 1 — Mode surface: push-and-hold + slots + echo  ✅ **LANDED** *(G-C ruled)*
**Goal:** the feature becomes a *place* with kid-simple turn-taking.
- New `src/spell_aloud/screen.rs` (the shipped input method in `super` is untouched —
  G-C keeps both). **Push-and-hold** mic: `pointerdown` = `start_letter_capture`,
  `pointerup/leave/cancel` = `stop_letter_capture` (finalizes). Each finalized
  utterance folds into a slot buffer via the pure `super::accept_into` (Phase 0);
  accepted letters render one slot per letter and are echoed on-device
  (`native_lang::speak`, space-separated so they read as letters). A whole word /
  babble nudges; **no submit** (turn completion via `done` is Phase 2). No target is
  ever read (G-A).
- `index.html`: `#spellAloud` surface (slots row, hold-mic, status) + hidden
  `#spellAloudOpen` entry (reachable only there until Phase 7 activation) + `.sa-*` CSS.
- `src/lib.rs`: `spell_aloud::screen::wire(app)`.
- Reuses the plugin events, `parse`/`interpret`/`contextual_strings` unchanged, and
  the existing `voiceSpell.*` / `tools.spellaloud.*` i18n keys (no new-key churn).
- **Covers acceptance:** the *accept* half of #1 ("c, a, t" fills CAT), pending the
  `done` command in Phase 2.
- **Verified:** host tests for `slots_html`/`assembled`/`accept_into` (23/23 spell_aloud,
  full lib suite **260/0**); wasm builds clean; browser-verified the surface opens and
  renders (empty state + filled `C A T` slots). Real push-and-hold capture is
  device-only (iOS native mic) and untestable in-browser.

### Phase 2 — Spoken commands (delete/borrar, clear, done/listo)  ✅ **LANDED**
**Goal:** hands-free editing and turn completion.
- `lexicons/letters/en.json` + `es.json`: new `commands` block (en:
  `delete`/`backspace`/`undo`/`back`, `clear`/`clear all`/`start over`/`erase all`,
  `done`/`finished`/`finish`/`enter`; es: `borrar`/`borra`/`borrar uno`/`atrás`,
  `borrar todo`/`empezar de nuevo`/`limpiar`, `listo`/`ya`/`terminé`/`hecho`) — in the
  **same single-source-of-record file** (I4), never a scattered map.
- `src/spell_aloud.rs`: a parallel `commands` table + `Command{Delete,Clear,Done}` and
  `Event{Letter,Command}`. New **`events(lang, transcript)`** does greedy longest match
  over letters AND commands (commands win ties; "borrar todo" beats "borrar"); letters
  and commands interleave ("c a t done"). **`apply_events(buffer, evs) -> Applied`**
  applies them: letters push, Delete pops, Clear empties, Done flags submit. The plain
  `parse`/`interpret` (input-method path) stay command-free.
- `src/spell_aloud/screen.rs`: the mode now drives its turn from `events`/`apply_events`;
  `done` submits the assembled word through the normal answer path (`set_answer` +
  `submit_guess`) then clears + closes. Whole word / babble → nudge (no events).
- **Covers acceptance:** completes #1 (`events("see ay tee done")` →
  `[c,a,t,Done]` → buffer "cat", submit); the *reject* half stays (a whole word
  yields zero events).
- **Verified:** spell_aloud 29/29 (incl. every-command-parses, greedy es, interleave,
  buffer edits); full lib suite **266/0**; wasm + web build clean.

### Phase 3 — Confusable handling + confidence surfacing  ✅ **LANDED** *(runtime needs a device mic)*
**Goal:** below-threshold letters offer a two-choice chip instead of guessing.
- **Rust core (LANDED, tested):**
  - **Clarifiers** — `lexicons/letters/{en,es}.json` `clarifiers` connectors
    (en `as in`/`for`/`like`; es `de`/`como en`/`como`). `events()` now confirms a
    letter and **drops the example word** after a connector, so "b as in **you**" is
    `[b]`, not `[b, u]`. Fully functional verbal disambiguation.
  - **Confusable classes** — `confusable` groups in the lexicon (en E-set
    `[b c d e g p t v z]` + `[m n]`; es `[b v]`). `are_confusable(lang, a, b)` +
    `decide_letter(lang, letter, confidence, alt) -> Accept | Chip{a,b}`: a confident
    or non-confusable letter is accepted; a low-confidence confusable letter with a
    same-class alternative offers a two-choice chip. Target never consulted (G-A).
  - **`CONFUSABLE_CONFIDENCE = 0.55` is a PROPOSED calibration value** (like the pace
    bands) — tune against the Phase-5 loopback suite; **needs Eric's sign-off** before
    the confidence-gated chips go live. *(Open item.)*
  - Verified: spell_aloud **32/32**; full lib suite **269/0**.
- **Native confidence feed (LANDED, build-validated):**
  - **Swift:** `SpeechListener` now captures the least-confident final segment's
    `confidence` + its top `alternativeSubstrings` reading and reports them; the plugin
    emits `letterFinal` as `{token, confidence, alt}` (**additive-safe** — the input
    method reads only `token`). `native-language-kit.js` forwards the object;
    `native_lang::start_letter_capture`'s `on_final` is now
    `(transcript, confidence, alt)` and parses it (string payloads still work).
  - **Rust wiring:** `screen::on_final` — in the single-letter turn, the alternative is
    parsed to a letter and `decide_letter(lang, letter, confidence, alt)` runs; a
    low-confidence confusable → `LetterDecision::Chip` → the `#saChip` UI (built in
    Phase 4). The input method ignores confidence/alt.
  - **Verified:** `xcodebuild` simulator build SUCCEEDED; full lib suite 277/0; wasm +
    web build clean. Runtime chip behaviour needs a device mic (real ASR confidence).
- **Chip UI:** built + browser-verified in **Phase 4** (bare es "be" gives it a
  deterministic trigger); the confidence-gated path above now feeds the same UI.
- **Covers acceptance:** foundation for #2's "bare `be` → chip" (Phase 4).

### Phase 4 — Spanish lexicon completion + b/v rule  ✅ **LANDED** *(G-B ruled)*
**Goal:** full Spanish variant set; the answer-safe ambiguity rule. *(This phase also
builds the two-choice chip UI deferred from Phase 3 — the bare-"be" rule is its first
deterministic trigger.)*
- `lexicons/letters/es.json`: new `ambiguous` block (`be`/`ve` → `[b, v]`) alongside
  the qualified variants (`be larga`/`ve corta`/`uve`/`doble ve`…) — all already
  present. Commands + clarifiers landed in Phases 2–3.
- `src/spell_aloud.rs`: `events()` gained an `Event::Chip(a,b)`. **An unqualified
  "be"/"ve" ALWAYS chips `{b,v}`** — never auto-resolved (that would leak the answer).
  Qualified names (`be larga`→b), `uve`→v, multigraphs (`doble ve`→w), and clarifiers
  resolve without a chip; **`be de burro`→b / `ve de vaca`→v** via the *example word's*
  first letter (user speech, not the target — G-A safe). `apply_events` surfaces a
  pending chip and holds the rest of the utterance. **The input-method `parse` is
  UNCHANGED** — bare `be`→b/`ve`→v there, so "libro"/"verde" still spell correctly.
- `src/spell_aloud/screen.rs` + `index.html`: the **`#saChip` two-choice UI** —
  `show_chip`/`resolve_chip`, delegated tap handler, `voiceSpell.pick` i18n (11 locales,
  325 keys parity). `eñe` stays first-class precomposed.
- **Covers acceptance:** #2 — `events("ene i eñe o")`→"niño" (no chip); `events("be")`
  → `Chip{b,v}` always.
- **Verified:** spell_aloud **37/37**; full lib suite **274/0**; browser-verified the
  chip renders (¿Cuál letra? · B / V) and a tap resolves (buffer + re-render).
- **Note:** the *confidence-gated* chips from Phase 3 (`decide_letter`) light up once
  the deferred native confidence feed lands; the chip UI itself is now done.

### Phase 5 — whisper.cpp loopback harness (the CI oracle)  ✅ **LANDED** *(G-B ruled)*
**Goal:** an automated oracle for the audio mode. *(The spec's "already in your
harness" whisper loop did NOT exist — the closest precedent is `tools/audio-verify/`,
whose fail-closed/offline conventions this mirrors.)* Delivered in two halves:
- **Offline oracle (runs on every PR)** — `src/spell_aloud/loopback.rs`: a **50-word
  suite per language** of realistic spoken transcripts (ASR homophones: see/sea→c,
  why→y, double u→w) scored for **letter accuracy ≥0.97** through the SAME parser, plus
  the loopback form of **#1** ("see ay tee done"→CAT+done; "cat"→rejected) and the es
  b/v chip. Deterministic, no audio. **en 0.97+ · es 0.97+.**
- **CLI bridge** — `examples/spell_aloud_parse.rs` (needs `pub mod spell_aloud`): reads
  `(lang, transcript)` → JSON `{letters, outcome, mode_word, done, chip}`, so any
  harness scores real ASR against the app's own parser — no duplicated letter logic (I4).
- **Real whisper harness (offline/manual)** — `tools/spell-aloud-loopback/loopback.py`
  + README: system TTS (`say`/`espeak-ng`) speaks letters at **3 speeds × 3 voices**,
  **whisper.cpp** transcribes, the CLI bridge parses, letter accuracy gated at **≥95%**.
  Fail-closed: no whisper/model/TTS → prints the exact setup command and exits 0
  (verified — skips cleanly here). Runs where the binaries exist, like `audio-verify`.
- **Covers acceptance:** #3 (50-word suite, 3 voices × 3 speeds, ≥95% — the audio bar
  in the harness, the parser bar in CI); the loopback form of #1.
- **Verified:** spell_aloud loopback 3/3; full lib suite **277/0**; harness skip-path OK.

### Phase 6 — Permission strings, airplane mode, denial handling  ✅ **LANDED**
**Goal:** review-safe permissions and graceful failure.
- `ios/App/App/{en,es}.lproj/InfoPlist.strings` (new): localized
  `NSMicrophoneUsageDescription` + `NSSpeechRecognitionUsageDescription` (+ camera/
  photo for Photo Import). **Spanish is the one that actually triggers** — Spell Aloud
  is es/en only, so a Spanish device now sees Spanish permission prompts instead of
  English (review-critical, guideline 2.3.6). Wired into `App.xcodeproj` as an
  `InfoPlist.strings` PBXVariantGroup (`SOURCE_ROOT`-anchored so Xcode can't misplace
  it) + `es` in `knownRegions`. **Build-validated:** `xcodebuild … BUILD SUCCEEDED`;
  the es strings are present in `App.app/es.lproj/InfoPlist.strings`.
- **Airplane mode** works by construction — `SpeechListener` sets
  `requiresOnDeviceRecognition = true` and refuses to start without on-device
  capability (fail-closed, no server fallback). **Permission denied** → the Rust
  `on_error` maps `PERMISSION_DENIED`/`UNAVAILABLE` to the gentle explainer and always
  reverts to typed text — **no crash** (in both the input method and the mode).
- **Covers acceptance:** #4 (airplane-mode round by construction; permission-denied graceful).
- **Note:** English is the dev-language fallback (stays in `Info.plist` + `en.lproj`).
  Other UI locales don't localize these because their Spell-Aloud entry is hidden
  (mode is en/es); camera/photo strings localize per language when Photo Import ships.

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
