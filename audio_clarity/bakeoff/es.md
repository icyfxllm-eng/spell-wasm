# Bake-off — es

60 words per voice, both recognizers, blind (I3).

**11 rows below, 8 actual voices. Measured 2026-10-02.**

Google answers a voice name it does not have by serving a different voice,
with a 200 and real audio. These candidate lists were built by keeping the
names that answered 200, so several names here are one voice wearing several
labels. The giveaway is in the table itself: aliased rows score identically to
the digit.

Confirmed by synthesizing one word per name straight against the API, under
the SSML the server actually sends, and comparing SHA-256. Same audio:

- `es-ES-Neural2-C` = `es-ES-Neural2-D` = `es-ES-Wavenet-D`
- `es-ES-Wavenet-B` = `es-ES-Wavenet-E`

The Pass numbers are still real measurements of real audio — but this table
offers 11 choices where 8 exist, and the differences between rows in one
group above are not differences at all. `scripts/audio-bakeoff.mjs` now
detects this during the run and reports an alias instead of scoring it twice.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| es-ES-Neural2-C | 38 | 14 | 8 |
| es-ES-Neural2-D | 38 | 14 | 8 |
| es-ES-Wavenet-C | 37 | 14 | 9 |
| es-ES-Wavenet-D | 37 | 14 | 9 |
| es-ES-Wavenet-F | 37 | 14 | 9 |
| es-ES-Neural2-A | 36 | 17 | 7 |
| es-ES-Neural2-F | 36 | 17 | 7 |
| es-ES-Neural2-B **(current)** | 34 | 21 | 5 |
| es-ES-Wavenet-B | 34 | 21 | 5 |
| es-ES-Wavenet-E | 34 | 21 | 5 |
| es-ES-Neural2-E | 29 | 20 | 11 |

## Missed by both recognizers

### es-ES-Neural2-C

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "de" / ""
- hoy -> "oi" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""
- valle -> "¡Ballé!" / "va"

### es-ES-Neural2-D

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "de" / ""
- hoy -> "oi" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""
- valle -> "¡Ballé!" / "va"

### es-ES-Wavenet-C

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "de" / ""
- hoy -> "oi" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""
- toque -> "\"Toke\"" / ""
- valle -> "¡Ballé!" / "va"

### es-ES-Wavenet-D

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "de" / ""
- hoy -> "oi" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""
- toque -> "\"Toke\"" / ""
- valle -> "¡Ballé!" / "va"

### es-ES-Wavenet-F

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "de" / ""
- hoy -> "oi" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""
- toque -> "\"Toke\"" / ""
- valle -> "¡Ballé!" / "va"

### es-ES-Neural2-A

- dos -> "2" / ""
- ola -> "hola" / "hola"
- té -> "de" / ""
- hoy -> "¡Oy!" / ""
- seis -> "¡Says!" / ""
- toque -> "OK." / ""
- valle -> "¡Vaya!" / ""

### es-ES-Neural2-F

- dos -> "2." / ""
- ola -> "Hola." / "hola"
- té -> "De" / ""
- hoy -> "¡Hoy!" / ""
- toque -> "¿Dónde?" / ""
- bu -> "¡Boom!" / ""
- cito -> "CIDO" / ""

### es-ES-Neural2-B

- ola -> "Hola." / "hola"
- té -> "\"T\"" / ""
- hoy -> "oi" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""

### es-ES-Wavenet-B

- ola -> "Hola." / "hola"
- té -> "\"T\"" / ""
- hoy -> "oi" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""

### es-ES-Wavenet-E

- ola -> "Hola." / "hola"
- té -> "\"T\"" / ""
- hoy -> "oi" / ""
- pared -> "¡Paret!" / ""
- cálido -> "Calido." / ""

### es-ES-Neural2-E

- dos -> "2." / ""
- ola -> "Hola" / "hola"
- té -> "de" / ""
- raíz -> "¡Gracias!" / ""
- zorro -> "Zordra." / ""
- hoy -> "¡Ay!" / ""
- seis -> "¡Seis!" / ""
- pared -> "¡Para!" / ""
- cálido -> "¡Calido!" / ""
- toque -> "¿Tóque?" / ""
- valle -> "¡Vaya!" / ""


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### es-ES-Neural2-C

- raíz -> "raíz" / ""
- brazo -> "¡Brazo!" / "brazo"
- toque -> "toque" / ""
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- tener -> "tener" / ""
- miles -> "miles" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "ta"
- azufre -> "a sufre" / "azufre"
- bu -> "\"bu\"" / ""
- cito -> "Cito." / ""

### es-ES-Neural2-D

- raíz -> "raíz" / ""
- brazo -> "¡Brazo!" / "brazo"
- toque -> "toque" / ""
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- tener -> "tener" / ""
- miles -> "miles" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "ta"
- azufre -> "a sufre" / "azufre"
- bu -> "\"bu\"" / ""
- cito -> "Cito." / ""

### es-ES-Wavenet-C

- raíz -> "raíz" / ""
- brazo -> "¡Brazo!" / "brazo"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- tener -> "tener" / ""
- miles -> "miles" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- nuestra -> "nuestra." / ""
- taylor -> "Taylor" / "ta"
- azufre -> "a sufre" / "azufre"
- bu -> "\"bu\"" / ""
- cito -> "Cito." / ""

### es-ES-Wavenet-D

