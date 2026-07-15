# Japanese content audit — `ja`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 2 warning · 5 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 42 | 1.9 | 2.0 | 0% | 0% |
| medium | 42 | 2.17 | 2.0 | 0% | 0% |
| hard | 42 | 3.36 | 3.0 | 0% | 0% |
| expert | 41 | 4.73 | 5 | 0% | 12% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] ja voice = ja-JP-Wavenet-B (locale ja-JP); configured in 1 place(s)
- `-:-` [audio-coverage] 167 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/ja/:-` [homophones-manual] Automated homophone detection is Spanish-only. Japanese has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/ja/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/ja.txt:-` [filter-layers] ja seed layer: 176 terms. Curation scan below is language-scoped (ja seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (2)**

- `src/i18n/locales/ja.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/ja.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'

