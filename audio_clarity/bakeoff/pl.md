# Bake-off — pl

60 words per voice, both recognizers, blind (I3).

**6 rows below, at most 4 actual voices. Measured 2026-10-02, corrected the same day.**

Google answers a voice name it does not have by serving a different voice,
with a 200 and real audio. These candidate lists were built by keeping the
names that answered 200, so several names here are one voice wearing several
labels. The giveaway is in the table itself: aliased rows score identically to
the digit.

Confirmed by synthesizing one word per name straight against the API, under
the SSML the server actually sends, and comparing SHA-256. Same audio:

- `pl-PL-Wavenet-D` = `pl-PL-Wavenet-E` = `pl-PL-Wavenet-F`

**What the groups above do and do not prove.** Google synthesis is NOT
deterministic: the same name, word and SSML returned three different byte
patterns over six calls, all of exactly the same length — one voice, varying
encoding. So byte-identical audio from two names proves they are the same
voice (two non-deterministic streams do not coincide by chance), but a
*non*-match proves nothing at all. The groups listed above are confirmed. The
names NOT grouped are **unconfirmed, not confirmed distinct**, and the true
voice count may be lower than 4.

The Pass numbers are still real measurements of real audio — each row scored a
fixed set of clips, frozen by the server's cache. But this table offers 6
choices where at most 4 exist, and the differences between rows inside one
group above are not differences at all.

`scripts/audio-bakeoff.mjs` now samples the first three clips of each
candidate rather than one, and says in its own header that unflagged rows are
unconfirmed. The single-clip version of that check gave a false negative the
day it was written.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| pl-PL-Wavenet-B **(current)** | 24 | 30 | 6 |
| pl-PL-Wavenet-C | 24 | 30 | 6 |
| pl-PL-Wavenet-D | 6 | 46 | 8 |
| pl-PL-Wavenet-E | 6 | 46 | 8 |
| pl-PL-Wavenet-A | 5 | 47 | 8 |
| pl-PL-Wavenet-F | 5 | 47 | 8 |

## Missed by both recognizers

### pl-PL-Wavenet-B

- wieś -> "Wies." / "wiesz"
- noc -> "Nodz." / "no"
- rok -> "Rock." / ""
- fantasy -> "Fantazy" / "Fanta"
- oddział -> "Oddażę." / ""
- strat -> "Strad" / ""

### pl-PL-Wavenet-C

- wieś -> "Wies." / "wiesz"
- noc -> "Nodz." / "no"
- rok -> "Rock." / ""
- fantasy -> "Fantazy" / "Fanta"
- oddział -> "Oddażę." / ""
- strat -> "Strad" / ""

### pl-PL-Wavenet-D

- wieś -> "Wjeźdź." / "wiesz"
- środek -> "Środyk" / "w środę"
- rok -> "Drog." / ""
- kilku -> "Kielko" / "kill"
- fantasy -> "Fantazy" / "Fanta"
- strat -> "Strad" / ""
- apetyt -> "Apetit!" / "apet"
- zamieszczone -> "Zamieszczony." / ""

### pl-PL-Wavenet-E

- wieś -> "Wjeźdź." / "wiesz"
- środek -> "Środyk" / "w środę"
- rok -> "Drog." / ""
- kilku -> "Kielko" / "kill"
- fantasy -> "Fantazy" / "Fanta"
- strat -> "Strad" / ""
- apetyt -> "Apetit!" / "apet"
- zamieszczone -> "Zamieszczony." / ""

### pl-PL-Wavenet-A

- wieś -> "Wjeźdź." / "wiesz"
- środek -> "Środyk" / "w środę"
- rok -> "Drog." / ""
- kilku -> "Kielko" / "kill"
- fantasy -> "Fantazy" / "Fanta"
- strat -> "Strad" / ""
- apetyt -> "Apetit!" / "apet"
- zamieszczone -> "Zamieszczony." / ""

### pl-PL-Wavenet-F

- wieś -> "Wjeźdź." / "wiesz"
- środek -> "Środyk" / "w środę"
- rok -> "Drog." / ""
- kilku -> "Kielko" / "kill"
- fantasy -> "Fantazy" / "Fanta"
- strat -> "Strad" / ""
- apetyt -> "Apetit!" / "apet"
- zamieszczone -> "Zamieszczony." / ""


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### pl-PL-Wavenet-B

