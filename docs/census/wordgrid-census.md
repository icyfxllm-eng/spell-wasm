# CC-WORDGRID §3 census

Run 2026-09-18 on branch `cc-wordgrid`, before any feature code. E1, E2, E4 and
E7 are measured by `cargo test --lib wordgrid_census -- --ignored --nocapture`
(`src/wordgrid_census.rs`), which reads the real banks through the game's own
accessors, the keyboard SSOT files in `assets/keyboards/`, and segments with the
same grapheme library the game uses. E3, E5 and E6 cannot be measured from this
repo; each says why rather than guessing.

## Per language

"Pool" counts words per tier whose every cell is typeable. It is **audio-agnostic**
— E3 is unmeasured — so the audio-served pool can only be the same or smaller.

| Lang | E1 script | E2 typeable (fail %) | Cells needing >1 press | E4 pool easy / medium / hard / expert (≥400?) | E3 audio | E5 decoys | E6 dictionary | E7 blocklist | Outcome |
|---|---|---|---|---|---|---|---|---|---|
| **en** | pass | 3170/3170 (0%) | none | 768 / 801 / 801 / 800 ✓ | unmeasured | **fail** | **weak** | yes | **stop** |
| **es** | pass | 6097/6097 (0%) | none | **268** / 877 / 1974 / 2978 | unmeasured | **fail** | **fail** | yes | **stop** |
| **ru** | pass (Cyrillic) | 6210/6210 (0%) | none | **267** / 970 / 1978 / 2995 | unmeasured | **fail** | **fail** | **none** | **stop** |
| fr | pass | 6109/6109 (0%) | none | **279** / 889 / 1967 / 2974 | unmeasured | fail | fail | yes | not eligible (E4) |
| de | pass | 6144/6144 (0%) | none | **272** / 954 / 1967 / 2951 | unmeasured | fail | fail | yes | not eligible (E4) |
| pt | pass | 6112/6112 (0%) | none | **272** / 908 / 1966 / 2966 | unmeasured | fail | fail | yes | not eligible (E4) |
| pl | pass | 6175/6175 (0%) | none | **267** / 961 / 1969 / 2978 | unmeasured | fail | fail | yes | not eligible (E4) |
| fil | pass | 4083/4083 (0%) | none | **247** / 701 / 1159 / 1976 | unmeasured | fail | fail | yes | not eligible (E4) |
| sw | pass | 2845/2845 (0%) | none | **262** / 703 / 1180 / 700 | unmeasured | fail | fail | **none** | not eligible (E4, E7) |
| vi | pass | 3245/4094 (**20.7%**) | tone marks not on the keyboard | **58** / 492 / 993 / 1702 | unmeasured | fail | fail | yes | not eligible (E2, E4) — **stop (>5% E2)** |
| hi | **question** (LTR abugida) | 48/2674 (**98.2%**) | कृ गृ तृ … (matra composition) | **22 / 15 / 4 / 7** | unmeasured | fail | fail | **none** | not eligible — **stop (>5% E2, multi-press)** |
| ja | **question** (LTR syllabary) | 6160/6160 (0%) | none | **253** / 967 / 1970 / 2970 | unmeasured | fail | fail | yes | not eligible (E4); E1 needs your ruling |
| ar | **fail** (RTL) | 6238/6238 (0%) | none | 290 / 968 / 1987 / 2993 | unmeasured | fail | fail | **none** | not eligible (E1, E7) |
| ko | excluded (D4) | 0/6234 | jamo composition | — | — | — | — | yes | excluded |
| zh | excluded (D4) | — | — | — | — | — | — | yes | excluded |

## The gates this repo cannot pass or measure

- **E5 — there is no language confusion matrix.** The only confusion data in the
  app is `reports::confusion_pairs(lang)` (`src/reports.rs:182`), which is built
  from *the player's own* miss log. D6 forbids exactly that as a decoy source. So
  every language fails E5 until a language-level matrix exists — built from
  pooled, anonymous data or authored by hand. Pooling across players would be new
  data collection, against the app's zero-telemetry posture, so it is your call.
  `TRAP_MISS` does not exist either.
- **E3 — audio is rendered on demand.** The resolver plays packs, then the live
  server's TTS, then on-device speech. No manifest records which words have audio,
  so the only way to measure ≥98% is to request every bank word from the live
  server: tens of thousands of text-to-speech renders, with real cost. Not done
  without your go-ahead.
- **E6 — no external wordlist for es or ru.** en has `/usr/share/dict/web2`
  (235,976 entries), but it is a 1913 dictionary with no plurals ("cats",
  "emails" are absent), so it cannot prove a decoy like *runs* is not a word.
  es and ru have nothing beyond their own banks.
- **E7 — no Russian profanity blocklist.** `assets/words/profanity/` has no
  `ru.txt` (nor sw, hi, ar), and Russian is a launch language.
- **E4 — every language but English fails at Easy.** The Easy tiers hold 247–290
  words against the 400 the no-repeat window needs; English holds 768.

## Other findings

- **Dependency specs not in the repo:** CC-BUILD219-FIXES, CC-SENSE-CUE,
  CC-RU-ORTHO, CC-PLAYER-CONTRACT. The code they describe partly exists
  (`src/sense_cue.rs`; `api::play_word` as the one resolver, per the precedent
  signed for Translate); `ambiguous_positions` does not, which F-C2's fallback
  already allows for.
- **Property harness exists:** `proptest` is already a dependency, so §9 needs no
  new one.
- **Platforms:** iOS, web and an `android/` project are all in the repo, so I8
  can be tested across the three.

## Stop conditions that fired

1. en, es and ru each fail at least one gate (E5 for all three; E6 for es and ru,
   weak for en; E4 at Easy for es and ru; E7 for ru; E3 unmeasured for all).
2. More than 5% of a bank fails E2: vi (20.7%) and hi (98.2%).
3. Cells that need more than one key press beyond handled diacritics: hi
   (vowel-sign composition) and vi (tone marks).
