# CC-SENSE-CUE v1.1: Sound-alike Tables

**Status:** REVIEW-GATED. Census run 2026-10-16 (`reports/sense-cue-v1.1-census.md`); Eric's census decisions recorded in section 4. Phases not started.
**Amends:** CC-SENSE-CUE v1 (whose spec text is not in this repo; it exists through `src/homophones.rs` and `tools/build_collisions.py`).
**Source:** Eric's spec as pasted in chat on 2026-10-16, with his census decisions applied. Changes from the pasted text are marked **[S1]** to **[S4]**.
**Sequencing:** post-build-56; must not delay release.
**Owns:** the sound-alike table: its schema, status, `kind` tag, **set identity [S1]**, generators, verification, read API, shared report.
**Does not own:** grading, cues, Second Look (SENSE-CUE v1); the bonus (CC-SOUNDALIKE-BONUS); the Impossible filter (CC-IMPOSSIBLE).

---

## 0. Intent

Today the table exists only for en, es and ru (`assets/words/{en,es,ru}/homophones.txt`). For every other language there is no table, and "no table" can mean "we checked and there are no sound-alikes" or "nobody has checked yet". The code cannot tell them apart. This file makes the table say which is true, fills it with one pipeline per language type, and gives every feature one table to read at its own strictness.

When the spec is silent: **never call a language safe without proof.** A wrong `missing` costs a feature; a wrong `verified` gives a player an unanswerable word.

Census finding (C3): an empty table does not fail closed today, it fails **strict**. A learner in one of the 12 languages without a table is marked wrong for a real sound-alike of the prompt. That gap is live now.

---

## 1. Ownership

| File | Relationship |
|---|---|
| CC-SENSE-CUE v1 | parent; its accept-all invariant reads this table |
| **This file [S1]** | **owns `(setId, memberId)`.** CC-ZH-PINYIN-DEPTH F5 (not started, held at Gate 0) adopts this identity when it starts; it does not define a second one |
| CC-RUSSIAN-STRESS / CC-RU-ORTHO | the ru key. In code today it is `key_ru` in `tools/build_collisions.py` (devoicing and assimilation only); registered here as the ru generator, unchanged |
| CC-EN-EXPERT-GROWTH Phase A | English pronunciation keys (`assets/words/en/en_pron.json`, merged); the en generator reads them |
| CC-AR-HI-PREAUDIT D2 | ar/hi sound-alike trap pairs, imported as seed sets |
| CC-AUDIO-CLARITY / CC-AUDIBILITY-TRIAGE | whisper.cpp and per-voice keying; F5 reuses them |
| CC-SOUNDALIKE-BONUS, CC-IMPOSSIBLE, CC-SPELL-O-PHONES | consumers only |

If an owner contradicts this file, stop and ask.

---

## 2. §0 Census

Done 2026-10-16. See `reports/sense-cue-v1.1-census.md`.

---

## 3. Features

### F1. Table schema
One structured record per language, able to say "checked and empty":
- `status` (F2), `voice` (the Layer-1 voice id it was verified against), `generator` (F3 id and version).
- `sets[]`: `setId`, `members[]` (memberId plus bank wordId), `kind` (F4), `mergerRules[]` (empty for `true`), `confirmed` (F6), `loopbackFlag` (F5).

Census C1: the current files cannot carry per-set `kind` (every reader treats every token on a line as a member), so migrate to one structured file per language. Existing en/es/ru sets must keep their members through migration.

**[S1] Set identity.** `setId` is minted here, derived from the set's sorted, NFC-normalised member list and the language, so regenerating an unchanged set reproduces the same id (I2). `memberId` is the member's bank wordId. Today's `src/homophones.rs` uses the line number as an implicit group id; that goes away with the migration.

### F2. Table status

| Status | Meaning | How it is earned |
|---|---|---|
| `verified` | built and machine-checked | an F3 generator ran and the F5 loopback ran; zero sets is legal |
| `none-by-construction` | sound-alikes are always typed identically, so no conflict exists | only by passing the proof test (I3); never set by hand |
| `missing` | not built | default for every language without the above |

**Lock scope (H1):** `missing` turns off, for that language only, SENSE-CUE cues, the sound-alike bonus and Impossible. Normal grading behaves exactly as today. Census C3: not a reversal of existing code.

### F3. Generators, one per language type
Contract: `phonetic_key(lang, voice, form, mergerRules=[]) -> Key`, registered implementations, shared conformance suite.