- nos -> "Nos." / "no"
- środek -> "Środek" / "środa"
- chmura -> "Chmura." / "chmur"
- skóra -> "Skóra." / ""
- mówić -> "Move it." / "mówić"
- lipcu -> "lipcu" / "Lips"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / "p"
- długie -> "Długie." / "długi"
- lidze -> "Lidze." / "nic"
- czego -> "Czego?" / "cześć"
- zespół -> "Zespół" / "zesp"
- rowerze -> "Rowerze." / "rower"
- systemów -> "Systemów" / "systemu"
- widelec -> "Widelać." / "widelec"
- wartości -> "Wartości" / "wartość"
- dekady -> "Dekady." / "dekada"
- zniknie -> "Zniknie!" / "znikł"
- względami -> "względami." / "względem"
- apetyt -> "Apetit!" / "apetyt"
- zmieniono -> "Zmieniono." / "zmieniam"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych." / "etniczny"
- białek -> "Białek." / "biały"
- województwa -> "Województwa" / "województwo"
- zestawieniu -> "Zestawieniu." / "zestawień"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "wdzięczny"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"

### pl-PL-Wavenet-C

- nos -> "Nos." / "no"
- środek -> "Środek" / "środa"
- chmura -> "Chmura." / "chmur"
- skóra -> "Skóra." / ""
- mówić -> "Move it." / "mówić"
- lipcu -> "lipcu" / "Lips"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / "p"
- długie -> "Długie." / "długi"
- lidze -> "Lidze." / "nic"
- czego -> "Czego?" / "cześć"
- zespół -> "Zespół" / "zesp"
- rowerze -> "Rowerze." / "rower"
- systemów -> "Systemów" / "systemu"
- widelec -> "Widelać." / "widelec"
- wartości -> "Wartości" / "wartość"
- dekady -> "Dekady." / "dekada"
- zniknie -> "Zniknie!" / "znikł"
- względami -> "względami." / "względem"
- apetyt -> "Apetit!" / "apetyt"
- zmieniono -> "Zmieniono." / "zmienią"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych." / "etniczny"
- białek -> "Białek." / "biały"
- województwa -> "Województwa" / "województwo"
- zestawieniu -> "Zestawieniu." / "zestawień"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "wdzięczny"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"

### pl-PL-Wavenet-D

- ziemia -> "Ziemia" / "ziem"
- nos -> "Nos" / "no"
- pchać -> "Pchacz" / "pchać"
- cena -> "cena" / "C"
- noc -> "Noc." / "no"
- chmura -> "chmura" / "hmu"
- biały -> "Biały" / "Biała"
- dom -> "Dom." / "do"
- skóra -> "Skóra" / ""
- mówić -> "Mówić" / "mówi"
- żelazo -> "Żelazo" / "żel a"
- lipcu -> "lipcu" / "NIP"
- zakupy -> "Zakupy" / "zakup"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / ""
- długie -> "Długie" / ""
- spraw -> "Spraw!" / ""
- lidze -> "Lidze" / "nic"
- kobiet -> "Kobiet" / "Cobi"
- czego -> "Czego?" / "cz"
- zespół -> "Zespół" / "zesp"
- zmarł -> "Zmarł." / "ZMA"
- dajcie -> "Dajcie!" / "daj"
- rowerze -> "Rowerze" / "rower"
- truskawka -> "Truskawka" / "Truskaw"
- systemów -> "Systemów." / "system"
- istnieją -> "Istnieją." / "istnieje"
- oddział -> "Oddział." / ""
- członkami -> "Członkami." / "członka"
- widelec -> "Widelać." / "widelec"
- czekolada -> "Czekolada." / ""
- wartości -> "Wartości" / "wartość"
- dekady -> "Dekady." / ""
- zniknie -> "Zniknie!" / "znicz"
- względami -> "Względami" / "względem"
- zmieniono -> "Zmieniono." / "zmienią"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych" / "etniczny"
- białek -> "Białek" / "biały"
- województwa -> "Województwa" / "województw"
- blokada -> "Blokada." / "Blocka"
- zestawieniu -> "Zestawieniu" / "zestawie"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "więc"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"

### pl-PL-Wavenet-E

- ziemia -> "Ziemia" / "ziem"
- nos -> "Nos" / "no"
- pchać -> "Pchacz" / "pchać"
- cena -> "cena" / "C"
- noc -> "Noc." / "no"
- chmura -> "chmura" / "hmu"
- biały -> "Biały" / "Biała"
- dom -> "Dom." / "do"
- skóra -> "Skóra" / ""
- mówić -> "Mówić" / "mówi"
- żelazo -> "Żelazo" / "żel a"
- lipcu -> "lipcu" / "NIP"
- zakupy -> "Zakupy" / "zakup"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / ""
- długie -> "Długie" / ""
- spraw -> "Spraw!" / ""
- lidze -> "Lidze" / "nic"
- kobiet -> "Kobiet" / "Cobi"
- czego -> "Czego?" / "cz"
- zespół -> "Zespół" / "zesp"
- zmarł -> "Zmarł." / "ZMA"
- dajcie -> "Dajcie!" / "daj"
- rowerze -> "Rowerze" / "rower"
- truskawka -> "Truskawka" / "Truskaw"
- systemów -> "Systemów." / "system"
- istnieją -> "Istnieją." / "istnieje"
- oddział -> "Oddział." / ""
- członkami -> "Członkami." / "członka"
- widelec -> "Widelać." / "widelec"
- czekolada -> "Czekolada." / ""
- wartości -> "Wartości" / "wartość"
- dekady -> "Dekady." / ""
- zniknie -> "Zniknie!" / "znicz"
- względami -> "WZGLĘDAMI" / "względem"
- zmieniono -> "Zmieniono." / "zmienią"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych" / "etniczny"
- białek -> "Białek" / "biały"
- województwa -> "Województwa" / "województw"
- blokada -> "Blokada." / "Blocka"
- zestawieniu -> "Zestawieniu" / "zestawie"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "więc"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"

