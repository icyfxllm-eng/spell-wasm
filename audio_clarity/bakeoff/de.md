# Bake-off — de

60 words per voice, both recognizers, blind (I3).

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
