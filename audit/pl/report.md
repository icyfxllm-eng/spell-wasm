# Polish content audit — `pl`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 3 warning · 5 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 4.15 | 4.0 | 0% | 0% |
| medium | 43 | 5.37 | 6 | 0% | 19% |
| hard | 40 | 7.88 | 8.0 | 0% | 90% |
| expert | 44 | 10.34 | 10.0 | 0% | 93% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] pl voice = pl-PL-Wavenet-B (locale pl-PL); configured in 1 place(s)
- `-:-` [audio-coverage] 167 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/pl/:-` [homophones-manual] Automated homophone detection is Spanish-only. Polish has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/pl/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/pl.txt:-` [filter-layers] pl seed layer: 42 terms. Curation scan below is language-scoped (pl seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1765-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (3)**

- `src/i18n/locales/pl.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/pl.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/pl.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

