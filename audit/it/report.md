# Italian content audit — `it`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 6 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (2)**

- `assets/words/it/hard.txt:13` [english-contamination] 'computer' also appears in the English word lists — confirm it's a legitimate it word
- `assets/words/it/hard.txt:23` [english-contamination] 'hamburger' also appears in the English word lists — confirm it's a legitimate it word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 4.33 | 4.0 | 0% | 0% |
| medium | 40 | 6.4 | 6.0 | 0% | 48% |
| hard | 40 | 8.45 | 8.0 | 0% | 100% |
| expert | 40 | 11.2 | 11.0 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] it voice = it-IT-Neural2-A (locale it-IT); configured in 1 place(s)
- `-:-` [audio-coverage] 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/it/:-` [homophones-manual] Automated homophone detection is Spanish-only. Italian has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/it/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/it.txt:-` [filter-layers] it seed layer: 156 terms. Curation scan below is language-scoped (it seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1765-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (4)**

- `src/i18n/locales/it.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/it.json:ph.email` [untranslated] value identical to English: 'email'
- `src/i18n/locales/it.json:ph.password` [untranslated] value identical to English: 'password'
- `src/i18n/locales/it.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

