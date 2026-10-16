# English score recalibration on Eric's graded sample (2026-10-16)

Data: 200 graded staged words (Easy 50 / Medium 92 / Hard 47 / Expert 11), plus the approved 150-word A3 calibration set.
The grader of the est_tier labels is not recorded; Eric reviewed and endorsed them. They are used as calibration evidence only, never as a word source.

## What changed in the score
- New feature: transparency (a compound of two common words, or a common stem plus a productive suffix) lowers the score.
- Weights refit: irregularity 0.3, rarity 0.3, length 0.2, transparency penalty 0.1 (was 0.6 / 0.3 / 0.1 / none).
  Length fit best at the 0.3 cap but is held at 0.2: the grades correlate with length, and the spec treats length as minor.
- 5-fold cross-validation, held-out Spearman against the graded tiers: 0.29 (old weights) -> 0.55-0.59 (refit). The same weights won in nearly every fold.
- A3 still passes on the approved set at its best-separating threshold (0.512: 85% / 6%).

## What the data cannot do
- Within staged words the score ranks tiers moderately well (AUC Expert vs the rest 0.82) but cannot isolate Expert.
  At a threshold that keeps 9 of the 11 graded Experts (0.62), 52 of the 189 non-Experts also pass: precision about 15%.
- The two calibration sets disagree on where the Expert line sits: ~0.52 for the A3 set (well-known misspelled words) vs ~0.62-0.65 for the graded staged sample (rare words, most of them easy to spell).
- `band_boundaries` is therefore left unset. Eric decides.
- The new weights score nearly all current bank rows low (only 53 of 797 current Expert rows reach 0.58), because they are common words. The old bank Expert tier is mostly common long words, as the census found.

## Counts at candidate thresholds (admitted 14,124 staged candidates; 3,528 low-band candidates at Zipf 2.0-2.5)
| threshold | staged (Zipf 2.5-4.0) | low band (Zipf 2.0-2.5) | current bank Expert rows |
|---|---|---|---|
| 0.58 | 3,437 | 1,754 | 53 |
| 0.62 | 1,788 | 1,229 | 35 |
| 0.65 | 921 | 820 | 23 |
| 0.70 | 186 | 263 | 10 |

At 5.5% true-Expert in the graded sample, staged rows hold roughly 290 genuine Experts; the rest of an Expert-band cut would be Hard or Medium words.

## D4 amendment (Eric, "1 and 2")
Expert candidates may be drawn from Zipf 2.0-4.0 (was 2.5-4.0) as a shortlist only. A 200-row sample of the low-band shortlist (score >= 0.62, 1,229 rows) is in `reports/en-lowband-review-sample.csv` for grading. Nothing below 2.5 goes live without his grading.