| Type | Languages | Generator |
|---|---|---|
| Existing | en, es, ru | en reads EN-EXPERT-GROWTH keys and **keeps today's heteronym exclusion** (census C5); es keeps its key with seseo moved to a merger rule **[S3]**; ru = `key_ru`, unchanged |
| Rule-based | pt, pl, de, fil, sw, vi, ko, ja, ar | rule table over the written form. Seeds: pl ó/u, rz/ż, h/ch; de final devoicing, vowel-length marks (Rad/Rat, Wal/Wahl); pt ss/ç/s/z, x/ch (concerto/conserto); ko coda neutralisation, liaison, tensification, nasalisation (낫/낮/낯); ja kana long-vowel and voicing pairs (おう/おお, ず/づ, じ/ぢ, こおり/こうり); vi per the active voice's dialect; ar imports AR-HI-PREAUDIT D2 pairs |
| Dictionary | fr | **WikiPron, cross-checked with espeak-ng. Lexique is skipped until its licence is clear [S4]** |
| Candidate for proof | zh | key = surface pinyin with tones as spoken (sandhi applied), so 3-3 sandhi collisions are found |
| Gated | hi, fa | follow their existing gates; `missing` until those lift |

espeak-ng (GPL v3+) and any other GPL tool runs at build time only, never linked into the app.

