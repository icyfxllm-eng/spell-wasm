# CC-VOICE-CLARITY v1 — §0 census

Run 2026-10-29, read-only. Repo state: `origin/main` at `8c98c5a5`. Cache measured: `~/spellgame-server/cache` (the live Mac mini cache, 175,428 `v4_*` clips plus 89,634 orphaned `v3_*`). Nothing was changed, rendered or installed.

## Headline

1. **Padding and a slow replay already exist.** CC-AUDIO-CLARITY v1.1 F1 shipped 200 ms lead and 150 ms trail silence on every clip, and the base game already has a Slow button backed by a server-rendered slow clip. F1 and F5 as written describe things that are partly built.
2. **F2 would speed words up, not slow them.** `SPEAKING_RATE_NORMAL` is `0.85` today. D7 says `0.9`. This is a conflict between the spec and the code.
3. **C3 does not support "the tail is cut in rendering".** Clips carry about 160 ms of trailing silence. `worm`, `main`, `were` and `made` are normal clips. The misheard final consonant is not explained by missing silence.
4. **Voices are app-assigned,** one per language (O1).
5. **There is no (word, voice) quarantine registry,** and the only quarantine is a bank-level move. F4.6 does not fit as written.

## C1. Render path

- **Where:** `backend/app.py`, `synthesize_to_cache`, route `/api/speak`. Runs on the Mac mini (`~/spellgame-server`, launchd `net.spellgame.backend`), not a Pi. Cache dir `AUDIO_CACHE_DIR=~/spellgame-server/cache`.
- **Endpoint:** Google Cloud TTS `synthesize_speech` for 13 languages, Azure Speech for `sw` and `fa` (`_synthesize_azure`), and a separate pinyin-keyed path for `zh`.
- **English voice:** `en-US-Neural2-E` (switched from `-D` on 2026-09-27).
- **Input type:** SSML. `<speak><break 200ms/>…word…<break 150ms/></speak>`, with an optional `<phoneme alphabet="ipa">` for a signed lexicon row.
- **Rate:** `SPEAKING_RATE_NORMAL = 0.85`, `SPEAKING_RATE_SLOW = 0.6`. Azure uses relative `-15%` / `-40%`.
- **Audio config:** MP3, 24,000 Hz, `volume_gain_db 4.0`, `effects_profile_id = headphone-class-device`.
- **Silence trim or normalisation:** none in the code. The padding is SSML breaks, not post-processing.
- **Cache key:** `md5("<word>|ipa:<hash if lexicon row>|voice:<voice, for en/de/ru/ko/fa only>")`, filename `<CACHE_VERSION>_<md5>_<variant>.mp3`. `CACHE_VERSION` is a single global (`v4`). Bumping it orphans every language's cache.
- **Not what F3 describes:** there is no `render_profile_version`. Adding one for English alone means a per-language key suffix like the existing voice suffix, not a global bump.

## C2. Voices

- **Active Google voices:** en `en-US-Neural2-E`, es `es-ES-Neural2-G`, fr `fr-FR-Neural2-F`, de `de-DE-Neural2-A`, pt `pt-BR-Neural2-B`, pl `pl-PL-Wavenet-B`, ru `ru-RU-Wavenet-A`, vi `vi-VN-Wavenet-A`, ko `ko-KR-Wavenet-D`, ja `ja-JP-Wavenet-B`, fil `fil-PH-Wavenet-A`, zh `cmn-CN-Wavenet-A`, ar `ar-XA-Wavenet-B`, hi `hi-IN-Neural2-A`.
- **Azure:** sw `sw-TZ-RehemaNeural`, fa `fa-IR-DilaraNeural` (a Farid option is planned).
- **O1 answer: the app assigns the voice per language.** There is no player voice picker for server voices. The only other voice route is the on-device fallback (AVSpeech / Web Speech). CC-FAMILY-VOICES V1 (Personal Voice, per profile) would make voices player-selectable, but it is not part of this file.

## C3. Tail measurement (English, `en-US-Neural2-E`, normal variant)

Method: all 3,158 English bank words (easy 763, medium 798, hard 800, expert 797 lines), cache key rebuilt as above, decoded with ffmpeg to 24 kHz mono, threshold −40 dBFS. All 3,158 clips were found. The script is not committed.

