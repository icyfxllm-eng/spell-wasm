# CC-VOICE-CLARITY v1 — Near-sound words and clipped final consonants

**Status:** REVIEW-GATED. Once Eric signs, only §0 recon may run, and it is read-only. Phases A–D stay blocked until Eric reviews the census.
**Sequencing:**
- Phases A and B are server-side and offline work. They need no app build and must not delay build 56.
- Phase C (app) is post-build-56.
- Phase D (other languages) runs after Phase B passes for English.
**Related files:**
- CC-SENSE-CUE v1.1 (homophone tables, voice loopback check)
- CC-SOUNDALIKE-BONUS v1
- CC-IMPOSSIBLE v1 / IMPOSSIBLE-COMPLETE / IMPOSSIBLE-LAUNCH (the replay cap, and the quarantine of the 41 loopback-flagged rows)
- CC-EN-EXPERT-GROWTH v1 (English pronunciation keys)
- CC-SPELLUZZLE v1.1 (Par boards: each Listen counts as a stroke)
- CC-RANDOSPELL v1.1 (Ear bet)
- CC-SPELL-KEYBOARD v1 (orb/replay key)
- CC-SECURITY-BASELINE v1 (child data)
- CC-BANK-PURITY v1 (quarantine-not-delete)

---

## Intent

Eric played rounds with the new voices and kept missing words he could spell. He heard **worm** as *were* and **main** as *made*. These are not homophones. They are near-sound pairs, and in both cases the **final consonant** was lost or blurred.

A spelling game that marks a player wrong for hearing correctly is broken at its root. The player must hear exactly one word.

This file:
1. Fixes clips that end too abruptly.
2. Gives final consonants room.
3. Adds a gate that measures, for each word in each voice, whether the clip can be told apart from its near-sound neighbours, and pulls the clips that can't.
4. Gives players a slow replay.
5. Turns misheard misses into evidence against the clip.

True homophones (sink/sync) are **out of scope**. CC-SENSE-CUE owns them.

## Constraints and non-goals

- Do not touch the SENSE-CUE / SOUNDALIKE-BONUS homophone tables, their read API, or their census report. This file reads them only.
- Do not change scoring, spaced-repetition math, Climb math, or any mode's win or loss rules.
- No new TTS vendor. No new voices. No LLM judging audio.
- No change to sentence or definition audio. Only single-word prompts are re-rendered.
- **Never lower a threshold to pass. Never pad a pool to cover a quarantine gap. Never delete a row.** Doing any of these means STOP and report.
- No silent fallbacks:
  - A word is never served in a different voice from the one the player chose.
  - A quarantined word is never swapped for another word without the existing pool logic choosing it.
- Strategy reversals or conflicts with another CC file: STOP and ask.

---

## §0 Recon (read-only on signature)

Write `reports/voice-clarity-census.md`. It must answer:

- **C1. Render path.** Where single-word clips are rendered and cached (Flask on the Pi?). Report:
  - TTS endpoint, voice IDs, input type (text / SSML / markup) and `speaking_rate`
  - audio encoding, sample rate and bitrate
  - any silence trim or normalisation step
  - the cache key format
- **C2. "New voices".** List every voice currently active per language. Say whether a player chooses the voice or the app assigns it per language. This answers O1.
- **C3. Tail measurement.** Decode every cached English single-word clip in each active voice and report:
  - trailing silence: the time from the last frame above −40 dBFS to the end of the file. Give the distribution and the % under 200 ms.
  - leading silence: the same measure from the start of the file.
  - `worm`, `main`, `were` and `made`, listed explicitly.
- **C4. Playback path.** How the client plays clips (HTMLAudioElement / Web Audio / native via Capacitor). Report any early `stop()`, `pause()`, fade or audio-session teardown that could cut the tail. Report the iOS audio-session category.
- **C5. Loopback tooling.**
  - What the existing whisper.cpp loopback and the SENSE-CUE voice loopback check can do.
  - Can either score a *given* transcript's log-probability against the audio (forced scoring), or only free-decode?
  - Name the smallest path to forced scoring, for example faster-whisper or openai-whisper in Python on the Mac Mini.
