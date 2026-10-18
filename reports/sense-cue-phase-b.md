# CC-SENSE-CUE v1.1 Phase B: en, es, ru (2026-10-17)

## TTS check

Synthesis works: two never-cached English words came back fresh from the local server (HTTP 200, two new clips in the v4 cache). Google TTS billing is on.

## Merger rules and kinds (F4)

Registry: `config/soundalike-merger-rules.json`; transforms in `tools/build_collisions.py`.

- **English (en-US voice):** cot-caught (not before R), pin-pen, mary-marry-merry are active. Only pin-pen finds pairs in the bank: *en/in*, *enter/inner*, *sense/since* (plus *center/sinner* in the re-tier table). These are `kind: accent`, so grading and Spelluzzle are unchanged.
- **Spanish (es-ES voice):** seseo is registered but **pending**: it is computed below for your review and not applied, because moving those sets to accent removes them from accept-all grading (S3 note).
- **Russian:** no merger rules. The stress-dependent ones need the stress data that is still dark.

## Spanish seseo: what would change (needs your call)

Under seseo-as-accent these sets **leave accept-all grading** (a player typing one for the other would be marked wrong):

- casa / caza
- cauce / cause
- cocer / coser
- concejo / consejo

These stay accept-all: bienes / vienes, botar / votar, hora / ora. (The four *ch* pairs would also stay under seseo alone, but they are a separate bug; see below.)

## Bug found: four Spanish sets are not sound-alikes at all

`key_es` removes every *h* as silent, including the *h* of *ch*, so *ch* collapses into *c*. That makes these live accept-all pairs, although *ch* (/tʃ/) and *c* (/k/) sound different in every accent:

- marca / marcha
- marcas / marchas
- roca / rocha
- chota / cota

Today a player typing *marcha* for *marca* is marked right. The fix is one line in `key_es` (protect *ch* before removing *h*). It makes grading stricter for those four pairs, so it waits for your go-ahead, like seseo.

## Voice loopback (F5, flag-only)

Method: each member synthesized by the Layer-1 voice through the local `/api/speak`, compared pairwise with DTW over log band energies (the Spell-O-Phones method), calibrated per run on same-sound and different-word controls. Flags never change a kind (I9); flagged sets go to the auditor sheet.

| Language | True sets | Flagged | Same-sound floor | Different ceiling | Bar |
|---|---|---|---|---|---|
| en | 283 | 56 | 0.5369 | 1.0172 | 0.6329 |
| es | 11 | 3 | 0.5925 | 0.8908 | 0.6522 |
| ru | 34 | 7 | 0.6211 | 0.7632 | 0.6495 |

English counts include the re-tier table's extra sets.

**What the loopback can and cannot see:**
- It caught real problems, such as English's chained set *air / are / er / ere / err / eyre / heir / hour / oar / or / ore / our / r / ur* (worst pair 1.23), which grading currently treats as one sound, and *years/yours*, *root/route/rout*, *use/youse*.
- It is coarse. It did **not** flag *marca/marcha* (0.62) or *casa/caza* (0.64), which a Castilian listener tells apart. Russian's floor and ceiling sit close together (0.62 vs 0.76), so its flags are weak evidence. That is why it only flags: the auditor decides.

## Status

en, es and ru now read `verified` (F2: the generator ran and the loopback measured every true set for the current voice). Under the Impossible v1.1 pointer that makes them eligible once the other gates pass. Two caveats: Spanish still carries the four *ch* false sets and the pending seseo decision, and English carries the chained *air/hour/or* set until the auditor (or a generator fix) splits it.

Flagged sets by language:

**en:** air / ar / are / er / ere / err ... (1.2313); corp / corporation (1.1615); cache / cachet / cash (1.1067); me / mi (1.049); tae / tai / tie / tye (1.0198); lai / lie / ly / lye (0.9614); testes / tests (0.9454); bae / buy / by / bye (0.9162); fee / fi / fie (0.9106); read / red / reed (0.905); di / die / dye (0.9041); c / sea / see / si / sie (0.9034); root / rout / route (0.9025); fort / forte (0.8877); use / youse (0.8649); hours / ours (0.8427); saint / st / street (0.8405); loop / loup / lupe (0.8402); years / yours (0.8072); lead / led (0.8033); dae / day / de / dey (0.8033); an / ann / un (0.7909); koto / kowtow (0.7773); mean / mein (0.7706); dais / days / daze (0.7671); code / coed (0.7619); gist / just (0.7582); refer / rougher (0.7534); liqueur / liquor (0.7486); cars / cause / coz (0.7431); beau / bo / bough / bow (0.7296); sail / sale / salle (0.7195); affect / effect (0.7035); fir / for / fore / four / fur (0.703); acts / ax / axe (0.7028); asked / ast (0.7022); sol / sole / soul (0.6866); device / devise (0.6831); acknowledgement / acknowledgment (0.6805); licence / license (0.6697); reading / redding (0.6684); right / rite / wright / write (0.6602); t / te / tea / tee / ti (0.6592); la / law (0.657); yore / your (0.6562); can / kan (0.6514); get / git (0.6497); plait / plate (0.6497); role / roll (0.6494); mor / more (0.6475); ensure / insure (0.6462); ware / wear / where (0.6401); road / rode / rowed (0.6377); manner / manor (0.6358); lat / lot (0.6343); we / wee (0.633)

