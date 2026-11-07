# CC-ORTHO-FIX-FIL-ROMANCE v1: section 0 census (read-only, 2026-11-06)

Nothing was changed except this file and the lists in reports/ortho-fix-census/. Phases A-C wait for review. Branch census/ortho-all, head 45538193.

## Read this first: where the spec meets what is already committed

1. **D1 reverses an earlier decision.** On 2026-11-05 Eric said "Keep Spain voice" (es-ES-Neural2-G). D1 now says Latin American (es-419). The repo cannot say whether an es-419 voice exists: there is no voice catalogue in it and I made no network calls. backend/app.py maps es to ("es-ES", "es-ES-Neural2-G") and has no Latin American entry. D1's own rule is "if no es-419 voice exists, stop and ask": I cannot confirm one, so this is a stop-and-ask. Consequences if the voice changes: (a) **es is not in VOICE_IN_KEY** (backend/app.py:245 is {en, de, ru, ko, fa}), so the audio cache key does not include the voice and the warmed Spain clips (12,198 per the comment at line 73) would keep being served until es is added to the key and re-warmed; (b) per my notes Google billing has been off since 2026-09-30, so a re-warm cannot run; (c) the es sound-alike table is "verified" **for es-ES-Neural2-G** (assets/words/es/soundalikes.json, loopback measured 2026-10-17); a new voice makes it unverified until the loopback is re-run, and F7 needs it verified.
2. **My earlier premise about casa/caza was not measured.** I treated the Spain voice as distinguishing c/z from s. The loopback measured casa/caza at 0.6441 and cocer/coser at 0.5947 against a bar of 0.6522 (same-sound floor 0.5925): both under the bar, i.e. the current voice sounds them alike by that measure. It is weak evidence (S2: isolated-word acoustics are not proof) but it points the other way from my assumption. The table already lists both as true sets, so D1's "casa/caza go in the es table" is already the state of the table.
3. **F1 is partly done.** Committed (45538193, reviewer sheet): istasyon, istilo, istruktura, iskultura, kemikal, kompanya, kombinasyon, kompetisyon, tradisyonal, inhinyeriya, legal, politiko merged; kuponan, espirito, kabuoan, dae, dai, babaeng, babaing, lalakeng quarantined. **Not done:** komplikado/kumplikado and politika/pulitika (both rows still in the bank, held as "Paul decides"), sunod-sunod/sunud-sunod (both rows in the bank). F1 now decides komplikado and politika (left form stored), which resolves the hold.
4. **sunod-sunod "close at Medium+" needs a mechanism that does not exist.** Variants (src/accepted.rs) have no tier column, and Filipino has no orthography profile. "Close" is wired only for German (see C2). It needs either a tier column on variants or a fil profile, plus fil close text.
5. **F3/D3/D7 supersede the Jr rule my profile code implements** (D14: Jr treats n-tilde and c-cedilla as marks). src/ortho_profile.rs returns Correct in Jr for ano/caca whenever the typed form is not a row; it does change under F3 (a real word is a miss in Jr). The real-word test also differs: the spec says "bank or dictionary word"; my `is_row` checks the bank only. Not changed yet.
6. **Most of the done-when example words are not bank rows** (checked): es ano, te, esta, está, caza, arbol, si/sí, tu/tú, el/él, mas/más, mi/mí, se/sé, de/dé are not rows (año, té, casa, árbol are); fr ou, où, a, à, sur, sûr are not rows (là, pêche, péché, été, cœur, élève are); pt avó, avô, caça, caca, facto are not rows (não, você, fato are). So "caza<->casa correct both ways" has no caza row, and "esta (target está) -> miss" has no está target. Those golden rows need synthetic fixtures or other real pairs (C6); adding rows is not allowed by the spec.

## C1. Filipino pairs

Row ids are bank tier plus line number (the bank has no other id). Tiers and lines are as of head 45538193.

