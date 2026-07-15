# Norwegian content audit — `nb`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 8 warning · 5 info**

## Feature 1 — Word list integrity

**WARNING (2)**

- `assets/words/nb/hard.txt:5` [english-contamination] 'dinosaur' also appears in the English word lists — confirm it's a legitimate nb word
- `assets/words/nb/expert.txt:26` [english-contamination] 'restaurant' also appears in the English word lists — confirm it's a legitimate nb word

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 3.52 | 4.0 | 0% | 0% |
| medium | 40 | 4.8 | 5.0 | 0% | 0% |
| hard | 40 | 7.12 | 7.0 | 0% | 72% |
| expert | 40 | 9.85 | 9.5 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] nb voice = nb-NO-Wavenet-B (locale nb-NO); configured in 1 place(s)
- `-:-` [audio-coverage] 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/nb/:-` [homophones-manual] Automated homophone detection is Spanish-only. Norwegian has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/nb/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/nb.txt:-` [filter-layers] nb seed layer: 38 terms. Curation scan below is language-scoped (nb seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (6)**

- `src/i18n/locales/nb.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/nb.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/nb.json:btn.hint` [untranslated] value identical to English: 'Hint'
- `src/i18n/locales/nb.json:vs.start` [untranslated] value identical to English: 'Start match'
- `src/i18n/locales/nb.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'
- `src/i18n/locales/nb.json:daily.bestStreak` [untranslated] value identical to English: 'best: {n}'

