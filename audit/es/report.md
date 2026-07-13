# Spanish content audit — `es`  (machine pass, REVIEW-GATED)

Totals: **2 critical · 1 violation · 24 warning · 4 info**

## Feature 1 — Word list integrity

**WARNING (1)**

- `assets/words/es/hard.txt:34` [english-contamination] 'hospital' also appears in the English word lists — confirm it's a legitimate es word

## Feature 2 — Difficulty tier calibration

Tier stats:

| tier | count | mean len | median len | accent% | trap% |
|--|--|--|--|--|--|
| easy | 50 | 4.12 | 4.0 | 6% | 46% |
| medium | 50 | 6.54 | 7.0 | 20% | 68% |
| hard | 50 | 8.66 | 8.0 | 18% | 56% |
| expert | 52 | 10.77 | 10.0 | 62% | 94% |


**VIOLATION (1)**

- `assets/words/es/hard.txt:hard` [non-monotonic-trap] trap density 0.560 < previous tier 0.680

**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] es voice = es-ES-Neural2-B (locale es-ES); configured in 1 place(s)
- `-:-` [audio-coverage] 202 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

_Note: `accent` twins are bucketed against a web frequency corpus, which contains unaccented misspellings (e.g. `arbol` for `árbol`). Treat those as noise; genuine minimal pairs like `camino/caminó`, `tomate/tómate`, `trabajo/trabajó` are the ones to rule on._

**WARNING (21)**

- `assets/words/es/:ave` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['abe', 'ave', 'have'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['ave']; other real words: ['abe', 'have']
- `assets/words/es/:árbol` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['arbol', 'árbol'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['árbol']; other real words: ['arbol']
- `assets/words/es/:vergüenza` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['verguenza', 'vergüenza'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['vergüenza']; other real words: ['verguenza']
- `assets/words/es/:camino` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['camino', 'caminó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['camino']; other real words: ['caminó']
- `assets/words/es/:casa` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['casa', 'caza'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['casa']; other real words: ['caza']
- `assets/words/es/:león` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['leon', 'león'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['león']; other real words: ['leon']
- `assets/words/es/:música` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['musica', 'música'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['música']; other real words: ['musica']
- `assets/words/es/:número` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['numero', 'número'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['número']; other real words: ['numero']
- `assets/words/es/:pie` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['pie', 'pié'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['pie']; other real words: ['pié']
- `assets/words/es/:teléfono` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['telefono', 'teléfono'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['teléfono']; other real words: ['telefono']
- `assets/words/es/:tomate` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['tomate', 'tómate'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['tomate']; other real words: ['tómate']
- `assets/words/es/:trabajo` [phonetic] [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['trabajo', 'trabajó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['trabajo']; other real words: ['trabajó']
- `assets/words/es/:árbol` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['arbol', 'árbol'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['árbol']; other real words: ['arbol']
- `assets/words/es/:camino` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['camino', 'caminó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['camino']; other real words: ['caminó']
- `assets/words/es/:león` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['leon', 'león'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['león']; other real words: ['leon']
- `assets/words/es/:música` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['musica', 'música'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['música']; other real words: ['musica']
- `assets/words/es/:número` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['numero', 'número'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['número']; other real words: ['numero']
- `assets/words/es/:pie` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['pie', 'pié'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['pie']; other real words: ['pié']
- `assets/words/es/:teléfono` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['telefono', 'teléfono'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['teléfono']; other real words: ['telefono']
- `assets/words/es/:tomate` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['tomate', 'tómate'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['tomate']; other real words: ['tómate']
- `assets/words/es/:trabajo` [accent] [accent-only pairs (papa/papá)] sounds identical to: ['trabajo', 'trabajó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['trabajo']; other real words: ['trabajó']

## Feature 5 — Profanity filter coverage

**CRITICAL (2)**

- `assets/words/es/easy.txt:26` [profanity-in-list] 'negro' is on the profanity filter but present in a word list
- `assets/words/es/easy.txt:39` [profanity-in-list] 'leche' is on the profanity filter but present in a word list

**INFO (1)**

- `assets/words/profanity/es.txt:-` [filter-layers] es seed layer: 58 terms; global union (all langs): 1689 terms; exclusions(build): 0; kid-exclude: 9

## Feature 6 — UI localization completeness

**WARNING (2)**

- `src/i18n/locales/es.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/es.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

