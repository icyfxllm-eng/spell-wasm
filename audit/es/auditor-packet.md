# es native-speaker auditor packet

Machine checks are done; these need a human who reads the language.

## PROFANITY ↔ word-list conflicts (a word is BOTH taught and filtered — resolve per pair)  (2)

- **26** — 'negro' is on the profanity filter but present in a word list
- **39** — 'leche' is on the profanity filter but present in a word list

## Regional vocabulary / English cognates to confirm  (1)

- **34** — 'hospital' also appears in the English word lists — confirm it's a legitimate es word

## Homophone pairs — pick a policy per pair (accept / remove rarer / require sentence)  (21)

- **ave** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['abe', 'ave', 'have'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['ave']; other real words: ['abe', 'have']
- **árbol** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['arbol', 'árbol'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['árbol']; other real words: ['arbol']
- **vergüenza** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['verguenza', 'vergüenza'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['vergüenza']; other real words: ['verguenza']
- **camino** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['camino', 'caminó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['camino']; other real words: ['caminó']
- **casa** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['casa', 'caza'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['casa']; other real words: ['caza']
- **león** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['leon', 'león'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['león']; other real words: ['leon']
- **música** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['musica', 'música'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['música']; other real words: ['musica']
- **número** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['numero', 'número'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['número']; other real words: ['numero']
- **pie** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['pie', 'pié'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['pie']; other real words: ['pié']
- **teléfono** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['telefono', 'teléfono'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['teléfono']; other real words: ['telefono']
- **tomate** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['tomate', 'tómate'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['tomate']; other real words: ['tómate']
- **trabajo** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['trabajo', 'trabajó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['trabajo']; other real words: ['trabajó']
- **árbol** — [accent-only pairs (papa/papá)] sounds identical to: ['arbol', 'árbol'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['árbol']; other real words: ['arbol']
- **camino** — [accent-only pairs (papa/papá)] sounds identical to: ['camino', 'caminó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['camino']; other real words: ['caminó']
- **león** — [accent-only pairs (papa/papá)] sounds identical to: ['leon', 'león'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['león']; other real words: ['leon']
- **música** — [accent-only pairs (papa/papá)] sounds identical to: ['musica', 'música'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['música']; other real words: ['musica']
- **número** — [accent-only pairs (papa/papá)] sounds identical to: ['numero', 'número'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['número']; other real words: ['numero']
- **pie** — [accent-only pairs (papa/papá)] sounds identical to: ['pie', 'pié'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['pie']; other real words: ['pié']
- **teléfono** — [accent-only pairs (papa/papá)] sounds identical to: ['telefono', 'teléfono'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['teléfono']; other real words: ['telefono']
- **tomate** — [accent-only pairs (papa/papá)] sounds identical to: ['tomate', 'tómate'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['tomate']; other real words: ['tómate']
- **trabajo** — [accent-only pairs (papa/papá)] sounds identical to: ['trabajo', 'trabajó'] — a player typing a correct-sounding twin would be marked wrong. Twins in lists: ['trabajo']; other real words: ['trabajó']

## Regionally-vulgar innocent words (Kid Mode risk)  (0)


## Open questions  (3)

- **-** — No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.
- **LANG_VOICES** — es voice = es-ES-Neural2-B (locale es-ES); configured in 1 place(s)
- **-** — 202 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Voice note
The Spanish TTS voice is **es-ES (Castilian, Spain)**. The seseo/yeísmo homophone pairs above (casa/caza, valla/vaya) are only homophones for *Latin American* speakers — a Castilian voice distinguishes s/z/c. If the target audience is Latin American (recommendation on file: es-419), the voice choice itself is an auditor/Eric decision.
