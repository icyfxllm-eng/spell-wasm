# Bake-off — fr

60 words per voice, both recognizers, blind (I3).

**12 rows below, at most 4 actual voices. Measured 2026-10-02, corrected the same day.**

Google answers a voice name it does not have by serving a different voice,
with a 200 and real audio. These candidate lists were built by keeping the
names that answered 200, so several names here are one voice wearing several
labels. The giveaway is in the table itself: aliased rows score identically to
the digit.

Confirmed by synthesizing one word per name straight against the API, under
the SSML the server actually sends, and comparing SHA-256. Same audio:

- `fr-FR-Wavenet-F` = `fr-FR-Wavenet-E`
- `fr-FR-Wavenet-A` = `fr-FR-Wavenet-C` = `fr-FR-Neural2-A` = `fr-FR-Neural2-C` = `fr-FR-Neural2-E` = `fr-FR-Neural2-F`
- `fr-FR-Neural2-B` = `fr-FR-Neural2-D`
- `fr-FR-Wavenet-B` = `fr-FR-Wavenet-D`

The SHIPPED voice `fr-FR-Neural2-A` is one of these labels — it is the same audio as `fr-FR-Wavenet-A`, `fr-FR-Wavenet-C`, `fr-FR-Neural2-C`, `fr-FR-Neural2-E`, `fr-FR-Neural2-F`.

**What the groups above do and do not prove.** Google synthesis is NOT
deterministic: the same name, word and SSML returned three different byte
patterns over six calls, all of exactly the same length — one voice, varying
encoding. So byte-identical audio from two names proves they are the same
voice (two non-deterministic streams do not coincide by chance), but a
*non*-match proves nothing at all. The groups listed above are confirmed. The
names NOT grouped are **unconfirmed, not confirmed distinct**, and the true
voice count may be lower than 4.

The Pass numbers are still real measurements of real audio — each row scored a
fixed set of clips, frozen by the server's cache. But this table offers 12
choices where at most 4 exist, and the differences between rows inside one
group above are not differences at all.

`scripts/audio-bakeoff.mjs` now samples the first three clips of each
candidate rather than one, and says in its own header that unflagged rows are
unconfirmed. The single-clip version of that check gave a false negative the
day it was written.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| fr-FR-Wavenet-F | 29 | 9 | 22 |
| fr-FR-Wavenet-A | 29 | 10 | 21 |
| fr-FR-Wavenet-C | 29 | 10 | 21 |
| fr-FR-Neural2-A **(current)** | 28 | 11 | 21 |
| fr-FR-Neural2-C | 28 | 11 | 21 |
| fr-FR-Neural2-E | 28 | 11 | 21 |
| fr-FR-Neural2-F | 28 | 11 | 21 |
| fr-FR-Wavenet-E | 28 | 11 | 21 |
| fr-FR-Neural2-B | 28 | 12 | 20 |
| fr-FR-Neural2-D | 28 | 12 | 20 |
| fr-FR-Wavenet-B | 28 | 12 | 20 |
| fr-FR-Wavenet-D | 28 | 12 | 20 |

## Missed by both recognizers

### fr-FR-Wavenet-F

- deux -> "de" / "2"
- faire -> "Fair." / ""
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens !" / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "Arrivé !" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Wavenet-A

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "Jol"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens !" / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "Arrivé !" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Wavenet-C

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens !" / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "arriver" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Neural2-A

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "Jol"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens." / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "arriver" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Neural2-C

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "Jol"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens." / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "arriver" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Neural2-E

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "Jol"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens." / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "arriver" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Neural2-F

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "Jol"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens." / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "arriver" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Wavenet-E

- deux -> "de" / "2"
- terre -> "There." / ""
- sept -> "SET" / "7"
- homme -> "Am." / ""
- lancé -> "Lancer" / "lancer"
- fruits -> "Fruit." / "fruit"
- noter -> "Notez." / "notez"
- formé -> "Formez." / "former"
- types -> "Tip" / "type"
- basses -> "BAS !" / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Trétant." / "très"
- autochtones -> "\"Otokton\"" / "autochtone"
- attaque -> "Attack !" / "ATA"
- devient -> "Deviens." / "Dave"
- faciles -> "facile" / "facile"
- fiers -> "Fier." / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "résidence" / "résidence"
- arrivaient -> "Arrivé !" / "arriv"
- gore -> "Daaah" / ""

### fr-FR-Neural2-B

- deux -> "2" / ""
- terre -> "Zaire" / ""
- sept -> "Set." / ""
- homme -> "Bonne." / ""
- lancé -> "Lancer." / "lancer"
- fruits -> "FUIT !" / "fruit"
- noter -> "Notez." / "not"
- formé -> "Formez." / "form"
- types -> "Tip" / ""
- basses -> "Thus." / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Très temps." / "très"
- autochtones -> "au toc ton" / "autochtone"
- faciles -> "Facile." / "facile"
- fiers -> "Fier !" / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "Résidence" / "résidence"
- odyssée -> "Odyssey" / "Odys"
- arrivaient -> "Arrivez." / "arriv"
- snow -> "Snoo" / ""

### fr-FR-Neural2-D

