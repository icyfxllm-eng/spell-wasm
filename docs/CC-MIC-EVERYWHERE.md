# Mic-everywhere — Spell It voice input in every language

Eric, 2026-07-28: "i tried the mic feature on every language and other than
english the mic just disappears when you try to click how can spell it be
added to EVERY language"

## Root cause

The mic renders only when on-device speech recognition is possible for the
language (`SpeechCapabilities.report` → `SFSpeechRecognizer
.supportsOnDeviceRecognition`). iPhones only carry on-device models for the
languages the user has enabled as **dictation keyboards** — on Eric's phone
that's English. Every other language failed closed (correctly, per the
privacy doctrine) and the tap-time re-check hid the mic.

## Fix — the capability ladder (all rungs 100% on-device)

`SpeechCapabilities.fullReport(lang)` now tries, in order:

1. **legacy** — `SFSpeechRecognizer` with an installed on-device model
   (exactly the shipped path; still first so nothing regresses).
2. **analyzer** — iOS 26 `SpeechTranscriber` (best models). Assets are
   **downloadable per locale** via `AssetInventory`.
3. **dictation** — iOS 26 `DictationTranscriber` (much wider locale set),
   same downloadable assets.

The report's new `state` drives the UI:

| state | mic | tap |
|-------|-----|-----|
| installed | live | starts capture (as before) |
| downloadable | shown with ⬇ badge | downloads the voice pack (progress % in the status line), then goes live |
| unavailable | hidden | — |

New plugin method `downloadSpeechAssets(lang)` streams `speechAssetProgress`
events; the web bridge exposes `downloadSpeechAssets(lang, onProgress)`;
`spell_aloud::mic_tap` runs the download flow on a downloadable-state mic.

## New capture engine

`SpeechListener` gained an iOS-26 analyzer path sharing the SAME tap + VAD
one-press structure as the legacy engine:

- buffers convert (AVAudioConverter) into one continuous
  `AsyncStream<AnalyzerInput>` — the stream never closes mid-word, so the
  legacy path's finalize-gap buffer stash isn't needed;
- each VAD letter boundary calls `analyzer.finalize(through: nil)` — the
  finalized text is that letter's segment; the session keeps listening;
- user stop closes the stream + `finalizeAndFinishThroughEndOfInput`.

The privacy doctrine is untouched at every rung: `available` is never true
without on-device support, downloads fetch Apple's LOCAL model assets, and
nothing ever falls back to a server.

## Server rung (added 2026-07-28 — Eric: "make the mic usable with ALL languages")

For languages with NO on-device path (sw, fil in practice), a fourth rung:
`spell_aloud` shows the mic with a 🌐 badge; the FIRST tap opens an explicit
consent card ("your audio goes to the Spell server only to be recognized,
then it's gone — never stored"); OK persists (`spell_stt_ok`) and capture
runs through `SpeechListener`'s server engine — same tap + VAD one-press,
segments converted to 16k mono LINEAR16 and POSTed to `/api/stt` (backend →
Google Cloud Speech-to-Text with letter-name `speechContexts` biasing).

Hard exclusions, enforced in `reflect`: never Kid Mode, never education
builds, never without consent, and never when ANY on-device rung exists.
The backend recognizes and discards — nothing written, nothing logged.

**BLOCKED ON ERIC**: the Google API key rejects Speech-to-Text
("Requests to this API speech.googleapis.com … are blocked"). In Google
Cloud console: enable **Cloud Speech-to-Text API** for the project AND, if
the key has API restrictions, add Speech-to-Text to its allowed list. The
endpoint is deployed and tested end-to-end up to that 403.

## Honest limits (flag for Eric)

- Which locales the on-device engines serve is a RUNTIME answer per
  device/OS. Whatever falls through lands on the consented server rung, so
  every language now has A path (Kid Mode stays on-device-only: a Kid-Mode
  child on sw/fil simply has no mic — by design).
- `AssetInventory` has a per-app reserved-locale budget (small, ~3): heavy
  language-switchers may hit DOWNLOAD_FAILED until an older pack is released.
  Not handled in v1 — retry after switching stays possible; ledgered.
- The new-engine path drops `contextualStrings` biasing (no public equivalent
  on SpeechAnalyzer) — the Rust lexicon parser (homophones/multigraphs)
  carries disambiguation alone there. Letter accuracy on analyzer languages
  needs Eric's on-device pass.
- Simulator fails closed for all engines (no speech assets) — on-device test
  only. Legacy behavior for English is byte-for-byte unchanged.
