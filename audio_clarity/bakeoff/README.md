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

**Cross-checked 2026-10-02, and four reports were affected.** `ar`, `en`,
`fil`, `hi`, `ko`, `pt`, `ru` and `vi` are clean: every scored candidate is a
distinct voice. The other four were scoring one voice under several labels —

| report | rows | actual voices |
|---|---|---|
| de | 11 | 5 |
| fr | 12 | 4 |
| es | 11 | 8 |
| pl | 6 | 4 |

— and each now carries the measured alias groups in its header. The giveaway
was already visible in the tables: aliased rows score identically to the
digit, six German rows at exactly 38/15/7.

`scripts/audio-bakeoff.mjs` catches this during the run, by hashing the first
three clips of each candidate and reporting a duplicate as an alias instead of
scoring it again. Those clips are scored anyway, so the check is free.

**It can prove sameness, never difference.** Google synthesis is not
deterministic — the same name, word and SSML returned three byte patterns over
six calls on 2026-10-02, all of identical length. Byte-identical audio from two
names therefore means one voice; a non-match means nothing, because two samples
of one voice can differ. Rows the guard does not flag are unconfirmed.

The first version of this check compared ONE clip and treated the absence of a
match as distinctness. It produced a false negative within the hour:
`fr-FR-Neural2-A` and `fr-FR-Wavenet-F` looked identical in one run and
different in the next. Three clips is better, not sufficient — the honest
method is comparing SETS of renderings across repeated samples, which the
harness cannot do through the server because the cache freezes the first
rendering per (voice, word).

It is a hash check rather than a catalogue lookup on purpose: the harness
holds no Google credential and reaches the API only through the server, and a
name can be perfectly listed and still be an alias.

**Two delisted shipped voices have a stable listed twin**, found 2026-10-02 by
sampling each five times directly against the API:

- `es-ES-Neural2-B` (shipped, delisted) = `es-ES-Neural2-G` (listed)
- `fr-FR-Neural2-A` (shipped, delisted) = `fr-FR-Neural2-F` (listed)

Both pairs were byte-stable across every sample. Renaming onto the listed twin
changes no audio, and neither language is in `VOICE_IN_KEY`, so the cache key
does not move and nothing re-warms. `pl-PL-Wavenet-B` has no such twin: the
listed Polish voices are genuinely different, so that one is a real choice.

**The German voice switch signed on 2026-09-28 is unaffected** —
`de-DE-Neural2-B` to `-A` was 33 to 38 Pass, and those two are different
audio, not two labels for one voice.
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
