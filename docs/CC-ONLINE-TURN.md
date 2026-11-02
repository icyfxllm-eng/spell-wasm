# CC-ONLINE-TURN v1 — Shared online turn engine for SpellGame

Status: SIGNED by Eric 2026-11-01, as amended, including the §3.2 defaults. Recon: docs/recon/RECON-ONLINE-TURN.md. Sequencing: post-build-56. Must not delay release. §0 recon is DONE. The hard gate (Head-to-Head has passed one real match on TestFlight across two networks, D-O14) was MET on 2026-11-01 (Eric, build 310, one phone on cellular). Phases A–E may run.

Parent: CC-GAME-SERVER v1 (rooms, join, reconnect, resend). This file adds the layer above it. Children: CC-BOARD-GAME-ONLINE v1 (first), CC-RANDOSPELL-ONLINE (later), CC-BEE-ONLINE (later). Not a child: CC-SPELLOFF-MULTI. Spell Off stays async on the Flask backend (D-O10).

## Amendments from recon (Eric, 2026-11-01)

- **A-1 Head-to-Head is already a turn engine.** Online H2H is alternating turns, longest chain wins, 2 players (CC-GAME-SERVER D-R2). The server already has a `ModeRules` trait and registry (`mode.rs`). This file **extends `ModeRules`** to 2–4 seats, phases and hard timers. H2H moves onto it only if its wire fixtures replay byte-identical (A8); otherwise H2H stays as is and the new engine is a second registry entry. There is no simultaneous engine online.
- **A-2 Profiles.** Every seat requires a profile. The host must be an **adult profile that passed the parent gate** (client-asserted in v1). Verified adult via the Flask login is a possible later upgrade. The server must validate `profile_kind` at create/join (a kid profile cannot host), since it trusts the client today.
- **A-3 No entitlement enforcement in v1.** No StoreKit exists (CC-GAME-SERVER C7/O2); online languages = `is_active_lang`, same as local. `error.not_entitled` is reserved but unused so an entitlement join hook can be added when IAP ships without a protocol bump. Revisit when IAP ships.
- **A-4 Protocol gating.** H2H is held at the old protocol minimum. New modes and message types ship behind a `PROTOCOL_CURRENT` bump; `SPELL_PROTOCOL_MIN` stays at 1. A client below the bumped version may only create/join H2H rooms; any other mode is rejected for it with `update_required`.
- **A-5 Recon facts the build must respect.** Client messages use `deny_unknown_fields`; resync is `hello.resume{room,last_seq}` plus a 512-frame outbox ring (no explicit ack message); the per-game seed is currently in memory only and never logged; the server's TTS path is the Flask `/api/speak` via `SPELL_TTS_BASE` with opaque `audio_ticket`s valid 60 s after the word resolves.

## 1. Intent

Players want to play BoardGame, Randospell, and a spelling bee with friends and family across devices. The game must not cheat, must not stall when a phone sleeps, and must let a player rejoin mid-game and see exactly what everyone else sees. The server already owns rooms. This file makes the server own the game: whose turn, what word, what roll, who won. Each later mode adds a small rules module on the server and an online screen in the app. Nothing else.

If this file is silent on a case, prefer the choice that keeps the room moving and keeps the client dumb.

## 0. Recon (DONE 2026-11-01, read-only)

Output: docs/recon/RECON-ONLINE-TURN.md. Answers: R1 only Spell Off async (`submit_result`) and `hello.profile_kind` are trusted client values; R2 **no**, the word does not leak; R3 **no** for new fields on existing client messages, **yes** for new modes behind a protocol bump (A-4); R4 yes (BoardGame is seedable and replayable); R5 NPC code is app-only; R6 see recon doc.

HALT conditions as originally written:

* Head-to-Head real-match gate not met. Cleared 2026-11-01.
* R2 = yes. Not fired.
* R3 = no and the fix would break shipped H2H clients. Resolved by A-4.

