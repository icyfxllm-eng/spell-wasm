# Russian stress audit — what the pass actually buys

Prepared 2026-10-06. The sheet is `build/ru-stress-sheet.csv`; the key it is
graded against is `build/ru-stress-sheet-DECOY-KEY.csv`.

`config/ru-stress-audit.json` has said `audited: false` since 2026-08-29, with
the note that Eric flips it after his own pass, expecting about a month of study
first. Before spending that month, three measurements.

## 1. All 973 rows are real judgements

No filler. The ingest already excluded the two row classes that need no human:

* single-vowel words — stress has only one place it can go;
* words containing ё — ё is always stressed in Russian and already written.

973 rows, 973 decisions. Nothing can be skimmed.

## 2. The audit buys easy and medium, and nothing else

Coverage of the shipping Russian bank:

| tier | bank words | in the stress table | covered |
|---|---|---|---|
| easy | 267 | 179 | 67% |
| medium | 970 | 794 | 81% |
| **hard** | 1977 | **0** | **0%** |
| **expert** | 2994 | **0** | **0%** |
| total | 6208 | 973 | 15% |

Every row in the table is a bank word (zero waste), but the table stops at
medium. Auditing all 973 makes easy and medium answerable and leaves 4,971 hard
and expert words reading flat — the "unanswerable question" `ru_stress.rs`
describes, unchanged, for 80% of the bank.

So the audit is worth doing and does not finish the job. Extending the table to
hard and expert is separate work, and it should probably happen BEFORE the audit
rather than after, so one pass covers everything instead of two.

## 3. A heteronym cannot be stored

`RU_STRESS` is `(&str, u8)` — one index per spelling, and 973 distinct words
with no duplicates. A word whose stress changes its meaning therefore cannot be
represented: за́мок (castle) and замо́к (lock) are one row, and whichever is
stored makes the other silently wrong.

Not live today: none of the ten classic heteronyms checked (замок, мука, атлас,
орган, хлопок, духи, белки, кружки, стрелки, пропасть) is in the table. But the
annotator already anticipates them — it flags "source gives more than one stress
for this spelling" — so the sheet can RAISE a heteronym that the table cannot
then STORE. Worth settling before a refill adds one.

## The sheet is ready

973 rows ordered easy first, each pre-filled with the Wiktionary stress rendered
as a marked string, because nobody can check an integer and everybody can check
`молоко́`. Correct any mark that sits on the wrong vowel, in place.

Six decoys are planted, carrying a deliberately wrong mark. Verified: returning
the sheet unchanged is REJECTED by `tools/ru_stress_ingest.py`, 6 missed against
a tolerance of 1, and nothing is written. A rejected sheet writes nothing at all;
there is no partial ingest, because an absent row is excluded from selection by
I5 while a wrong row is not.

Then:

    python3 tools/ru_stress_ingest.py \
      --sheet build/ru-stress-sheet.csv \
      --key   build/ru-stress-sheet-DECOY-KEY.csv --write

and flip `audited` in `config/ru-stress-audit.json` — by hand, because no tool
sets a human claim.

## One observation about who does it

The decoy mechanism exists to gate PAYMENT (v2 Feature 2). The workflow was
built expecting a hired native auditor, not a month of self-study — the sheet,
the decoys and the tolerance are all machinery for trusting someone else's work.
Handing this sheet to a Russian speaker is the path it was designed for. Eric's
call, but the month is not the only option.
