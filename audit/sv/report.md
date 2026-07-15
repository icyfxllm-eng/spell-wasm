# Swedish content audit — `sv`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 6 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (1)**

- `assets/words/sv/easy.txt:23` [english-contamination] 'hand' also appears in the English word lists — confirm it's a legitimate sv word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 3.45 | 3.0 | 0% | 0% |
| medium | 40 | 5.1 | 5.0 | 0% | 0% |
| hard | 40 | 6.75 | 7.0 | 0% | 65% |
| expert | 40 | 9.5 | 9.0 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] sv voice = sv-SE-Wavenet-C (locale sv-SE); configured in 1 place(s)
- `-:-` [audio-coverage] 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/sv/:-` [homophones-manual] Automated homophone detection is Spanish-only. Swedish has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/sv/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/sv.txt:-` [filter-layers] sv seed layer: 39 terms. Curation scan below is language-scoped (sv seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (5)**

- `src/i18n/locales/sv.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/sv.json:level.expert` [untranslated] value identical to English: 'Expert'
- `src/i18n/locales/sv.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/sv.json:btn.definition` [untranslated] value identical to English: '📖 Definition'
- `src/i18n/locales/sv.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

