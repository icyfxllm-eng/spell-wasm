# CC-AUDIO-REPLAY — the replay banner

**Reported.** Eric, on device, build 202: tapping "hear it again" showed
"Couldn't reach the audio server for this word — spelling still counts."

**What was actually wrong.** Not the audio. The word was audible; the banner
was lying about it.

The router resolves one word through an ordered chain (`api.rs`):

    Pack -> ServerCache -> NativeTts

Only the router knows how that chain ended. But the banner was painted by the
`<audio>` element's own `onerror`, deep inside the ServerCache source. So a
ServerCache miss — an expired clip, a network blip, a cold cache — announced
total failure, and NativeTts then went on to speak the word perfectly. The
message was true about one source and false about the outcome.

Nothing cleared it, either. `voiceNote` is only reset when the WORD changes
(`game::update_voice_note`), so a single transient miss left the banner on
screen for the rest of that word, including across every later successful
replay tap.

**The law.** The banner belongs to the router's wall, not to a source. Only
`set_source` paints it, only on `"none"` — every source tried, none played —
and any source that succeeds clears it. Enforced by
`scripts/audio-router-check.mjs`, whose selftest lesions each law separately
and fails the build if any law survives its lesion.

## Four defects on the same path, found while fixing it

**A cached element is a corpse once it fails.** `play_word_html` reused the
`CURRENT` `<audio>` element when the word/variant/lang key matched. An
`HTMLMediaElement` LATCHES its failure: once `error` is set, `play()` on that
element can only reject. So the first failure made every subsequent "hear it
again" tap on that word re-fail against the same dead element, without ever
re-entering the router. The reuse test now also requires `.error().is_none()`,
and a dead element is dropped rather than replayed.

**`play()`'s promise was discarded.** `let _ = audio.play()` threw away the
rejection, so a failure never reached `on_fail` and never advanced the router.
Worse, `set_source("server-cache")` was recorded on the line after — before
playback had started — so the source readout the QA and loopback harnesses
depend on reported successes that never happened. The promise is awaited now,
and the source is recorded only once audio is actually playing.

**But AbortError is not a source failure.** Awaiting the promise introduced a
new way to be wrong: tapping replay faster than the clip loads makes the older
`play()` reject with `AbortError` because a newer one superseded it. The word
IS playing. Treating that as a failure would advance the router and stack
native TTS on top of audible speech. `AbortError` is now explicitly not a
failure — which is the whole reason the rejection is inspected rather than
merely counted.

**`onerror` was a `Closure::once`.** A media element can fire `error` more than
once. The second fire would invoke an already-consumed `FnOnce` and abort wasm
— a crash, not a degraded playback. It is an `FnMut` now, and the two failure
routes (the `error` event and the rejected promise) share one
take-once cell so whichever arrives first is the only one to advance the router.

## Investigated and deliberately NOT changed

`audio-native.js`'s LRU skips the `unload` when the clip being evicted is the
currently-playing one, while still deleting it from the `preloaded` map. That
reads like a map/native desync that would make iOS reject the next `preload`
of that assetId with "Audio Asset already exists" — permanently muting the one
word you are most likely to replay.

It is unreachable. Every insertion into `preloaded` happens in the same step
that assigns `current`, so `current` is always the most-recently-used entry and
can never be the oldest. Verified by executing the real bridge against a fake
Capacitor layer that rejects duplicate preloads the way iOS does: the replay
succeeds and the asset unloads normally.

Recorded here because it was claimed as the root cause twice before it was
tested, and because a plausible-looking fix to unreachable code would have been
change without a defect — noise in the diff, and a new chance to break the
thing it pretended to repair.
