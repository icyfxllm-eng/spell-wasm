# Portuguese (pt) native-speaker auditor packet

Machine checks are done; these need a human who reads the language.

## Cross-language profanity (valid here, on another language's seed — kept, not flagged)  (0)


## Homophones / same-sound spellings — list any (grading can then accept-any via assets/words/pt/homophones.txt)  (1)

- **-** — Automated homophone detection is Spanish-only. Portuguese has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/pt/homophones.txt).

## English cognates to confirm  (0)


## Open questions  (3)

- **-** — No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.
- **LANG_VOICES** — pt voice = pt-BR-Neural2-B (locale pt-BR); configured in 1 place(s)
- **-** — 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

