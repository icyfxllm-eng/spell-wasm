# Bake-off — de

60 words per voice, both recognizers, blind (I3).

**11 rows below, at most 5 actual voices. Measured 2026-10-02, corrected the same day.**

Google answers a voice name it does not have by serving a different voice,
with a 200 and real audio. These candidate lists were built by keeping the
names that answered 200, so several names here are one voice wearing several
labels. The giveaway is in the table itself: aliased rows score identically to
the digit.

Confirmed by synthesizing one word per name straight against the API, under
the SSML the server actually sends, and comparing SHA-256. Same audio:

- `de-DE-Neural2-A` = `de-DE-Neural2-C` = `de-DE-Neural2-F`
- `de-DE-Wavenet-A` = `de-DE-Wavenet-F`
- `de-DE-Neural2-B` = `de-DE-Neural2-D`
- `de-DE-Wavenet-B` = `de-DE-Wavenet-D` = `de-DE-Wavenet-E`

The SHIPPED voice `de-DE-Neural2-A` is one of these labels — it is the same audio as `de-DE-Neural2-C`, `de-DE-Neural2-F`.

**What the groups above do and do not prove.** Google synthesis is NOT
deterministic: the same name, word and SSML returned three different byte
patterns over six calls, all of exactly the same length — one voice, varying
encoding. So byte-identical audio from two names proves they are the same
voice (two non-deterministic streams do not coincide by chance), but a
*non*-match proves nothing at all. The groups listed above are confirmed. The
names NOT grouped are **unconfirmed, not confirmed distinct**, and the true
voice count may be lower than 5.

The Pass numbers are still real measurements of real audio — each row scored a
fixed set of clips, frozen by the server's cache. But this table offers 11
choices where at most 5 exist, and the differences between rows inside one
group above are not differences at all.

`scripts/audio-bakeoff.mjs` now samples the first three clips of each
candidate rather than one, and says in its own header that unflagged rows are
unconfirmed. The single-clip version of that check gave a false negative the
day it was written.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| de-DE-Neural2-A | 38 | 15 | 7 |
| de-DE-Neural2-C | 38 | 15 | 7 |
| de-DE-Neural2-F | 38 | 15 | 7 |
| de-DE-Wavenet-A | 38 | 15 | 7 |
| de-DE-Wavenet-C | 38 | 15 | 7 |
| de-DE-Wavenet-F | 38 | 15 | 7 |
| de-DE-Neural2-B **(current)** | 33 | 17 | 10 |
| de-DE-Neural2-D | 33 | 17 | 10 |
| de-DE-Wavenet-B | 33 | 17 | 10 |
| de-DE-Wavenet-D | 33 | 18 | 9 |
| de-DE-Wavenet-E | 32 | 18 | 10 |

## Missed by both recognizers

### de-DE-Neural2-A

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quarzen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Neural2-C

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quarzen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Neural2-F

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quarzen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-A

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quatsen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-C

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quatsen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-F

- osten -> "Austin" / "Austin"
- tee -> "T" / ""
- endete -> "Ende" / "Ende"
- kurzen -> "Quatsen" / "kurz"
- trieb -> "Trip" / "Tri"
- ueber -> "Über" / "über"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Neural2-B

- osten -> "Austin." / "oh"
- tee -> "T" / ""
- endete -> "Ende de." / "Ende"
- ersten -> "1." / ""
- trieb -> "TRIP" / "Tri"
- begab -> "The Gap" / "bi"
- mitteilen -> "mit Teilen" / "mitteil"
- ueber -> "Über" / "über"
- frohes -> "For us." / "froh"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Neural2-D

- osten -> "Austin." / "oh"
- tee -> "T" / ""
- endete -> "Ende de." / "Ende"
- ersten -> "1." / ""
- trieb -> "TRIP" / "Tri"
- begab -> "The Gap" / "bi"
- mitteilen -> "mit Teilen" / "mitteil"
- ueber -> "Über" / "über"
- frohes -> "For us." / "froh"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-B

- osten -> "Austin." / "oh"
- tee -> "T" / ""
- endete -> "Ende de." / "Ende"
- ersten -> "1." / ""
- trieb -> "TRIP" / "Tri"
- begab -> "The Gap" / "bi"
- mitteilen -> "mit Teilen" / "mitteil"
- ueber -> "Über" / "über"
- frohes -> "For us." / "froh"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-D

- osten -> "Austin." / "oh"
- tee -> "T" / ""
- endete -> "Ende de." / "Ende"
- ersten -> "1." / ""
- trieb -> "TRIP" / "Tri"
- begab -> "The Gap" / "bi"
- ueber -> "Über" / "über"
- frohes -> "For us." / "froh"
- diplomatie -> "Diplomati" / "diplomat"