## 2. Features

F1 Turn engine (server). One module that **extends `ModeRules`** (A-1). Owns seat order, active seat, phase, and a hard timer per phase. Phases: lobby, awaiting_roll, awaiting_spell, awaiting_choice, resolving, ended. Modes may add phases by registering them; they cannot bypass the engine's timer.

F2 Server-authoritative randomness and verdicts. Rolls, board seeds, word draws, trap and boost placement, spelling verdicts: server only. The client sends one thing per turn: its final answer or its choice. Every random value derives from one **logged** per-game seed (new work: today the seed is in memory only).

F3 Word secrecy. The server sends {audio_ticket, length_hint?}. The client fetches audio by ticket from the server, which sources it from the Flask TTS path. The spelling appears only in the verdict event. All seats receive the verdict and the word at the same moment.

F4 Snapshot contract. Every mode implements snapshot(game) -> bytes and restore(bytes). A rejoining client receives the snapshot, not an event replay. Snapshot carries a content hash; the client echoes it so the server can confirm sync.

F5 Disconnect policy. 60 s grace on socket loss. Past grace, the engine resolves that seat's active phase as a miss (or the phase's declared default choice). Two consecutive auto-resolutions: the mode's on_abandon(seat) runs. BoardGame and Randospell convert the seat to an NPC. Bee eliminates it. The room never waits on an absent human.

F6 Room rules. Invite code only; no public listing, no matchmaking. **Every seat requires a profile. The host must be an adult profile that passed the parent gate** (A-2). A kid profile may join an adult-hosted room; it may not host. No free-text chat. Emoji reactions from a fixed server-side set only. Room locks one language and one tier at creation; the language must be an active language (`is_active_lang`). The server validates `profile_kind` at create and join.

F7 Protocol envelope. {room_id, seq, mode, protocol_version, mode_version, event}, expressed in the existing envelope (`v`, `t`, `seq`, `cid`, `body`) with the new fields added through a `PROTOCOL_CURRENT` bump (A-4). A mismatched protocol_version is rejected with `update_required`. mode_version mismatch rejects at join for that mode only. Head-to-Head traffic is unchanged.

F8 Load gate. The existing load scenario (`server/tools/botload`) extended to 4-seat rooms at the same ceilings recon R6 recorded. No 3+ seat mode ships before this passes.

## 3. Decisions

### 3.1 Signed by Eric ("I like the recs", 2026-11-01)

* D-O1 Server owns all randomness and verdicts.
* D-O2 Spelling revealed to every seat only in the verdict event.
* D-O3 (amended A-1) H2H is already a turn engine. This file extends `ModeRules`; no third engine, no simultaneous engine online.
* D-O4 NPC fill: BoardGame and Randospell yes; Bee no.
* D-O5 60 s grace → auto-resolve as miss → after two consecutive, NPC conversion (BoardGame, Randospell) or elimination (Bee).
* D-O6 Hard server timers. Timeout = miss. Randospell Bank/Roll-again timeout = Bank.
* D-O7 Invite-code rooms only. Adult-profile host. No text chat. Emoji reactions only.
* D-O8 Order: BoardGame → Randospell → Bee. Spell Off multi runs in parallel on its own path.
* D-O9 The 4-seat load test gates every 3+ seat mode.
* D-O10 Spell Off stays async. Not on this engine.
* D-O11 Bee = elimination format, own word per player per round.
* D-O12 (amended A-3) Room locks language + tier; the language must be active. No entitlement check in v1.
* D-O13 Online modes app-only in v1. spellgame.net later under its own amendment.
* D-O14 Nothing past §0 runs until Head-to-Head passes a real TestFlight match.

### 3.1b Signed by Eric in the recon session, 2026-11-01

* D-O26 Extend `ModeRules` (A-1).
* D-O27 Every seat requires a profile; host = adult profile past the parent gate (A-2).
* D-O28 No entitlement enforcement in v1; `not_entitled` reserved (A-3).
* D-O29 H2H held at the old protocol minimum; new modes behind a bump (A-4).

