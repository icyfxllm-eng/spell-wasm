# Filipino (fil) native-speaker auditor packet

Machine checks are done; these need a human who reads the language.

## Cross-language profanity (valid here, on another language's seed — kept, not flagged)  (3)

- **1** — 'aso' is a valid fil word but is on the profanity seed for: ['nl']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- **16** — 'guro' is a valid fil word but is on the profanity seed for: ['en']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- **23** — 'pinto' is a valid fil word but is on the profanity seed for: ['pt']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.

## Homophones / same-sound spellings — list any (grading can then accept-any via assets/words/fil/homophones.txt)  (1)

- **-** — Automated homophone detection is Spanish-only. Filipino has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/fil/homophones.txt).

## English cognates to confirm  (0)


## Open questions  (3)

- **-** — No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.
- **LANG_VOICES** — fil voice = fil-PH-Wavenet-A (locale fil-PH); configured in 1 place(s)
- **-** — 184 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

