# F4 bake-offs — scope, and the three languages that cannot run one

`scripts/audio-bakeoff.mjs --lang <l> --voices a,b,c` ranks candidate voices by
how often two independent recognizers hear the right word, and RECOMMENDS ONLY:
switching a language's default needs Eric's signature, per language (D6).

## Which languages can be measured

Eleven: **es, fr, de, pt, pl, ru, vi, ko, fil, ar, hi**.

**How candidates must be found, and how they were found the first time.**
Enumerate the voices Google actually offers:

```
GET https://texttospeech.googleapis.com/v1/voices?languageCode=<lc>
```

The original lists (2026-09-27) were built differently: one word was
synthesized per guessed `<locale>-<family>-<letter>` name and the names that
answered 200 were kept. **A 200 does not prove a voice exists.** Google
answers a name it does not have by serving something else, so a guessed name
produces a real clip, a real transcript and a real-looking row.

That was found on 2026-10-01 in `fil`, where two scored candidates were
`fil-PH-Neural2-A` and `-D`. Google names them `fil-ph-Neural2-A` and `-D`,
with a **lowercase region** — the only language of the fifteen where it does
this. Both wrong-case names returned 200, and the audio was a substitute.

**The other reports' candidate lists have not been re-verified.** Cross-checking
them against the live catalogue is cheap and has not been done; until it is,
treat any row in them as possibly describing a voice that was never asked for.
A related trap found at the same time: four SHIPPED voices (`de-DE-Neural2-A`,
`es-ES-Neural2-B`, `fr-FR-Neural2-A`, `pl-PL-Wavenet-B`) are no longer in
Google's catalogue at all, yet still synthesize. Delisted is not withdrawn —
but it is the warning before it.

**Standard voices are excluded before scoring**, the same way F4 step 2
excludes a wrong-variety voice: Standard is the generation Wavenet and Neural2
replaced, so moving a language onto one would be a downgrade whatever it
scored. Note that pl and ru have no Neural2 voices at all — Wavenet is their
top tier, which is what LANG_VOICES already uses for both.

## The three that cannot, and why it is not a scoping choice

- **ja — unscorable by census C8.** A recognizer may return kanji for a kana
  entry, which would read as a false FAIL. The comparison key is the thing
  that is undecided, so there is nothing to rank against. Eric's call, and the
  spec's own mechanism for exactly this.
- **zh — the backend cannot serve a candidate voice.** `synthesize_to_cache`
  dispatches to `_synthesize_zh` *before* the `voice_override` branch is
  reached, so a zh bake-off request is synthesized in the default voice and
  every candidate would score identically. Mandarin also requires a `py`
  reading on every request (Invariant 4), which the bake-off does not send.
- **sw — same shape.** Swahili is Azure, and `_synthesize_azure` is likewise
  dispatched before the override, so `?voice=` does nothing. Azure's Swahili
  catalogue is four named voices (see AZURE_VOICES in backend/app.py), so a
  bake-off there is worth having.

Both zh and sw are a backend change — thread `voice_override` through those two
paths, and give zh's bake-off URLs a reading — which means a deploy, and the
standing default is that Eric deploys. Neither is a large change; they are
listed here so the gap is a decision rather than an omission.
