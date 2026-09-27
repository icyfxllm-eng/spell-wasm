# Bake-off — en

68 words per voice, both recognizers, blind (I3).

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| en-US-Neural2-E | 61 | 4 | 3 |
| en-US-Neural2-G | 57 | 8 | 3 |
| en-US-Neural2-J | 57 | 8 | 3 |
| en-US-Neural2-H | 56 | 8 | 4 |
| en-US-Neural2-C | 55 | 11 | 2 |
| en-US-Neural2-D | 54 | 8 | 6 |
| en-US-Neural2-I | 53 | 9 | 6 |
| en-US-Neural2-F | 49 | 12 | 7 |
| en-US-Neural2-A | 47 | 11 | 10 |

## Missed by both recognizers

### en-US-Neural2-E

- for -> "4." / "4"
- some -> "Sum" / ""
- hiss -> "His." / "h"

### en-US-Neural2-G

- for -> "4." / "4"
- some -> "sum" / ""
- hiss -> "Yes." / "piss"

### en-US-Neural2-J

- for -> "4." / "4"
- some -> "sum" / ""
- hiss -> "His." / ""

### en-US-Neural2-H

- of -> "\"Bove\"" / ""
- for -> "4." / "4"
- he -> "key" / ""
- cup -> "pub" / "tub"

### en-US-Neural2-C

- for -> "4." / "4"
- hiss -> "Yes." / "h"

### en-US-Neural2-D

- leaf -> "Leave." / "leave"
- for -> "4." / "4"
- its -> "Edge" / "kids"
- some -> "sum" / ""
- hiss -> "Here's" / ""
- safe -> "Save." / "save"

### en-US-Neural2-I

- sock -> "Suck!" / "suck"
- for -> "4." / "4"
- cup -> "Cub" / "cub"
- kite -> "Cade." / "Kate"
- hiss -> "His." / "yes"
- roof -> "Ruth." / "Ruth"

### en-US-Neural2-F

- hill -> "Hell." / ""
- for -> "4." / "4"
- some -> "sum" / ""
- corn -> "Horn." / "horn"
- kite -> "height" / "height"
- hiss -> "Yes." / "h"
- fifth -> "FES" / "5"

### en-US-Neural2-A

- leaf -> "leave" / "Le"
- of -> "Love." / ""
- for -> "4" / "4"
- his -> "Here's" / ""
- its -> "It." / ""
- bed -> "Bad." / ""
- cup -> "Hop!" / ""
- duck -> "Doc" / "doc"
- kite -> "height" / "height"
- hiss -> "Yes." / ""


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### en-US-Neural2-E

- fox -> "== Fox" / "Fox"
- he -> "he" / ""
- her -> "Huh." / "her"
- fifth -> "Fifth" / "5th"

### en-US-Neural2-G

- hat -> "Had." / "hat"
- ship -> "Shep" / "ship"
- he -> "He" / ""
- cat -> "CAD" / "cat"
- cup -> "Cub" / "cup"
- book -> "bug" / "book"
- corn -> "corn" / "horn"
- fifth -> "Fifth" / "5th"

### en-US-Neural2-J

- sun -> "Sun" / ""
- ship -> "Shit." / "ship"
- he -> "he" / ""
- had -> "had" / ""
- has -> "has." / ""
- duck -> "duck" / ""
- half -> "half" / ""
- fifth -> "Fifth." / "5th"

### en-US-Neural2-H

- as -> "as" / ""
- some -> "Some" / ""
- very -> "Very." / ""
- book -> "book" / ""
- duck -> "Duck" / ""
- kite -> "kite" / "tight"
- hiss -> "hiss" / "his"
- fifth -> "Fifth." / "5"

### en-US-Neural2-C

- horse -> "PORSE" / "horse"
- sock -> "sock" / "socked"
- he -> "he" / ""
- some -> "sum" / "some"
- her -> "Huh." / "her"
- bed -> "bad" / "bed"
- cup -> "Cup" / ""
- book -> "Buck." / "book"
- desk -> "Dask" / "desk"
- duck -> "Duck" / "doc"
- fifth -> "Fifth." / "5th"

### en-US-Neural2-D

- ship -> "Shep" / "ship"
- hill -> "Hill" / ""
- sock -> "SOC" / "sock"
- his -> "Here's" / "his"
- duck -> "Done." / "duck"
- half -> "Have." / "half"
- fifth -> "Fifth." / "5th"
- roof -> "Roof" / "Ruth"

### en-US-Neural2-I

- sun -> "Sun" / ""
- ship -> "Shed" / "ship"
- he -> "He." / ""
- had -> "had." / ""
- its -> "idz" / "it's"
- some -> "Some" / ""
- very -> "Very." / ""
- cat -> "CAD" / "cat"
- book -> "Bug" / "book"

