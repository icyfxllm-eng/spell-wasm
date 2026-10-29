# CC-CLIP-REPORT -- "Can't tell what it says"

**Status:** BUILT, flag `cant_tell` **OFF by default** (2026-10-29). Nothing ships lit. No TestFlight build was cut. Everything below marked **CONFIRM** is Eric's call before the flag is turned on.

## Why

A 2026-10-29 recording of "coup" played the iPhone fallback voice and was unintelligible. Recognizer verdicts (`config/audio-verdicts.json`) are a guess; a player saying "I could not make this out" is better evidence, and it separates a bad server clip from the on-device fallback voice (the `source` field).

## What already existed (recon)

- **The rescue** (CC-AUDIO-CLARITY F6a): `Can't hear it` opens a panel with `Hear it slowly` and `Show me the word`. "Show me the word" is a **void**: `game::reveal_word` sets `answered` + `voided`, and both outcome sinks (`on_correct`, `finalize_incorrect_ex`) consult `round_voided` before writing anything, so no streak, miss, stat, shield or learner movement. The word is held out of that player's draws on-device (D15) and in the Daily the entry is replaced so the run keeps its length (D12). Caps (`void_cap`): Daily 2, Climb 1 (the counter `voids_used` is never reset, so these are per page load in practice), practice unlimited; past a cap the step turns into "Skip this word" = Give up, a real miss. It is **local and sends nothing** (I12).
- **Give up** (`game::give_up`) is a miss: `finalize_incorrect(GaveUp)`, streak to 0.
- **Replay counters:** `note_replay` only constrains Impossible (`MAX_REPLAYS = 1`); Impossible hides `Can't hear it` entirely (no aids).
- **Telemetry** (CC-TELEMETRY-FOUNDATION v1.1): crash (F1) and performance (F5) only, a closed schema with no string type. I3 says no word, answer, miss or attempt leaves the device. Routes: Standard = events, Jr / unknown age / Education = aggregate counts with no identifier, toggle off or kill switch = nothing. The server's own `/api/speak` log line also carries "no word".
- **Audio source** (`api::last_audio_source`): `server-cache`, `native-tts`, `human`, `pack` (and `web-speech`, `none`).
- **Mode exits:** `mode-exit-check.mjs` covers the end screens of the other modes (Impostor, Chains, Bee, ...). The play screen's rescue panel keeps `Back`; Give up is always there. The new step adds a way past a bad clip, never removes one.

## The design

"Can't tell" is a **third step inside the existing rescue panel** (not a new top-level button: the play screen is already crowded, and the panel is where a player who cannot hear lands). It is the existing void with two differences: it does **not** show the word, and it queues a report. Because it is the same void, it cannot score, cannot touch the streak, and cannot award credit; `void_round(app, reveal)` is the single implementation and `reveal_word` calls it too.

Strings: `rescue.cantTell` ("Can't make it out -- skip") and `rescue.cantTellDone` ("Skipped. Your streak is safe."), in all 15 locales with real translations (machine-made, so listed in `needs_audit` like the rest; native pass still owed). `i18n-translated-check` tracks both.

## Per-mode decisions (CONFIRM)

| Where | Offered? | Why |
|---|---|---|
| Practice levels (non-Climb) | yes, up to the session cap | the rescue is unlimited here already |
| The Climb | yes, **1** (the existing void cap; `voids_used` is never reset, so in effect 1 per page load) | after that only Give up, a real miss |
| Daily Challenge | yes, **2 per play** (existing cap); the word is substituted so the run keeps its length | |
| Misses / review | yes | nothing is scored in a void; the word stays in the queue |
| Versus | **no** | a free skip hands the turn over; fairness |
| Racing Daily and the online spell-off | **no** | a running lap clock / a head-to-head would be gamed |
| Impossible (all, incl. Impossible Daily and Mix) | **no** | no aids by design; `Can't hear it` is hidden there |
| Timed levels | yes; the timer stops with the void (existing behaviour) | |
| Spell Jr / unknown age / Education | skip **yes**, report **never sent** | the skip helps the child; telemetry rules keep their data off the wire |
| My Words (custom lists) | skip yes, report **never sent** (I4: My Words content never leaves) | |
| Quick Bee, Chains, Impostor, Spelluzzle, Word Picture, SpellDoku, Spell Search, Boardgame, Randospell, Online | **not in v1** | they have their own screens (some their own rescue panel) and rules; one screen first |

## Abuse guard (CONFIRM the number)

