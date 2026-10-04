# Bake-off — en

106 words per voice, both recognizers, blind (I3).

2 voice(s) scored from 2 candidate name(s).

Rows that were NOT flagged as aliases are **unconfirmed, not confirmed
distinct**. Google synthesis is non-deterministic, so two samples of one
voice can differ; a match proves sameness, a non-match proves nothing.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| en-US-Neural2-C | 99 | 5 | 2 |
| en-US-Neural2-E **(current)** | 94 | 6 | 6 |

## Missed by both recognizers

### en-US-Neural2-C

- chiaroscuro -> "Kiaroskiro" / "Kiara skurow"
- inchoate -> "In Co-It" / "in Co"

### en-US-Neural2-E

- chiaroscuro -> "Kiara Skuro" / "Kiara skurow"
- chip -> "Ship" / "ship"
- choose -> "Shoes" / "shoes"
- chose -> "Shows" / "shows"
- inchoate -> "In Poet" / "in poet"
- search -> "Surge" / "surge"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### en-US-Neural2-C

- anachronism -> "Anachronism" / "anacronismo"
- chose -> "chose" / "shows"
- inch -> "inch" / "in"
- machiavellian -> "Machiavellian" / "Makaveli"
- schadenfreude -> "Schadenfreude" / "chadam freuda"

### en-US-Neural2-E

- anachronism -> "Anachronism" / "an acronym"
- chain -> "Shane." / "chain"
- chrysanthemum -> "Crisanthemum" / "chrysanthemum"
- machiavellian -> "Machiavellian" / "Makaveli"
- mischievous -> "Ms. Chavez" / "mischievous"
- schadenfreude -> "Schadenfreude" / "shot in Florida"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
