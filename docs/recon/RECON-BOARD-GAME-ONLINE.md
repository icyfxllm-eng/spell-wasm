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

**Phase C (2026-11-02).** The app plays a board room behind `online.boardgame` (default off; it also needs `online_enabled`). With the flag off the hello is byte for byte the old one (protocol 1, no `mode_versions`), so Spell Off online is untouched. With it on the app says protocol 2 and `boardgame: 1`.

The online game is the solo screen, not a copy. `src/online_board.rs` turns the server's view, payloads and effects into a display-only `BoardGameState` and the solo game's own `Event`s; `src/boardgame_screen/online.rs` sends each tap to the room instead of calling `apply`. The mirror never holds a word (it has no pools and no dice) and is never handed to `apply`. Every seat but mine is drawn as an NPC seat, so the solo screen's "not your turn" paths cover other players. NPC turns are paced on the client at the solo 1.5 s per turn (D10); the server still plays them at once.

Server additions for the screen (server crate only, no core change):
- Every phase payload carries a `table` (each seat's tile, ward, streak, bonus, skip, npc, piece, and the revealed tiles), so a client is right again at every turn even where effects alone cannot say (a streak count).
- The Stretch choice's payload names both moves (tier and tile each); it had only `kind`.
- A snapshot keeps the phase payload (F7: a rejoin mid-word gets the same tier, length and tile) and the view gains the room's feature switches.
- D-B12 piece picking: in the lobby a `turn.choice` of `piece:N` picks a piece, first come first served; a taken piece gets `piece_taken`. No new message type, so the protocol golden is unchanged. A seat that leaves frees its piece. Unpicked humans, then NPCs, take the lowest free pieces. The picks are in the replay record.

F8: each phone writes only its own seat's words, right and wrong, to its own no-repeat history (the solo ledger), and counts online games in `bg_online_matches_v1` (played, won). Nothing is sent anywhere.

Tests: `online_board` unit tests (view, payloads, every effect, my-verdict-only history, ranking, the names matching the server's); `parses_the_servers_golden_protocol` now reads all 25 server messages. It was red on main since CC-ONLINE-TURN Phase A added the turn messages (that broke the app's Editions job), and this phase fixes it. Server: picks kept, taken refused, freed on leave, replayed; payload in the snapshot; both Stretch moves named. Mutation: ignoring picks fails the picks test. E2E `tests/e2e/specs/online-board.mjs` against a real server: two app players and an NPC pick pieces, play a Sprint game to its winner, survive a 3 s cut while Dad's word is live (same seat, same word after), agree on the ranking, see the online result card, write their own histories, and Back to room returns to the lobby. The TTS stub tells the test the word, never the app. It caught one real bug: after a resume, the first frame rebuilt the board from the snapshot held from before the drop.

Solo: all 41 solo Board Game browser tests pass unchanged (A7).

The board has the room's eight emoji reactions, shown only online (hidden while spelling, so the key grid keeps its room). Not done here: the 3D corridor stays solo. A kid profile cannot reach a board room while online kids is off. Phase D is Eric's real match.