| Spec pair (store <- accept) | Stored row | Other form |
|---|---|---|
| istasyon <- estasyon | hard, line 138 | quarantined (hard) |
| istilo <- estilo | medium, 322 | quarantined (medium) |
| istruktura <- estruktura | expert, 619 | quarantined (expert) |
| iskultura <- eskultura | expert, 1888 | quarantined (expert) |
| kemikal <- kimikal | hard, 478 | quarantined (hard) |
| kompanya <- kumpanya | hard, 122 | quarantined (hard) |
| kombinasyon <- kumbinasyon | expert, 1486 | quarantined (expert) |
| kompetisyon <- kumpetisyon | expert, 734 | quarantined (expert) |
| komplikado <- kumplikado | expert, 1729 | **still a row**, expert, 1791 |
| politika <- pulitika | hard, 328 | **still a row**, hard, 961 |
| politiko <- pulitiko | hard, 353 | quarantined (hard) |
| tradisyonal <- tradisyunal | expert, 216 | quarantined (expert) |
| inhinyeriya <- inhenyeriya | expert, 1635 | quarantined (expert) |
| legal <- ligal | medium, 263 | quarantined (medium) |
| sunod-sunod <- sunud-sunod | expert, 1531 | **still a row**, expert, 1716 |

Quarantine list: kuponan, espirito, kabuoan (misspelled_nonstandard), dae, dai (wrong_language_other), babaeng, babaing, lalakeng (not_a_dictionary_headword): all already in assets/words-quarantine/fil with manifest rows. Roots exist: babae (easy, 16), lalaki (easy, 132). Kept targets of the earlier quarantines: espiritu (hard, 573), koponan (hard, 141), kabuuan (hard, 159).

**Modes that serve a Filipino bank row** (every caller of words::tier_for in src/): Standard, The Climb, Daily and Versus (game.rs, daily.rs), Bee, Board Game, Word Chains, Letter Forge, Impostor, Racing, Spelldoku word mode, Practice and the learner, Quick Bee Draw (language-gated), photo matching, Definition Match. I did not verify which of these are enabled for fil; chains, forge and impostor have fil in their language tables.

**My Words and miss history.** These live on the player's device (Misses under localStorage key byear_misses_v1, a MissEntry of word, lang, tier, misses, due). The repo holds no player data, so I cannot say whether any real player has either spelling in their history; only telemetry could, and I did not look. **What I can say from the code:** src/misses.rs never consults the bank, so a miss recorded for a form that is now quarantined stays in the player's list and is served as its own target. The variant table is one-way (stored -> accepted), so for a stale entry such as estasyon, typing the new stored form istasyon would not be accepted. I read this in the code and did not run it; it is worth a test before Phase A ships, and a migration (rewrite old entries to the stored form) would close it.

