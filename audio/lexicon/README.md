# Pronunciation overrides (CC-AUDIO-CLARITY v1.1 F3)

One file per language: `<lang>.tsv`, tab-separated, with this header:

    word	ipa	provider	auditor	signed_at

Fix a word once, at the source, without swapping the voice.

**An entry may only come from a native-speaker auditor.** Never from an LLM,
never from a grapheme-to-phoneme tool, unless an auditor has signed the result.
The `auditor` and `signed_at` columns are not decoration: an unsigned row is a
guess about how a language sounds, and this file exists because guesses are
what broke the audio in the first place.

Two more rules, both enforced rather than trusted:

- **An override counts only after its regenerated clip passes F2.** A
  successful API call is not evidence that it applied. `scripts/audio-verdicts.mjs`
  is what says it worked.
- **An entry belongs to one provider.** A voice from a different provider needs
  its own row, entered and verified separately (F3 step 7). Russian stress and
  Mandarin tones are where this bites.

The entry's hash is part of the clip's cache key, so editing one row
regenerates exactly that one clip and nothing else.

## What this is for

The Phase B pilot found `half` transcribed as "have" and `leaf` as "leave" by
two independent recognizers, while `thief` and `fifth` passed — word-final /f/
rendered close enough to /v/ to be misheard, in `en-US-Neural2-D`. A row here
with the IPA an auditor signs off is the narrow fix. The wide fix is a
different voice, which is F4's bake-off and needs Eric's signature per
language (D6).

No rows are shipped. Both candidate words are waiting on an auditor.