- **C6. Pronunciation keys.** The location and format of the English keys from CC-EN-EXPERT-GROWTH, and their coverage of the English bank (%).
- **C7. Quarantine registry.** Where the IMPOSSIBLE-LAUNCH loopback quarantine lives. Confirm this file can add reason codes to it rather than create a second registry.
- **C8. Replay and listen limits.** Every rule in every mode that limits, costs or bets on replaying audio. At minimum:
  - the Impossible replay cap
  - Spelluzzle Par strokes and silent and secret words
  - the Randospell Ear bet
  - Quick Bee timers
  - Spell Jr
- **C9. Cost.** Estimated TTS characters and dollars to re-render the English single-word cache once per active voice.
- **C10. Coverage floors.** The current floors that a quarantine could breach:
  - Randospell cells (20)
  - Impossible (≥150)
  - each tier's minimum in each mode

**C3 is the diagnosis.** If most clips have under 200 ms of trailing silence, the tail is being cut in rendering or caching. If C3 is clean but C4 shows early teardown, the cut happens in playback. Report which, with evidence.

---

## Features

### F1. Edge padding (server-side, Phase A)
**Intent:** no clip ends on a consonant with nothing after it. Small speakers, Bluetooth and audio-session wake-up all eat the edges of a clip.
**Behaviour:**
- After rendering, append 250 ms of digital silence at the end and prepend 100 ms at the start.
- This is a post-processing step, so it works whatever the voice accepts as input. Do not depend on SSML `<break>`: sources disagree on whether Chirp 3 HD accepts SSML.
- Any existing silence-trim step must run *before* padding, never after it.
- If C4 finds a client-side tail cut, fix that cut in Phase C. Padding does not excuse it.

### F2. Word-prompt rate (server-side, Phase A)
**Intent:** give final consonants time to land.
**Behaviour:**
- Single-word prompts render with `audioConfig.speakingRate = 0.9` (D7).
- Sentences and definitions keep their current rate.
- The rate is a field of the render profile.

### F3. Render-profile versioning (server-side, Phase A)
**Intent:** old clips are never served after the profile changes.
**Behaviour:**
- The cache key includes `render_profile_version`. This file sets it to `vc1`.
- Clips under the old profile are not deleted until Eric's ear check passes. Then they become eligible for cleanup under the normal cache policy.
- Re-rendering is English first. It is budgeted from C9.
  - If C9 exceeds $25 per voice: STOP and ask.

### F4. Confusability gate (offline tool, Phase B)
**Intent:** catch a bad clip before a player does.
**Runs on:** the Mac Mini or CI, never the Pi (D11).

**F4.1 Neighbours.** For each target word, build its set of real-word neighbours. A neighbour is any bank word in any tier, or any word in the English dictionary authority, whose pronunciation is exactly one phoneme away from the target's:
- one phoneme substituted,
- one phoneme deleted (including the final consonant), or
- one phoneme inserted.

Exclusions:
- **Distance-0 words (true homophones) are excluded.** SENSE-CUE owns them.
- Proper nouns and profanity are excluded.
- If the set has more than 12 neighbours, keep the 12 with the highest frequency.

Required examples:
- `worm` must include `were`, `warm` and `word`.
- `main` must include `made`, `maim` and `mine`.
- `mane` must not appear for `main`, because it is a homophone.

**F4.2 Phone simulation.** Score each clip twice:
- **raw**
- **phone-sim:** high-pass at 350 Hz (2nd order), then a small-speaker EQ and limiter. The filter chain is pinned in the tool and versioned.

The gate uses the **worse** of the two scores (D6).

**F4.3 Score.** Use forced scoring with whisper:
- `s(w)` = the length-normalised mean token log-probability of the transcript `w`, given the audio.
- `margin = s(target) − max over neighbours s(n)`.

Also run a free decode. **If the free decode equals a neighbour, the clip fails, whatever its margin.**

**F4.4 Threshold.** `τ` is calibrated in Phase B from Eric's blind listen set (A3), not chosen by hand.

Calibration rules:
- At the old render profile, `worm` and `main` **must fail**. If they pass, the gate is wrong: STOP and report.
- A provisional `τ` is recorded in the census. The final `τ` is written into the tool with the calibration data that produced it.