| | min | p5 | p25 | median | p75 | p95 | max |
|---|---|---|---|---|---|---|---|
| trailing silence (ms) | 133 | 145 | 156 | 164 | 172 | 185 | 210 |
| leading silence (ms) | 266 | 270 | 273 | 281 | 293 | 300 | 338 |
| clip length (ms) | 696 | 864 | 960 | 1080 | 1200 | 1392 | 1752 |

- **Trailing under 200 ms: 99.6%.** Under 150 ms: 13.4%. Under 100 ms: 0%.
- **Leading under 100 ms: 0%.**
- **Named words:**

| word | length | lead | trail |
|---|---|---|---|
| worm | 960 | 286 | 161 |
| main | 912 | 279 | 160 |
| were | 888 | 278 | 184 |
| made | 888 | 277 | 163 |

**Diagnosis.** The tail is not cut in rendering or caching: every clip has silence after its last audible frame, and it matches the 150 ms break. The 99.6% figure means the pad is shorter than I1's 200 ms, not that clips are truncated. Because the measure is time from the last frame above −40 dBFS, a quiet final /m/ or /n/ that decays below −40 dBFS early would also read as "silence". This census therefore cannot say whether the final consonant is audible. It shows only that no gross truncation happened.

The earlier CC-AUDIO-CLARITY note `audio_clarity/f3-padding-ruled-out-en.md` (2026-10-04) found the same thing for word onsets: padding of 0, 200 and 400 ms made no difference to `ban`, `bear`, `bay` and `ben`, and that run concluded the cause was the voice, not the padding. The final-consonant case may follow the same pattern. It is a hypothesis, not a finding.

**Against I1:** with a 150 ms trail break, I1 (≥200 ms below −50 dBFS) fails for 99.6% of clips today. Lengthening the trail break to 250 ms is within what SSML breaks can do, so the F1 post-processing route is not needed on Neural2-E.

## C4. Playback path

- **Three routes, in order (`parse_source_order` in `src/api.rs`):** server cache clip via the native-audio plugin (Capacitor `NativeAudio`, `audio-native.js`), the browser `<audio>` element, then on-device TTS (AVSpeech or Web Speech).
- **iOS audio session:** one owner, `ios/App/App/AudioSessionOwner.swift`. Category `.playback`, switched to `.playAndRecord` while the mic is in use and back after. `NativeAudio.configure({focus: true, fade: false})`, so no fade-out.
- **Tail-cut candidates:** I found none in normal play.
  - `api::stop()` pauses the current `<audio>` only on mode teardown.
  - `SpellAudio.playWord` stops the previous asset when a different one starts, which is expected.
  - Neither runs while a word is still playing.
- **Not verified:** a device-speaker playback test. A `.playAndRecord` to `.playback` switch racing a clip start is the one place a tail or onset could be lost, and it can only be checked on device.
- **Relevant to F5:** the native route cannot vary playback speed (`src/api.rs` comment: "NativeAudio can't vary playback speed… plays the clip at its natural speed"). Only the `<audio>` fallback honours `playbackRate`, and it already sets `preservesPitch`. A client time-stretch at 0.75× would not work on the main iOS path.

## C5. Loopback tooling

- **whisper.cpp:** `/opt/homebrew/bin/whisper-cli`, models `ggml-small.bin` and `ggml-large-v3.bin` in `~/.cache/whisper`.
- **Existing loopbacks:** `tools/soundalike_loopback.py` (SENSE-CUE F5, writes `assets/words/<lang>/soundalike-loopback.json` for en, es, ru), `tools/spell-aloud-loopback/loopback.py`, `tools/zh-tts-loopback.py`, `scripts/audio-bakeoff.mjs`.
- **What they do:** free decode, then a text comparison. None scores a given transcript's log-probability against the audio.
- **Forced scoring:** not available today. No `faster-whisper`, `openai-whisper` or `torch` in the system Python or in `~/spellgame-server/venv`; numpy is missing too.
- **Smallest path:** a new Python venv on the Mac mini with `faster-whisper` (CTranslate2), which exposes decoder logits for a forced transcript. I did not install it or check that pip can reach the network.

## C6. Pronunciation keys

