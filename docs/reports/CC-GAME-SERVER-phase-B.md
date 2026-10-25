# CC-GAME-SERVER — Phase B report (Online Head-to-Head: F7, F8, F9, F10 server side)

**Result: every Phase B acceptance test passes.** Branch `game-server-phase-a`
(continued), worktree `~/repos/spell-wasm-game-server`, on top of Phase A
(`99caa9b2` on main). Date: 2026-10-24. Phase C (the iOS client) has not
started.

## What exists now

| Path | What |
|---|---|
| `server/spell-server/src/h2h.rs` | Online Head-to-Head. Its scoreboard IS the app's `versus::Versus`; the file adds turn control, word serving, tickets, deadlines and the two online-only idle rules |
| `server/spell-server/src/audio.rs` | F9: the Mac mini fetch (the app's exact `/api/speak` request), tag stripping, 64 MB LRU, 3 retries in 5 s, the fetch worker |
| `server/spell-server/src/names.rs` | Kid names in all 15 UI locales, with each language's word order and agreement (machine-drafted, needs audit) |
| `server/spell-server/src/hub.rs` | + audio tickets (room-bound, expire 60 s after the word resolves), per-seat RTT grace, reconnect and audio hooks |
| `server/spell-server/src/conn.rs` | + timestamped pings for RTT, and `ui_lang` must be an app UI locale |
| `src/game.rs`, `src/native_api.rs` | Three functions widened to `pub`, and `pub use` lines (diff below) |

`create_room{mode:"h2h"}` now plays Online Head-to-Head.

## Acceptance tests

Commands:
```
cargo test -p spell-server --all-features
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p grader-parity --target wasm32-unknown-unknown
cargo clippy --no-deps -p spell-server -p grader-parity --all-targets --all-features -- -D warnings
node scripts/game-server-core-diff-check.mjs 99caa9b2
```

| Test | Where | Result |
|---|---|---|
| A7.1 rules-table rows 1, 4, 5–7, 8, 9–10, 11, 12, 13, 14, 15, 16–17 (row 2 is the names tests; 18 is A-DATA.1 + G-D10) | `src/h2h_tests.rs` | PASS |
| A7.2 1,000 recorded matches replay identically (random tickets masked) | `src/h2h_tests.rs` | PASS |
| A7.3 timed deadlines on the server clock: 40 ms and 250 ms RTT both on time when sent 50 ms early; 400 ms late is a timeout; grace capped at 300 ms | `src/h2h_tests.rs`, `src/hub_tests.rs` | PASS |
| A7.4 300 matches: the server scoreboard equals a locally driven `Versus` after every word | `src/h2h_tests.rs` | PASS |
| A8.1 10,000 matches × 16 languages: no in-match repeat, every word from its tier's bank | `src/h2h_tests.rs` | PASS |
| A8.2 10,000 kid-safe matches × 16: nothing above the Spell Jr ceiling, nothing on a kid list | `src/h2h_tests.rs` | PASS (mutation-checked: disabling the kid filter fails it) |
| A8.3 an inactive language → `lang_unavailable` | `src/hub_tests.rs` (`xx`; ar/fa are active, per your call) | PASS |
| A9.1 512 words × 16 languages, every server→client byte (WebSocket + normal and slow audio, both players) searched for NFC, NFD, lowercase, each Mandarin half, and the Mac mini cache-key MD5s: zero hits before each result | `tests/phase_b.rs` | PASS (mutation-checked: skipping tag stripping fails on the first word) |
| A9.2 member 200 with `no-store`, no ETag, no Content-Disposition, tags stripped; stranger, no token, unknown ticket → 404; expired (resolve + 60 s) → 404 | `tests/phase_b.rs`, `src/hub_tests.rs` | PASS |
| A9.3 TTS outage: 1 try + 3 retries per word, `word_paused{audio_unavailable}`, a success resets the count, three in a row → `match_ended{audio_unavailable}` | `src/h2h_tests.rs`, `src/audio.rs`, `tests/phase_b.rs` (end to end) | PASS |
| A10.3 a kid's `display_name` is ignored; the server's name is in the kid's language | `tests/phase_b.rs` | PASS |
| A10.4 = A3.4 + A8.2 | | PASS |
| A5.2 (H2H) untimed: an active seat gone 61 s → the word is a miss, the seat is released, `match_ended{player_left}` | `src/hub_tests.rs` | PASS |
| A5.5 an idle untimed word → miss at 120 s, and the turn passes | `src/hub_tests.rs` | PASS |
| Timed: the deadline keeps running while the active seat is away; coming back restarts the untimed idle clock | `src/hub_tests.rs` | PASS |
| A full match over sockets at the real 1300 ms hand-over | `tests/phase_b.rs` | PASS |
| Every Phase A test (re-run), A1.1 golden (one additive type), A6.1 WASM parity, clippy, A6.2 core diff | | PASS |

## The core diff for Phase B (A6.2)

```
src/game.rs        fn clamp_base_tier          -> pub fn clamp_base_tier
                   fn tier_for_streak          -> pub fn tier_for_streak
                   pub(crate) fn voice_target  -> pub fn voice_target
src/native_api.rs  + pub use of tier_time, CORRECT_DELAY_MS, the three above,
                     kid_allowed, Versus, Side
```

## Findings and choices for Eric

1. **The app's kid filter has a silent fallback.** `kid_filter::filter_kid`
   returns the **whole, unfiltered pool** when every word would be excluded
   (`src/kid_filter.rs:78`). Online, that would let a kid room serve a listed
   word, so the server checks each word with `kid_allowed` and ends the match
   `bank_exhausted` instead.
   - It never happens with today's banks.
   - Local Kid Mode still has the fallback. Fixing that is app work outside
     this spec, so I've left it alone.
2. **Audio is fetched before the word starts.** `word_start` goes out only
   once the clip is in the server's cache, so a slow TTS never eats a timed
   player's seconds.
3. **A paused word is void,** like the app's "can't hear it" round: no score
   moves, and the same player taps for a new word. A failed **slow** replay
   doesn't pause anything; the normal clip is already there.
4. **Protocol additions:**
   - A server `audio{variant, audio_ticket}` message answers `replay_audio`.
     It is the same ticket for the same variant.
   - New error codes: `word_live`, `no_word`, `bad_ui_lang`.
   - New `match_ended` reasons: `bank_exhausted`, `audio_unavailable`,
     `player_left`.
   - `word_result.word` is the bank entry, so Mandarin arrives as
     `pinyin|hanzi`. Phase C splits it the way the app does.
   - The A1.1 golden changed by exactly one added line.
5. **An idle player at turn start has no limit.** The spec's 120 s rule
   covers an idle *word*. A player who never taps the orb to begin their turn
   holds the match until someone leaves or the 3-hour room lifetime ends.
   **Do you want a turn-start idle limit too** (for example 120 s, then the
   turn counts as a miss)? It would be a new online-only rule, so I haven't
   added it.
6. **Kid names are machine-drafted in all 15 UI locales.** They handle
   gender agreement (Abeja Roja / Libro Rojo, Rote Biene / Rotes Buch), the
   Filipino linker and Swahili noun-class connectors.
   - They need a native speaker's audit. Every list is in one file.
   - About 1% of combinations trip the app's profanity filter and are
     redrawn. The filter blocks the number 13 outright, and its leet layer
     reads "kijani 99" as a slur. Both behaviours are the filter's own.
   - `hello` now requires `ui_lang` to be one of the app's UI locales, and
     anything else is refused explicitly.
7. **Watchers hear the word too.** Both players may fetch the ticket, as both
   hear it when sharing a phone locally.
8. **A9.1 search limits:**
   - Strings shorter than 4 characters are matched only as whole JSON values,
     since "eat" is inside "seat".
   - Protocol vocabulary is exempt (a bank word "easy" is not leaked by
     `tier:"easy"`).
   - The MD5 forms cover the base key and the voice key for en/de/ru/ko. A
     lexicon `|ipa:` suffix isn't reproduced, and neither is Persian's newer
     voice key.
9. **The test-only `h2h_fast` mode** plays the same rules with no hand-over
   pauses, so the 512-word leak test runs in real time. It exists only under
   `--features test-modes`, like the `test` modes.
10. **A8 runs at the specified 10,000 matches per language.** Set
    `SPELL_A8_MATCHES` to run fewer while iterating.

## Next

Phase C is the iOS client:
- F12: the online entry, lobby and match UI.
- F13: flags, kill switch and failure states.
- F10 client side: the parent toggle and gating.
- F2 client side: the Keychain methods on `NativeLanguageKitPlugin`.
- F9 client side: `play_ticket`.

I'd like your answer on finding 5 first, though nothing in Phase C depends on
it.
