# German content audit — `de`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 8 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (5)**

- `assets/words/de/easy.txt:12` [english-contamination] 'Hand' also appears in the English word lists — confirm it's a legitimate de word
- `assets/words/de/easy.txt:22` [english-contamination] 'Wind' also appears in the English word lists — confirm it's a legitimate de word
- `assets/words/de/easy.txt:23` [english-contamination] 'Wolf' also appears in the English word lists — confirm it's a legitimate de word
- `assets/words/de/hard.txt:5` [english-contamination] 'Computer' also appears in the English word lists — confirm it's a legitimate de word
- `assets/words/de/hard.txt:33` [english-contamination] 'Restaurant' also appears in the English word lists — confirm it's a legitimate de word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 4.08 | 4.0 | 0% | 0% |
| medium | 40 | 5.83 | 6.0 | 0% | 28% |
| hard | 40 | 8.6 | 9.0 | 0% | 98% |
| expert | 46 | 12.26 | 12.0 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] de voice = de-DE-Neural2-B (locale de-DE); configured in 1 place(s)
- `-:-` [audio-coverage] 166 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/de/:-` [homophones-manual] Automated homophone detection is Spanish-only. German has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/de/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/de.txt:-` [filter-layers] de seed layer: 66 terms. Curation scan below is language-scoped (de seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (3)**

- `src/i18n/locales/de.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/de.json:btn.definition` [untranslated] value identical to English: '📖 Definition'
- `src/i18n/locales/de.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