### en-US-Neural2-F

- ship -> "Shep" / "ship"
- his -> "Here's" / "his"
- he -> "he" / ""
- had -> "had" / ""
- her -> "Huh." / "her"
- if -> "if" / ""
- very -> "Very." / ""
- cat -> "Cat" / "hat"
- cup -> "Cup" / ""
- tree -> "Tree" / "3"
- bird -> "bird" / "Verge"
- gift -> "Guest" / "gift"

### en-US-Neural2-A

- sun -> "Sun" / ""
- ship -> "Shit!" / "ship"
- hill -> "Hill" / ""
- as -> "As." / ""
- he -> "he" / ""
- had -> "Had." / ""
- some -> "Some." / ""
- if -> "If" / ""
- cat -> "Cat" / "chat"
- book -> "Buck!" / "book"
- half -> "half" / ""


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).

---

## Correction, 2026-09-27 — two scoring bugs, and the winner is unchanged

The table above was produced by a normaliser that had two faults, both found
while wiring F2 over the full English bank:

1. **Digit forms were never folded.** `DIGIT_WORDS` was declared in
   `audio-verdicts.mjs` with a comment explaining exactly why it was needed,
   and then applied to nothing. That is why `fifth` appears as a split
   decision for every single voice: one recognizer wrote "Fifth" and the other
   wrote "5th", and "5th" was scored as a different word.
2. **whisper.cpp decoration was not stripped.** It prefixes some transcripts
   with `==` — the `fox -> "== Fox" / "Fox"` line above is a clip both
   machines heard perfectly, scored Weak on punctuation.

Both tools now compare through one shared normaliser
(`scripts/lib/audio-norm.mjs`) instead of two drifting copies; the bake-off's
was the older one and had neither fix.

Re-scored from the transcripts already recorded above:

| Voice | Pass | Weak | Fail | was |
|---|---|---|---|---|
| en-US-Neural2-E | 63 | 2 | 3 | 61 / 4 / 3 |
| en-US-Neural2-G | 58 | 7 | 3 | 57 / 8 / 3 |
| en-US-Neural2-J | 58 | 7 | 3 | 57 / 8 / 3 |
| en-US-Neural2-C | 56 | 10 | 2 | 55 / 11 / 2 |
| en-US-Neural2-H | 56 | 8 | 4 | 56 / 8 / 4 |
| en-US-Neural2-D | 55 | 7 | 6 | 54 / 8 / 6 |
| en-US-Neural2-I | 53 | 9 | 6 | 53 / 9 / 6 |
| en-US-Neural2-F | 49 | 12 | 7 | 49 / 12 / 7 |
| en-US-Neural2-A | 47 | 11 | 10 | 47 / 11 / 10 |

**E still wins, by more.** Its lead over the runners-up goes from 4 clips to 5,
and it is the only voice above 60. The switch Eric signed stands on the
corrected numbers; nothing here reopens it.

What the correction does change is the *three remaining failures*, which are
now the whole of E's deficit: `for` (heard as "4" by both — a digit case the
fold does not rescue, because "four" is genuinely a different word from "for"),
`some` (heard as "sum"), and `hiss`. All three are homophone or near-homophone
collisions rather than synthesis faults, and none of the three is a word in the
English bank — they were probe words chosen for this list. See the note on the
native fallback below.

## The fallback is not a safety net for this class

I1 says a FAIL clip is never served and the resolver moves to its next source,
which on iOS is the platform voice. That is only an improvement if the platform
voice is better. Measured 2026-09-27, macOS Samantha (the AVSpeechSynthesis
family iOS draws from) through the same two recognizers:

| word | whisper | Google STT | vs en-US-Neural2-E |
|---|---|---|---|
| hiss | His. | kiss | both fail |
| half | Have | (nothing) | **E passes, native fails** |
| leaf | Leave. | leave | **E passes, native fails** |
| safe | safe. | save | **E passes, native splits** |
| some | Sum. | psalm | both fail |
| for | 4. | 4 | both fail |
| its | It's | (nothing) | E passes |
| fifth | Fifth. | 5th | both pass (after the fold) |
| thief | Thief | Thief | both pass |
| hip | Hit. | hip | E passes, native splits |

The platform voice is **worse on exactly the word-final voicing class** that
started this whole investigation: it loses `half`, `leaf` and `safe`, the three
words the bake-off was run to fix. So for this failure class, withholding the
Google clip and falling through would hand the player a less intelligible clip
than the one that was withheld.

This is not an argument against I1 — a clip neither machine can hear is still
not evidence of a clip a person can — but it means "FAIL" cannot be treated as
"solved by the fallback". A failing bank word needs a signed F3 row or a voice
that gets it right; the fallback is a last resort, not a fix. Whether any bank
word is actually in this state is what the full-bank F2 run answers.