### 3.2 Defaults applied, unsigned (Eric may object; build proceeds on these)

* D-O15 Max seats per room = 4. Hard-coded constant in one place.
* D-O16 Phase timer defaults: awaiting_roll 20 s, awaiting_spell = mode's solo timer + 5 s grace, awaiting_choice 15 s. Modes may shorten, never lengthen past 60 s. (Solo timers are soft rings that never auto-fail, D16; online is a hard timer by D-O6.)
* D-O17 Timer shown to all seats; the server's clock is truth. The client never enforces a timeout.
* D-O18 Host leaving does not end the game. Host role passes to the next human seat in order. If no human remains, the room ends and no result is recorded.
* D-O19 A game with fewer than two humans at start cannot start. (Solo-plus-NPCs stays the app's offline mode.)
* D-O20 Results recorded per seat to the same stores the solo mode writes (no-repeat history, misses, My Words) — only for the words that seat spelled.
* D-O21 Online games never write to the Climb or any leaderboard in v1.
* D-O22 Emoji set: 8 reactions, defined once on the server, same in every language.
* D-O23 NPC seats are visibly labelled as computer players to every seat.
* D-O24 Audio tickets are single-use per room: a stale ticket returns 404.
* D-O25 (amended, Eric 2026-11-01) All server logs of a game are keyed by game_id and retain the seed, every event type, and every snapshot hash. Typed answers and words are never logged, because the log could hold a child's input. Retention follows CC-SECURITY-BASELINE O3.

### 3.3 Open (build default in parentheses)

* O1 Seat count above 4 later (no; needs its own load gate).
* O2 Reaction emoji chosen by Eric (build uses a placeholder set of 8, Eric swaps before release).
* O3 Should a converted-NPC seat revert to human if the player reconnects later in the same game? (Yes, if the game hasn't ended; the seat's NPC turns stand.)

Do not re-propose: public matchmaking, text chat, a third engine, porting Spell Off to the live server, online leaderboards in v1.

## 4. Invariants

* I1 No client message can change another seat's state or any verdict.
* I2 A rejoining seat sees the identical state the server holds. Snapshot hash matches.
* I3 A room with one or more absent humans still reaches ended.
* I4 The target spelling never leaves the server before that seat's verdict.
* I5 (amended A-3) A seat cannot join a room whose language is not active. No bypass via invite code.
* I6 Every random value in a game is reproducible from the logged seed.
* I7 Solo rules are unchanged. Online adds a rules module; it never forks the mode.
* I8 Head-to-Head behaviour and wire format are byte-identical before and after this file (turn_start, word_start, word_result, turn_end, match_result; old-protocol clients keep working, A-4).
* I9 (amended A-2) Kid profiles never host and never see free text from another seat. The server enforces `profile_kind` at create/join.

## 5. Protocol

5.1 Envelope: see F7. seq is per-room monotonic from the server. **Resync is the parent file's mechanism, unchanged**: the client sends `hello.resume{room,last_seq}`; the server replays frames from its 512-frame outbox or falls back to a full room.state snapshot. There is no separate ack message.

5.2 Engine events (server → clients unless marked):

* room.state — snapshot (F4). Sent on join, rejoin, and on request.
* turn.begin {seat, phase, deadline_ms}
* turn.audio {seat, audio_ticket} — target for this seat's spell phase.
* turn.answer {answer} — client → server. Only the active seat's answer is accepted.
* turn.choice {choice} — client → server. Only when phase = awaiting_choice.
* turn.verdict {seat, word, answer, correct, effects[]} — to every seat.
* turn.auto {seat, reason} — timeout or disconnect auto-resolution, to every seat.
* seat.status {seat, status} — connected | grace | npc | eliminated | left.
* game.end {ranking[], summary}
* error.* {code, message} — update_required, not_adult_host, room_full, bad_phase, inactive_language; `not_entitled` reserved, unused in v1.

5.3 Mode module interface (server): the `ModeRules` extension.

init(room, seed) -> state
on_turn_begin(state, seat) -> phase, payload
on_answer(state, seat, answer) -> verdict, effects
on_choice(state, seat, choice) -> effects
on_auto(state, seat, phase) -> effects      # timeout / disconnect default
on_abandon(state, seat) -> effects          # after two consecutive autos
is_over(state) -> bool
ranking(state) -> [seat]
snapshot(state) -> bytes ; restore(bytes) -> state

The engine calls these. A module never touches the socket, the timer, or another seat's connection.

## 6. Phases

Phase A — Engine (extending `ModeRules`) + envelope + snapshot contract + protocol bump with H2H held at the old minimum. Behind flag online.turn. Head-to-Head untouched (I8 test in CI).
Phase B — Timers, disconnect grace, auto-resolution, NPC/elimination hooks (F5). Seed logging (D-O25).
Phase C — Room rules: invite codes, profile requirement, adult-profile-host check, kid-join rule, active-language check at join, emoji reactions (F6).
Phase D — Load: 4-seat scenario added to the harness at the R6 ceilings (F8). Gate.
Phase E — A null test mode that exercises every interface method, used by CI only. Then CC-BOARD-GAME-ONLINE v1 Phase A may begin.

Each phase ends with its tests green in CI and a one-paragraph note appended to docs/recon/RECON-ONLINE-TURN.md.

## 7. Acceptance tests (CI, against the home-hosted server)

* A1 Null mode, 4 seats, one client socket killed mid awaiting_spell, reconnected after 40 s: same phase, same audio_ticket, deadline within 1 s of the original.
* A2 Same, not reconnected: turn.auto at deadline, second turn.auto next turn, then seat.status = npc. Game reaches ended.
* A3 Tampered client sends turn.verdict and turn.answer out of phase: server ignores both, logs bad_phase, game state unchanged (hash compare).
* A4 Packet capture of a full game contains no target word before its turn.verdict.
* A5 Kid profile attempts host: error.not_adult_host. Joins adult-hosted room: seat.status = connected.
* A6 (replaced, A-3) A join to a room whose language is inactive is refused with error.inactive_language.
* A7 (amended) Replay from the logged seed plus a recorded input list: identical event sequence, identical final hash. The logs alone do not hold the inputs (D-O25).
* A8 Head-to-Head wire-format fixture recorded before Phase A replays byte-identical after Phase E; an old-protocol client plays H2H unchanged and receives `update_required` for any other mode.
* A9 Load: 50 concurrent 4-seat null-mode rooms within the R6 ceilings.
* A10 Stale audio_ticket request returns 404.
* A11 A seat without a profile is refused at create/join.

## 8. Done when

* A1–A11 green in CI.
* online.turn flag defaults off in App Store builds.
* Recon doc updated with per-phase notes.
* Eric has reviewed and signed the §3.2 defaults or amended them.

## 9. Non-goals

* Any change to solo rules, scoring, word banks, the Climb, or leaderboards.
* Public matchmaking, ranked play, chat, voice.
* Spectators joining from outside the room.
* spellgame.net play.
* Porting Spell Off or Head-to-Head onto this engine (H2H moves onto it only under A-1's byte-identical condition).
* Entitlement enforcement (A-3).

## 10. Stop and ask

Stop and report rather than infer when:

* A phase needs a change to the parent CC-GAME-SERVER envelope that would affect Head-to-Head clients.
* A timer default in D-O16 conflicts with a signed solo-mode timer.
* Anything here conflicts with CC-SECURITY-BASELINE's child-data rules (not yet read against this file).
* Old-client decoding of new server message types turns out to leak past the mode gate (`src/online_net.rs`, `online_model.rs` not yet inspected).
