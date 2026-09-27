# Bake-off summary — eleven languages, 2026-09-27

86 candidate voices, 60 words each, both recognizers, blind (I3). Word lists
are a stratified sample of each real bank with a per-language seed, so a
re-run measures the same clips. Per-language detail is in the files beside
this one; `README.md` says why ja, zh and sw are not here.

**Recommendation only.** Switching a language's default needs Eric's
signature for that language (D6), and the switch itself is atomic: every
clip regenerates and passes F1 and F2 before a player hears one (F4 step 5),
one language at a time, never mid-session (D16).

## Read the two recognizer columns first

A Pass needs BOTH recognizers to return the word, so a language where one of
them cannot do the language at all can never score a Pass, however good the
voice is. That is not hypothetical: **whisper-small returns nothing usable
for Filipino — 0 of 60 — while Google STT gets 51.** Every Filipino voice
therefore scores 0 Pass and the ranking between them is noise.

So the absolute numbers are NOT comparable across languages, and in three of
them the method does not work at all yet. The per-recognizer split is the
only thing that distinguishes a bad voice from a recognizer that cannot hear
the language, which is why it is in the table rather than in a footnote.

The thresholds below are judgement, and worth arguing with: a language is
called **not measurable** when the weaker recognizer is under 15 of 60, and
a lead is called **weak evidence** when the weaker one is under 25 and the
lead is under 10 clips. They are round numbers chosen to separate the cases
the split above makes obvious, not thresholds anything was fitted to.

| | Current voice | P/W/F | Best measured | P/W/F | whisper | Google | Verdict |
|---|---|---|---|---|---|---|---|
| Spanish | `es-ES-Neural2-B` | 34/21/5 | `es-ES-Neural2-C` | 38/14/8 | 47/60 | 42/60 | **switch candidate** (+4) |
| French | `fr-FR-Neural2-A` | 28/11/21 | `fr-FR-Wavenet-F` | 29/9/22 | 32/60 | 35/60 | no change (+1 is inside the noise) |
| German | `de-DE-Neural2-B` | 33/17/10 | `de-DE-Neural2-A` | 38/15/7 | 49/60 | 34/60 | **switch candidate** (+5) |
| Portuguese | `pt-BR-Neural2-B` | 45/9/6 | `pt-BR-Neural2-B` | 45/9/6 | 48/60 | 51/60 | keep — the shipped voice wins |
| Polish | `pl-PL-Wavenet-B` | 24/30/6 | `pl-PL-Wavenet-B` | 24/30/6 | 51/60 | 27/60 | keep — the shipped voice wins |
| Russian | `ru-RU-Wavenet-D` | 27/26/7 | `ru-RU-Wavenet-A` | 37/18/5 | 48/60 | 31/60 | **switch candidate** (+10) |
| Vietnamese | `vi-VN-Wavenet-A` | 3/12/45 | `vi-VN-Wavenet-D` | 4/15/41 | 7/60 | 11/60 | **not measurable** — a recognizer is at the floor |
| Korean | `ko-KR-Wavenet-A` | 19/21/20 | `ko-KR-Wavenet-D` | 33/15/12 | 35/60 | 24/60 | **switch candidate** (+14) |
| Filipino | `fil-PH-Wavenet-A` | 0/51/9 | `fil-PH-Wavenet-C` | 0/46/14 | 0/60 | 51/60 | **not measurable** — a recognizer is at the floor |
| Arabic | `ar-XA-Wavenet-B` | 7/26/27 | `ar-XA-Wavenet-D` | 11/29/20 | 17/60 | 23/60 | weak evidence (+4, both recognizers shaky) |
| Hindi | `hi-IN-Neural2-A` | 9/32/19 | `hi-IN-Wavenet-A` | 9/31/20 | 10/60 | 40/60 | **not measurable** — a recognizer is at the floor |

## What I would put in front of you to sign

**Three, on evidence that holds.** Russian `ru-RU-Wavenet-A` over the shipped
`ru-RU-Wavenet-D` (+10 clips, and both recognizers competent). Korean
`ko-KR-Wavenet-D` over `ko-KR-Wavenet-A` (+14, the largest lead in the whole
run). German `de-DE-Neural2-A` over `de-DE-Neural2-B` (+5). Spanish
`es-ES-Neural2-C` is +4 over the incumbent but takes three more outright
failures with it (8 against 5), so it is the one I would leave alone of the
four — a voice that trades Weaks for Fails is not obviously better, and I1
acts on Fails.

**Two need nothing.** Portuguese and Polish already ship the best voice
measured, which is worth knowing: it says the original choices were good, not
that the test is insensitive — Portuguese has a clear spread across its eight
candidates.

**Three need a better recognizer before they can be ranked at all.**
Filipino, Hindi and Vietnamese. For Filipino and Hindi the fix is probably
cheap: whisper-small is the weak half and a larger multilingual model
(medium or large-v3) is a download, not a code change. Vietnamese is harder —
both recognizers are at the floor (7 and 11 of 60), and for a tonal language
that may be a real audio problem rather than a measurement one. It is the
language I would look at first by ear.

**One is honest but thin.** Arabic's `ar-XA-Wavenet-D` leads by 4 with both
recognizers under 25 of 60. ar-XA is Modern Standard Arabic and the
recognizers may be expecting dialect; CC-AR-HI-PREAUDIT is the place that
question belongs.

## A note on what a low Pass rate here does NOT mean

None of these numbers say a language's audio is bad. English scored 63 of 68
with the same harness, and English is the language both recognizers are best
at by a wide margin. A Filipino 0 and an English 63 differ mostly in how well
a machine knows the language. Within a language, against one bank, with both
recognizers fixed, the comparison between voices is fair — that is all the
bake-off claims, and all it should be read as claiming.