### pl-PL-Wavenet-A

- ziemia -> "Ziemia" / "ziem"
- nos -> "Nos" / "no"
- pchać -> "Pchacz" / "pchać"
- cena -> "cena" / "C"
- noc -> "Noc." / "no"
- chmura -> "chmura" / "hmu"
- biały -> "Biały" / "Biała"
- dom -> "Dom." / "do"
- skóra -> "Skóra" / ""
- mówić -> "Mówić" / "mówi"
- żelazo -> "Żelazo" / "żel a"
- lipcu -> "lipcu" / "NIP"
- zakupy -> "Zakupy" / "zakup"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / ""
- długie -> "Długie" / ""
- spraw -> "Spraw!" / ""
- lidze -> "Lidze" / "nic"
- kobiet -> "Kobiet" / "Cobi"
- czego -> "Czego?" / "cz"
- zespół -> "Zespół" / "zesp"
- zmarł -> "Zmarł." / "ZMA"
- dajcie -> "Dajcie!" / "daj"
- rowerze -> "Rowerze" / "rower"
- truskawka -> "Truskawka" / "Truskaw"
- systemów -> "Systemów." / "system"
- istnieją -> "Istnieją." / "istnieje"
- oddział -> "Oddział." / ""
- członkami -> "Członkami." / "członka"
- widelec -> "Widelać." / "widelec"
- czekolada -> "Czekolada." / ""
- wartości -> "Wartości" / "wartość"
- odkryć -> "Odkryć." / ""
- dekady -> "Dekady." / ""
- zniknie -> "Zniknie!" / "znicz"
- względami -> "Względami" / "względem"
- zmieniono -> "Zmieniono." / "zmienią"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych" / "etniczny"
- białek -> "Białek" / "biały"
- województwa -> "Województwa" / "województw"
- blokada -> "Blokada." / "Blocka"
- zestawieniu -> "Zestawieniu" / "zestawie"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "więc"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"

### pl-PL-Wavenet-F

- ziemia -> "Ziemia" / "ziem"
- nos -> "Nos" / "no"
- pchać -> "Pchacz." / "pchać"
- cena -> "cena" / "C"
- noc -> "Noc." / "no"
- chmura -> "chmura" / "hmu"
- biały -> "Biały" / "Biała"
- dom -> "Dom." / "do"
- skóra -> "Skóra" / ""
- mówić -> "Mówić" / "mówi"
- żelazo -> "Żelazo" / "żel a"
- lipcu -> "lipcu" / "NIP"
- zakupy -> "Zakupy" / "zakup"
- świata -> "Świata." / "świat"
- pełna -> "Pełna." / ""
- długie -> "Długie" / ""
- spraw -> "Spraw!" / ""
- lidze -> "Lidze" / "nic"
- kobiet -> "Kobiet" / "Cobi"
- czego -> "Czego?" / "cz"
- zespół -> "Zespół" / "zesp"
- zmarł -> "Zmarł." / "ZMA"
- dajcie -> "Dajcie!" / "daj"
- rowerze -> "Rowerze" / "rower"
- truskawka -> "Truskawka" / "Truskaw"
- systemów -> "Systemów." / "system"
- istnieją -> "Istnieją." / "istnieje"
- oddział -> "Oddział." / ""
- członkami -> "Członkami." / "członka"
- widelec -> "Widelać." / "widelec"
- czekolada -> "Czekolada." / ""
- wartości -> "Wartości" / "wartość"
- odkryć -> "Odkryć." / ""
- dekady -> "Dekady." / ""
- zniknie -> "Zniknie!" / "znicz"
- względami -> "Względami" / "względem"
- zmieniono -> "Zmieniono." / "zmienią"
- operacyjnego -> "Operacyjnego." / "operacyjne"
- dokładny -> "Dokładny." / "dokładnie"
- etnicznych -> "etnicznych" / "etniczny"
- białek -> "Białek" / "biały"
- województwa -> "Województwa" / "województw"
- blokada -> "Blokada." / "Blocka"
- zestawieniu -> "Zestawieniu" / "zestawie"
- wielkiego -> "Wielkiego." / "Wielkie"
- wdzięczni -> "Wdzięczni." / "więc"
- rosyjskiego -> "Rosyjskiego" / "rosyjskie"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