**es:** cauce / cause (1.0969); chota / cota (0.7408); roca / rocha (0.6563)

**ru:** экз / экс (1.1486); об / оп (0.8769); перед / перёд (0.7951); узнает / узнаёт (0.7299); лед / лет / лёд (0.7255); всем / всём (0.6923); чем / чём (0.658)


## Gate

Full gate: GATE PASS (e2e 422/422 app, 17/17 site) on the third run. The two earlier runs each failed one timing-sensitive e2e test unrelated to these tables (a Board Game tap-mid-animation position check, a SpellDoku 4-second wait, and the known flaky human-audio test). Run on its own, the Board Game area passed 2 of 3 times with this change and 2 of 2 on baseline. Grading's sets are unchanged (en 263, es 11, ru 34 true sets).

## Eric's decisions (2026-10-17)

- **Seseo:** keep accepting. Seseo stays in the Spanish base key, so casa/caza, cauce/cause, cocer/coser and concejo/consejo remain `kind: true` (accept-all). Recorded as declined in `config/soundalike-merger-rules.json`. Impossible still excludes them.
- **ch bug:** fixed in `key_es`. Spanish drops from 11 to 7 sets; marca/marcha, marcas/marchas, roca/rocha and chota/cota are no longer accepted for each other. The Spanish auditor sheet was rebuilt (`es-soundalike-20261017`).
- Full gate after both changes: GATE PASS on the second run (the first failed only the known flaky human-audio test).

## Same-sound sheets ingested (2026-10-17)

Flagged-only sheets (`*-soundalike-flagged-20261017`), all four checks passed:
- **English:** 19 yes, 25 no, 11 unsure, plus 19 member overrides on 14 sets: 16 members dropped (letters, abbreviations, slang, foreign words, and *for* out of *fir / fur*) and 3 one-way sets (*bough* ← *bow*, *red* ← *read*, *led* ← *lead*: the voice says a lone *bow*, *read*, *lead* as "boh", "reed", "leed").
- **Russian:** 3 yes, 4 no, plus *лет* dropped from *лед / лёд*.
- **Spanish:** *cauce / cause* recorded unsure (seseo pairs stay accepted, as Eric ruled).
- **licence / license:** the "no" (one word, two spellings) is recorded, but the pair stays accepted because it is a curated variety pair (`reviewFlag: auditor-no-kept-by-variety-seed`).
- **Grading effect:** "no" sets move to accent and stop being accepted (for example *asked / ast*, *device / devise*, *years / yours*, *are / or / err*).
- **Still open:** *экз* is an abbreviation in the Russian bank and should be quarantined under CC-BANK-PURITY; not done here.
- Spelluzzle seeds and its golden board file were re-pinned. Full gate: GATE PASS on the second run (the first failed two random-board e2e checks; the Spelluzzle area passed on its own twice).

## экз quarantined (2026-10-17)

Through the CC-BANK-PURITY path: a signed override (`tools/bank/purity_overrides.csv`, class `abbreviation_or_noise`), the row moved from `assets/words/ru/hard.txt` to `assets/words-quarantine/ru/hard.txt` with a manifest line, `src/word_data.rs` rebuilt, and the Russian ledger and its Hard tier hash updated. `purity_admit` now rejects экз. The Russian sound-alike table drops the экз/экс set (33 sets). Two digest pins that hash the banks were re-pinned with the reason beside them: Spell Search's 500-seed grid golden (`src/wordsearch/tests.rs`) and its e2e copy.

Gate: PURITY gate and selftest pass; the full cargo suite passes. Four full-gate runs each failed exactly one e2e test, a different known-flaky one each time (SpellDoku 4-second timeout, Board Game tap position, human-audio fallback); each passed in other runs or on its own.