**F4.5 Re-render ladder.** For each clip that fails, within the **same voice**:
- R1: `vc1` profile (pad + 0.9)
- R2: rate 0.85
- R3: rate 0.8

The first rung that passes is cached. If the clip still fails at R3, it is quarantined (F4.6).

The ladder never changes voice (D2, and the no-silent-fallback rule).

**F4.6 Quarantine.**
- Add the row to the existing registry (C7) with:
  - reason `near_sound_unclear`
  - `voice_id`
  - `margin`
  - the neighbour that won
- A quarantined (word, voice) pair is never served in that voice in any mode.
- Other voices are unaffected.
- No row is deleted (D3).
- **If a quarantine drops any pool below a C10 floor: STOP and report.** Never pad.

**F4.7 Report.** Write `reports/voice-clarity-gate.md`. For each voice, report:
- clips scored
- passes at R0, R1, R2 and R3
- clips quarantined
- the 20 worst margins
- floor checks

### F5. Slow replay (app, Phase C)
**Intent:** a player who isn't sure gets a second, slower listen. It shows no letters, so it is not a hint.
**Behaviour:**
- A slow-replay control plays the current word at 0.75× with pitch preserved.
- v1 uses client-side time-stretch (`playbackRate = 0.75`, `preservesPitch = true`, or the native equivalent). No extra renders (D9).
- Placement: a second control beside replay. A code-drawn icon, per VISUAL-ECLIPSE V-D7's default. If SPELL-KEYBOARD has landed, it goes on the orb/replay key.
- **It obeys every limit found in C8:**
  - In Spelluzzle Par it costs a stroke, the same as Listen (D5).
  - Where Listen is unavailable (silent and secret words), slow replay is too.
  - It counts against Impossible's replay cap.
  - It never bypasses a replay limit in any mode.
- Any mode whose replay rules C8 could not determine (for example the Randospell Ear bet): the control is hidden there. STOP and ask.
- Spell Jr: available, free.

### F6. Ear-miss tagging (app + server, Phase C)
**Intent:** players' misses become evidence against bad clips. Graders' tags never change a player's outcome.
**Behaviour:**
- A wrong answer that exactly equals a gate neighbour of the target (NFC, case-folded) is logged as an `ear_miss` event: `{word, voice_id, typed_neighbour, render_profile_version}`.
- It is still a miss. Score, spaced repetition and streaks are unchanged (D1).
- **No user, device or profile identifier** is logged. The events are aggregate counters keyed only by (word, voice, neighbour), per CC-SECURITY-BASELINE.
- When a (word, voice) pair reaches ≥3 `ear_miss` events with clips passing under `vc1`, it is enqueued for re-gating (D10).
  - Counters cannot tell sessions apart, so 3 is a floor, not proof. The re-gate decides.

### F7. Other languages (Phase D)
**Intent:** the same protection everywhere, once each language has pronunciations to build neighbours from.
**Behaviour:**
- Extend F4 to each language whose SENSE-CUE table status is `verified`. Reuse that language's generator or phonemiser to build neighbours.
- Languages with status `missing` are skipped and listed in the report. No neighbour rules are invented here.
- Order follows SENSE-CUE v1.1's H8 language order.

---

## Invariants

- **I1.** Every served single-word clip has ≥200 ms below −50 dBFS after the last frame above −40 dBFS.
- **I2.** Every served single-word clip has a `render_profile_version` that is `vc1` or later.
- **I3.** No (word, voice) pair that is quarantined under `near_sound_unclear` is served in that voice.
- **I4.** The ladder never changes voice. A word is never served in a voice the player didn't choose or the app didn't assign.
- **I5.** Near-sound answers are never accepted as correct. The grading code path is untouched by this file.
- **I6.** Slow replay never gives more listens than a mode's existing rules allow.
- **I7.** `ear_miss` storage holds no identifiers.
- **I8.** One quarantine registry. No row deleted. No pool padded.
- **I9.** Homophone tables are unchanged. A byte diff shows zero changes.

---

## Decisions

