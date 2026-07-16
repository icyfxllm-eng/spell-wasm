# Spanish content audit — `es`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 24 warning · 6 info**

## Feature 1 — Word list integrity

**WARNING (1)**

- `assets/words/es/hard.txt:34` [english-contamination] 'hospital' also appears in the English word lists — confirm it's a legitimate es word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | accent% | trap% |
|--|--|--|--|--|--|
| easy | 50 | 4.12 | 4.0 | 6% | 42% |
| medium | 50 | 6.54 | 7.0 | 20% | 50% |
| hard | 50 | 8.66 | 8.0 | 18% | 50% |
| expert | 52 | 10.77 | 10.0 | 62% | 67% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] es voice = es-ES-Neural2-B (locale es-ES); configured in 1 place(s)
- `-:-` [audio-coverage] 202 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

_Note: `accent` twins are bucketed against a web frequency corpus, which contains unaccented misspellings (e.g. `arbol` for `árbol`). Treat those as noise; genuine minimal pairs like `camino/caminó`, `tomate/tómate`, `trabajo/trabajó` are the ones to rule on._

**WARNING (21)**

- `assets/words/es/:ave` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['abe', 'ave', 'have'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['ave']; other real words: ['abe', 'have']
- `assets/words/es/:árbol` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['arbol', 'árbol'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['árbol']; other real words: ['arbol']
- `assets/words/es/:vergüenza` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['verguenza', 'vergüenza'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['vergüenza']; other real words: ['verguenza']
- `assets/words/es/:camino` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['camino', 'caminó'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['camino']; other real words: ['caminó']
- `assets/words/es/:casa` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['casa', 'caza'] | PROPOSED BUCKET: accept-any — CONFIRMED (Eric), already wired in homophones.txt | Twins in lists: ['casa']; other real words: ['caza']
- `assets/words/es/:león` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['leon', 'león'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['león']; other real words: ['leon']
- `assets/words/es/:música` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['musica', 'música'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['música']; other real words: ['musica']
- `assets/words/es/:número` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['numero', 'número'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['número']; other real words: ['numero']
- `assets/words/es/:pie` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['pie', 'pié'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['pie']; other real words: ['pié']
- `assets/words/es/:teléfono` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['telefono', 'teléfono'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['teléfono']; other real words: ['telefono']
- `assets/words/es/:tomate` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['tomate', 'tómate'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['tomate']; other real words: ['tómate']
- `assets/words/es/:trabajo` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['trabajo', 'trabajó'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['trabajo']; other real words: ['trabajó']
- `assets/words/es/:árbol` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['arbol', 'árbol'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['árbol']; other real words: ['arbol']
- `assets/words/es/:camino` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['camino', 'caminó'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['camino']; other real words: ['caminó']
- `assets/words/es/:león` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['leon', 'león'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['león']; other real words: ['leon']
- `assets/words/es/:música` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['musica', 'música'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['música']; other real words: ['musica']
- `assets/words/es/:número` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['numero', 'número'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['número']; other real words: ['numero']
- `assets/words/es/:pie` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['pie', 'pié'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['pie']; other real words: ['pié']
- `assets/words/es/:teléfono` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['telefono', 'teléfono'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['teléfono']; other real words: ['telefono']
- `assets/words/es/:tomate` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['tomate', 'tómate'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['tomate']; other real words: ['tómate']
- `assets/words/es/:trabajo` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['trabajo', 'trabajó'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['trabajo']; other real words: ['trabajó']

## Feature 5 — Profanity filter coverage

**INFO (3)**

- `assets/words/profanity/es.txt:-` [filter-layers] es seed layer: 59 terms. Curation scan below is language-scoped (es seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1821-term all-language union — that over-block is intentional for user imports.
- `assets/words/es/easy.txt:26` [cross-lang-profanity] 'negro' is a valid es word but is on the profanity seed for: ['en', 'fr']. Not flagged for es (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- `assets/words/es/easy.txt:39` [cross-lang-profanity] 'leche' is a valid es word but is on the profanity seed for: ['fil']. Not flagged for es (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.

## Feature 6 — UI localization completeness

**WARNING (2)**

- `src/i18n/locales/es.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/es.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

