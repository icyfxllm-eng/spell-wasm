

---

## Phase D census: Spell Jr coverage (F7), run 2026-10-14

Jr pools Easy + Medium only (the Jr policy resolves through `experience::allowed_tiers`), applies the kid filter, the profanity and audio gates, and the proper-noun rule. F7 maps one die, faces 1–6, to word lengths 2–7 (face n → length n+1); D4's threshold of 20 words and its explicit nearest-covered remap still apply.

| lang | 2 | 3 | 4 | 5 | 6 | 7 | covered | remap (face → length played) |
|---|---|---|---|---|---|---|---|---|
| en | 25 | 164 | 535 | 372 | 372 | 17 | 5 | face 6 (length 7)→6 |
| es | 2 | 27 | 56 | 512 | 529 | 78 | 5 | face 1 (length 2)→3 |
| fr | 4 | 42 | 60 | 449 | 585 | 77 | 5 | face 1 (length 2)→3 |
| de | 2 | 22 | 95 | 411 | 569 | 80 | 5 | face 1 (length 2)→3 |
| pt | 4 | 37 | 68 | 509 | 522 | 76 | 5 | face 1 (length 2)→3 |
| pl | 1 | 29 | 58 | 455 | 494 | 93 | 5 | face 1 (length 2)→3 |
| ru | 43 | 165 | 460 | 480 | 533 | 71 | 6 | - |
| vi | 100 | 367 | 368 | 684 | 61 | 0 | 5 | face 6 (length 7)→6 |
| ar | 24 | 150 | 166 | 562 | 378 | 5 | 5 | face 6 (length 7)→6 |
| sw | 1 | 16 | 76 | 355 | 459 | 19 | 3 | face 1 (length 2)→4; face 2 (length 3)→4; face 6 (length 7)→6 |
| fil | 0 | 9 | 66 | 347 | 378 | 34 | 4 | face 1 (length 2)→4; face 2 (length 3)→4 |

**Finding: Jr coverage contradicts F7's premise.** Only ru covers all six lengths 2–7. Ten of the eleven eligible languages need remaps, and the remaps are not cosmetic:

* en loses length 7 (17 words); face 6 plays as 6.
* es, fr, de, pt, pl lose length 2 (1–4 words); face 1 plays as 3.
* vi has no length-7 word at all and 61 at length 6.
* ar has 5 at length 7.
* sw and fil lose lengths 2 and 3 (and sw loses 7), so a Jr die in those languages plays three or four distinct lengths with four or five faces stacked on 4.
* Census item 3's six-covered-lengths test, read against the six Jr lengths, passes for ru alone.

Under D4 the die still works, but F7's "faces 1–6 → lengths 2–7" holds only for ru; elsewhere it is a remapped die with uneven odds per length. The Jr word supply is also thin where it matters most for no-repeat: sw length 3 has 16 words, fil length 3 has 9 (below the floor).


---

## v1.1 census additions (C5–C9), run 2026-10-14

Run on the shipped banks (pending-purity rows excluded, as in the v1 census). C7 and C8 are Python simulations of the D-R3 payout order and Marathon-rule table play; they are calibration evidence, not engine output. **C9 was not run**: it needs a real SE-class phone and a simulator frame time would not be honest.

### C5 — Expert words per length (steal pool; Stretch at Hard→Expert)

| lang | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | length 7 ≥ 20? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 0 | 0 | 0 | 2 | 3 | 5 | 7 | 312 | 213 | 119 | 78 | **NO** (5) |
| es | 4 | 16 | 54 | 152 | 203 | 301 | 314 | 554 | 414 | 264 | 140 | yes |
| fr | 3 | 21 | 69 | 131 | 218 | 309 | 295 | 558 | 390 | 249 | 150 | yes |
| de | 1 | 12 | 45 | 135 | 226 | 233 | 226 | 505 | 443 | 328 | 204 | yes |
| pt | 7 | 24 | 63 | 153 | 212 | 257 | 319 | 552 | 426 | 255 | 148 | yes |
| pl | 7 | 38 | 76 | 199 | 268 | 325 | 315 | 555 | 407 | 282 | 154 | yes |
| ru | 4 | 30 | 69 | 192 | 264 | 303 | 336 | 561 | 407 | 296 | 182 | yes |
| vi | 0 | 0 | 0 | 0 | 0 | 17 | 1345 | 693 | 286 | 96 | 54 | **NO** (17) |
| ar | 13 | 140 | 420 | 482 | 292 | 254 | 107 | 448 | 149 | 47 | 17 | yes |
| sw | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 266 | 183 | 107 | 46 | **NO** (0) |
| fil | 4 | 14 | 36 | 68 | 58 | 75 | 81 | 250 | 154 | 86 | 37 | yes |

### C6 — Stretch availability (tier t+1 holds ≥ 20 words at every length where tier t does)

| lang | easy→medium | medium→hard | hard→expert |
|---|---|---|---|
| en | missing 2,3,4 (0/3 ok) | missing 5,6 (1/3 ok) | missing 7,8 (0/2 ok) |
| es | missing 3,4 (3/5 ok) | all | all |
| fr | missing 3,4 (3/5 ok) | all | all |
| de | missing 3,4 (3/5 ok) | all | all |
| pt | missing 3,4 (3/5 ok) | all | all |
| pl | missing 3,4 (3/5 ok) | all | all |
| ru | missing 2,3,4 (3/6 ok) | all | all |
| vi | missing 2,3 (1/3 ok) | missing 4,5 (1/3 ok) | missing 6,7 (0/2 ok) |
| ar | all | all | all |
| sw | missing 4 (2/3 ok) | all | missing 5,6,7,8 (2/6 ok) |
| fil | missing 4,7 (2/4 ok) | all | all |