### de-DE-Wavenet-E

- osten -> "Austin." / "oh"
- tee -> "T" / ""
- endete -> "Ende de." / "Ende"
- ersten -> "1." / ""
- trieb -> "TRIP" / "Tri"
- begab -> "The Gap" / "bi"
- mitteilen -> "mit Teilen" / "mitteil"
- ueber -> "Über" / "über"
- frohes -> "For us." / "froh"
- diplomatie -> "Diplomati" / "diplomat"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### de-DE-Neural2-A

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "zackte" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Neural2-C

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "zackte" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Neural2-F

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "zackte" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Wavenet-A

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "zackte" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Wavenet-C

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "ZAKTE" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Wavenet-F

- lang -> "lang" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "Sehen!" / ""
- besten -> "besten" / "Westen"
- fanden -> "Fandnen" / "fanden"
- sagte -> "ZAKTE" / "sagte"
- einem -> "einem" / ""
- begab -> "Wicke Up!" / "begab"
- diebstahl -> "Diebstahl" / "diepst"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehnte lang" / "jahrzehntelang"
- polnischen -> "\"Ponition\"" / "polnischen"
- wacker -> "Wacker" / "wack"
- frohes -> "Frohes!" / "fro"

### de-DE-Neural2-B

- boden -> "Borden" / "Boden"
- metall -> "Metall" / "met"
- lang -> "Lang" / ""
- Nacht -> "Nacht" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "sehen" / ""
- besten -> "besten" / "WE"
- kurzen -> "kurzen" / ""
- lagen -> "Lagen" / "L"
- einem -> "Einem" / ""
- diebstahl -> "Diebstahl" / "diepst"
- direkten -> "Direkten" / "direk"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehntelang" / ""
- financial -> "Financial" / "finan"
- wacker -> "Wacker" / "wack"

### de-DE-Neural2-D

- boden -> "Borden" / "Boden"
- metall -> "Metall" / "met"
- lang -> "Lang" / ""
- Nacht -> "Nacht" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "sehen" / ""
- besten -> "besten" / "WE"
- kurzen -> "kurzen" / ""
- lagen -> "Lagen" / "L"
- einem -> "Einem" / ""
- diebstahl -> "Diebstahl" / "diepst"
- direkten -> "Direkten" / "direk"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehntelang" / ""
- financial -> "Financial" / "finan"
- wacker -> "Wacker" / "wack"

### de-DE-Wavenet-B

- boden -> "Borden" / "Boden"
- metall -> "Metall" / "met"
- lang -> "Lang" / ""
- Nacht -> "Nacht" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "sehen" / ""
- kurzen -> "kurzen" / ""
- lagen -> "Lagen" / "L"
- einem -> "Einem" / ""
- diebstahl -> "Diebstahl" / "diepst"
- direkten -> "Direkten" / "direk"
- einfallen -> "Einfallen" / "Einfall"
- rechtswidrig -> "Rechtswidrig." / ""
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehntelang" / ""
- financial -> "Financial" / "finan"
- wacker -> "Wacker" / "wack"

### de-DE-Wavenet-D

- boden -> "Borden" / "Boden"
- metall -> "Metall" / "met"
- lang -> "Lang" / ""
- Nacht -> "Nacht" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "sehen" / ""
- besten -> "besten" / "WE"
- kurzen -> "kurzen" / ""
- lagen -> "Lagen" / "L"
- einem -> "Einem" / ""
- mitteilen -> "mit Teilen" / "mitteilen"
- diebstahl -> "Diebstahl" / "diepst"
- direkten -> "Direkten" / "direk"
- einfallen -> "Einfallen" / "Einfall"
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehntelang" / ""
- financial -> "Financial" / "finan"
- wacker -> "Wacker" / "wack"

### de-DE-Wavenet-E

- boden -> "Borden" / "Boden"
- metall -> "Metall" / "met"
- lang -> "Lang" / ""
- Nacht -> "Nacht" / ""
- Blatt -> "Blatt" / "Bl"
- sehen -> "sehen" / ""
- besten -> "besten" / "WE"
- kurzen -> "kurzen" / ""
- lagen -> "Lagen" / "L"
- einem -> "Einem" / ""
- diebstahl -> "Diebstahl" / "diepst"
- direkten -> "Direkten" / "direk"
- einfallen -> "Einfallen" / "Einfall"
- rechtswidrig -> "Rechtswidrig." / ""
- wiege -> "Wiege" / "wie"
- jahrzehntelang -> "Jahrzehntelang" / ""
- financial -> "Financial" / "finan"
- wacker -> "Wacker" / "wack"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
