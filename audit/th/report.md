# Thai content audit — `th`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 2 warning · 5 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = base clusters):

| tier | count | mean len | median len | tone-mark% | complex-orth% |
|--|--|--|--|--|--|
| easy | 41 | 2.29 | 2 | 7% | 22% |
| medium | 41 | 3.15 | 3 | 58% | 37% |
| hard | 41 | 4.66 | 4 | 46% | 46% |
| expert | 41 | 6.02 | 6 | 24% | 58% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] th voice = th-TH-Neural2-C (locale th-TH); configured in 1 place(s)
- `-:-` [audio-coverage] 164 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/th/:-` [homophones-manual] Automated homophone detection is Spanish-only. Thai has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/th/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/th.txt:-` [filter-layers] th seed layer: 31 terms. Curation scan below is language-scoped (th seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1689-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (2)**

- `src/i18n/locales/th.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/th.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

