# Korean content audit — `ko`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 4 warning · 5 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 45 | 1.4 | 1 | 0% | 0% |
| medium | 44 | 1.73 | 2.0 | 0% | 0% |
| hard | 44 | 2.48 | 2.5 | 0% | 0% |
| expert | 44 | 2.77 | 3.0 | 0% | 0% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] ko voice = ko-KR-Wavenet-A (locale ko-KR); configured in 1 place(s)
- `-:-` [audio-coverage] 177 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/ko/:-` [homophones-manual] Automated homophone detection is Spanish-only. Korean has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/ko/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (1)**

- `assets/words/profanity/ko.txt:-` [filter-layers] ko seed layer: 72 terms. Curation scan below is language-scoped (ko seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1768-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (4)**

- `src/i18n/locales/ko.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/ko.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'
- `src/keyboard.rs:153` [lang-branch] possible language-conditional on 'ko' outside the i18n layer: keyboard_locale(app) == "ko"
- `src/keyboard.rs:615` [lang-branch] possible language-conditional on 'ko' outside the i18n layer: if code == "ko" {

