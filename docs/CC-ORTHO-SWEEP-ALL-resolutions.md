# CC-ORTHO-SWEEP-ALL v1: section 0 resolutions (Eric, 2026-11-05)

Intent: unblock Phase 1 without shrinking banks on a noisy heuristic, and without shipping any data whose licence obliges Eric beyond attribution. Solo dev; zero-cost, permissive-or-self-authored sources only.

Also decided earlier today: the es voice stays es-ES-Neural2-G (Spain). es_scz is an accent rule for it.

## Decisions

- D-LIC (new). Shipped data (anything under assets/) may derive only from CC BY, MIT, Apache-2.0, BSD, the Unicode licence, official rule texts with pairs authored by hand, or the auditors' work. Banned from shipped data: BY-SA, GPL/LGPL, any NC licence, unknown. Build-time tools may use any permissive licence; their output is not shipped code. Unknown licence does not ship. This supersedes the open C6 items.
- C6. Leipzig downloads are CC BY per wortschatz.uni-leipzig.de/en/usage (the web portal and apps are CC BY-NC; the project used downloads). LICENSES.md goes from "as recorded" to "confirmed 2026-11-05". pypinyin (MIT) and OpenCC (Apache-2.0): build-time only, allowed. Lexique 3, OpenCorpora, CC-CEDICT (for variants) and copyleft Hunspell: manual cross-check only, never a source of rows. Read data/<lang>/SOURCES.md for es, fil, ko, vi, zh and flag any source that fails D-LIC.
- S2. Do not quarantine on the C4 heuristic. Tag flagged rows F-<lang>-cit in the purity ledger with class inflected_candidate. Quarantine only rows an auditor confirms. I7 becomes "no auditor-confirmed inflected form in any bank". Bank floors stay.
- S3. fold_yo is one-way: typed е matches stored ё; typed ё does not match stored е. Same-word duplicates (ещё/еще, даёт/дает, ведёт/ведет, others in c3-collisions-ru.tsv): keep the ё row, quarantine the е row as class yo_duplicate. Different-word pairs (все/всё, всем/всём, вел/вёл): keep both. I1 for ru is measured under the one-way fold. The report lists each of the 28 pairs with its verdict.
- S5. sw: add an apostrophe key to assets/keyboards/sw.json. The build filter keeps U+0027 and U+2019 for sw only, normalized to U+0027. Add ng' rows from the Leipzig source if present. Rewrite the F2 note premise in src/norm.rs so it is true (sw spends ' as a letter after this change). Check fold_apostrophes does not break ng' grading. fr: non-goal this sweep; no elision or hyphen compounds.
- I1 carve-out, zh. Under pinyin modality, rows with the same toned-pinyin key and different hanzi are homophones, exempt from I1. Groups with identical hanzi still count: quarantine one row from each.
- D7, zh monosyllables. Drop a monosyllable unless its toned key is unique. Report the count before dropping. If more than 500 rows would go, stop and ask.

## Cleanups (Phase 1)

- fr: one row per word for événement/évènement. Keep the 1990 form; the other becomes an accepted variant.
- vi: quarantine the đựơc typo row.
- vi hí/hý, fil lalake/lalaki, hi 7 nukta pairs: one row per pair (KWF form for fil, nukta form for hi); the other becomes an accepted variant.
- pt: the 3 pre-AO90 / PT-PT forms become accepted variants; the stored form becomes BR AO90.

## C8
Eric checks the Cloudflare dashboard. Not blocking, except fil sign-off stays open until Paul's email shows a yes.

## Non-goals
Scoring logic, tier assignments, voice selection, keyboards other than sw. No network licence lookups. No new dictionaries.

## Done when
1. tools/ortho_census.py re-run reports I1 = 0 for every language under these rules.
2. grep -ri "unknown\|as recorded" assets/words/LICENSES.md returns nothing.
3. Every row added to an accepted-variants.tsv cites a rule in a source column.
4. cargo test passes, with new tests: ru one-way fold (все accepted for всё, всё rejected for все); sw ng'ombe correct with both ' and U+2019.
5. The report lists every quarantined row with its class.

Stop and ask if a step would quarantine more than 2% of a language's rows, change a signed file other than norm.rs's F2 comment, or contradicts anything here.
