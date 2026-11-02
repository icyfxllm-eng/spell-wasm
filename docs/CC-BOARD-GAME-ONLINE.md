# CC-BOARD-GAME-ONLINE v1 — Spell BoardGame on the shared turn engine

Status: SIGNED by Eric 2026-11-01, as amended, including the §3.2 defaults. Amendment atop CC-BOARD-GAME v1 and CC-BOARD-GAME-POLISH v1. Sequencing: post-build-56. Must not delay release. Depends on: CC-ONLINE-TURN v1 Phases A–E merged and A1–A11 green. **HALTED**: CC-ONLINE-TURN is not built, and its own gate (a real Head-to-Head TestFlight match) is unmet. Executable on signature: §0 recon only (read-only). Phases A–D gated on Eric's recon review. Fulfils: CC-BOARD-GAME v1 D7 ("online later") and D11 ("online fills empty seats with NPCs").

## Amendments from recon (2026-11-01)

- **A-1** The turn engine extends `ModeRules` (CC-ONLINE-TURN A-1), not a separate engine.
- **A-2** Entitlement is not enforced in v1 (CC-ONLINE-TURN A-3). D-B10 and D-B11 are reworded below.
- **A-3** Every seat requires a profile; host is an adult profile past the parent gate (CC-ONLINE-TURN A-2).
- **A-4 Facts found in the main checkout.** Rules are Rust only and authoritative (`src/boardgame/`: mod 510, engine 547, rules 209, board 46, balance 365, golden 77, tests 1144 lines; `boardgame_grade.rs`, `boardgame_pools.rs`, `boardgame_ring.rs`, `boardgame_input.rs`, `boardgame_screen.rs`). RNG is `spelldoku::rng::Rng` (splitmix64) in the state; about 9–10 call sites (board.rs:33; rules.rs:191, 199; engine.rs:43, 48, 50, 83, 172, 177, 212). The count for I6 needs a careful pass in Phase A. The engine is a pure seeded reducer, replayable from seed (`new_game`, `digest()`, wasm golden parity).
- **A-4b SUPERSEDES the file sizes, RNG sites and flag notes in A-4 to A-6:** see "Re-recon against origin/main" in docs/recon/RECON-ONLINE-TURN.md. It also lists open conflicts: the rules reject humans plus NPCs (`engine.rs:29`), and the core-crate law forbids logic changes.
- **A-4c Signed 2026-11-01.** (1) One exception to the CC-GAME-SERVER core-crate law: the BoardGame rules may gain mixed humans and NPCs and room-mean accuracy, provided solo output is identical for every seed (goldens, I5, A7). (2) The server rebuilds a game from seed + cfg + pools + action log; clients still receive a per-seat snapshot with secrets removed.
- **A-5 Spec drift to resolve before Phase A.** (a) There are **5 trap kinds** (BackToStart, LongWord, LoseRoll, SwitchTiles, DoubleExpert), with 6 trap tiles on a Full board, not "6 kinds". (b) CORRECTED: boosts and streak ARE in the rules on `origin/main` (`src/boardgame/mod.rs`); the recon read a checkout 297 commits behind, so the file sizes, call sites and A-6 answers in this block must be re-read against `origin/main` before Phase A. (c) No `docs/CC-BOARD-GAME.md` exists on `origin/main` under that name. (d) `board_stretch()` defaults OFF and `boardgame()` defaults ON. No separate D-P16 flag list found.
- **A-6 R-answers from recon.** The turn is multi-step through `engine::apply(&mut state, Action)` (Roll, ChooseStretch, SubmitSpelling, ChooseSwitchTarget, AdvanceNpc); there is no single `(state, roll, answer)` function, and the roll comes from the internal RNG. Switch Tiles target is a rules call (`Phase::AwaitSwitchTarget`). Stretch is chosen before any word is drawn. The human's running accuracy (`human_acc_milli`, `mod.rs:414`) is computed inside the state from verdicts the server already sees, so no client-reported number is needed (the §9 stop-and-ask does not fire).

## 1. Intent

Spell BoardGame is built, polished, and the mode Eric most wants to play with other people. Online means two to four humans on their own phones, same board, same rules as solo, with computer players filling empty seats. A player who steps away must not freeze the table. A player who comes back must land on the same tile, same turn, same word. Nothing about the solo game changes.

## 0. Recon (read-only)

Output: docs/recon/RECON-BOARD-GAME-ONLINE.md (not yet written; partial answers are in RECON-ONLINE-TURN.md and A-4..A-6 above). Change no code.

0.1 Locate, with paths:

* Solo BoardGame rules: board generation, tile kinds, trap effects (5 kinds + D12 fail paths), boost tiles (D-P2), Stretch (D-P1), streak (D-P3), Sprint/Full (D-P4), Jr variant, NPC accuracy (D13, D18 deltas), timer (D16), pass-and-play flow.
* The RNG call sites in the above. Count them.
* Whether rules live in the Rust/WASM core, in app TypeScript, or both. (Answered: Rust only.)
* The BoardGame screen: ring board, centre stage, piece animation, result card.
* Per-feature flags from POLISH D-P16 and their current defaults.

0.2 Answer:

* R1 Can the solo rules module run on the server unchanged (pure, no UI imports)? If not, list every impure dependency.
* R2 Is there one function that advances one turn given (state, roll, answer)? If not, what is the smallest refactor that produces one?
* R3 How does the Switch Tiles trap pick its target today? UI picker or rules call?
* R4 How is NPC accuracy fed today (the human's running accuracy per D13)? Where is that number computed?
* R5 Does Stretch choice happen before word content is shown (D-P9)? Confirm the order of events in code.

0.3 HALT conditions:

* CC-ONLINE-TURN not merged or its tests red. **FIRED, in force.**
* R1 lists an impure dependency that cannot be injected. Stop and ask before refactoring the solo core.

## 2. Features

F1 Server rules module. Implements the CC-ONLINE-TURN §5.3 interface (the `ModeRules` extension) by wrapping the solo rules. One rules codebase, two hosts (app for solo, server for online). The server never re-implements a rule.

F2 Board and seed. init(room, seed) generates the board (84 Full or 42 Sprint, Jr 40), places traps and boosts, picks the first seat, locks clockwise order. All from the seed. Jr vs Standard follows the room's tier lock.

F3 Turn flow (per seat):

1. awaiting_roll — the seat taps the orb; the server rolls. Timeout → server rolls anyway (a roll is not a decision).
2. If Stretch is enabled and offered: awaiting_choice {stretch: yes/no} before any word audio (D-P9). Timeout → no.
3. awaiting_spell — turn.audio to the seat; the word's tier, length hint, and "tile N of M" to every seat; the spelling to no one.
4. turn.verdict — word, answer, correct, and effects (move, trap triggered, boost gained, streak, Switch Tiles target request).
5. Switch Tiles: awaiting_choice {target: seat | nobody} for the triggering seat. Timeout → nobody (resolves CC-BOARD-GAME O2 for online as "nobody allowed").
6. Resolve. Next seat.

F4 NPC fill and conversion. Empty seats at start fill with NPCs (D11) up to the room's declared seat count. NPC accuracy = the room's mean human running accuracy minus the D13 delta for the room's NPC difficulty (set by host at room creation; default Normal), computed server-side from verdicts. A converted-NPC seat (TURN D-O5) keeps its piece, tile, boosts, and streak. NPC turns are compressed per D10 (≤7 s), including the audio the other seats hear.

F5 Timers. awaiting_spell = D16's 30 s + 5 s grace = 35 s hard (solo's 30 s ring is soft; online is hard by D-B4). Timeout = miss (piece stays; streak lost per D-P14). awaiting_choice = 15 s.

F6 Online screen (app). Lobby: create room (seat count 2–4, Full/Sprint, Jr/Standard, NPC difficulty, language+tier) → invite code shown → seats fill → host starts. In game: the existing BoardGame screen with seat labels, a "spelling…" state for other seats (orb pulses, no letters), the shared timer, emoji reactions, and reconnect overlay. Result card gains per-seat rows and an "online" badge.

F7 Snapshot. Board (seed suffices), tile kinds revealed so far, each seat's tile, boosts held, streak, trap-ward state, lost-roll flags, active seat, phase, deadline, pending audio_ticket, pending choice kind, ranking so far.

F8 Results. Each seat writes its own spelled words to its no-repeat history and misses (TURN D-O20). No Climb, no leaderboard (TURN D-O21). Win/loss recorded to a local "online matches" count for a future badge; nothing server-side beyond the game log.

## 3. Decisions

### 3.1 Signed by Eric ("I like the recs", 2026-11-01)

* D-B1 Online BoardGame runs on CC-ONLINE-TURN (the `ModeRules` extension); no separate engine.
* D-B2 Empty seats fill with NPCs (reaffirms CC-BOARD-GAME D11).
* D-B3 NPC accuracy baseline = room mean human accuracy minus the D13 delta.
* D-B4 Spell timer 35 s hard online; timeout = miss.
* D-B5 Switch Tiles timeout = nobody. "Nobody" is a legal pick online.
* D-B6 Solo rules unchanged; one rules codebase hosted in two places.
* D-B7 BoardGame is the first mode on the engine; Randospell and Bee follow.

### 3.2 Defaults applied, unsigned

