# Bake-off — en

> **This run did not decide anything, and was not needed to.** The full
> 3,165-word head-to-head in `en-fullbank-C-vs-E.md` had already settled
> C vs E on 2026-10-01 — E 2927/141/97 against C 2894/165/106, with the
> damage concentrated in Easy tier — and the answer was: do not switch.
> This targeted pair was run on 2026-10-04 in the mistaken belief that the
> full-bank run was still outstanding. It cost nothing (every clip was
> already cached) and it agrees with the full bank, so it is kept for the
> one thing it adds: splitting the two phonetic classes apart names the
> MECHANISM of C's CVC losses, which a bank-wide total averages away.
> For the shipping decision, read `en-fullbank-C-vs-E.md`, not this.

155 words per voice, both recognizers, blind (I3).

2 voice(s) scored from 2 candidate name(s).

Rows that were NOT flagged as aliases are **unconfirmed, not confirmed
distinct**. Google synthesis is non-deterministic, so two samples of one
voice can differ; a match proves sameness, a non-match proves nothing.

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| en-US-Neural2-E **(current)** | 129 | 15 | 11 |
| en-US-Neural2-C | 119 | 26 | 10 |

## Missed by both recognizers

### en-US-Neural2-E

- ban -> "then" / "then"
- bay -> "They" / ""
- ben -> "Then," / "then"
- buy -> "Bye." / "bye"
- dot -> "." / "do"
- for -> "4." / "4"
- led -> "Lead" / "lead"
- row -> "Bro." / ""
- some -> "Sum" / ""
- son -> "Sun" / "Sun"
- won -> "1" / "1"

### en-US-Neural2-C

- bet -> "bat" / "bat"
- buy -> "Bye!" / "bye"
- dot -> "." / "do"
- for -> "4." / "4"
- gun -> "Done." / ""
- key -> "he" / ""
- per -> "Huh." / "her"
- son -> "Sun" / "Sun"
- win -> "when" / "when"
- won -> "1." / "1"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### en-US-Neural2-E

- bet -> "Bet." / ""
- day -> "Day" / ""
- don -> "Don." / "Dawn"
- gun -> "Done." / "gun"
- he -> "he" / ""
- her -> "Huh." / "her"
- lay -> "Lay." / "Le"
- not -> "Not." / "0"
- per -> "per" / "her"
- put -> "Put." / ""
- ran -> "ran" / "Ren"
- sex -> "Sex" / "6"
- sum -> "Sum" / ""
- war -> "War" / ""
- win -> "win" / "when"

### en-US-Neural2-C

- bed -> "bad" / "bed"
- ben -> "Ben" / "been"
- bid -> "bed" / "bid"
- big -> "beg" / "big"
- bit -> "bet" / "bit"
- book -> "Buck." / "book"
- boy -> "Bye." / "boy"
- cup -> "Cup" / ""
- cut -> "Cut." / "hot"
- desk -> "Dask" / "desk"
- duck -> "Duck" / "doc"
- he -> "he" / ""
- her -> "Huh." / "her"
- horse -> "PORSE" / "horse"
- jet -> "Jatt" / "jet"
- lay -> "Lay." / "Le"
- led -> "led" / "lead"
- log -> "Log" / "blog"
- men -> "man" / "men"
- pay -> "Pay." / "hey"
- set -> "Sat." / "set"
- sit -> "set" / "sit"
- sock -> "sock" / "socked"
- some -> "sum" / "some"
- sum -> "sum" / "some"
- way -> "way" / ""


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
