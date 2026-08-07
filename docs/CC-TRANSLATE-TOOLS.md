CC-TRANSLATE-TOOLS — The Spell Translator Suite (Tools 1–13)
[stashed verbatim from Eric 2026-08-02 — full text as pasted in chat]
Status: REVIEW-GATED. Subordinate to CC-BANK-TRANSLATE (authoritative on gloss pivot, ingest validation, decoys — FILE NOT YET IN HAND). Phase A gloss column untouchable. Slots AFTER CC-CALENDAR per Eric.

## FULL TEXT (recovered from session transcript 2026-08-05 — the stub had pointed at the transcript only)

add this after the calender CC-TRANSLATE-TOOLS — The Spell Translator Suite (Tools 1–13)
Status: REVIEW-GATED. Subordinate to CC-BANK-TRANSLATE (this is the Phase B expansion; BANK-TRANSLATE stays authoritative on the gloss pivot, ingest validation, and decoys). CC-BANK-TRANSLATE Phase A — the gloss column riding the Fiverr audit round — remains deadline-critical and is untouchable by this file. Tools 14–15 (themed charts, printable flash cards) are recorded as deferred, not cut.
Intent
The translator's entire safety and App Store story is Eric's closed-space rationale: it translates only between the audited word banks via the sense-locked English-gloss pivot — lookup-only, no machine translation, ever. A closed output space cannot surface filth, cannot hallucinate, and cannot earn a strike. Every tool below must preserve that property; a tool that needs a word outside the banks doesn't get the word — it gets "not in my word list yet."
Second thesis: the translator is an on-ramp to gameplay, not a utility dead-end. Every card ends in a door — spell it, plan it, stamp it, play it. And it is the showcase of Spell's identity: one concept across fifteen writing systems is a card no competitor can render.
Features
Core (ship with the tab):
1. All-Languages Fan-Out card
Look up any bank word → the concept across every entitled + glossAudited language, side by side, native script, each row with its audio. Rows for unentitled or gloss-dark languages are absent, not locked (widget D3 precedent). This card is the product's face.
2. Spell-the-Translation loop + Add-to-Plan
Every result card ends in two doors: "Now spell it" (drops the word into the real spelling engine via the standard session path, scored normally) and "Add to plan" (places it on a Spell Calendar day via the planner's existing band-legality filter — an illegal word gets the planner's honest audited decline, never a silent drop). No translator-specific scoring or session logic exists.
3. Pair audio compare
On pair and fan-out cards: tap-to-hear either side, or tap-both for A-then-B playback with a beat between. Existing TTS wiring only; pack-resolution order per CC-OFFLINE-PACKS I3.
4. Transliteration toggle
Non-Latin rows flip native script ↔ romanization (pinyin machinery generalized; romanization schemes come from Script Paths data where present, else the row simply has no toggle). This is the accessibility unlock that makes tool 1 readable to a kid who can't decode Cyrillic yet.
Wave 2 (habit + loop):
5. One Word, Fifteen Ways daily
Deterministic date-seeded daily fan-out card (Daily Challenge seed machinery, separate seed stream). Candidate for the App Group widget snapshot (needs CC-IOS-SURFACES sign-off as a snapshot field — D5 there governs).
6. Polyglot Passport
Per profile: spelling the same concept correctly across languages stamps that concept's passport page per language ("Water: 6 of 15"). Computed purely from existing scoring events keyed by gloss-concept — zero new gameplay, zero new writes beyond the stamp store. Conquest framing; no "missing stamps" guilt states (CALENDAR I4 shared lint).
7. Home Pair heritage bridge
A profile may pin one pair (e.g., English ↔ Tagalog); the translator defaults to it, the daily card runs it, Passport spotlights it. When CC-FAMILY-VOICES ships, an approved Personal Voice may read its side of the pair — dependency declared, zero voice code here.
8. Translation Match mode
Registry mode riding Definition Match machinery: given the gloss, spell the target-language word; and a memory-pairs board matching words across two chosen languages. Gloss column is the only content source; inherits Definition Match's audio, exit standard, and card-speed rules.
Wave 3 (decision-gated, each needs its D signed):
9. False-friend & cognate flags (D3)
Pair cards flag cognates and false friends from small curated per-pair audited tables. Authored content: enters the same audit pipeline, ships per-pair only when its table is audited (defs-dark pattern per pair).
10. Camera lookup (D6)
The existing OCR hub stub (with its profanity pass) feeds the translator: recognized bank word → fan-out card; unknown → "not in my word list yet." On-device recognition only; no image retained (PHOTO-PICTURE retention posture).
11. Word Globe (D4)
Fan-out rendered geographically from the existing entitlements country-language map — tap a region, hear the word. Stylized, label-light globe; this file adds no new geographic assertions beyond the map the app already ships (geopolitical caution is D4's whole subject).
12. Language Detective (D7)
Audio plays a word; kid picks the language from choice chips — never free text. Existing audio assets only; daily-card sibling cadence.
13. Loanword Explorer (D8)
Curated audited packs of borrowings ("Japanese words inside English"). Authored content, finite, audit-gated per pack; pack words must already exist in the relevant banks (closed space holds).
Decisions

* D1 (proposed): Free/Complete split — lookup, fan-out, pair audio, transliteration, daily, Passport free (they compound the daily habit); Home Pair, Translation Match full depth, camera, Globe, Detective archive, Loanword packs Complete. Sign off.
* D2 (proposed): Kid Mode gets the full core + Passport + daily; camera stays out of Kid Mode (camera permission is a parent-context act). Little Speller: no translator at all.
* D3/D6/D4/D7/D8: per-tool gates above — each Wave 3 tool builds nothing until its D is signed; D3 and D8 additionally carry audit-budget implications Eric owns.
* D5 (proposed): Platforms: tools 5–13 app-only (`platforms: ["app"]`, buy-driver logic); basic lookup + fan-out (1–4) also on web where glossAudited. If you want the whole suite app-exclusive, say so and 1–4 join the wall.
* D9 (decided): No sentences, no phrases, ever — word-level only; a phrase request path may not exist even as a stub.

Invariants

* I1 (closed space): Only bank words are ever rendered, spoken, matched, stamped, or suggested anywhere in the suite; every miss path renders the single audited "not in my word list yet" state. CI: no MT library, translation API client, or network-translation symbol exists in any target (symbol scan, web-wall machinery).
* I2: Glosses are sense-locked to the row's audited definition; per-language visibility = entitlement ∧ glossAudited (single resolver, no per-tool checks).
* I3: Read-only over banks, scoring, learner model; the Passport stamp store and Home Pair pin are this file's only writes.
* I4: No free-text input on any kid-facing surface (Detective = chips; Match = existing input paths; custom My Words rows without audited glosses show "no translation yet", never a generated one).
* I5: All interface strings from audited pools; unaudited-language fallback layouts (defs-dark pattern); conquest framing + no-guilt lint shared with CALENDAR/REPORTS.
* I6: On-device; zero network in every tool flow.

Non-goals
Do not touch CC-BANK-TRANSLATE Phase A scope, the gloss schema, ingest validation, or the Gig A sheets. No MT, no phrase/sentence handling, no third-party dictionary APIs, no user-editable glosses, no social sharing surfaces (fan-out share cards would be a CC-FINALE-pattern amendment, separately gated). Tools 14–15 deferred.
Acceptance tests / Done

1. Fan-out fixture: profile entitled to 6 languages, 4 glossAudited → card renders exactly those 4 rows, absent-not-locked verified; audio per row plays.
2. Closed-space: lookup of a non-bank word, camera capture of a non-bank word, and a My-Words-without-gloss row all land on the single audited miss state; planted MT symbol fails the CI scan.
3. Doors: "Now spell it" reaches a scored standard session containing the word; "Add to plan" lands on the chosen day or produces the planner's audited band-decline — no third outcome.
4. Passport: scripted scoring fixture (same concept correct in 3 languages) → exactly 3 stamps, fired only from scoring events; no stamp from lookup alone.
5. Transliteration goldens: zh (pinyin), ru, ko rows flip correctly; a language without a romanization scheme shows no toggle.
6. Daily determinism: same date + entitlements → identical card across reinstalls; different seed stream from Daily Challenge (both fixtures on one date differ).
7. Detective: chip-only input verified; no text field in the view hierarchy (snapshot + accessibility audit).
8. Match mode passes Definition Match's inherited gates (exit standard, card speeds, safe-area).
9. Globe renders exclusively from the shipped country-language map fixture; adding a country in the map is the only way a region appears (planted test).
10. Kid Mode: camera entry absent; Little Speller build: zero translator symbols.
11. Proxy run across all 13 tool flows: zero network requests.
12. Wave 3 check: with D3/D4/D6/D7/D8 unsigned, repo contains their design docs and zero executable code paths (CI symbol check).
## DECISIONS SIGNED (Eric, 2026-08-05, verbatim: "Lets greenlight all
these for the app not the website")
- D5 SIGNED, strong form: the WHOLE suite is app-only — tools 1-4 join
  the platform wall (`platforms: ["app"]`); nothing renders on web.
- D1 SIGNED as proposed: lookup/fan-out/pair-audio/transliteration/
  daily/Passport free; Home Pair, Translation Match depth, camera,
  Globe, Detective archive, Loanword packs = Complete.
- D2 SIGNED as proposed: Kid Mode gets core + Passport + daily, no
  camera; Little Speller has no translator.
- NOT swept by this verdict (each still awaits its own signature, and
  D3/D8 carry audit budget Eric owns): Wave-3 per-tool gates
  D3/D4/D6/D7/D8 — zero code stands (CI-enforced).
- Wave 2 build state: tools 5 (daily), 6 (Passport store), 7 (Home
  Pair) landed pivot-independent this window; tool 8 stays dark until
  the gloss column (CC-BANK-TRANSLATE, file still owed).

## WAVE 2 COMPLETE 2026-08-05 — tools 5, 6, 7, 8 built
Tool 5 daily fan-out (date-seeded, own stream), tool 6 Polyglot
Passport (concept-keyed stamps, fired only from the scoring seam —
never from lookup), tool 7 Home Pair pin, and tool 8 Translation Match
(spell-the-translation prompt + memory-pairs board). All four are
CLOSED-SPACE by construction and render DARK until a language is
glossAudited: with no gloss table the builders decline rather than
invent a board (tested). Wave 3 (tools 9-13) remains zero-code by its
own acceptance test 12 until D3/D4/D6/D7/D8 are signed.

## WAVE 3 SIGNED (Eric, 2026-08-05) — all five tools
D3 (false friends/cognates), D4 (Word Globe), D6 (camera lookup),
D7 (Language Detective), D8 (Loanword Explorer): ALL SIGNED. D3 and D8
carry the audit-budget spend Eric owns and stay per-pair / per-pack
gated — a table or pack ships only when ITS audit lands (defs-dark
pattern per pair). Acceptance test 12 is hereby satisfied by signature,
not by absence: Wave 3 code may exist. The camera tool's own
precondition — the Vision language matrix — was MEASURED the same day
(see CC-PHOTO-IMPORT-PLAN G-B): 12 of 14 languages Native, fil + sw on
the English recognizer.

GATE AMENDED with the Wave-3 signatures: acceptance test 12's zero-code
scan is RETIRED (its premise — unsigned Ds — no longer holds). What
replaces it guards what the signed specs still demand: the
audited-content accessors (`pair_table`, `loanword_packs`) may exist in
exactly one resolver, the per-pair/per-pack tables stay EMPTY until each
audit lands (asserted in-engine by
`authored_tables_are_dark_until_audited`), and the machine-translation
ban — the one guard that never relaxes — stands untouched.

## Gloss wave 1 — 39 rows to 2,662 (2026-08-06, post-136)

Eric: "build the 300 concept gloss for all languages so chinese is an
empty shell and should just be cut?"

CHINESE IS NOT AN EMPTY SHELL — I reported that wrongly and the
correction matters. zh holds 6,500 words, second-deepest in the game,
plus 1,170 CC-CEDICT rows in backend/zh_glosses.json, its own keyboard,
kid-exclude list, definition pool and bank builder. My zero came from
grepping src/word_data.rs, which holds fourteen languages; zh lives
alone in src/words.rs because it stores `pinyin|hanzi` pairs. Nothing
was cut.

The gloss now covers all fourteen banked languages. Every row was
authored from one 300-concept core list and then FILTERED by the three
laws, so the yield is a measurement, not a target — the counts are in
config/gloss/README.md. scripts/gloss-check.mjs learned to read
words.rs for zh; translate.rs include_str!s all fourteen files.

Everything stays DARK. `audited` is still false everywhere and the
resolver still gates on it, which is now covered by a test that loads
all 2,662 rows through the real include_str! pipe and asserts the
resolver yields None anyway — rows existing must never be the thing
that lights a language up.

TWO BANK DEFECTS FOUND, both filed rather than worked around:

* BD-G1 GERMAN BANK VOCABULARY. `mann`, `frau`, `mutter`, `vater`,
  `kopf`, `bein` are absent in every casing from a 6,809-word bank that
  does contain `kind`, `haus`, `hund`, `wasser`. German glosses cannot
  exceed the bank, so de landed 169 where its peers landed ~250.
* BD-G2 GERMAN CASING. The bank is 97% lowercase — 170 of 6,809 words
  capitalised, all in the EASY tier. German nouns are therefore stored
  against German orthography, and `Großmutter` is in the bank as
  `grossmutter` (ß is inconsistent too: 73 words use ß, 263 use ss).
  A German speller is being taught wrong spelling. This is a bank fix,
  not a gloss fix, and it needs Eric's call on whether the bank
  recapitalises or the comparator formally goes case-blind.

OPEN, unchanged by this wave: no language renders until a named native
speaker signs one. That is now the ONLY thing between the translator
and being usable — the tables, the loader, the CI and the resolver are
all done. Spanish is the obvious first ask at 258 rows.

### CORRECTION + widening: BD-G3, core-vocabulary coverage (all banks)

I told Eric Korean's 38% was "probably verb form, recoverable, unlike
German." That was wrong and the correction changes the action. Of ko's
169 misses only 41 are `-다` verbs, and only 12 of those have a stem in
the bank. The other 128 are plain nouns the bank simply does not have:
태양 sun, 곰 bear, 팔 arm, 피 blood, 뼈 bone, 소년 boy. Korean is the
same disease as German, not a different one.

So I measured it directly — fifteen words no general bank can lack,
checked against each bank in its own stored form:

```
es 15/15   fil 15/15   pt 15/15   fr 14/15   pl 14/15   sw 14/15
ru 13/15   ko 13/15    ar 12/15   vi 12/15   ja 11/15   de 10/15
hi  9/15
```

hi is missing सूरज sun, आदमी man, औरत woman, रोटी bread, दूध milk and
किताब book. A Hindi speller cannot be asked to spell "book".

THE SHAPE OF THE DEFECT: the banks are not thin, they are MISCOMPOSED.
ru/ar/ko/ja each hold ~6,800 words while missing words a first-week
learner needs. The generation wave optimised for count against a
per-language floor, and nothing ever asserted that the commonest words
were among them. Volume was measured; coverage never was.

USEFUL SIDE EFFECT: gloss yield IS a coverage metric. Each language's
row count is exactly "how many of one fixed 300-concept core list this
bank can express", measured through the same three laws for every
language. es 258 and hi 119 is not a statement about Spanish and Hindi
— it is a statement about two banks. That number is now recomputed by
scripts/gloss-check.mjs on every run, so the regression is watched
even before anyone decides to fix it.

NOT FIXED, needs Eric: filling core vocabulary means regenerating parts
of thirteen banks, which moves word IDs and therefore touches saved
progress, offline packs and the composite pin law. That is a wave, not
a patch, and it is his call whether it precedes or follows a native
speaker signing Spanish.