* D-B8 Room defaults: 4 seats, Sprint, Standard, NPC Normal. Host may change before start, not after.
* D-B9 Stretch, boosts, and streak follow the POLISH per-feature flags; online does not add flags of its own. If a flag is off in the build (or the feature is not on main), the online game runs without that feature for every seat.
* D-B10 (amended) Host picks the room language from the active languages. Joining seats may sit in any active-language room; no entitlement check in v1 (TURN A-3).
* D-B11 Jr rooms: only Jr profiles and the adult-profile host may sit. Standard rooms: any profile. Jr trap/boost rules per POLISH D-P11.
* D-B12 Humans and NPCs share one piece pool (hat, wand, bee, book); humans pick first in join order; NPCs take what is left; NPC names = piece nouns (D17).
* D-B13 Haptics and tile-by-tile hop animation (D-P6) play on every seat's device for every move, including NPC and other-human moves; Reduce Motion honoured per seat.
* D-B14 (dropped, Eric 2026-11-01) The first finisher wins and the game ends, as in solo. No seat plays on or spectates.
* D-B15 (amended, Eric 2026-11-01) The game ends when any seat finishes, including an NPC, or when no human remains. NPCs never prolong a game past the last human.
* D-B16 Win-rate CI (D18) is not run against online games. Online NPCs reuse the solo deltas untested.

### 3.3 Open (build default in parentheses)

* O1 Should the audio of another seat's word play on your device? (Yes — a real board game is heard by the whole table. Mute toggle per seat.)
* O2 Trap-kind weighting (CC-BOARD-GAME O1) still open; online uses the solo default.
* O3 Rematch with the same seats: one tap, new seed. (Yes, in v1, host only.)

Do not re-propose: a separate online engine, server-side re-implementation of rules, online leaderboards, ghost players.

## 4. Invariants

* I1 Given the same seed and the same sequence of answers and choices, online and solo produce the same board, moves, and winner.
* I2 No seat ever receives another seat's target spelling before that seat's verdict.
* I3 A converted-NPC seat's prior state is preserved exactly (tile, boosts, streak, flags).
* I4 Every seat's piece position after turn.verdict is derivable from the verdict's effects alone.
* I5 Solo BoardGame tests pass unchanged after every phase.
* I6 The number of RNG call sites in the shared rules equals the number recon 0.1 counted, and every one draws from the injected seeded RNG.

## 5. Phases

Phase A — Rules extraction. Make the solo rules callable with an injected roll and clock where needed (they are already a pure seeded reducer); add a turn-level wrapper over `engine::apply` if the server needs one call per step. Solo app uses it. Solo tests green.
Phase B — Server module. Implement the TURN §5.3 interface over the extracted rules. Seed replay test (A4). Null-client CI game, 4 seats, all NPC after turn 1.
Phase C — App online screen (F6), snapshot restore (F7), results (F8). Behind flag online.boardgame, default off.
Phase D — TestFlight real match: 3 humans + 1 NPC, two networks, one forced disconnect and rejoin. Eric plays. Eric signs off before the flag can default on.

## 6. Acceptance tests

* A1 (amended, Eric 2026-11-01) 4-seat room, one client killed during awaiting_spell, relaunched after 20 s: same seat, same tile, same audio ticket, same deadline. Back after the 35 s hard timer (D-B4): the word was a miss, same seat, same tile.
* A2 Same, not relaunched: two turn.auto, then seat.status = npc, game reaches ended, NPC seat's tile and boosts match its last human state.
* A3 Tampered client sends roll=6 each turn: ignored, logged; actual moves match the seeded rolls.
* A4 Replay from seed with recorded inputs: identical board, traps, boosts, rolls, words, winner (I1).
* A5 Switch Tiles triggers; target seat ignores the prompt: resolves to nobody at 15 s.
* A6 Stretch on: awaiting_choice precedes turn.audio in the packet log, every time (D-P9).
* A7 Solo BoardGame test suite passes unchanged after Phases A–C.
* A8 Jr room: a Standard profile attempt to join is refused.
* A9 Packet capture contains no target spelling before its verdict.
* A10 Phase D real match completed; Eric's sign-off recorded in the recon doc.

## 7. Done when

* A1–A10 green (A10 manual).
* online.boardgame and online.turn default off in App Store builds.
* No diff in solo BoardGame behaviour (A7 + I5).
* Recon doc updated with per-phase notes and the real-match log.

## 8. Non-goals

* Changes to trap kinds, boost kinds, board sizes, scoring, Stretch, or streak rules.
* The 3D corridor view online (stays flagged and solo-only until its own note).
* Online leaderboards, badges beyond a local match count, rematch history.
* Public rooms, chat, voice.
* spellgame.net.

## 9. Stop and ask

* Any §0 HALT (CC-ONLINE-TURN unbuilt: in force).
* Phase A requires touching rules in a way that changes solo output for any seed.
* A POLISH flag interacts with online in a way this file does not state (boosts and streak are flag-gated in the rules on `origin/main`).
* Anything here conflicts with CC-SECURITY-BASELINE's child-data rules or CC-ONLINE-TURN's signed decisions.