### F4. `kind` via named merger rules
Each language lists named merger rules in its registry entry. Examples: en `cot-caught`, `pin-pen`, `mary-marry-merry`, `wine-whine`; es **`seseo` [S3]**, `yeismo` if the voice turns out to distinguish ll/y; pt `br-pt`; vi `north-south` (whichever the voice is not); fr `brin-brun`, `pate-patte`.
- Members collide under the base key (the active voice's accent): `kind: true`.
- Members collide only with one or more merger rules on: `kind: accent`, with `mergerRules` recording which.

**[S3] Spanish.** The voice is es-ES-Neural2-G (Castilian), which normally distinguishes /θ/ from /s/. Seseo (s/z/soft c) is therefore a merger rule for this voice, and pairs such as casa/caza are `kind: accent`, not `true`. Consequence: they leave the bonus, and they leave SENSE-CUE's accept-all grading, which reads `true` only. Phase B must show Eric the Spanish sets that move from accepted to not accepted before that grading change ships (see S3 note in section 4).

### F5. Voice loopback check **[S2] flag-only**
For every `true` set, synthesise all members with the active Layer-1 voice and compare (whisper.cpp output and the CLARITY audio-distance metric). If the voice appears to tell them apart, **set `loopbackFlag: voice-may-distinguish` and send the set to the auditor sheet (F6). The loopback never changes `kind` itself.**
Reason: whisper on an isolated word guesses from its language model, so identical clips can be transcribed as different words; an automatic demotion would drop a real sound-alike out of accept-all grading. Only auditor `no` (F6) demotes.
Keyed per voice; a Layer-1 voice switch forces a re-run. Synthesis needs working TTS billing; check before Phase B.

### F6. Auditor confirmation
Add a `same sound? yes / no / unsure` column to the Gig B sheet for every candidate set, and every F5-flagged set. Ingest through the existing locked-sheet path only.
- `yes`: `confirmed: true`, flag cleared.
- `no`: demote to `accent`, `mergerRules: [auditor-distinguishes]`, flagged for Eric.
- `unsure`: unchanged.
Machines never set `confirmed` (I5) and never demote (I9).

### F7. One read API
`soundalikes(lang, voice) -> { status, sets[] }`. Every consumer goes through it:
- SENSE-CUE grading, CC-SOUNDALIKE-BONUS, CLARITY `EXEMPT_HOMOPHONE`: `kind: true` only.
- CC-IMPOSSIBLE: `true` and `accent`.
- CC-SPELL-O-PHONES: may read `accent` with its own rules, later.
Census C2 consumers to migrate: `src/homophones.rs` callers (game.rs, boardgame_grade.rs, spelldoku, spelluzzle, wordcross, wordsearch); raw-file readers `scripts/audio-verdicts.mjs`, `tools/human-audio/qc.py`, `tools/audit/audit_lang.py`; the parallel cmudict scan in `scripts/f3-worklist.mjs`. A symbol scan fails CI if anything else reads the raw files.

### F8. Shared census report
CI emits one report per build, per language: status, set counts by `kind`, `confirmed` counts, loopback flags, bank words with at least one bonus-eligible member after profanity and proper-noun filters, and the word count Impossible's filter removes. CC-SOUNDALIKE-BONUS and CC-IMPOSSIBLE read it; neither rescans.

---

## 4. Decisions

**Signed by Eric, 2026-10-16 (census review):**
- **S1** This file owns a stable `(setId, memberId)`. ZH-PINYIN-DEPTH adopts it later.
- **S2** The F5 loopback is flag-only. It never demotes; only the auditor does.
- **S3** Spanish seseo is an accent merger rule for the es-ES voice.
- **S4** Skip Lexique until its licence is clear (lexique.org states CC BY-SA 4.0 but links to CC BY-NC 4.0).

**Recommended defaults, unsigned:**
- **H1** Three statuses; `missing` turns off only cues, bonus and Impossible for that language.
- **H2** One table with a `kind` tag; Impossible excludes `accent` too.
- **H3** Generators per F3.
- **H4** Voice loopback (F5, flag-only per S2) plus the Gig B confirm column (F6).
- **H5** Order for English: EN-EXPERT-GROWTH Phase A (merged), then its staged rows live, then the English table regenerated, then CC-IMPOSSIBLE Phase A for English.

**Open:**
- **H6** Gig B timing. Recommendation: add the column now for en/es/ru; other languages' candidate sets go on a later sheet once their generators run.
- **H7** Licence. Ship only derived `kind`/`setId` data, never pronunciation strings; legal check before first release.
- **H8** Phase C language order. Recommendation: pt, pl, de, ja, ko, vi, fil, sw, ar.
- **S3 note, resolved 2026-10-17:** Eric chose to keep seseo pairs accept-all. Seseo stays in the Spanish base key (so those sets are `kind: true`), and the rule is recorded as declined in `config/soundalike-merger-rules.json`. Impossible still excludes them. In the same review he approved fixing the *ch* bug in `key_es` (marca/marcha and three more false sets).

**Correction carried from the pasted spec:** ja and zh are not `none-by-construction` by assumption. Japanese kana has real pairs (こおり/こうり), and Chinese tone sandhi can make two citation spellings sound alike. Both go through the proof test (I3).

---

## 5. Invariants

- **I1** One table and one API per language; no module reads raw table files (symbol scan).
- **I2** A set's `setId` is identical across regenerations when its members are unchanged.
- **I3** `none-by-construction` only when the proof test passes: any two bank rows with equal spoken keys under the active voice have identical accepted-answer sets. Failing rows are listed by name.
- **I4** `verified` with zero sets is legal and distinct from `missing`.
- **I5** Only auditor ingest sets `confirmed`.
- **I6** Every table records its voice; a mismatch with the active Layer-1 voice reads as `missing`.
- **I7** Every `accent` set names at least one registered merger rule.
- **I8** No bank edits. No telemetry.
- **I9 [S2]** No automated step changes a set's `kind` from `true` to `accent`; only a registered merger rule or an auditor `no` does.

---

## 6. Phases

- **A** Schema, status, `setId` [S1], API; migrate en/es/ru with no new data; migrate every C2 consumer; proof test; shared report; Gig B column for en/es/ru.
- **B** Merger rules and `kind` for en/es/ru, including Spanish seseo [S3] with Eric's S3-note confirmation; flag-only loopback [S2] for en/es/ru; English regenerated per H5.
- **C** Rule-based generators in H8 order; flag-only loopback for each.
- **D** French dictionary generator (WikiPron and espeak-ng; Lexique only if its licence clears [S4]).

Each language moves from `missing` to `verified` on its own.

---

## 7. Acceptance tests

1. A `missing` language: Normal play grades exactly as before (golden replay byte-identical); cues, bonus and Impossible absent.
2. A `verified` language with zero sets: cues, bonus and Impossible available; the Impossible filter removes nothing.
3. zh proof test passes, or fails listing each colliding pair; status follows the result.
4. ja proof test fails on こおり/こうり; ja stays `missing` until the rule-based generator runs.
5. en: `sink/sync` is `kind: true`; `cot/caught` is `kind: accent` with `mergerRules: [cot-caught]`.
6. **[S2]** A fixture pair the loopback hears as different stays `kind: true` with `loopbackFlag` set and appears on the auditor sheet; an auditor `no` then demotes it to `accent`.
7. Switching the Layer-1 voice fixture: the table reads `missing` until regenerated.
8. Symbol scan: a deliberate direct read of `homophones.txt` in a test module fails CI.
9. Auditor `no` on a `true` set gives `accent`, flagged; no machine path can set `confirmed`.
10. Regenerating en with unchanged sets gives identical setIds.
11. The shared report exists, and CC-SOUNDALIKE-BONUS and CC-IMPOSSIBLE read it with no rescan code.
12. pl `może/morze`, de `Rad/Rat`, pt `concerto/conserto`, ko `낫/낮/낯` are found as `true` (or flagged by the loopback).
13. **[S3]** es: `casa/caza` is `kind: accent` with `mergerRules: [seseo]` for es-ES-Neural2-G.
14. **[S1]** No second set identity exists in code (symbol scan for any other set-id scheme).

---

## 8. Non-goals

No grading, cue or Second Look changes (beyond what S3 implies, which waits on Eric). No near-homophones (lose/loose). No bank edits or tier changes. No new TTS voices. No change to hi/fa gates. No pronunciation strings shipped in the app.
