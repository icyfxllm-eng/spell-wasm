# CC-AUDIO-CLARITY v1.1 — §0 census

Ran 2026-09-22 against main at 62f5e908 (live build 240). Read-only: nothing
in the audio pipeline was changed. Eric confirmed the file is signed on
2026-09-22 — D0, D9 and D10 carry his signature in the spec; the Status line at
the top of the file still reads REVIEW-GATED and should be updated.

**Three HALT conditions fired: C1, C10, C11.** C10 is the serious one — it is a
shipped fairness bug, and it is worse than the spec anticipates, because the
premise it rests on (that each language's bank grades ONE variety) is false.

## C1 — Pipeline map — HALT

One resolver, as CC-BUILD219-FIXES requires: `api::play_word_with` ->
`play_chain` over `source_order()`. No second resolver exists.

The order is `[Human, Pack, ServerCache, NativeTts]` (`api.rs:113`).

**The HALT:** `NativeTts` is a system-voice fallback — the device's own TTS,
last in the chain. Mandarin already removes it (a system voice cannot be handed
a reading, so it would guess a polyphone). It matters here because **F2 can
never gate it**: there is no cached clip to transcribe, no clip hash to key a
verdict on, and the voice differs per device and OS version. I1 ("no unheard
word") and I13 ("no unchecked voice") cannot hold while it is in the chain.

Three ways out, none of them mine to choose:
1. drop `NativeTts` from the order once server/pack coverage is proven per
   language — the strictest reading of I1, and it makes "audio unavailable"
   more common;
2. keep it, and write an explicit exception into I1/I13 saying the rescue
   voice is ungated by design;
3. keep it but suppress it for languages whose gated coverage is complete.

## C2 — Formats

| Provider | Languages | As synthesized | As cached and served |
|---|---|---|---|
| Google TTS | 14 (see C5) | MP3, 24 kHz, `volume_gain_db=4.0`, `effects_profile_id=["headphone-class-device"]` | same file, no re-encode |
| Azure Speech | sw | `audio-24khz-48kbitrate-mono-mp3` | same file, no re-encode |

Against D3 (AAC-LC mono >=24 kHz >=64 kbps): the sample rate clears the floor
everywhere, the **codec is MP3, not AAC-LC**, and **Azure's 48 kbps is under
the 64 kbps floor**. Google's MP3 bitrate is not set explicitly and needs
measuring off a real clip.

**`effects_profile_id=["headphone-class-device"]` is worth Eric's attention.**
It is an EQ profile applied by Google at synthesis. F1 says only padding and
resampling may touch a clip; this is neither. It is also tuned for headphones,
while most players are on a phone speaker. It is a plausible contributor to the
"half" report and it can be changed in one line — but changing it invalidates
every cached clip, so it belongs to the Phase A re-cache, not before it.

Also relevant: `SPEAKING_RATE_NORMAL = 0.85`. The "normal" clip is already
synthesized at 85% speed.

## C3 — Trimmer: none found

No silence trimming anywhere in the served path. No ffmpeg, sox or VAD step
exists in the backend; the synthesized bytes are written to the cache and
served unchanged. **The spec's first hypothesis is not supported by the code.**
That raises the weight of the other two (onset, bandwidth) and of C2's EQ
profile.

## C4 — Onset

iOS activates its audio session at launch, not at the orb press:
`AVAudioSession.setCategory(.playback)` + `setActive(true)` in
`AppDelegate.swift:13`. The app also preloads clips ahead of use through the
native plugin, capped at 8 (`audio-native.js:95`), and `api::preload_word`
warms the next word.

So F1 step 3 is largely already true on iOS. Whether the first 50–150 ms
survives is still a measurement (A-3), not an assumption.

## C5 — Voices

Google (`LANG_VOICES`): en en-US-Neural2-D · es es-ES-Neural2-B · fr
fr-FR-Neural2-A · de de-DE-Neural2-B · pt pt-BR-Neural2-B · pl pl-PL-Wavenet-B
· ru ru-RU-Wavenet-D · vi vi-VN-Wavenet-A · ko ko-KR-Wavenet-A · ja
ja-JP-Wavenet-B · fil fil-PH-Wavenet-A · zh cmn-CN-Wavenet-A · ar
ar-XA-Wavenet-B · hi hi-IN-Neural2-A.

Azure (`AZURE_VOICES`): sw sw-TZ-RehemaNeural. The file notes sw-KE voices as
the alternative, which is a live variety question for Swahili.

Candidate alternates for F4's bake-off are not enumerated here: they are a
provider API listing, not a repo fact, and pulling them is Phase C work.

## C9 — Volume

89,634 cached MP3s: 44,897 `normal`, 44,736 `slow`, 1 `sentence`. Plus ~1,070
cached meaning JSONs. The cache key carries a version (`CACHE_VERSION = "v3"`),
so F1 step 5's "old clips cannot be served" mechanism already exists — bump it
and everything regenerates.

Per-language counts are not derivable: the filename is
`{version}_{md5(key)}_{variant}.mp3` and the language lives inside the hashed
key. Getting them needs a pass over the bank, re-deriving each key.

## C10 — Bank variety — HALT (corrected 2026-09-22)

The census asks which variety each bank grades. **No bank declares one, and at
least two contain more than one.** That part stands.

English (3,170 tier words), voice en-US, before the fix:
- US: color, favor, labor, center, defense, license, gray, program, jewelry,
  behavior, neighbor
- GB: centre, theatre, grey, travelled, behaviour
- Both members of the same pair: center/centre, gray/grey, behavior/behaviour

Portuguese (6,112), voice pt-BR: esporte beside **desporto**, and **equipa**
where Brazil says equipe.

### What I got wrong, and the correction

I first reported this as a live fairness bug: "a player hears /ˈsɛntər/, both
spellings are in the bank, only one is the answer, and nothing says which."
**That is not true, and I repeated it several times before checking.**

Every one of those English pairs is already in the collision table as an
accept-all set — `center centre`, `gray grey`, `behavior behaviour`,
`theater theatre`, `traveled travelled` — and `game.rs` runs
`homophones::accepts` on every English answer (CC-SENSE-CUE F2(a)). A player
typing either spelling was already accepted. The layer built for this was
already doing its job.

The Portuguese case was never ambiguous either: desporto and esporte do not
sound alike, so a player hears one word and types that word.

### What the real problem is

Not ambiguity. **Variety drift**: a US voice asking for "theatre", and a
Brazilian player being taught "desporto". That breaks D17, which requires a
voice's variety to match the variety its bank grades, and it is a teaching
quality issue rather than an unwinnable item. Worth fixing; not the emergency I
called it.

### What was done

`config/bank-variety.json` declares one variety per language, each matching the
voice already shipping (C5), because D17 requires them to agree and re-recording
every clip to match a bank instead would cost more than it is worth.
`scripts/bank-variety-check.mjs` holds the banks to it in the gate, with a
CURATED pair list — a first draft derived pairs from suffix rules and "proved"
that absence should be spelled absense.

Removed: en centre, theatre, grey, travelled, behaviour; pt desporto, equipa.
Their collision sets stay, so typing the other spelling is still accepted; the
bank simply no longer asks for the wrong variety.

Two words the check does NOT treat as European, though a marker list would:
**comboio** is ordinary Brazilian for a convoy and **elétrico** is the everyday
adjective. A first draft of the list would have deleted both.

## C11 — Round lifecycle — HALT

A round's outcome is recorded in more than one place:

| Path | Where |
|---|---|
| Correct | `game.rs::on_correct` (1799) |
| Wrong | `game.rs::on_wrong` (2543) |
| Wrong, Climb | `game.rs::on_wrong_climb` (2450) |
| Wrong, extra attempt | `game.rs::on_wrong_extra_attempt` (2432) |
| Daily tally | `daily.rs::record_result` (317) |
| Learner / review record | `review.rs::on_correct` (89) |
| Streak | `game.rs::bump_streak` (1768) |
| Shield | consumed around `game.rs:2474` |

F6a's void would have to be expressed at each of them, and I10 says a void must
leave no trace in ANY of score, shield, streak, missed words, confusion matrix,
learner record or leaderboard. Adding it path by path is how a leak happens.

**Recommendation for A2:** do not add a void branch to each path. Add a single
`round_voided` guard that the outcome paths consult before writing anything,
and make A-13 (byte-identical state after a void) the test that proves it. That
is a smaller change than eight edits and it fails loudly if a new path forgets.

## C7 — Recognizers

`whisper-cli` (whisper.cpp) is installed at `/opt/homebrew/bin/whisper-cli`.
Models are not in the repo and need fetching; which of the 15 languages each
model covers has to be measured, not assumed.

The second, independent recognizer D2 asks for already exists: the backend
calls Google STT (`app.py:712`, with `sampleRateHertz`), wired for the
voice-spell feature. No new contract is needed.

**No HALT:** the condition is "no second recognizer for more than 3 languages",
and Google STT covers all 15. Per-language whisper coverage is still open.

## C8 — Comparison keys

The bank's stored surface form, which differs by script:

| Language | Bank form | Key for comparison |
|---|---|---|
| en es fr de pt pl sw fil vi | Latin surface | surface, NFC, case-folded |
| ru | Cyrillic surface (автобус) | surface, with the signed ё/е equivalence |
| hi | Devanagari (अँधेरा) | surface |
| ar | Arabic, unvocalized (آلة) | surface; diacritics must be stripped from the recognizer side |
| ja | kana (あお) | surface; a recognizer may return kanji and will need mapping |
| ko | Hangul syllables (가게) | surface, NFC |
| zh | `pinyin|hanzi` (ai4|爱) | **hanzi**, not the pinyin the player types |

Two need a decision before F2 can score them: **ja**, where a recognizer
returning kanji for a kana entry is a false FAIL, and **zh**, where the
comparison key is the hanzi even though the typed answer is pinyin.

## C6 — Provider overrides

Not completed. It is a provider-capability question per language, and the spec
already names the known gaps (Google Chirp 3 HD custom pronunciations exclude
vi-vn and sw-ke; Azure lexicons are single-locale and case-sensitive).
Confirming it per language means provider API calls, which belong with Phase C
(F3), not with a read-only census.

## What this census recommends

1. **Answer C10 first.** It is the only finding that is hurting players today,
   and it blocks F4, D17 and F7's variety gate.
2. **Phase A's re-cache is the moment to settle C2** — codec, bitrate floor and
   the `headphone-class-device` profile all invalidate the cache, so decide
   them together and regenerate once.
3. **Do not build F6a path by path.** C11 says one guard, not eight edits.
4. **C3 changes the theory of the bug.** There is no trimmer. If "half" is
   still unclear after padding and the EQ decision, the cause is the voice or
   the encode, not a lost edge — which makes the Phase B measurement the real
   diagnosis rather than Phase A.

---

## Phase B pilot, 2026-09-22 — the first real measurement

A whisper.cpp model (`ggml-small.bin`, multilingual, 487 MB, at
`~/.cache/whisper/`) plus the Google STT the backend already wires makes F2's
two recognizers real. A 60-word English pilot ran: 40 words whose first or last
sound is a low-energy fricative, where the reported failure lives, and 20
ordinary controls.

**51 Pass, 5 Weak, 1 exempt, 3 Fail.** The verdicts are NOT shipped — see the
end of this section.

### The finding: word-final /f/ is heard as /v/

| word | whisper | Google STT |
|---|---|---|
| half | "Have" | "half" (second choice: "Haff") |
| leaf | "Leave" | "leave" |

`half` is the word the testers reported, and a machine hears it the same way.
`leaf` fails identically and nobody reported it. Meanwhile `fifth`, `thief`,
`hip`, `cat`, `dog` and `bridge` all pass, so this is not a weak recognizer and
not the pipeline: the padding is on these clips, and `thief` carries the same
fricative fine.

It is word-specific in en-US-Neural2-D, which points at **F3's pronunciation
override or F4's bake-off — Phase C**. Nothing Phase A could have done would
have fixed it, and C3 had already shown there was no trimmer to blame.

`hiss` deserves its own line: neither recognizer heard anything at all, Google
returning an empty transcript at 0.0 confidence. Nobody reported it. That is
what F2 is for.

### Two bugs in the harness, found by running it

1. **Digit forms.** Recognizers write "fifth" as `5th` and "seven" as `7`. That
   is a transcription convention, not a mishearing; scoring it FAIL would
   withhold clear clips. Normalisation now folds them.
2. **No homophone exemption.** F2 step 4 exempts collision-table words, and the
   harness did not implement it. Without it the gate withheld `for` (both
   recognizers heard "four") and `some` ("sum") — common words, clear clips,
   silenced by a test that did not know they were homophones.

### Why the pilot's verdicts are not shipped

After the exemption landed, `some` became exempt but **`for` stayed FAIL** —
and `for` IS in the collision table (`fir for fore four fur`), so it should
have been exempt too. The transcripts were not recorded, so the failure cannot
be explained after the fact.

A store containing a FAIL nobody can account for would withhold a common word
on evidence that cannot be checked, so the store ships EMPTY. The harness now
records what each recognizer heard beside every verdict, which is what makes a
verdict arguable instead of merely authoritative. The measurement is worth
re-running once that is in hand, over a fair sample rather than a
fricative-heavy one.