- deux -> "2" / ""
- terre -> "Zaire" / ""
- sept -> "Set." / ""
- homme -> "Bonne." / ""
- lancé -> "Lancer." / "lancer"
- fruits -> "FUIT !" / "fruit"
- noter -> "Notez." / "not"
- formé -> "Formez." / "form"
- types -> "Tip" / ""
- basses -> "Thus." / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Très temps." / "très"
- autochtones -> "au toc ton" / "autochtone"
- faciles -> "Facile." / "facile"
- fiers -> "Fier !" / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "Résidence" / "résidence"
- odyssée -> "Odyssey" / "Odys"
- arrivaient -> "Arrivez." / "arriv"
- snow -> "Snoo" / ""

### fr-FR-Wavenet-B

- deux -> "2" / ""
- terre -> "Zaire" / ""
- sept -> "Set." / ""
- homme -> "Bonne." / ""
- lancé -> "Lancer." / "lancer"
- fruits -> "FUIT !" / "fruit"
- noter -> "Notez." / "not"
- formé -> "Formez." / "form"
- types -> "Tip" / ""
- basses -> "Thus." / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Très temps." / "très"
- autochtones -> "au toc ton" / "autochtone"
- faciles -> "Facile." / "facile"
- fiers -> "Fier !" / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "Résidence" / "résidence"
- odyssée -> "Odyssey" / "Odys"
- arrivaient -> "Arrivez." / "arriv"
- snow -> "Snoo" / ""

### fr-FR-Wavenet-D

- deux -> "2" / ""
- terre -> "Zaire" / ""
- sept -> "Set." / ""
- homme -> "Bonne." / ""
- lancé -> "Lancer." / "lancer"
- fruits -> "FUIT !" / "fruit"
- noter -> "Notez." / "not"
- formé -> "Formez." / "form"
- types -> "Tip" / ""
- basses -> "Thus." / "bas"
- jolies -> "Joli !" / "jolie"
- traitant -> "Très temps." / "très"
- autochtones -> "au toc ton" / "autochtone"
- faciles -> "Facile." / "facile"
- fiers -> "Fier !" / "fier"
- cultivées -> "Cultiver" / "cultiver"
- résidences -> "Résidence" / "résidence"
- odyssée -> "Odyssey" / "Odys"
- arrivaient -> "Arrivez." / "arriv"
- snow -> "Snoo" / ""


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### fr-FR-Wavenet-F

- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Wavenet-A

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Wavenet-C

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Neural2-A

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- odyssée -> "odyssey" / "odyssée"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Neural2-C

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- odyssée -> "odyssey" / "odyssée"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Neural2-E

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- odyssée -> "odyssey" / "odyssée"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Neural2-F

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- odyssée -> "odyssey" / "odyssée"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Wavenet-E

- faire -> "faire" / ""
- bon -> "Boom !" / "bon"
- neuf -> "Neuf." / "9"
- oiseau -> "oiseaux" / "oiseau"
- classe -> "+" / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess" / "baisse"
- succès -> "succès" / "suc"
- odyssée -> "odyssey" / "odyssée"
- bologne -> "Boulogne" / "Bologne"
- snow -> "Snow" / ""

### fr-FR-Neural2-B

- faire -> "faire" / ""
- neuf -> "Neuf." / "9"
- oiseau -> "Oiseaux" / "oiseau"
- classe -> "Plus." / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess !" / "baisse"
- offre -> "Offre." / "off"
- attaque -> "Attac !" / "attaque"
- ordonne -> "Pardon." / "ordonne"
- devient -> "devient." / "Dave"
- bologne -> "Bologni" / "Bologne"
- embouchure -> "en bouchure" / "embouchure"

### fr-FR-Neural2-D

- faire -> "faire" / ""
- neuf -> "Neuf." / "9"
- oiseau -> "Oiseaux" / "oiseau"
- classe -> "Plus." / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess !" / "baisse"
- offre -> "Offre." / "off"
- attaque -> "Attac !" / "attaque"
- ordonne -> "Pardon." / "ordonne"
- devient -> "devient." / "Dave"
- bologne -> "Bologni" / "Bologne"
- embouchure -> "en bouchure" / "embouchure"

### fr-FR-Wavenet-B

- faire -> "faire" / ""
- neuf -> "Neuf." / "9"
- oiseau -> "Oiseaux" / "oiseau"
- classe -> "Plus." / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess !" / "baisse"
- offre -> "Offre." / "off"
- attaque -> "Attac !" / "attaque"
- ordonne -> "Pardon." / "ordonne"
- devient -> "devient." / "Dave"
- bologne -> "Bologni" / "Bologne"
- embouchure -> "en bouchure" / "embouchure"

### fr-FR-Wavenet-D

- faire -> "faire" / ""
- neuf -> "Neuf." / "9"
- oiseau -> "Oiseaux" / "oiseau"
- classe -> "Plus." / "classe"
- secret -> "Secrets" / "secret"
- baisse -> "Bess !" / "baisse"
- offre -> "Offre." / "off"
- attaque -> "Attac !" / "attaque"
- ordonne -> "Pardon." / "ordonne"
- devient -> "devient." / "Dave"
- bologne -> "Bologni" / "Bologne"
- embouchure -> "en bouchure" / "embouchure"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