### C7 — Simulated per-word payout (10,000 solo Marathon games, 85% accuracy, Cautious banking at pot ≥ 12)

Dice are two fair dice; heat, doubles, jackpot, Stretch, Ear and the Call follow D-R3 in order; ceil once; cap 150. "All bets" = the bot Stretches every word, Ears when offered, and calls a random segment.

| scenario | words | p50 | p90 | p99 | max | words over 150 before the cap | share |
|---|---|---|---|---|---|---|---|
| no bets (doubles + heat only) | 125786 | 7 | 12 | 48 | 72 | 0 | 0.000% |
| all bets | 55310 | 18 | 28 | 72 | 75 | 0 | 0.000% |

### C8 — NPC calibration (500 games per cell; no Stretch/Call/Ear/steal on either side, per D-R8; Ward on; Last Call on)

Human is a scripted bot that banks at pot ≥ 12. Board-game D13: NPC accuracy = clamp(human − delta, 0.30, 0.95). Targets are CC-BOARD-GAME D18 for a 3-NPC table (Easy 50–60%, Normal 35–45%, Tough 21–33%).

| NPCs (3) | delta | human 100% | human 70% | human 50% | in D18 range? |
|---|---|---|---|---|---|
| Cautious ×3 | easy (0.30) | 85.6% | 89.2% | 72.2% | no (target 50–60%) |
| Cautious ×3 | normal (0.20) | 66.4% | 69.4% | 72.2% | no (target 35–45%) |
| Cautious ×3 | tough (0.10) | 40.6% | 42.6% | 44.6% | no (target 21–33%) |
| Gambler ×3 | easy (0.30) | 49.0% | 93.2% | 92.4% | no (target 50–60%) |
| Gambler ×3 | normal (0.20) | 15.8% | 72.6% | 92.4% | no (target 35–45%) |
| Gambler ×3 | tough (0.10) | 2.8% | 37.8% | 69.4% | no (target 21–33%) |
| Mirror ×3 | easy (0.30) | 97.6% | 98.6% | 96.2% | no (target 50–60%) |
| Mirror ×3 | normal (0.20) | 96.8% | 96.8% | 96.2% | no (target 35–45%) |
| Mirror ×3 | tough (0.10) | 93.0% | 88.6% | 90.4% | no (target 21–33%) |
| Cautious+Gambler+Mirror | easy (0.30) | 75.6% | 92.2% | 85.6% | no (target 50–60%) |
| Cautious+Gambler+Mirror | normal (0.20) | 45.0% | 79.8% | 85.6% | no (target 35–45%) |
| Cautious+Gambler+Mirror | tough (0.10) | 16.4% | 56.6% | 67.2% | no (target 21–33%) |

#### A26 personality statistics (500 games per personality, Normal delta, human 80%, three NPCs of the same personality)

| personality | mean bank size | bust rate per turn |
|---|---|---|
| cautious | 18.7 | 14.5% |
| gambler | 40.7 | 33.1% |
| mirror | 9.6 | 6.8% |

#### C8 follow-up: which NPC accuracy offset hits D18 (Cautious ×3, 500 games per cell)

Board-game deltas (0.30 / 0.20 / 0.10) do NOT carry over: Marathon-rule play is decided by accuracy streaks, so an NPC trailing the human by 0.20 loses about two games in three. D-R8 says to stop and ask when calibration differs, so these are findings, not changes.

| delta | human 100% | human 70% | human 50% |
|---|---|---|---|
| +0.25 | 77.8% | 79.2% | 72.2% |
| +0.20 | 66.4% | 69.4% | 72.2% |
| +0.15 | 56.4% | 56.2% | 61.6% |
| +0.10 | 40.6% | 42.6% | 44.6% |
| +0.05 | 32.2% | 34.2% | 33.2% |
| +0.00 | 32.2% | 25.6% | 27.0% |

Read against D18 (Easy 50–60%, Normal 35–45%, Tough 21–33%): about 0.15 for Easy, 0.10 for Normal and 0.00 for Tough. A perfect human cannot be pushed below about 32% because NPC accuracy is capped at 0.95, which makes the Tough range reachable only just.

### Findings

1. **Cap (C7):** routine play never reaches the 150 cap. No bets: p99 payout 48, max 72. Every bet taken: p99 72, max 75, zero words over 150 before the cap. The Ear ceiling guard is not needed for balance, only as a safety net.
2. **Steal pool (C5):** English has 5 Expert words of length 7, vi 17, sw 0. Steal as written ("an Expert word of length 7") would remap to 9 letters in en.
3. **Stretch (C6):** banks are length-banded, so a harder tier often has no word at the rolled length. English easy→medium has none at lengths 2–4 (0 of 3 covered lengths work), medium→hard 1 of 3, hard→expert 0 of 2. Es, fr, de, pt, pl are fine from medium upward. ar is complete.
4. **Mirror (C8):** against three Mirrors the human wins 89–99% at every delta, far above the other personalities, with a mean bank of 9.6 against Cautious's 18.7. As written ("repeats the human's last Bank/Roll choice") it is the weakest personality by a wide margin. I have not diagnosed why beyond that; the simulation's Mirror may be reading the human's choice differently from how the engine would, so treat this as a flag for review, not a fault found.
