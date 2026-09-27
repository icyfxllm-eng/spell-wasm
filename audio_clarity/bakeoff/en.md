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
