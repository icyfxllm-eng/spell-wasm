# French content audit — `fr`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 5 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (2)**

- `assets/words/fr/hard.txt:32` [english-contamination] 'restaurant' also appears in the English word lists — confirm it's a legitimate fr word
- `assets/words/fr/expert.txt:21` [english-contamination] 'occurrence' also appears in the English word lists — confirm it's a legitimate fr word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 4.17 | 4.0 | 0% | 0% |
| medium | 40 | 6.35 | 6.0 | 0% | 45% |
| hard | 40 | 9.25 | 9.0 | 0% | 98% |
| expert | 40 | 10.2 | 10.0 | 0% | 98% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] fr voice = fr-FR-Neural2-A (locale fr-FR); configured in 1 place(s)
- `-:-` [audio-coverage] 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/fr/:-` [homophones-manual] Automated homophone detection is Spanish-only. French has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/fr/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/fr.txt:-` [filter-layers] fr seed layer: 80 terms. Curation scan below is language-scoped (fr seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (3)**

- `src/i18n/locales/fr.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/fr.json:level.expert` [untranslated] value identical to English: 'Expert'
- `src/i18n/locales/fr.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

