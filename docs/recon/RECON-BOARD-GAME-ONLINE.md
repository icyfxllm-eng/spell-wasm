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
