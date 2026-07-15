# Dutch content audit — `nl`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 11 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (5)**

- `assets/words/nl/easy.txt:22` [english-contamination] 'hand' also appears in the English word lists — confirm it's a legitimate nl word
- `assets/words/nl/medium.txt:3` [english-contamination] 'wind' also appears in the English word lists — confirm it's a legitimate nl word
- `assets/words/nl/medium.txt:4` [english-contamination] 'wolf' also appears in the English word lists — confirm it's a legitimate nl word
- `assets/words/nl/hard.txt:7` [english-contamination] 'computer' also appears in the English word lists — confirm it's a legitimate nl word
- `assets/words/nl/hard.txt:26` [english-contamination] 'restaurant' also appears in the English word lists — confirm it's a legitimate nl word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 3.7 | 4.0 | 0% | 0% |
| medium | 43 | 5.3 | 5 | 0% | 5% |
| hard | 40 | 8.12 | 8.0 | 0% | 92% |
| expert | 43 | 11.95 | 12 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] nl voice = nl-NL-Wavenet-B (locale nl-NL); configured in 1 place(s)
- `-:-` [audio-coverage] 166 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/nl/:-` [homophones-manual] Automated homophone detection is Spanish-only. Dutch has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/nl/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/nl.txt:-` [filter-layers] nl seed layer: 165 terms. Curation scan below is language-scoped (nl seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (6)**

- `src/i18n/locales/nl.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/nl.json:level.expert` [untranslated] value identical to English: 'Expert'
- `src/i18n/locales/nl.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/nl.json:btn.hint` [untranslated] value identical to English: 'Hint'
- `src/i18n/locales/nl.json:vs.start` [untranslated] value identical to English: 'Start match'
- `src/i18n/locales/nl.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