- `assets/words/en/en_pron.json`: ARPAbet, stress stripped, all pronunciations per word, 17,307 keys, generated by `tools/en_growth/build_pron.py`.
- **Coverage of the English bank: 99.8%** (3,158 words; about 6 uncovered).
- Spot check: worm `W ER M`, were `W ER`, warm `W AO R M`, word `W ER D`, main `M EY N`, made `M EY D`, maim `M EY M`, mine `M AY N`, mane `M EY N`. The B3 examples are all derivable. `main` and `mane` have the same key, so the homophone exclusion works.
- Note: "any word in the English dictionary authority" in F4.1 needs a source; the 17,307 keys are the available dictionary.

## C7. Quarantine registry

- **What exists:** `assets/words-quarantine/<lang>/<tier>.txt` plus `assets/words-quarantine/manifest.csv` (columns `lang,word,tier,pos,class,evidence,date`).
- **What it is:** a bank-level move. A quarantined word leaves the bank for every voice and every mode. The IMPOSSIBLE-LAUNCH loopback quarantine (41 Expert rows, source `reports/impossible-bank-hygiene-from-loopback.csv`) used it. Its recon notes the move touches tier hashes, purity files, the sound-alike tables and pinned digests, so it is not cheap.
- **There is no (word, voice) registry.** F4.6 says "add reason codes to the existing registry" and "other voices are unaffected". The existing one cannot express a voice. Either F4.6 creates a second registry, which I8 forbids, or it extends the manifest with a voice column, which changes a file other tools read.
- **For Eric to decide before Phase B.**

## C8. Replay and listen limits

- **Base game:** Replay, Slow and the "can't hear" rescue buttons all go through `game::note_replay` (`src/lib.rs`). **A Slow button already exists** (`#slowBtn`, `game::replay_slow`), playing the server "slow" variant at rate 0.6, which is a separately rendered clip, not a client stretch.
- **Impossible:** `MAX_REPLAYS = 1` per word (`src/impossible/engine.rs`), with an assertion that no other file keeps its own copy.
- **Spelluzzle:** Listen is only on silent slots, costs the Codebreaker star and counts in the Par strokes (`src/spelluzzle/play.rs`, `par.rs`). The secret word is never heard early (test at `src/spelluzzle/tests.rs:619`).
- **Randospell:** an `ear` flag exists in `src/randospell/mod.rs` (the Ear choice, chosen before the word is drawn). I did not read its rules.
- **Quick Bee / Quick Bee Draw:** plays through `api::play_word`; a 60 second timer. No replay limit found.
- **Spell Jr:** `juniorPolicy` in `config/modes.json` controls tiers, not replays. No Jr replay rule found.
- **F5 consequence.** The spec's slow replay duplicates `replay_slow` in the base game. Where limits already apply, the existing path already obeys them. The new work is whatever other modes need, not a new control in the base game.

## C9. Cost

- English bank: 3,158 distinct words, 21,473 characters.
- Google Neural2 at $16 per million characters: about **$0.34 per voice** for the normal variant, about **$0.69** with the slow variant. Far under the $25 stop. Only one English voice is active.
- Time is the larger cost: the live cache must be re-warmed in place, and `bin/warm-run.sh` is rate-limited.

## C10. Coverage floors

I did not count them. Sources:
- Board Game: `min_pool` and `LONG_WORD_FLOOR` in `src/boardgame_pools.rs`, with an A8 test.
- Impossible: the ≥150 floor in the IMPOSSIBLE files.
- Randospell: the 20-cell floor in `src/randospell/`.
- English tier sizes: easy 763, medium 798, hard 800, expert 797.

Counting a quarantine against each floor needs the gate's output, so it belongs to Phase B (B2).

## Spec points that need Eric before Phase A

1. **F2 rate.** `0.9` is faster than today's `0.85`. Keep `0.85`, or deliberately speed up?
2. **F1 padding.** Do it as a longer SSML trail break (150 to 250 ms; lead is already 200 ms, spec asks 100 ms) rather than post-processing?
3. **F3 key.** A per-language profile suffix for English instead of a global `CACHE_VERSION` bump?
4. **F5.** Slow replay already exists, so the Phase C scope changes. Native playback cannot time-stretch.
5. **F4.6.** A (word, voice) quarantine is new. A voice-aware extension or a second registry?
6. **C3 is not a diagnosis.** If the real cause is the voice, padding and rate will not fix worm and main. The gate and the A3 listen test are what would show it.

## Not done

- No device-speaker playback test.
- Floors in C10 not counted.
- No forced-scoring tooling installed.
- Non-English clips were not measured (Phase D).
