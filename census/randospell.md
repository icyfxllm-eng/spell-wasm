

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
