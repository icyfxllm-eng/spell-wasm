# Portuguese content audit — `pt`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 3 warning · 5 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 40 | 3.9 | 4.0 | 0% | 0% |
| medium | 40 | 6.15 | 6.0 | 0% | 35% |
| hard | 40 | 7.75 | 8.0 | 0% | 90% |
| expert | 40 | 10.65 | 10.0 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] pt voice = pt-BR-Neural2-B (locale pt-BR); configured in 1 place(s)
- `-:-` [audio-coverage] 160 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/pt/:-` [homophones-manual] Automated homophone detection is Spanish-only. Portuguese has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/pt/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/pt.txt:-` [filter-layers] pt seed layer: 65 terms. Curation scan below is language-scoped (pt seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (3)**

- `src/i18n/locales/pt.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/pt.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/pt.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