**`CANT_TELL_SESSION_CAP = 5` streak-safe skips per page load** (in memory; a relaunch resets it), **on top of** the existing void cap (Climb 1, Daily 2). After the cap the step is hidden and the only way past a word is Give up (a miss). Five is enough for a player who genuinely hits a few bad clips in one sitting and small enough that skipping cannot be a strategy. A skipped word also stays out of that player's draws, so the same clip does not come back to be skipped again. A skip never scores and never awards credit: it is a void, and `void_round.rs` / `scripts/clip-report-check.mjs` hold that structurally.

## The report (CONFIRM -- this departs from signed telemetry rules)

New record `clip_report`, batched as `clip_batch`, `POST /telemetry/v1/clips` on the existing Cloudflare Worker, table `clip_reports` (migration `0003_clips.sql`), schema bumped by **adding** records only (`SCHEMA_VERSION` stays 1, so older builds and the old Worker keep working; an un-redeployed Worker answers 404 and the client keeps its small capped queue).

Fields: `lang` (enum), `item_hash`, `variant` (`normal` | `slow`: which playback the player last asked for), `source` (`server_cache` | `native_tts` | `human` | `pack` | `other`). Plus `v`, `build`, `platform`. **No session id**, so two reports from one launch are not linked; no IP (the Worker never reads it); no account.

**The word is a 64-bit hash, not a string.** The schema has no string type, by design (I2), and the lint forbids a field called "word". `item_hash` = FNV-1a of `"<lang>|<bank entry>"`. `scripts/clip_report.py` hashes every bank entry with the same function to turn hashes back into words. So the Worker's data cannot be read as words without the banks, but anyone holding the banks can invert it: **it is not anonymous about which word**, only about who. That is the one deliberate departure from "no word leaves the device" (CC-TELEMETRY I3 / the speak-log rule) and it is why the flag stays off.

Rules that hold:
- Sent only by a **Standard** player with the telemetry toggle on and the kill switch on. Jr, unknown age and Education devices never queue one (even while the server is still unknown). My Words never queues one. Toggle off or kill switch off clears the queue.
- Posted with the existing batching (every 10 min and when the app hides).
- Retention: 90 days, like raw events (`telemetry-purge.sh` deletes `clip_reports` too).

**CONFIRM before the flag is on:**
1. Is a per-word hash an acceptable amendment to I3 (it identifies a *clip*, not a learner's attempt, but it is a word)? The privacy page (`privacy.html`) and the App Privacy answers must say so first: this is "Product Interaction / Other Usage Data, not linked, not tracking", and `privacy.html` currently says no word is collected. Eric / the privacy session own that text.
2. The session cap of 5.
3. The per-mode table (above), especially Versus and Racing off, and the other screens deferred.
4. `variant` meaning "normal vs slow" (the clip variant, as `audio-verdicts.json` keys use it), not British/American spelling.
5. Deploy order: apply `migrations/0003_clips.sql`, deploy the Worker (README), install the new `telemetry-purge.sh`, **then** ship a build. The flag itself is local-only (`spell_flag_cant_tell`, or a build default change).

## Reading the reports

```bash
bash scripts/clip_report.sh
```

lists words by report count with the variant and source mix. Read `source` first: reports mostly `native_tts` mean the on-device fallback played (the server clip was missing or slow), and quarantining the word fixes nothing. Reports mostly `server_cache` or `human` are the clip's fault: quarantine through CC-BANK-PURITY. One report is a guess; three from different launches is a pattern (reports carry no launch id, so count, not distinct players).

## Gates and tests

- `cargo test --lib`: `game::cant_tell_tests` (every mode row, the cap), `void_round` (both entries go through the one void; `cant_tell` calls no outcome path and no network), `telemetry` (hash round trip, My Words and Jr never queue, schema, generated bindings).
- `node scripts/clip-report-check.mjs --selftest`: **mutation check**. Five lesions (skip becomes a give-up, skip resets the streak by hand, flag ships ON, button starts visible, handler loses its gate) must each turn the check red.
- e2e `tests/e2e/specs/cant-tell.mjs`: flag off = rescue unchanged; with the flag on, spell a word right (streak 1), skip the next, streak stays 1, no miss and no stat written, the word is not shown, one report queued and posted, schema-valid, no word on the wire; the cap hides the step after five; the Climb allows one then hides it; a Jr device queues nothing.
- Worker: `npm test` in `workers/telemetry` (store, refusals, kill switch, purge).
- `i18n-translated-check`, `i18n-check`, `dom-id-live-check`, `dom-id-check`, `mode-exit-check`.
