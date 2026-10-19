# A-15 stability run (CC-IMPOSSIBLE-LAUNCH Feature 5)

Run 2026-10-19 on origin/main 7a06b4eb (code identical to 931c272c plus tools only), against the test web bundle built after the 41-row quarantine. Procedure: for each group, `node tests/e2e/run.mjs <group>` ten times in a row on one machine; a failed run would stop that group for a root-cause fix and restart its count (none failed). Logs: one per run, kept outside git.

**Result: 30 of 30 runs clean. No fixes were needed and no run was repeated.**

| Group | Specs in group | Tests per run | Runs | Clean | Seconds per run (min / median / max) |
|---|---|---|---|---|---|
| spelldoku | spelldoku, spelldoku-tier | 35 | 10 | 10 | 80 / 89 / 95 |
| boardgame | boardgame, boardgame-layout, -polish, -sprint, -stretch, -boost | 41 | 10 | 10 | 289 / 316 / 372 |
| spelluzzle | spelluzzle, v11, par, g, duel | 44 | 10 | 10 | 49 / 51 / 53 |

Each run: every test in the group passed (35/35, 41/41, 44/44) and the process exited 0.

## Per-run results

| Group | Run | Result | Seconds |
|---|---|---|---|
| spelldoku | 1 | 35/35 passed, exit 0 | 93 |
| spelldoku | 2 | 35/35 passed, exit 0 | 88 |
| spelldoku | 3 | 35/35 passed, exit 0 | 89 |
| spelldoku | 4 | 35/35 passed, exit 0 | 87 |
| spelldoku | 5 | 35/35 passed, exit 0 | 86 |
| spelldoku | 6 | 35/35 passed, exit 0 | 95 |
| spelldoku | 7 | 35/35 passed, exit 0 | 91 |
| spelldoku | 8 | 35/35 passed, exit 0 | 89 |
| spelldoku | 9 | 35/35 passed, exit 0 | 80 |
| spelldoku | 10 | 35/35 passed, exit 0 | 86 |
| boardgame | 1 | 41/41 passed, exit 0 | 317 |
| boardgame | 2 | 41/41 passed, exit 0 | 307 |
| boardgame | 3 | 41/41 passed, exit 0 | 365 |
| boardgame | 4 | 41/41 passed, exit 0 | 313 |
| boardgame | 5 | 41/41 passed, exit 0 | 316 |
| boardgame | 6 | 41/41 passed, exit 0 | 303 |
| boardgame | 7 | 41/41 passed, exit 0 | 309 |
| boardgame | 8 | 41/41 passed, exit 0 | 371 |
| boardgame | 9 | 41/41 passed, exit 0 | 372 |
| boardgame | 10 | 41/41 passed, exit 0 | 289 |
| spelluzzle | 1 | 44/44 passed, exit 0 | 51 |
| spelluzzle | 2 | 44/44 passed, exit 0 | 53 |
| spelluzzle | 3 | 44/44 passed, exit 0 | 52 |
| spelluzzle | 4 | 44/44 passed, exit 0 | 53 |
| spelluzzle | 5 | 44/44 passed, exit 0 | 50 |
| spelluzzle | 6 | 44/44 passed, exit 0 | 51 |
| spelluzzle | 7 | 44/44 passed, exit 0 | 49 |
| spelluzzle | 8 | 44/44 passed, exit 0 | 51 |
| spelluzzle | 9 | 44/44 passed, exit 0 | 51 |
| spelluzzle | 10 | 44/44 passed, exit 0 | 51 |

## Notes and limits

- This is the e2e stability check the Complete spec's A-15 names (the three groups whose flaky tests commit f821855c deflaked). It does not include the full gate's cargo suite, which passed separately (1,313 tests) before this run.
- The runs were back to back on one Mac mini that was also doing light unrelated work, so they say nothing about a loaded CI machine.
- A different known-flaky test (randospell particles) failed once in a full 429-test e2e run earlier the same day and passed 3 of 3 on rerun; it is outside these three groups.
- Nothing outside Impossible was touched to get these results.
