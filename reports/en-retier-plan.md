# English re-tier plan (signed 2026-10-17; no tiers changed yet)

Written 2026-10-17 on `origin/main`. Parent: CC-EN-EXPERT-GROWTH F4 (re-tier), F7 (promotion), F8 (flags).

## Inputs

Graded so far: 1,913 unique words (`calibration/en_graded.jsonl` plus the first staged sheet).

| Group | Expert | Hard | Medium | Easy | cut | unsure |
|---|---|---|---|---|---|---|
| Staged (Zipf 2.5 to 4.0, admitted under F3) | 123 | 470 | 439 | 62 | 2 | 6 |
| Low band (Zipf 2.0 to 2.5, Expert candidates only, D4 amendment) | 122 | 320 | 342 | 15 | 4 | 8 |

Provenance: 113 rows were graded blind by Eric. The rest started as model suggestions and were recorded as Eric's grades at his call. The calibration reports say so.

Current bank: easy 763, medium 798, hard 800, expert 797 (`assets/words/en/<tier>.txt`). The current Expert tier is mostly common long words and inflections (abandoned, addresses, activities...). A random-80 read suggested 100 to 150 genuine Experts in it.

## The plan

### Step 1. Grade the current 797 Expert rows (one more sheet)
Same workbook, columns and shortcuts. Each grade decides where the row goes:
- **Expert:** stays in Expert.
- **Hard / Medium / Easy:** moves to that tier.
- **cut:** quarantined (never deleted, I9).
- **unsure:** stays in Expert for now, listed for later.

Why grade rather than demote by rule: the score is about 14% precise on its shortlist, so a rule would demote real Experts. A cheap pre-flag is possible (plurals and -ed/-ing forms whose base word is in the bank, about a third of the 797 by the earlier count), but those still go through the sheet, pre-marked as a suggestion.

### Step 2. Add the graded new words
- **Expert:** all 245 graded Experts (123 staged + 122 low band).
- **Hard / Medium / Easy:** the staged words graded at those tiers (470 / 439 / 62).
- **Low-band words graded below Expert (677): left out.** The D4 amendment admits Zipf 2.0 to 2.5 words as Expert candidates only.
- **Cut and unsure staged words:** stay staged.

### Step 3. Build it behind `en.retier` (F4, default OFF)
- **Where the new tiers live:** they're generated into a separate set of tier files. With the flag off, English plays exactly as today (I8). With it on, the new tiers serve. This is the first app-code change in this project: the tier lookup reads the flag.
- **PURITY:** every new row goes through `purity_admit` and gets a ledger line before it can ship.
- **Definitions (D3):** new rows play on spelling surfaces only until their definitions are audited. Definition Match and definition panels must not see them.

## What the final Expert tier holds

| Source | Words |
|---|---|
| Graded Experts (new) | 245 |
| Experts kept from the current 797 | about 100 to 150 (step 1 decides) |
| **Total** | **about 345 to 395** |

That is far below D1's 3,000. It is the honest yield, and the spec says to report it rather than pad.

Other tiers grow: Hard to about 1,300 plus whatever the 797 sheet demotes, Medium to about 1,250 plus demotions, Easy to about 825.

## Blockers and decisions for you

1. **English is frozen.** `config/bank_floors.json` marks English `frozen: true` (CC-BANK-COMPLETE D5). Growing the bank needs that flipped. Your call.
2. **Tier ceilings.** The word-list builder caps each tier at the larger of 840 and 1.25 times its pool floor. For English, Medium's cap is 1,250 and Easy's is 840. Medium will pass 1,250 once the 797 sheet demotes words into it, and Easy lands near the 840 cap. The English pool floors need raising (signed data), or some graded words wait.
3. **Spell Jr and Kid Mode (I5).** New Easy and Medium rows would flow into Jr pools, because Jr draws Easy and Medium filtered by the kid exclusion list. I5 says Jr and Kid sets stay byte-identical. So every new row must be kept out of Jr and Kid, for example with a frozen Jr word list. Eight new Easy/Medium words also carry a JR? flag: teat, arouse, miscarriage, dysfunction, heterosexuality, castration, raunchy, knickers.
4. **Audio.** About 1,300 new words have no warmed TTS clips. My notes say Google TTS billing was off on 2026-09-30. If it still is, new words have no audio. Check before the flag flips.
5. **Daily re-deal.** English Daily seeds include a hash of each tier pool, so changing the tiers re-deals English Daily and resets its no-repeat history. That's expected, just know it happens.
6. **Server side.** `backend/def_pools/en.json` must be rebuilt for the new tiers and deployed by the definition-pools-only path (rsync, no restart).
7. **Sound-alike table.** It is regenerated over the new bank before the flag flips (H5). Several new words are flagged homophones (feint, knave, sleigh, sachet, bettor, ciao...).

