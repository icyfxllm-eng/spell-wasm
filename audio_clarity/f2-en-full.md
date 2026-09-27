# F2 over the whole English bank — 2026-09-27

3165 words, the `normal` variant, both recognizers, blind (I3). This is the
first measurement of a complete bank; every earlier run was a 60- or 68-word
probe list.

| Verdict | Clips | Share |
|---|---|---|
| Pass | 2927 | 92.48% |
| Weak | 141 | 4.45% |
| ExemptHomophone | 62 | 1.96% |
| Fail | 35 | 1.11% |

**1.11% FAIL**, against the 10% at which F7 section 10 says to stop and look
for a pipeline or voice fault. So there is no systemic fault here — but the 35
are not 35 bad clips either, and that is the finding.

## What the 35 failures actually are

**5 — number or letter form the recognizer wrote as a symbol.**

- `am` -> 'M' / 'm'
- `an` -> 'N' / 'n'
- `sees` -> "C's" / 'C'
- `tend` -> '10.' / '10'
- `ones` -> '1s' / 'on'

**8 — a true or near homophone missing from the collision table.**

- `adds` -> 'ads' / 'ads'
- `patients` -> 'Patience' / 'patience'
- `whose` -> "Who's?" / "who's"
- `discussed` -> 'Disgust' / 'disgust'
- `plays` -> 'Please.' / 'please'
- `coat` -> 'Code' / 'code'
- `search` -> 'Surge' / 'surge'
- `axis` -> 'Access' / 'access'

**6 — a rare word the recognizers do not appear to know.**

- `chiaroscuro` -> 'Kiara Skuro' / 'Kiara skurow'
- `connoisseur` -> 'Kana Sir' / 'Kana sir'
- `ebullient` -> 'Abolient' / 'a byant'
- `inchoate` -> 'In Poet' / 'in poet'
- `mnemonic` -> 'Nemanik' / 'pneumonic'
- `onomatopoeia` -> 'Anamata Pia' / 'anamata Pia'

**16 — the clip itself is in question.**

- `aims` -> 'Ames' / 'as'
- `and` -> 'End.' / ''
- `ban` -> 'then' / 'then'
- `bay` -> 'They' / ''
- `bear` -> 'There.' / 'there'
- `ben` -> 'Then,' / 'then'
- `chip` -> 'Ship' / 'ship'
- `choose` -> 'Shoes' / 'shoes'
- `chose` -> 'Shows' / 'shows'
- `coup` -> 'Who?' / 'who'
- `dot` -> '.' / 'do'
- `managed` -> 'Manage.' / 'manage'
- `rate` -> 'Great.' / 'great'
- `rear` -> 'ROOR-R-E-R' / 'Maria'
- `row` -> 'Bro.' / ''
- `than` -> 'Then,' / 'then'

## The recommendation: do NOT mark `en` measured yet

Adding `en` to `measured` is what arms I1, and I1 withholds a FAIL clip and
lets the resolver fall through. Three things say that would make the game
worse, not better, today:

1. **Most of these are not failures of the audio.** By the grouping above,
   somewhere around nineteen of the thirty-five are a recognizer writing a
   convention (`am` -> "M"), colliding with a homophone that is simply not in
   the table yet (`patients` / "patience"), or not knowing the word at all
   (`onomatopoeia`). The clip is fine in every one of those cases.
2. **The fallback is worse for this class.** Measured the same day (see
   `bakeoff/en.md`): macOS Samantha, the AVSpeechSynthesis family iOS falls
   through to, loses `half`, `leaf` and `safe` — the three words the voice
   switch was made to fix. Withholding a Google clip hands the player a less
   intelligible one.
3. **The list is full of extremely common words.** `and`, `an`, `am`, `bear`,
   `coat`, `rate`, `than`, `search`, `chip`, `choose`. Silencing those would be
   the most visible change the audio has ever had, on the strength of a
   measurement that is wrong about most of them.

None of that is an argument against I1. It is an argument that the gate needs
either the homophone table extended and the conventions folded first, or a
per-word exemption that an auditor signs — which is F3, and F3 is what
`scripts/f3-worklist.mjs --lang en` now hands them.

## What is worth a second look, and what would settle it

Three words fail as a **class**: `chip` -> "ship", `choose` -> "shoes",
`chose` -> "shows". Every one loses the stop closure of the affricate /tʃ/,
leaving /ʃ/, and both recognizers agree each time. That is the same shape as
the word-final /f/ -> /v/ class that started this whole feature, and it is a
property of a voice rather than of a word.

It is **not** a duration artifact: measured at the `slow` variant as well, all
three are heard identically wrong, so the affricate is not being clipped by
rate or padding.

A second group is suggestive and much less certain — `ban`, `ben`, `than`,
`bear`, `bay` all heard with a /ð/ onset ("then", "there", "they"). These are
one-syllable, very high-frequency neighbours, and a recognizer given a single
word with no context leans on frequency, so this may be a property of the
MEASUREMENT rather than of the clip. Distinguishing the two needs the same
words in another voice, which is what the F4 machinery does.

**The honest limit of this method, stated plainly:** ASR on an isolated
single word is not a human listener. It is a good proxy — it found `half`,
which a tester independently reported — but `onomatopoeia` failing tells us
about the recognizer's vocabulary and nothing about whether a player can hear
the word. A FAIL is a reason to look, not a verdict on the clip.

## Coverage, and what is not covered

The `slow` variant is **not** measured. It is the rescue path, a different
question, and doubling the run to include it would also have doubled the bill
before anyone had read the first half. Naming it here rather than letting
`measured` imply it is the reason `en` is not in that list on coverage grounds
as well.
