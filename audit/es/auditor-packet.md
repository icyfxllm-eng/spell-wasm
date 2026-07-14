# Spanish (es) native-speaker auditor packet

Machine checks are done; these need a human who reads the language.

## Already resolved (decision addendum 2026-07 — please ratify)  (3)

- **26** — 'negro' is a valid es word but is on the profanity seed for: ['en', 'fr']. Not flagged for es (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- **39** — 'leche' is a valid es word but is on the profanity seed for: ['fil']. Not flagged for es (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- **negro (username)** — now in `backend/blocklist.txt`; rejected as a username in ANY locale (`backend/test_usernames.py`), while staying a valid Spanish puzzle word.
- **accept-any homophones** — casa/caza, botar/votar, cocer/coser wired into grading.

## Homophones / same-sound spellings — list any (grading can then accept-any via assets/words/es/homophones.txt)  (21)

- **ave** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['abe', 'ave', 'have'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['ave']; other real words: ['abe', 'have']
- **árbol** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['arbol', 'árbol'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['árbol']; other real words: ['arbol']
- **vergüenza** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['verguenza', 'vergüenza'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['vergüenza']; other real words: ['verguenza']
- **camino** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['camino', 'caminó'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['camino']; other real words: ['caminó']
- **casa** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['casa', 'caza'] | PROPOSED BUCKET: accept-any — CONFIRMED (Eric), already wired in homophones.txt | Twins in lists: ['casa']; other real words: ['caza']
- **león** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['leon', 'león'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['león']; other real words: ['leon']
- **música** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['musica', 'música'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['música']; other real words: ['musica']
- **número** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['numero', 'número'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['número']; other real words: ['numero']
- **pie** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['pie', 'pié'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['pie']; other real words: ['pié']
- **teléfono** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['telefono', 'teléfono'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['teléfono']; other real words: ['telefono']
- **tomate** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['tomate', 'tómate'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['tomate']; other real words: ['tómate']
- **trabajo** — [b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j] sounds identical to: ['trabajo', 'trabajó'] | PROPOSED BUCKET: accept-any (both members common) — PROPOSED, confirm | Twins in lists: ['trabajo']; other real words: ['trabajó']
- **árbol** — [accent-only pairs (papa/papá)] sounds identical to: ['arbol', 'árbol'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['árbol']; other real words: ['arbol']
- **camino** — [accent-only pairs (papa/papá)] sounds identical to: ['camino', 'caminó'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['camino']; other real words: ['caminó']
- **león** — [accent-only pairs (papa/papá)] sounds identical to: ['leon', 'león'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['león']; other real words: ['leon']
- **música** — [accent-only pairs (papa/papá)] sounds identical to: ['musica', 'música'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['música']; other real words: ['musica']
- **número** — [accent-only pairs (papa/papá)] sounds identical to: ['numero', 'número'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['número']; other real words: ['numero']
- **pie** — [accent-only pairs (papa/papá)] sounds identical to: ['pie', 'pié'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['pie']; other real words: ['pié']
- **teléfono** — [accent-only pairs (papa/papá)] sounds identical to: ['telefono', 'teléfono'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['teléfono']; other real words: ['telefono']
- **tomate** — [accent-only pairs (papa/papá)] sounds identical to: ['tomate', 'tómate'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['tomate']; other real words: ['tómate']
- **trabajo** — [accent-only pairs (papa/papá)] sounds identical to: ['trabajo', 'trabajó'] | PROPOSED BUCKET: no-action — the accent is a stress difference the audio CAN carry, so this is a legitimate spelling test (many 'twins' here are just unaccented corpus typos) | Twins in lists: ['trabajo']; other real words: ['trabajó']

## English cognates to confirm  (1)

- **34** — 'hospital' also appears in the English word lists — confirm it's a legitimate es word

## Open questions  (3)

- **-** — No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.
- **LANG_VOICES** — es voice = es-ES-Neural2-B (locale es-ES); configured in 1 place(s)
- **-** — 202 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Voice note
The Spanish TTS voice is **es-ES (Castilian)**. seseo pairs (casa/caza) are homophones only for Latin-American ears; if the audience is Latin American (es-419 on file), the voice is an Eric decision.
