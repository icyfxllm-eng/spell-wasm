# English re-tier: promotion evidence (second look, 2026-10-19)

Written for the CC-IMPOSSIBLE-LAUNCH R4 gate. Everything here is evidence for a decision; nothing in the bank, the flags or a build was changed by it.

## 1. Spot check of the built bank (100 words, graded)

Sheet: `reports/en-retier-spotcheck-sheet.csv` (blank) and `reports/en-retier-spotcheck-graded.csv` (graded copy as received 2026-10-19). Sample: seeded, shuffled, 20 Easy / 25 Medium / 25 Hard / 30 Expert from the built re-tier bank, in the bank's own new/existing mix. Who graded it is not recorded in the file; I have treated it as Eric's.

| Built tier | n | Agreed | Graded lower | Graded higher |
|---|---|---|---|---|
| Easy | 20 | 18 | 0 | 2 |
| Medium | 25 | 15 | 6 | 4 |
| Hard | 25 | 8 | 14 | 3 |
| **Expert** | **30** | **27** | **3** | **0** |
| Total | 100 | 68 | | |

- **New rows** (1,210 words added by the re-tier): 34 of 43 agree. **Existing rows**: 34 of 57 agree.
- **Expert, the tier Impossible uses:** 26 of 30 built-Expert words kept; 3 would drop to Hard (cynicism, yacht, irrepressible) and 1 is a cut (voyeurism). 3 Hard words would rise to Expert (transcendence, rotisserie, ebullient). So about 87% of sampled Expert rows stand, and an estimated few percent of Hard rows belong in Expert (3 of 25 sampled; a small sample, so treat any extrapolation as rough).
- **The weak spot is not Expert.** 20 of the 35 sampled *existing* Medium and Hard rows (the ones the re-tier did not regrade) were judged too high: common regular words such as superior, physical, failure, research, along, named, things. Those tiers still carry the old length-based assignment. Hard agrees only 8 of 25. That does not touch Impossible, but it is what "re-tier promotion" would have to either accept or fix for Normal play.
- 12 of the 30 Expert rows have no difficulty score (all of them Expert rows; the score table has no entry), so those were placed by grading, not by score.
- Graded notes to act on: voyeurism marked **cut**; sibyl "confirm common-noun use"; marquess and brougham "check sound-alike before it stays in Impossible" (see section 3).

## 2. Audio for the new words

- Before: 1,184 of the 1,210 new English words had **no** normal-speed clip in the server cache, and none had a slow one. All existing words were cached.
- Now: every one of the 1,210 has both clips (generated through the live server on 2026-10-19, about a dollar of Google speech). `reports/en-retier-audio-check.csv` lists every word.
- Intelligibility check, whisper-small, one transcript per clip: it heard the spoken word in 929 of 1,210 (76.8%). For the **239 new words in the Impossible pool** it heard 155. Whisper-large-v3 then re-checked the 84 pool words small missed and heard 54 more. **30 pool words are still not heard by either recogniser.**
- Those 30 split into two kinds (a recogniser is a lead, not a verdict):
  - *A real different word or spelling came back*, so the word may not be spellable from sound alone: marquess (heard "Marquis"), sibyl ("Sybil"), comptroller ("Controller"), boatswain ("Boson"), straitjacket ("straight jacket"), saccharin and saccharine (each heard as the other), disburse ("Disperse"), myrrh ("Mer"), cetacean, praetor, piquant, milieu, lascivious.
  - *Garbled or unclear*, so the audio itself may be poor: caisson ("Tyson"), brougham ("Vroom"/"Brone"), pharynx, strychnine, challis, bauxite, dachshund, demesne, discotheque, marchioness, obeisance, opprobrium, oeuvre, sachet, yachtsman, aggrandizement.
  Someone with ears should listen to these 30 before they ship in Impossible.

## 3. A fairness gap in the Impossible gate (needs a decision)

The Impossible exclusion gates compare each word only with **other words in our own bank**. The master invariant (I1) is wider: if two valid spellings could match what the player heard, the word should be out. Words whose twin is not in the bank get through. Checking the 271-word English pool against CMUdict (every word that shares a pronunciation with any listed word) finds 11: anesthesiology/anaesthesiology and archaeological/archeological (both are valid variant spellings), rescission/recision, sibyl/sibil, yacht/yott, myrrh/mer, caisson, challis, demesne, nisei, oeuvre. The recognisers add marquess/marquis, comptroller/controller, boatswain/bosun and straitjacket/straight jacket. None of those twins is a bank word, so none was excluded. The same gap exists for the other launch languages, whose gates were built the same way.

## 4. Where that leaves R4

- Met: word admission (your 200-of-200 ruling, 2026-10-16), the re-tier build and its tests, the Expert tier's spot check (about 87%), and audio now exists for every new word.
- Open: the 30 audio leads and the section 3 decision for English Impossible; and whether the weaker existing Medium and Hard tiers need regrading before en.retier goes to everyone.
- Suggested edits, not applied (they are bank changes outside the launch file's one allowed bank edit): cut voyeurism; move cynicism, yacht and irrepressible to Hard; move transcendence, rotisserie and ebullient to Expert.
