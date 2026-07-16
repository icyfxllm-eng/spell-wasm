# French content audit — `fr`  (machine pass, REVIEW-GATED)

Totals: **0 critical · 0 violation · 30 warning · 5 info**

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

- `assets/words/profanity/fr.txt:-` [filter-layers] fr seed layer: 81 terms. Curation scan below is language-scoped (fr seed + universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses the 1821-term all-language union — that over-block is intentional for user imports.

## Feature 6 — UI localization completeness

**WARNING (28)**

- `src/i18n/locales/fr.json:top.theClimb` [untranslated] value identical to English: '🏔 The Climb'
- `src/i18n/locales/fr.json:level.expert` [untranslated] value identical to English: 'Expert'
- `src/i18n/locales/fr.json:btn.hearSlowly` [untranslated] value identical to English: 'Hear it slowly'
- `src/i18n/locales/fr.json:daily.progress` [untranslated] value identical to English: '🗓 {i}/{n} · ✓{c}'
- `src/i18n/locales/fr.json:import.dictSkipped` [untranslated] value identical to English: 'Skipped {d} not found in the dictionary.'
- `src/i18n/locales/fr.json:import.dictAllSkipped` [untranslated] value identical to English: 'None of those were found in the dictionary.'
- `src/i18n/locales/fr.json:import.langHint` [untranslated] value identical to English: 'These look like {lang}.'
- `src/i18n/locales/fr.json:so.entry` [untranslated] value identical to English: '🌍 Challenge a friend'
- `src/i18n/locales/fr.json:so.title` [untranslated] value identical to English: 'Challenge a friend'
- `src/i18n/locales/fr.json:so.blurb` [untranslated] value identical to English: 'Play the same words as a friend, on your own time. Create a match and share the code, or enter a friend's code. Winner has the most correct.'
- `src/i18n/locales/fr.json:so.create` [untranslated] value identical to English: 'Create a match'
- `src/i18n/locales/fr.json:so.creating` [untranslated] value identical to English: 'Creating…'
- `src/i18n/locales/fr.json:so.shareHint` [untranslated] value identical to English: 'Share this code with your friend, then start:'
- `src/i18n/locales/fr.json:so.play` [untranslated] value identical to English: 'Play your words'
- `src/i18n/locales/fr.json:so.codePh` [untranslated] value identical to English: 'CODE'
- `src/i18n/locales/fr.json:so.join` [untranslated] value identical to English: 'Join with a code'
- `src/i18n/locales/fr.json:so.close` [untranslated] value identical to English: 'Close'
- `src/i18n/locales/fr.json:so.refresh` [untranslated] value identical to English: 'Check again'
- `src/i18n/locales/fr.json:so.doneTitle` [untranslated] value identical to English: 'Submitted'
- `src/i18n/locales/fr.json:so.wonTitle` [untranslated] value identical to English: 'You won!'
- `src/i18n/locales/fr.json:so.wonMsg` [untranslated] value identical to English: 'You got more words right than your friend. Nice spelling!'
- `src/i18n/locales/fr.json:so.lostTitle` [untranslated] value identical to English: 'You lost'
- `src/i18n/locales/fr.json:so.lostMsg` [untranslated] value identical to English: 'Your friend got more words right this time. Rematch?'
- `src/i18n/locales/fr.json:so.tieTitle` [untranslated] value identical to English: 'It's a tie!'
- `src/i18n/locales/fr.json:so.tieMsg` [untranslated] value identical to English: 'You both spelled the same number correctly.'
- `src/i18n/locales/fr.json:so.waitingMsg` [untranslated] value identical to English: 'Your result is in. Waiting for your friend to finish…'
- `src/i18n/locales/fr.json:so.errNetwork` [untranslated] value identical to English: 'Couldn't reach the server. Check your connection.'
- `src/i18n/locales/fr.json:so.errGeneric` [untranslated] value identical to English: 'Something went wrong. Please try again.'