- raíz -> "raíz" / ""
- brazo -> "¡Brazo!" / "brazo"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- tener -> "tener" / ""
- miles -> "miles" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- nuestra -> "nuestra." / ""
- taylor -> "Taylor" / "ta"
- azufre -> "a sufre" / "azufre"
- bu -> "\"bu\"" / ""
- cito -> "Cito." / ""

### es-ES-Wavenet-F

- raíz -> "raíz" / ""
- brazo -> "¡Brazo!" / "brazo"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- tener -> "tener" / ""
- miles -> "miles" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- nuestra -> "nuestra." / ""
- taylor -> "Taylor" / "ta"
- azufre -> "a sufre" / "azufre"
- bu -> "\"bu\"" / ""
- cito -> "Cito." / ""

### es-ES-Neural2-A

- pared -> "pared" / "pare"
- cálido -> "¡Cálido!" / "cálido"
- brazo -> "¡Brazo!" / "brazo"
- copias -> "¡Copies!" / "copias"
- corte -> "corte" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "Taylo"
- complejidad -> "complejidad" / ""
- igualar -> "igualar" / ""
- económico -> "Económico" / ""
- azufre -> "¡Azufre!" / "azufre"
- convenga -> "¡Convenga!" / "convenga"
- bu -> "Bu" / ""
- doblaje -> "dobla G" / "doblaje"
- cito -> "CITO" / ""

### es-ES-Neural2-F

- seis -> "Seis." / ""
- pared -> "pared" / ""
- cálido -> "Calido." / "cálido"
- brazo -> "¡Blazo!" / "brazo"
- pierde -> "pierde" / "Pier"
- calor -> "calor" / ""
- dueño -> "¡Dueño!" / "dueño"
- elegir -> "Elegín." / "elegir"
- pido -> "Pido." / ""
- valle -> "Valle." / ""
- quedo -> "Quedo." / ""
- nuestra -> "Nuestra." / ""
- julián -> "Julián." / "Juli"
- taylor -> "Taylor" / "Tay"
- europeos -> "Eurobéos." / "europeos"
- igualar -> "Igualán." / "igualar"
- azufre -> "Azufli" / "azufre"

### es-ES-Neural2-B

- dos -> "Dos." / ""
- raíz -> "raíz" / ""
- zorro -> "Zorro." / ""
- seis -> "Seis." / "6"
- brazo -> "¡Brazo!" / "brazo"
- toque -> "toque" / "to"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- corte -> "corte" / ""
- elegir -> "elefir" / "elegir"
- pido -> "Pido." / ""
- valle -> "Valle." / "va"
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "Tay"
- complejidad -> "Complegidad." / "complejidad"
- bélgica -> "BELFICA" / "Bélgica"
- sufra -> "¡Sufra!" / "sufra"
- azufre -> "¡Azufre!" / "azufre"
- convenga -> "Convenda." / "convenga"
- bu -> "Bu" / ""
- cito -> "Cito." / ""

### es-ES-Wavenet-B

- dos -> "Dos." / ""
- raíz -> "raíz" / ""
- zorro -> "Zorro." / ""
- seis -> "Seis." / "6"
- brazo -> "¡Brazo!" / "brazo"
- toque -> "toque" / "to"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- corte -> "corte" / ""
- elegir -> "elefir" / "elegir"
- pido -> "Pido." / ""
- valle -> "Valle." / "va"
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "Tay"
- complejidad -> "Complegidad." / "complejidad"
- bélgica -> "BELFICA" / "Bélgica"
- sufra -> "¡Sufra!" / "sufra"
- azufre -> "¡Azufre!" / "azufre"
- convenga -> "Convenda." / "convenga"
- bu -> "Bu" / ""
- cito -> "Cito." / ""

### es-ES-Wavenet-E

- dos -> "Dos." / ""
- raíz -> "raíz" / ""
- zorro -> "Zorro." / ""
- seis -> "Seis." / "6"
- brazo -> "¡Blazo!" / "brazo"
- toque -> "toque" / "to"
- copias -> "¡Copias!" / "copias"
- pierde -> "pierde" / ""
- corte -> "corte" / ""
- elegir -> "elefir" / "elegir"
- pido -> "Pido." / ""
- valle -> "Valle." / "va"
- quedo -> "Quedo." / ""
- taylor -> "Taylor" / "Tay"
- complejidad -> "Complegidad." / "complejidad"
- bélgica -> "BELFICA" / "Bélgica"
- sufra -> "¡Sufra!" / "sufra"
- azufre -> "¡Azufre!" / "azufre"
- convenga -> "Convenda." / "convenga"
- bu -> "Bu" / ""
- cito -> "Cito." / ""

### es-ES-Neural2-E

- caminar -> "caminar" / "camina"
- brazo -> "¡Brazo!" / "brazo"
- verde -> "verde" / ""
- pierde -> "pierde" / "Pier"
- tener -> "tener" / ""
- corte -> "Corte" / ""
- dueño -> "dueño" / ""
- pido -> "Pido." / ""
- quedo -> "Quedo." / ""
- nuestra -> "nuestra" / ""
- julián -> "Julián" / "Juli"
- taylor -> "Taylor" / "ta"
- cumplir -> "cumplir" / ""
- sufra -> "Sufra." / "su"
- igualar -> "igualar" / ""
- azufre -> "¡Azufre!" / "azufre"
- convenga -> "con venga" / "convenga"
- bu -> "Bu" / ""
- pescador -> "Pescador" / ""
- cito -> "CITO" / ""


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
