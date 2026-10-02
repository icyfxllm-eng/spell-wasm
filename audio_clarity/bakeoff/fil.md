# Bake-off — fil

60 words per voice, both recognizers, blind (I3).

**Re-run 2026-10-01, and the earlier version of this file was void.** Every
voice scored 0 Pass there, which read as "whisper cannot do Tagalog". It could
not do `-l fil`: whisper's code for Tagalog is `tl`, the CLI rejected the
argument outright and wrote nothing, and the harness scored that silence as a
miss. Fixed in `scripts/lib/audio-norm.mjs`.

**`fil-ph-Neural2-A` is not a candidate and cannot be one.** Under the SSML the
server actually sends — padding breaks, 0.85 rate, headphone profile — Google
returns audio **byte-identical** to `fil-PH-Wavenet-A` for it. Verified
directly against the API, bypassing the server; with plain text the two differ.
Since every real request carries that SSML, choosing this voice would silently
ship Wavenet-A. `fil-ph-Neural2-D` has no such problem and is scored below.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| fil-PH-Wavenet-A **(current)** | 41 | 14 | 5 |
| fil-PH-Wavenet-B | 40 | 14 | 6 |
| fil-ph-Neural2-D | 36 | 14 | 10 |
| fil-PH-Wavenet-D | 36 | 15 | 9 |
| fil-PH-Wavenet-C | 36 | 18 | 6 |

## Missed by both recognizers

### fil-PH-Wavenet-A

- yelo -> "Yellow" / "yellow"
- berde -> "Verde" / "Verde"
- alon -> "Halun!" / "Allen"
- alkalde -> "Alkao D." / "Alcalde"
- impeksiyon -> "Impeksyon" / "impeksyon"

### fil-PH-Wavenet-B

- kamera -> "Camera" / "camera"
- buwan -> "1." / "Buan"
- berde -> "Bear Day" / "Verde"
- alon -> "Alun." / "Allen"
- amang -> "Among" / "tamang"
- impeksiyon -> "Impeksyon" / "impeksyon"

### fil-ph-Neural2-D

- kama -> "Karma" / "tama"
- yelo -> "Yellow" / "yellow"
- kamera -> "Camera" / "camera"
- klase -> "Classic." / "classic"
- berde -> "Verde." / "Verde"
- alon -> "Aalun." / "Allen"
- amang -> "Amam." / "ama"
- estadong -> "Estadom" / "estado"
- hukbong -> "Hook Boom" / "hukbo"
- impeksiyon -> "Impeksyon" / "impeksyon"

### fil-PH-Wavenet-D

- kama -> "Karma" / "tama"
- yelo -> "Yellow" / "yellow"
- kamera -> "Camera" / "camera"
- klase -> "Classy!" / "classic"
- berde -> "Verde." / "Verde"
- amang -> "Amam." / "ama"
- estadong -> "Estadom" / "estado"
- hukbong -> "Hook Boom" / "boom boom"
- impeksiyon -> "Impeksyon" / "impeksyon"

### fil-PH-Wavenet-C

- kamera -> "Camera" / "camera"
- berde -> "Verde" / "Verde"
- alon -> "Alun." / "Allen"
- estadong -> "Estadom" / "estado"
- impeksiyon -> "Impeksyon" / "impeksyon"
- tradisyunal -> "Tradisyonal" / "traditional"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### fil-PH-Wavenet-A

- bituin -> "Bitoin!" / "bituin"
- kama -> "Kama." / "tama"
- kamera -> "Kamera" / "camera"
- klase -> "Class A" / "klase"
- sibil -> "Sibyl" / "sibil"
- signal -> "Signal" / "Cignal"
- etniko -> "Ethnico" / "etniko"
- tungo -> "Tungo!" / "tubo"
- hukbong -> "Hook Bung" / "hukbong"
- gerilya -> "Merilya" / "gerilya"
- mamatay -> "Mematay." / "mamatay"
- ipasa -> "Ipasah." / "ipasa"
- okasyon -> "O kasyon!" / "okasyon"
- tradisyunal -> "Tradisyonal" / "tradisyunal"

### fil-PH-Wavenet-B

- bituin -> "Bitoin!" / "bituin"
- kama -> "Kama" / "tama"
- yelo -> "Yellow." / "yelo"
- klase -> "Class A" / "klase"
- sibil -> "Sibyl" / "sibil"
- signal -> "Signal" / "Cignal"
- etniko -> "Ethnico" / "etniko"
- estadong -> "Estado." / "estadong"
- hukbong -> "Hukbong" / "f*** mom"
- gerilya -> "Guerilla" / "gerilya"
- ipasa -> "Ipasahe." / "ipasa"
- pumatay -> "Pumatae." / "pumatay"
- teknolohiya -> "Teknologiya" / "teknolohiya"
- tradisyunal -> "Tradisyonal" / "tradisyunal"

### fil-ph-Neural2-D

- bituin -> "B2IN" / "bituin"
- ginto -> "Gintok" / "ginto"
- ibenta -> "I-venta." / "ibenta"
- sibil -> "Sibil" / "cbo"
- karera -> "Carrera" / "karera"
- signal -> "Signal" / "Cignal"
- tanong -> "Thanum" / "tanong"
- etniko -> "Ethnico" / "etniko"
- tungo -> "Tungo" / "sumo"
- gerilya -> "Guerilla" / "gerilya"
- ipasa -> "Ipasah." / "ipasa"
- alkalde -> "Alcalde" / "alkalde"
- mapabuti -> "Mapa Buti" / "mapabuti"
- tradisyunal -> "Tradisyonal" / "tradisyunal"

### fil-PH-Wavenet-D

- bituin -> "B2IN" / "bituin"
- ginto -> "Gintok" / "ginto"
- ibenta -> "I-venta." / "ibenta"
- alon -> "A-Lon" / "Allen"
- sibil -> "Sibil" / "cbo"
- karera -> "Carrera" / "karera"
- signal -> "Signal" / "Cignal"
- tanong -> "Thanum" / "tanong"
- etniko -> "Ethnico" / "etniko"
- tungo -> "Tungo" / "sumo"
- gerilya -> "Guerilla" / "gerilya"
- ipasa -> "Ipasah." / "ipasa"
- alkalde -> "Alcalde" / "alkalde"
- mapabuti -> "Mapa Buti" / "mapabuti"
- tradisyunal -> "Tradisyonal" / "tradisyunal"

### fil-PH-Wavenet-C

- bituin -> "Bitoim" / "bituin"
- yelo -> "Yellow" / "yelo"
- ginto -> "Gintok." / "ginto"
- klase -> "LASE" / "klase"
- sibil -> "Sibil" / "Cebu"
- signal -> "Signal" / "Cignal"
- taglay -> "Taglay!" / "tagline"
- amang -> "Amang!" / "tamang"
- etniko -> "Ethnico" / "etniko"
- hukbong -> "Hookbong" / "hukbong"
- katulad -> "Katulan." / "katulad"
- mamatay -> "MAMATAE" / "mamatay"
- ipasa -> "Ipasahe." / "ipasa"
- pumatay -> "Pumatae." / "pumatay"
- average -> "Average" / "average age"
- alkalde -> "Alkalde" / "Alcalde"
- batayan -> "Batayan!" / "Bataan"
- kongresista -> "Kongresista" / "kung krisis"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