**Signed by Eric ("Yes with the recommendations", 2026-10-29):**
- **D1.** Near-sound answers are never accepted. A miss stays a miss. True homophones stay with SENSE-CUE / SOUNDALIKE-BONUS.
- **D2.** The gate runs per voice, not per word. Every new voice gets a full gate pass before it is served.
- **D3.** Quarantine, never delete.
- **D4.** English first. Other languages follow as their SENSE-CUE tables reach `verified`.
- **D5.** Slow replay is free everywhere except Spelluzzle Par boards, where it costs a stroke like Listen. It obeys every other existing listen or replay limit.

**Defaults applied, unsigned (Eric can object):**
- **D6.** Gate on the worse of the raw and phone-sim scores.
- **D7.** Word-prompt rate 0.9. Pads of 250 ms trailing and 100 ms leading.
- **D8.** `τ` is calibrated from Eric's blind set. Changing it later requires a new calibration run, never a hand edit.
- **D9.** Slow replay uses client time-stretch at 0.75×. If Eric hears smearing in A3, switch to lazily rendered 0.75 clips (a new amendment, with a cost estimate).
- **D10.** 3 ear-miss events trigger a re-gate.
- **D11.** The gate runs on the Mac Mini or CI, never the Pi.
- **D12.** Neighbour cap of 12, chosen by frequency.
- **D13.** Re-render budget stop at $25 per voice.

**Open:**
- **O1.** Are voices player-chosen or assigned per language? C2 answers this. If they are player-chosen, a quarantined word is still served to players on other voices. Confirm that is acceptable.
- **O2.** Teaching on reveal: after a near-sound miss, play the target and the typed neighbour back to back? Default: not in v1 (it would be its own amendment).
- **O3.** Randospell Ear bet interaction with slow replay. Default: the control is hidden in Randospell until Eric rules.

---

## Phases

- **§0 Recon.** Read-only. Ends at the census. Eric reviews it.
- **Phase A. Edges + rate (server).**
  - F1–F3
  - English re-render under `vc1`
  - A3: Eric's blind ear check
- **Phase B. Gate (offline).**
  - F4: neighbours, phone-sim, scoring, calibration, ladder, quarantine, report
- **Phase C. App (post-build-56).**
  - F5 slow replay
  - F6 ear-miss tagging
  - any playback tail fix found by C4
- **Phase D. Languages.** F7.

---

## Done when

1. **A1.** `python tools/voice_clarity/tailcheck.py --lang en --profile vc1` reports 0 clips violating I1, in every active voice.
2. **A2.** A cache-key test confirms that no `vc0` clip is served when a `vc1` clip exists (I2).
3. **A3.** Eric's blind phone-speaker listen test on the device speaker with no earbuds, after Phase A:
   - 50 near-pair clips, shuffled, including `worm`, `main`, `were` and `made`.
   - He types what he hears.
   - **≥48/50 correct.**
   - The answers are saved as the gate's calibration set.
4. **B1.** `python tools/voice_clarity/gate.py --lang en --calibrate` meets all of:
   - `worm` and `main` fail at the old profile.
   - `τ` is written with its calibration data.
   - Every misheard A3 clip scores below `τ`.
5. **B2.** `gate.py --lang en --all-voices` writes the gate report.
   - Every quarantined pair is in the registry with its reason code.
   - Zero floor breaches, or the run STOPPED and reported them.
6. **B3.** Neighbour unit tests pass:
   - `worm` ⊇ {were, warm, word}
   - `main` ⊇ {made, maim, mine}
   - `mane` ∉ `main`
   - no proper nouns and no profanity in any neighbour set
7. **B4.** A serving test confirms that a quarantined (word, voice) pair is never drawn in that voice in any mode (I3).
8. **C1.** A mode matrix test confirms that slow replay obeys every C8 rule: it costs a Par stroke, is unavailable on silent words, and counts against the Impossible cap (I6).
9. **C2.** An `ear_miss` schema test confirms there are no identifier fields (I7). A grading regression test confirms near-sound answers are still marked wrong (I5).
10. **C3.** A byte diff of the homophone tables shows zero changes (I9).
11. **C4.** Eric's TestFlight pass: 3 rounds in English with the new voices, on the phone speaker. **No miss that he attributes to hearing.**

STOP and report if any check can only pass by lowering `τ`, skipping a voice, padding a pool, or changing grading.
