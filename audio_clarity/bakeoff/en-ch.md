# Bake-off — en

63 words per voice, both recognizers, blind (I3).

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| en-US-Neural2-C | 60 | 2 | 1 |
| en-US-Neural2-E **(current)** | 56 | 2 | 5 |
| en-US-Neural2-J | 56 | 2 | 5 |
| en-US-Neural2-F | 56 | 4 | 3 |
| en-US-Neural2-I | 56 | 5 | 2 |
| en-US-Neural2-H | 55 | 6 | 2 |
| en-US-Neural2-G | 53 | 5 | 5 |
| en-US-Neural2-D | 52 | 7 | 4 |
| en-US-Neural2-A | 50 | 7 | 6 |

## Missed by both recognizers

### en-US-Neural2-C

- chiaroscuro -> "Kiaroskiro" / "Kiara skurow"

### en-US-Neural2-E

- chiaroscuro -> "Kiara Skuro" / "Kiara skurow"
- chip -> "Ship" / "ship"
- choose -> "Shoes" / "shoes"
- chose -> "Shows" / "shows"
- search -> "Surge" / "surge"

### en-US-Neural2-J

- chain -> "Shane." / "shein"
- chiaroscuro -> "PR Esquiro" / "PR escuro"
- chip -> "Ship" / "ship"
- choose -> "Shoes" / "shoes"
- chose -> "Shows" / "shows"

### en-US-Neural2-F

- chiaroscuro -> "He are a skewer-o." / "here are a skurow"
- choose -> "Shoes" / "shoes"
- chose -> "Chos." / "shows"

### en-US-Neural2-I

- chiaroscuro -> "Kiara Skuro" / "Kiara skurow"
- chose -> "Chos." / "shows"

### en-US-Neural2-H

- chiaroscuro -> "Kiariskuro" / "key areas curro"
- chose -> "Shows" / "shows"

### en-US-Neural2-G

- changed -> "Change." / "change"
- charged -> "Charge" / "charge"
- chiaroscuro -> "Kiaroskuro" / "Kiara skurow"
- choose -> "Shoes" / "shoes"
- chose -> "Shows" / "shows"

### en-US-Neural2-D

- chiaroscuro -> "Kiara Skiro" / "Kiara skurow"
- rich -> "Ridge" / "Ridge"
- safe -> "Save." / "save"
- leaf -> "Leave." / "leave"

### en-US-Neural2-A

- chiaroscuro -> "He are a skewer oh" / "he are auro"
- chief -> "Sheave" / "she"
- chip -> "Sh*t!" / ""
- choose -> "Shoes" / "shoes"
- chose -> "Shows" / "shows"
- leaf -> "leave" / "Le"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### en-US-Neural2-C

- chose -> "chose" / "shows"
- inch -> "inch" / "in"

### en-US-Neural2-E

- chain -> "Shane." / "chain"
- chrysanthemum -> "Crisanthemum" / "chrysanthemum"

### en-US-Neural2-J

- ship -> "Shit." / "ship"
- half -> "half" / ""

### en-US-Neural2-F

- chip -> "CHEP" / "chip"
- inch -> "inch" / "in"
- cat -> "Cat" / "hat"
- ship -> "Shep" / "ship"

### en-US-Neural2-I

- chip -> "Chib" / "chip"
- rich -> "Ridge" / "rich"
- search -> "Surge." / "search"
- cat -> "CAD" / "cat"
- ship -> "Shed" / "ship"

### en-US-Neural2-H

- chip -> "Shep" / "chip"
- choose -> "Shoes" / "choose"
- church -> "Surge" / "Church"
- each -> "each" / ""
- inch -> "inch" / "in"
- search -> "Surge" / "search"

### en-US-Neural2-G

- chip -> "Shep" / "chip"
- inch -> "inch" / "in"
- rich -> "Ridge" / "rich"
- cat -> "CAD" / "cat"
- ship -> "Shep" / "ship"

### en-US-Neural2-D

- chip -> "CHEP" / "chip"
- chose -> "Chos" / "chose"
- inch -> "inch" / "in"
- search -> "Surge" / "search"
- which -> "Wedge." / "which"
- ship -> "Shep" / "ship"
- half -> "Have." / "half"

### en-US-Neural2-A

- chain -> "Chain." / "shein"
- each -> "each" / ""
- inch -> "inch" / "bench"
- much -> "Much." / ""
- cat -> "Cat" / "chat"
- ship -> "Shit!" / "ship"
- half -> "half" / ""


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
