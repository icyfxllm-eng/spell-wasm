# Bank growth to the signed pool floors, 2026-10-12

On Eric's instruction ("hold off on audit sheets and just add to the word banks"). Tool: `tools/bank/purity_grow.py`
(`scripts/build-bigbank.py --to-floors`). Candidates come from each language's pinned Leipzig corpus list, in frequency
order; every one passes the CC-BANK-PURITY F1 admit gate (dictionary headword, not a name, not another language) and
the variety check; quarantined words and the non-declared variety are never admitted; no tier goes past its
`poolFloors` value. New rows carry a `dictionary` ledger verdict. No LLM output, no generated forms. No sampled audit
was run (CC-BANK-REBUILD R8 was open).

## What is in the banks (Medium, Hard and Expert only)

| Lang | Rows added | Medium | Hard | Expert |
|---|---|---|---|---|
| es | 635 | 857 → 1000 | 1878 → 2000 | 2630 → 3000 |
| fr | 736 | 862 → 1000 | 1846 → 2000 | 2556 → 3000 |
| de | 534 | 950 → 1000 | 1866 → 2000 | 2650 → 3000 |
| pt | 747 | 887 → 1000 | 1799 → 2000 | 2567 → 3000 |
| ar | 764 | 898 → 1000 | 1813 → 2000 | 2525 → 3000 |
| fil | 839 | 574 → 600 | 923 → 1200 | 1464 → 2000 |
| ru | 267 | 949 → 1000 | 1900 → 2000 | 2884 → 3000 |
| pl | 350 | held | 1865 → 2000 | 2785 → 3000 |
| sw | 95 | (above floor) | 1105 → 1200 | no tier-4 floor |

4,967 rows in all. Easy is unchanged everywhere.

## Corrections made the same day

- **Easy growth reverted.** The first pass also grew Easy (es 232, fr 221, de 228, pt 228, ar 210, fil 53, sw 38: 1,210
  rows). A frequency list fills 2–4 letter words with function words (`de la que`, `der und die`, `de et le`,
  `في من على`, `ng sa na ang`), which CC-BANK-REBUILD R9 forbids; the warm dry-run exposed it. They were removed, so
  Easy stays below its floor (report-only) until a content-word filter exists, which needs part-of-speech data this
  path does not have. `purity_grow.py` now skips Easy unless `--tiers` names it.
- **Variety.** `equipa` (pt-PT) was admitted into the pt-BR bank on the first attempt and the variety check caught it.
  The grower now skips the non-declared member of every pair in `scripts/bank-variety-check.mjs`.

## Not done, and why

- **pl Easy and Medium (held).** Growing them lets Letter Forge Jr sustain a board of 9 every day, and
  `every_junior_ready_language_gets_a_board_every_day` then requires `junior_min_pool("pl")` (now 7) to change. That is
  a named threshold the CC-BANK-PURITY and CC-BANK-REBUILD hard constraints forbid editing without Eric. pl Easy is 267
  of 500 and Medium 949 of 1000 until that is decided.
- **ko, ja, hi.** This path cannot reach their floors (admitted candidates: ko Medium +579, Hard +15, Expert 0; ja none
  past Easy; hi Expert +51). They need the CC-BANK-REBUILD headword sources, the two-source rule and `license_cleared`.
- **en.** Hard and Expert admit thousands of rows, but a new English row would have no definition and nothing fails
  (CC-BANK-REBUILD C8). Held until definitions can be covered.
- **zh, vi, fa.** Separate builders / out of scope.

## Side effects

- `wordgrid_determinism` and the e2e copy of the digest were re-pinned (reason beside the pin).
- es and ru collision tables regenerated.
- Players see these rows only in the next app build.
- **Audio is not warmed.** 4,967 words, 9,934 clips, of which 3,666 fall in fr, pt and pl, which were already cold.
  Google TTS billing lapsed 2026-09-30. `warm-run.sh` reads banks from `~/repos/spell-wasm` (a stale checkout) and the
  tool's self-check aborts on a grown bank; both need handling before the new words can be warmed.
- **Definitions** for the new rows are not in `backend/def_pools` (served; Definition Match is unaffected, no gate
  counts them).
