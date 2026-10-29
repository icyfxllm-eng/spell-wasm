# Unfair-by-ear candidates (English bank)

Read-only sweep after the coup ruling (09751197). Nothing is quarantined here; Eric rules per word, a later session applies the quarantine with the same mechanism. Regenerate: `.corpus-cache/wf/v/bin/python tools/bank/unfair_by_ear.py`. Full list: `reports/unfair-by-ear-candidates.csv` (the `eric_verdict` column is blank for yes/no).

Rank = severity x hit share. Hit share is a judgment, not a measurement: Easy 1.0 (every player), Medium 0.8, Hard 0.4, Expert 0.2 (Junior plays Easy+Medium, Standard plays all four). Severity: class A is the irregularity percentile scaled by rarity, +0.15 on a known silent-letter pattern; class B 0.6 on a target-only set else 0.2; class C 0.7 when the clip verdict is Fail, 0.4 when it is ExemptHomophone; +0.05 per extra class. No candidate is already quarantined (quarantined words are out of the bank).

| Class | Easy | Medium | Hard | Expert | Total |
|---|---|---|---|---|---|
| A irregular or silent letters | 35 | 24 | 14 | 14 | 87 |
| B homophone, both in bank, no cue shown | 74 | 21 | 6 | 2 | 103 |
| C both recognizers heard another real word | 15 | 3 | 1 | 3 | 22 |
| Distinct words | 118 | 41 | 20 | 18 | 197 |

58 candidates already carry the audio exempt-homophone verdict.

## Caveats

- Class A screens with the spelling-given-sound irregularity percentile (tools/en_growth/score.py) plus a silent-letter pattern list. The aligner is noisy near the margin, so expect false positives among common Easy words (leaf, fee, look).
- Class B is already graded fairly: English accepts any member of a true sound-alike set (accept-all, F2a), and en cueMode is off, so no meaning cue is shown anywhere. The residual unfairness is the target-only sets (read/lead; the CSV marks them NOT accepted).
- Class C excludes members of the same homophone set and number words. It lists clips where both recognizers agreed on a different word, which points at a clip or voice problem as much as an unfair word.

## Top 25 for yes/no

| # | Word | Tier | Classes | Evidence | Status |
|---|---|---|---|---|---|
| 1 | heir | easy | A+B | irregularity pct 0.976 ; worst unit: he spells nothing; silent letters: he; known irregular/silent-letter pattern | exempt-homophone-audio |
| 2 | folk | easy | A | irregularity pct 0.994 ; worst unit: olk spells K; known irregular/silent-letter pattern | none |
| 3 | knew | easy | A+B | irregularity pct 0.994 ; worst unit: k spells N; known irregular/silent-letter pattern | exempt-homophone-audio |
| 4 | debt | easy | A | irregularity pct 0.951 ; worst unit: ebt spells T; known irregular/silent-letter pattern | none |
| 5 | whom | easy | A | irregularity pct 0.939 ; worst unit: who spells UW; known irregular/silent-letter pattern | none |
| 6 | know | easy | A+B | irregularity pct 1.000 ; worst unit: k spells N; known irregular/silent-letter pattern | exempt-homophone-audio |
| 7 | khan | easy | A | irregularity pct 0.987 ; worst unit: han spells N | none |
| 8 | two | easy | A+B | irregularity pct 0.981 ; worst unit: w spells UW; known irregular/silent-letter pattern | none |
| 9 | ease | easy | A | irregularity pct 0.997 ; worst unit: ase spells Z | none |
| 10 | sure | easy | A | irregularity pct 0.996 ; worst unit: ure spells R; known irregular/silent-letter pattern | none |
| 11 | mouse | easy | A | irregularity pct 0.989 ; worst unit: ou spells nothing | none |
| 12 | says | easy | A | irregularity pct 0.958 ; worst unit: ays spells Z; known irregular/silent-letter pattern | none |
| 13 | corps | medium | A+B | irregularity pct 0.968 ; worst unit: rps spells AO; known irregular/silent-letter pattern | exempt-homophone-audio |
| 14 | aim | easy | A | irregularity pct 0.993 ; worst unit: aim spells M | none |
| 15 | fee | easy | A | irregularity pct 0.992 ; worst unit: ee spells IY | none |
| 16 | tie | easy | A | irregularity pct 0.986 ; worst unit: e spells nothing | none |
| 17 | sea | easy | A+B | irregularity pct 0.999 ; worst unit: sea spells IY | exempt-homophone-audio |
| 18 | cook | easy | A | irregularity pct 0.995 ; worst unit: ook spells K | none |
| 19 | writes | medium | A+B | irregularity pct 0.934 ; worst unit: wri spells AY; known irregular/silent-letter pattern | exempt-homophone-audio |
| 20 | seek | easy | A | irregularity pct 0.986 ; worst unit: eek spells K | none |
| 21 | tea | easy | A | irregularity pct 1.000 ; worst unit: ea spells IY | exempt-homophone-audio |
| 22 | who | easy | A | irregularity pct 0.984 ; worst unit: who spells UW; known irregular/silent-letter pattern | none |
| 23 | edge | easy | A | irregularity pct 0.993 ; worst unit: e spells EH | none |
| 24 | scene | medium | A+B | irregularity pct 0.999 ; worst unit: ene spells N; known irregular/silent-letter pattern | none |
| 25 | lie | easy | A | irregularity pct 0.989 ; worst unit: e spells nothing | none |

## Coup-like words in Hard and Expert (outside the Easy/Medium ranking)

wright (hard, A+B), leisure (hard, A), exposure (hard, A), questionnaire (hard, A), machines (hard, A), reached (hard, A), brought (hard, A), written (hard, A), neighbor (hard, A), thought (hard, A), unknown (hard, A), occurred (hard, A), through (hard, A), release (hard, A), connoisseur (expert, A+C), rendezvous (expert, A), pneumonia (expert, A), conscientious (expert, A), yacht (expert, A), psychology (expert, A), colonel (expert, A), psychological (expert, A), technique (expert, A), gauge (expert, A), scheduled (expert, A), knowledge (expert, A), throughout (expert, A), beautiful (expert, A)
