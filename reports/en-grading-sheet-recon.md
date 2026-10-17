# CC-EN-GRADING-SHEET v1 §0 recon

Run 2026-10-16 on `origin/main` 66f2d690. Read-only: nothing built, nothing changed.

## R1. The low-band sheet

- Path: `reports/en-lowband-review-sample.csv`.
- 200 rows, Zipf 2.0 to 2.5, all scoring 0.62 or higher, drawn with seed 20261016.
- **No grades yet.** The word-verdict, est_tier, traps and note columns are empty in every row, so there is nothing to carry over. No graded copy has been uploaded.

## R2. Staged English rows scoring 0.62 or higher

- **1,788** of the 14,124 staged rows (`assets/words/en/staged.tsv`), scored by `assets/words/en/en_difficulty.json` under the current weights (0.3 / 0.3 / 0.2, transparency penalty 0.1).
- **Overlap to decide:** 61 of those 1,788 are in the 200-word staged sample you already graded (`reports/en-staged-review-sample-graded.csv`). The spec only carries over low-band grades. Recommendation: either exclude those 61 from the new 300-word draw, or carry their existing grades in as `carried`. Otherwise you could be asked to grade a word twice.

## R3. The auditor sheet template

There is no single template. Three sheets exist, with two different column sets:

| Sheet | Columns |
|---|---|
| `reports/en-staged-review-sample-graded.csv` (the one you returned) | `word`, `pos`, `candidate_definition`, `verdict (keep/cut/unsure)`, `note`, `est_tier`, `spelling_traps`, `flag` |
| `reports/en-lowband-review-sample.csv` | `word`, `pos`, `candidate_1..3`, `word verdict (keep/cut/unsure)`, `est_tier (Easy/Medium/Hard/Expert)`, `spelling_traps`, `note` |
| `reports/en-staged-review-sample.csv` | as above, but `definition verdict` instead of `est_tier` |

Allowed values in use:
- verdict: `keep`, `cut`, `unsure`
- est_tier: `Easy`, `Medium`, `Hard`, `Expert` (your graded sheet used exactly these four)
- traps: free text (e.g. "ch=k; schwa a"); no fixed list
- flag: free text (homophone and variant notes)

**Mismatch with the spec:** the spec's `keep_cut` dropdown has no `unsure`, but every existing sheet allows it. Since the workbook's columns must match the auditor sheet's (D5), one of them has to change. Recommendation: keep `unsure` in the dropdown, and treat `unsure` as "not graded" for calibration.

## R4. Pronunciation and definition sources

- **Pronunciation:** CMUdict (ARPAbet, stress stripped). It covers all 200 low-band words and all 1,788 staged words. `en_pron.json` already holds the staged words, but not the low-band ones; those come straight from `tools/wordpipe/sources/cmudict.dict`.
  - **Readability:** ARPAbet looks like `AH K AA M AH D EY T` (accommodate). That may be hard to read at a glance. Showing it is your call; it would be the only pronunciation available without a new source.
- **Definition:** the first Wiktionary gloss from the cached kaikki dump (`.corpus-cache/en-wikt/`), using the three-candidate ranking.
  - **Missing:** 1 of the 200 low-band words has no gloss (*plumbed*, adjective). All 1,788 staged words have one.
  - **Quality caveat:** your earlier review found about 14% of first glosses give a sense learners won't expect.

## R5. Output paths

| What | Path |
|---|---|
| Workbook | `~/repos/en-grading/en-expert-grading-<date>.xlsx` (outside the repo, like the other auditor sheets) |
| Graded ledger (ingest output) | `calibration/en_graded.jsonl` in the repo, one row per graded word, tagged `grader`, workbook name and seed |
| Calibration report | `reports/en-grading-calibration-<date>.md` |
| Code | `tools/en_growth/grading_workbook.py`, `tools/en_growth/grading_ingest.py`, `tools/en_growth/test_grading.py` |

**Tooling note:** writing `.xlsx` with dropdowns needs `openpyxl`, which isn't installed. I'd add it to the gitignored build venv (`.corpus-cache/wf/v`); it is a build-time dependency only and never ships.

**Key-sheet note:** a hidden, protected `_key` sheet in an `.xlsx` hides the score from casual view, but Excel sheet protection is not real security; anyone can unhide it. If the point is that you never see the scores even by accident, a separate key file kept outside the workbook (as the decoy keys are) is safer. Your call.

## Decisions needed before Phase A

1. The 61 overlap words: exclude from the draw, or carry their grades over.
2. `keep_cut` with or without `unsure`.
3. Show the ARPAbet pronunciation, or leave the column out.
4. Hidden `_key` sheet inside the workbook, or a separate key file.