## Eric's decisions (2026-10-17)
- **Step 1:** grade the current 797 Expert rows on one more sheet.
- **Freeze:** unfreeze English (`frozen: false` for en in `config/bank_floors.json`).
- **Caps:** raise the English pool floors so the caps fit (proposed Easy 800, Medium 1,600, Hard 2,000, Expert 3,000; exact values set when the tier files are built).
- **Jr / Kid:** freeze today's Jr and Kid word sets, so new rows can never reach them.

## Order once you sign off
1. You decide items 1 to 3; I build the 797 sheet.
2. Ingest it, generate the new tier files, PURITY admission, Jr freeze list, regenerate the sound-alike table.
3. App code for `en.retier` (flag off), tests A8 to A11, full gate.
4. Audio warm and def-pool deploy, then the TestFlight flag flip.

## Built 2026-10-17 (steps 2 and 3)

- **Re-tier bank** (`assets/words/en/retier/`, generated by `tools/en_growth/build_retier.py`): Easy 825, Medium 1,753, Hard 1,310, Expert 279. 201 current Expert rows quarantined (cut by the re-grade). 6 graded words over the 16-letter cap held out (`held.txt`).
- **PURITY:** all 1,210 new rows pass the classifier and are recorded in `assets/words/purity/en-retier.tsv`.
- **Config:** English unfrozen; Medium pool floor raised from 1,000 to 1,500 (cap 1,875).
- **App:** `en.retier` flag, default OFF. `words::en_tier` serves the re-tier only with the flag on and only for a standard profile. Spell Jr and Kid Mode (Kid Mode preference or age-gate "kid") keep today's tiers (I5).
- **Tests:** A8 passes (median score Easy 0.305 < Medium 0.394 < Hard 0.436 < Expert 0.694). A9, A10, A11 and the partition test pass. Full gate: GATE PASS, e2e 422/422 app and 17/17 site.
- **Sound-alike table: flag-aware (2026-10-17).** English now has two tables. `soundalikes.json` covers the current bank (263 sets) and is what everyone reads with the flag off, so Spelluzzle and grading are unchanged. `soundalikes-retier.json` covers the current bank plus the re-tier rows (282 sets) and is read only while `words::retier_active()` (flag on, not Spell Jr / Kid Mode), the same switch the bank uses. A first attempt at a single combined table changed flag-off play (9 Spelluzzle tests failed), which is why there are two.
- **Flags are local, not remote.** `src/flags.rs` reads localStorage; only telemetry has a server-driven flag. F8's "remote flags, no resubmission" does not hold yet: turning `en.retier` on for players needs either a build with the default flipped or a remote-flag mechanism.

## Amendment 2026-10-19: the 800 old Hard rows go to Medium (Eric)

After reading a random 100 of the 1,598 current Medium and Hard rows the re-tier had left untouched, Eric ruled that they all look like Medium words. The old Hard tier was a letter-count tier (7-8 letters). Rule now in `tools/en_growth/build_retier.py`: current Hard rows are placed in Medium; current Easy and Medium rows are unchanged. Result: Easy 825, Medium 2,553, Hard 510, Expert 278. To fit, the English Medium pool floor in `config/bank_floors.json` rises from 1,500 to 2,050 (cap 2,562). English is not launched, so none of these floors is a red build yet; the Hard tier (510) is far under its 2,000 floor and the Expert tier under its 3,000 floor, which is a known state, not a regression. The model pre-grade that proposed a three-way split is kept in `reports/en-regrade-proposal.csv` as a record and was not used.

## Amendment 2026-10-19 (later): Hard rebuilt from a graded sheet (Eric)

Eric graded `reports/en-hard-candidates-sheet.csv` (250 top-scoring untouched rows plus 30 blind controls; graded copy `reports/en-hard-candidates-graded.tsv`). 86 words were marked hard and one (exist) easy; they are recorded as signed per-word overrides in `calibration/en_tier_overrides.csv`, which `tools/en_growth/build_retier.py` applies last. The controls show how far the difficulty score is a guide: 82 of the 249 shortlist words (33%) went to Hard against 3 of the 30 controls (10%). Result: Easy 826, Medium 2,466, Hard 596, Expert 278. The notes on the sheet also say to accept British spellings for judgment, neighbor, jewelry, license, fulfill and behavior; that is a grading change for Normal play, not a tier change, and is not part of this commit.