**Other root + linker rows (the spec's "any other row that is only root + linker").** The candidate list is reports/ortho-fix-census/c1-fil-linker-candidates.tsv. My heuristic finds **220** rows ending in -ng whose root is also a row (taong, bagong, limang, batang, malaking, lalaking...) and **360** more ending in a vowel + -ng whose root is not a row (bubong, ilong, sining, tulong, bawang, noong, bilang, upang...). The second group is mostly real words, not linker forms, and some of the first are real standalone words too (matang, batang, perang, asong), so I cannot separate them without a native reader. For scale: the Filipino bank is 4,014 rows, so 220 is 5.5 percent. The standing rule from the earlier sweep was to stop and ask before quarantining over 2 percent of a language, and this spec does not repeat it. **Stop-and-ask: I will not quarantine these without your say-so on scope.**

**Pairs not on your list** (the ">50 more pairs" stop): 7 remain with an e/i or o/u difference: bola/bula, noon/nuon, noong/nuong, pares/paris, plus the three above that your list covers. Well under 50, so no stop; I scanned only e/i and o/u, not other variation (for example d/r).

## C2. es/fr/pt grader routing, golden rows, close text

- **Routing.** src/game.rs (about line 2310): if flags::ortho_profile(lang) is on and a profile exists, the profile's verdict decides; otherwise norm::answer_matches_in, the old grader. All three flags default off, so today es, fr and pt use the old grader, with accepted variants already live through norm::same_answer.
- **Golden rows today** (tests/ortho/golden): es 244, fr 338, pt 325 rows; every lane at least 30. They encode the current spec (D14: Jr treats n-tilde and c-cedilla as marks) and will change under F3.
- **Close text.** The strings exist in every UI locale (de.close.letters, de.close.split in en, es, fr, pt, de), because the UI language is the player's. But the code path is German-only: game.rs de_close_retry returns None unless the spelled language is de, and the near-miss detector is norm::de_near_miss. Nothing produces a close for es, fr or pt, and the key names are German-specific. "Reuse the German F5 letter diff" is feasible: the diff and strings exist; the routing and key names need generalising. A close is also not available when a retry was used or in Versus (the existing guard).

## C3. Spanish voices

The repo does not list available voices (no catalogue, no voices.list result, no network calls made). Wired today: es-ES / es-ES-Neural2-G (Google); fr-FR-Neural2-F; pt-BR-Neural2-B; Azure is used only for sw and fa. From general knowledge, not verified here: Google's Latin American Spanish voices are under es-US (and es-MX for some families); I do not believe "es-419" is a Google locale code. Please confirm the exact voice before anything is changed. See item 1 above for the cost of switching.

## C4. Sound-alike table status

| Lang | Status | Detail |
|---|---|---|
| es | **verified** (voice es-ES-Neural2-G, loopback 2026-10-17) | 7 true sets: bienes/vienes, botar/votar, casa/caza, cauce/cause, cocer/coser, concejo/consejo, hora/ora. cauce/cause is flagged voice-may-distinguish (0.109 over the bar). The tilde-only same-sound pairs (te/té...) are not in the table. |
| fr | **missing** | no generator |
| pt | **missing** | no generator |

F7 needs a verified table, so only es can ever meet it; fr and pt need a generator first (a SENSE-CUE job).

## C5. Keyboard reach

Spell keyboard (src/keyboard.rs), checked against every letter in each bank: **every bank letter is reachable in one tap or a long-press** for all three, so nothing is flagged for F6 on the Spell keyboard. Long-press-only letters (two actions): es o-acute 559, i-acute 287, a-acute 172, e-acute 101, u-acute 47, u-diaeresis 9 (n-tilde is a direct key). fr: e-acute 1,798, e-grave 187, e-circumflex 45, i-circumflex 28, c-cedilla 28, o-circumflex 24, a-circumflex 20, oe 12, u-circumflex 8, a-grave 3, i-diaeresis 2. pt: a-tilde 356, i-acute 189, a-acute 183, o-tilde 123, e-circumflex 107, e-acute 101, o-acute 95, u-acute 49, a-circumflex 41, o-circumflex 26 (c-cedilla is a direct key). **Not checked: the default device keyboard.** That depends on the player's OS and keyboard language and cannot be read from the repo. I did not test whether a long-press counts as the "2 taps" the spec means, and the Show accents as keys option (K-R11) would make the accent keys direct.

## C6. Accent-only minimal pairs

Groups that differ only by a mark, n-tilde, c-cedilla or oe/ae, listed in reports/ortho-fix-census/c6-{es,fr,pt}.tsv: **es 46 groups (93 rows), fr 131 (262), pt 19 (38).** Sound split: the es table lists none of them, fr and pt have no table, so "same-sound" cannot be established from the tables for any group; every group is unclassified. Examples: es trabajo/trabajo-acute, donde/donde-acute, musica/musica-acute; fr classe/classe-acute, ferme/ferme-acute, marche/marche-acute; pt porque/porque-circumflex, medico/medico-acute. Two pt groups are pre-AO90 leftovers (estreia/estreia-acute, assembleia/assembleia-acute; the acute forms are 'pending' in the ledger): a pt AO90 cleanup I missed.

## Stop-and-asks

1. D1 / es-419: confirm the voice exists, and decide on the cache key, re-warm (billing is off) and re-verification cost.
2. Scope of the -ng linker rows (220 to 580 candidates) against the 5.5 percent of the bank.
3. How to give sunod-sunod its Easy/Jr-only rule (variants have no tier column; fil has no profile or close text).
4. Which real pairs to use for the done-when rows, since most example words are not bank rows.
5. The stale Misses/My Words entries for quarantined forms.

## Resolutions

- **D1 (Eric, 2026-11-06): keep the Spain voice (es-ES-Neural2-G).** D1's es-419 default is withdrawn. No cache-key change, no re-warm, the es sound-alike table stays verified for the voice. casa/caza and cocer/coser stay in the table as they are (the loopback measured both as alike on this voice). Still open: stop-and-asks 2 to 5 above.

## Amended resolutions applied (Eric, 2026-11-06)

1. **-ng linker rows:** the 220 rows whose root is also a row are tagged `linker_candidate` in assets/words/purity/linker-candidates/fil.tsv (tags only, none cut). babaeng, babaing and lalakeng stay cut. bank_floors.json untouched, nothing padded. **Filipino shortfall against the pool floors (not enforced while fil is unlaunched):** Easy 247 of 300 (-53), Medium 597 of 600 (-3), Hard 1,191 of 1,200 (-9), Expert 1,978 of 2,000 (-22). This blocks Filipino's launch, not this file.
2. **sunud-sunod** quarantined as misspelled_nonstandard (tools/ortho/sweep_all_batch5.py); a miss for now. Add it to a close list when Phase B generalizes close feedback. The "Easy and Jr only" clause is struck.
3. **Golden rows** added from real bank pairs, graded by the profile function against the real bank lookup: es donde/dónde and quien/quién both ways are `pending_table` (the spec answer is correct as valid_other_sense; the es sound-alike table holds no tilde-only pairs yet, so today the profile says miss and the table is not asked); fr classe/classé both ways are `miss`; pt porque/porquê both ways are `pending_table` and tagged for the Gig B "Same sound in your accent?" column (data/{es,pt}/sound-alike-rules.json gigb_pairs, reports/ortho-gigb-*.csv). No bank row added.
4. **Saved words** (src/saved_words.rs, data/saved-word-map.tsv from tools/ortho/build_saved_word_map.py, gated by --check):
   - a. Misses, My Words lists and the older flat set are rewritten on load from the map, merging when the stored form already exists (counts summed, earlier due kept, stars and earliest added time kept); idempotent; every rewrite is logged to the device (key byear_ortho_rewrite_log_v1: list, lang, old, new, merged) so it can be reversed. The sync pull applies the same rewrite before storing.
   - b. 5 quarantined fil rows have no twin (dae, dai, babaeng, babaing, lalakeng): hidden from the due queue (misses::due_misses_at), never deleted.
   - c. A served saved word is graded through its current stored form (game.rs: resolve), so that row's accepted spellings apply.
   - d. Scope is the map's languages: fil, es, fr, pt, vi, hi, ru (+1 zh row). 61 rows (22 fil, 22 ru, 7 hi, 4 pt, 2 es, 2 vi, 1 fr, 1 zh). Cover: Misses and both My Words stores. **Server sync does exist** (backend/accounts.py sync_records, kinds list and miss). I did not edit any server data: the client repairs pulled records before storing them, and the existing tombstone step then retires the old records. A device that never updates would keep pushing old spellings until it does.
   - The map generator exits non-zero if an old form maps to two stored forms or is a current row. Neither happened.

## Findings from this work

- **zh quarantines from the first sweep never reached the app.** The app serves zh from the ZH_* consts in src/words.rs (scripts/build-zh-bank.py), which I did not regenerate. hu2li2|狐狸 is still served; the two banks already differ elsewhere (for example 牠 against 它). The map leaves it out ("NOT EFFECTIVE"). Fixing it means regenerating words.rs, which changes the served zh bank beyond this file: needs your call.
- The "e/i, o/u" merges for the 10 pairs rely on the old spelling being accepted as a variant; a saved miss in the old spelling now rewrites to the stored one.

## Verification

cargo test: 1,675 lib tests pass, plus the golden files (es 252, fr 342, pt 329 rows). Quick checks pass: collision tables, sound-alike report, Impossible pools, purity gate, map check, profile check, provenance gate. The wasm app build succeeds; e2e specs mywords-lists, wordpic-vi-tone, review-queue, accounts and gameplay pass (33/33). Not run: the full gate, and no end-to-end test of the rewrite inside a browser with real saved data.

## zh bank fixed (2026-11-06, Eric "Fix zh bank first")

**Cause.** zh is the one language whose served bank is not generated by build-wordlists.py: the app reads the ZH_* consts in src/words.rs, which scripts/build-zh-bank.py emits from assets/words/zh. The BANK-PURITY sweep (3e54e8d6) and my first sweep edited the assets and never re-emitted words.rs, so the app still served four stale strings and one row my sweep had quarantined.

**Fix.** tools/ortho/sync_zh_words_rs.py re-emits the four consts from the assets (step 5 of the build script only; no corpus step, no network, no new rows). The diff is exactly six lines: removed you3zhu4|有著 and hu2li2|狐狸; ta1|牠 became ta1|它 and jie1zhu4|接著 became jie1zhu4|接着. src/zh_sandhi.rs got the same four row changes by hand (its generator, tools/build-zh-sandhi.py, would delete the test module at the bottom of the file; I did not run it for that reason, and the hand edit matches its output: 6,180 rows, 5,886 none). The gate now runs sync_zh_words_rs.py --check so the two banks cannot drift again.

**Saved words.** The map now covers zh properly: hu2li2|狐狸 and you3zhu4|有着 (my sweep's two), plus the three forms the app served until now (you3zhu4|有著, ta1|牠, jie1zhu4|接著). 65 rows in total.

**Checked.** cargo test 1,675 pass; zh-bank-sandhi-check, zh-grading-path-check, zh-audio-path-check, pinyin-inventory-check and pinyin-display-check pass; wasm app build passes; 71 e2e specs pass (gameplay, modes, impossible, quickbee-draw, keyboard, script-render, spell-search, review-queue, mywords-lists). Not checked: the zh audio cache and human-audio clips are keyed by reading, not by the hanzi, so I expect no change, but I did not look at any cached file.

## Phase C (F6, F7), 2026-11-06

- **F6 keyboard reach:** nothing to add. The existing `every_word_char_is_typeable` test pins that every character of every es/fr/pt bank word is reachable on the Spell keyboard (all passed in the Phase B full run), and census C5 flagged no letter. Still unchecked: the default device keyboard (OS-dependent) and whether a long-press counts as the "2 taps".
- **F7 flag gates:** `python3 tools/ortho/f7_status.py` (es, then pt, then fr). All three HOLD OFF: es has a verified table but 30 golden rows still wait on it (tilde-only pairs te/té etc. are accepted by the interim same-sound list, not the table, and es_scz needs a re-check against the Spain voice); pt and fr have no sound-alike generator (a SENSE-CUE job). A pass would also need an in-app check that a player who misses only an accent sees the fix, which has not been run. Flags stay off.

## es table re-check, 2026-11-06

- Added donde/dónde and quien/quién (the two bank tilde-only pairs) to `assets/words/es/homophones.seed.txt`; the table now has 9 true sets. Loopback re-run on es-ES-Neural2-G (bar 0.652): donde/dónde 0.586, quien/quién 0.627, casa/caza 0.644. All under the bar, so the voice does not distinguish them; the table reads verified. cauce/cause (1.10), chota/cota and roca/rocha stay flagged voice-may-distinguish.
- **es_scz:** casa/caza clears the bar by only 0.008. I changed its rule from "accent" to "true" with that margin written into the description, since the table and the done-when already grade it correct both ways. A native Spain ear on the Gig B "Same sound in your accent?" column still decides.
- **Grading side effect, flag off:** the table feeds `homophones::accepts`, so with the profile flag off, a prompt of dónde now also accepts donde (and quien/quién), as another sense. That is what your amendment asked for, but it is a live change in es.
- Golden: the tilde monosyllables (te/té etc.) are now labelled correct_other_sense through the F4 list. 18 `pending_table` rows remain, all lexical pairs with one member outside the bank (hola/ola, hecho/echo, baca/vaca...). I did not add them to the seed: that would widen es grading without an auditor. F7 for es still HOLDS OFF on those 18 and on the in-app close check.

## Filipino leftovers, 2026-11-06

- komplikado←kumplikado and politika←pulitika merged (left form stored, right quarantined and accepted at every tier) by `tools/ortho/sweep_all_batch6.py`; saved-word map regenerated (67 rows, fil 24); generated files re-emitted.
- `assets/words/purity/native-confirm/fil.tsv`: needs_native_confirm tags for komplikado, politika, politiko, kompanya, istilo, inhinyeriya, legal, sunod-sunod. Tags only, no grading change.
- The first full gate run failed at soundalike-api-check because `make_golden.py` read the table file directly; it now uses `tools/soundalikes.py`. Golden output unchanged.
