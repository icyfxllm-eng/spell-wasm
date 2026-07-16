# Filipino content audit — `fil`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 33 warning · 8 info**

## Feature 1 — Word list integrity

**0 findings.**

## Feature 2 — Difficulty tier calibration

Tier stats (len = characters):

| tier | count | mean len | median len | n/a% | long% |
|--|--|--|--|--|--|
| easy | 46 | 3.98 | 4.0 | 0% | 0% |
| medium | 46 | 5.11 | 5.0 | 0% | 0% |
| hard | 46 | 6.43 | 6.0 | 0% | 44% |
| expert | 46 | 9.15 | 9.0 | 0% | 100% |


**INFO (1)**

- `src/consts.rs:-` [min-pool-undefined] No minimum-pool-size constant exists in code; the 'above minimum pool' invariant has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). OPEN QUESTION for Eric: define the min tier pool size.

## Feature 3 — Audio & TTS config

**INFO (2)**

- `backend/app.py:LANG_VOICES` [tts-voice] fil voice = fil-PH-Wavenet-A (locale fil-PH); configured in 1 place(s)
- `-:-` [audio-coverage] 184 unique words emitted to audio-manifest.txt. Cache coverage lives on the server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed (probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.

## Feature 4 — Homophone / hearing-ambiguity map

**INFO (1)**

- `assets/words/fil/:-` [homophones-manual] Automated homophone detection is Spanish-only. Filipino has real same-sound/different-spelling homophones that a spell-by-ear player could miss, but flagging them needs a native speaker + phonetic transcription. AUDITOR TASK: list any list words whose pronunciation matches another common word, so grading can accept-any (the accept-any mechanism in src/homophones.rs already supports any language via assets/words/fil/homophones.txt).

## Feature 5 — Profanity filter coverage

**INFO (4)**

- `assets/words/profanity/fil.txt:-` [filter-layers] fil seed layer: 93 terms. Curation scan below is language-scoped (fil seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1821-term all-language union — that over-block is intentional for user imports.
- `assets/words/fil/easy.txt:1` [cross-lang-profanity] 'aso' is a valid fil word but is on the profanity seed for: ['nl']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- `assets/words/fil/easy.txt:16` [cross-lang-profanity] 'guro' is a valid fil word but is on the profanity seed for: ['en']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.
- `assets/words/fil/medium.txt:23` [cross-lang-profanity] 'pinto' is a valid fil word but is on the profanity seed for: ['pt']. Not flagged for fil (kept per decision addendum). Note: it stays blocked in free-text usernames via the global/English path.

## Feature 6 — UI localization completeness

**WARNING (33)**

- `src/i18n/locales/fil.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/fil.json:btn.hearSlowly` [untranslated] value identical to English: 'Hear it slowly'
- `src/i18n/locales/fil.json:kb.enter` [untranslated] value identical to English: 'Enter'
- `src/i18n/locales/fil.json:settings.readable` [untranslated] value identical to English: 'Readable mode'
- `src/i18n/locales/fil.json:settings.background` [untranslated] value identical to English: 'Background'
- `src/i18n/locales/fil.json:ph.username` [untranslated] value identical to English: 'username'
- `src/i18n/locales/fil.json:ph.email` [untranslated] value identical to English: 'email'
- `src/i18n/locales/fil.json:ph.password` [untranslated] value identical to English: 'password'
- `src/i18n/locales/fil.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'
- `src/i18n/locales/fil.json:import.dictSkipped` [untranslated] value identical to English: 'Skipped {d} not found in the dictionary.'
- `src/i18n/locales/fil.json:import.dictAllSkipped` [untranslated] value identical to English: 'None of those were found in the dictionary.'
- `src/i18n/locales/fil.json:import.langHint` [untranslated] value identical to English: 'These look like {lang}.'
- `src/i18n/locales/fil.json:so.entry` [untranslated] value identical to English: '🌍 Challenge a friend'
- `src/i18n/locales/fil.json:so.title` [untranslated] value identical to English: 'Challenge a friend'
- `src/i18n/locales/fil.json:so.blurb` [untranslated] value identical to English: 'Play the same words as a friend, on your own time. Create a match and share the code, or enter a friend's code. Winner has the most correct.'
- `src/i18n/locales/fil.json:so.create` [untranslated] value identical to English: 'Create a match'
- `src/i18n/locales/fil.json:so.creating` [untranslated] value identical to English: 'Creating…'
- `src/i18n/locales/fil.json:so.shareHint` [untranslated] value identical to English: 'Share this code with your friend, then start:'
- `src/i18n/locales/fil.json:so.play` [untranslated] value identical to English: 'Play your words'
- `src/i18n/locales/fil.json:so.codePh` [untranslated] value identical to English: 'CODE'
- `src/i18n/locales/fil.json:so.join` [untranslated] value identical to English: 'Join with a code'
- `src/i18n/locales/fil.json:so.close` [untranslated] value identical to English: 'Close'
- `src/i18n/locales/fil.json:so.refresh` [untranslated] value identical to English: 'Check again'
- `src/i18n/locales/fil.json:so.doneTitle` [untranslated] value identical to English: 'Submitted'
- `src/i18n/locales/fil.json:so.wonTitle` [untranslated] value identical to English: 'You won!'
- `src/i18n/locales/fil.json:so.wonMsg` [untranslated] value identical to English: 'You got more words right than your friend. Nice spelling!'
- `src/i18n/locales/fil.json:so.lostTitle` [untranslated] value identical to English: 'You lost'
- `src/i18n/locales/fil.json:so.lostMsg` [untranslated] value identical to English: 'Your friend got more words right this time. Rematch?'
- `src/i18n/locales/fil.json:so.tieTitle` [untranslated] value identical to English: 'It's a tie!'
- `src/i18n/locales/fil.json:so.tieMsg` [untranslated] value identical to English: 'You both spelled the same number correctly.'
- `src/i18n/locales/fil.json:so.waitingMsg` [untranslated] value identical to English: 'Your result is in. Waiting for your friend to finish…'
- `src/i18n/locales/fil.json:so.errNetwork` [untranslated] value identical to English: 'Couldn't reach the server. Check your connection.'
- `src/i18n/locales/fil.json:so.errGeneric` [untranslated] value identical to English: 'Something went wrong. Please try again.'

