# RECON-BOARD-GAME-ONLINE

The §0 findings are in docs/recon/RECON-ONLINE-TURN.md, under "Re-recon against origin/main". This file holds the phase notes and, later, the real-match log.

## Phase notes

**Phase A (2026-11-01).** Signed core-law exception A-4c. `GameConfig::mixed_table` allows any mix with at least one human; solo and pass-and-play configs leave `mixed` false and still refuse it. `human_acc_milli` is now the mean over the seats a human holds, which is the same number when there is one human. `set_npc` hands a seat over and back and changes nothing else, and it refuses while that seat is mid-turn. `native_api::boardgame` exports the rules. No `advance(state, input)` was added: `apply` and its Actions already are one step, and each is one engine phase. No roll or clock injection was needed either: the seeded state draws the roll and the engine has no clock.

Solo evidence: every existing BoardGame test passes with no line of it changed, including the seven pinned golden digests, in both feature configs. New tests cover a mixed table, its replay, the mean, and the hand-over. Goldens pin fixed seeds, so "every seed" rests on those plus the size of the change: the `mixed` check, and a mean that divides by one.

Not checked: `scripts/boardgame-wasm-golden.mjs` no longer builds. Its crate has no `impossible` module, which `engine.rs` has called since the Impossible work, before this phase. It is not in CI, so wasm parity of the goldens is unverified.

To decide at Phase B or C:
1. A1 (back at 40 s, same word) cannot hold with the 35 s timer (D-B4).
2. D-B14 and D-B15 describe a game that continues after a seat finishes, but the rules end the game at the first finisher (`Phase::Finished`), as solo does. Changing that would change solo rules, so the first finisher ends the game online too unless Eric says otherwise.
3. TURN's `on_auto` for this mode maps to existing actions: a roll timeout is `Roll`, Stretch is `ChooseStretch(false)`, a word is `SubmitSpelling("")`, Switch Tiles is `ChooseSwitchTarget(None)`.

**Phase B (2026-11-01).** Eric decided: 35 s stays hard, so A1 reconnects at 20 s; the first finisher ends the game, so D-B14 is dropped and D-B15 reworded. `server/spell-server/src/boardgame.rs` implements `TurnRules` over the app's rules through `native_api`, with no rule of its own. It is mode `boardgame`, behind `SPELL_ONLINE_TURN`, off by default. Room settings travel in a new optional `create_room.options` (seats, size, npc, stretch, boosts, streak), which leaves H2H's bytes unchanged. Empty seats fill with NPCs. A snapshot is seed + config + action log, and a replay reaches the same digest. The rules set whose turn it is, so `TurnRules` gained `current_seat`, `table`, `on_return`, `admits` and a `Config`; `on_roll` now returns an `Outcome`, because a roll can end the game.

Tests: A2-A6, A8, A9 in `boardgame.rs` and `hub_tests.rs`, with A1 in `a_word_times_out_at_35_s...`. Mutation checks: putting the word in the view fails A9, and a silent Switch picking a seat fails A5. Fixed: `boardgame-wasm-golden.mjs` builds again, with a stub for the Impossible grader that no golden game reaches; `wasm32 == host` on all witnesses.

Choices made here, to be seen before release:
- The Impossible word is not served online: the pool's availability reads client state, so the server gives none and Double Expert plays as two Expert words. Solo is untouched.
- Pieces are assigned by seat for now; D-B12's picking belongs to the lobby (Phase C).
- A kid at a Standard table gets Standard words minus the kid-blocked list, as H2H does. A language short of words after that ends the game with `bank_exhausted` at the start. Online kids stay off by flag.
- NPCs draw no words in the rules (I11), so F4's "audio the other seats hear" does not apply to NPC turns.
- `word` from a verdict is the citation form, and the payload's `length` counts its characters.
