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
