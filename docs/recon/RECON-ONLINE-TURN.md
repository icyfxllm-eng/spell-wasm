# RECON-ONLINE-TURN (CC-ONLINE-TURN v1 §0, plus CC-BOARD-GAME-ONLINE §0 where answerable)

Date: 2026-11-01. Read-only; no code changed. Run at Eric's explicit instruction ("Run recon anyway") **despite a HALT**.

## CORRECTION (2026-11-01): the app half of this recon read a stale checkout

The local `main` in `/Users/eric/repos/spell-wasm` is 297 commits behind `origin/main`. The app-side findings below came from that stale tree. Checked against `origin/main`: **Randospell is built** (`src/randospell/`), **boost and streak are in the rules** (`src/boardgame/mod.rs` `boosts`, `streak`), `server/` and `docs/CC-GAME-SERVER.md` are on main, and the phase report is `docs/reports/CC-GAME-SERVER-phase-E.md`. Five trap kinds still holds. `src/boardgame/engine.rs` is 740 lines, not 547, so **every app-side line number, line count, RNG call-site count and R-answer below needs re-reading against `origin/main`**. Server-side findings came from a worktree at 5a0027c3, which is `origin/main`.

## HALTs

1. **H2H real-match gate (D-O14) MET 2026-11-01**, on Eric's report: build 310 on TestFlight, one phone on cellular, "it ran perfectly". Phases A-E now wait only on Eric's signature of the amended specs.
2. **R3 = no for existing client messages** (see R3). Additive new modes/messages are possible behind a protocol bump. Needs Eric's call.
3. CC-BOARD-GAME-ONLINE: HALT (CC-ONLINE-TURN not built).
4. R2 = no (word does not leak). Not a HALT.

## Premise mismatches Eric should see before signing

- **Online Head-to-Head is NOT simultaneous.** docs/CC-GAME-SERVER.md (C1, D-R2): alternating turns, longest chain wins, 2 players. D-O3 ("Simultaneous engine, existing") and the A9/I8 wording assume otherwise. The server already has a `ModeRules` trait and registry (`mode.rs`), and rooms already hold up to 4 seats. CC-ONLINE-TURN would overlap or replace that trait rather than add a layer.
- **Entitlement is not enforced on the server at all** (F6, I5, A6). Online allows `is_active_lang`; the client filters in `src/online_screen.rs:307`. Regional grants are Flask-only (`backend/entitlements.py`, CF-IPCountry). Complete/free-tier: not found server-side. Per §10, this is a stop-and-ask.
- **Adult gate is client-asserted.** `hello.profile_kind` ("adult"|"kid") is trusted (`conn.rs:87`). No account or age verification on the game server. "Adult account" only exists as the Flask login used by async Spell Off. F6 "host must be an adult account" and I9 need new server work. Stop-and-ask.
- **No explicit ack message.** Spec 5.1 says "client acks by seq"; the real mechanism is `hello.resume{room,last_seq}` plus a 512-frame outbox ring, falling back to a full `room_state` snapshot.
- **The per-game seed is not logged or persisted** (in-memory only, OS CSPRNG, `h2h.rs on_start`). D-O25/I6/A7 are new work. Games are not persisted at all.
- TTS: spec says "Pi cache"; actual path is the Flask `/api/speak` via `SPELL_TTS_BASE` (https://spellgame.net, Mac mini).
- Repo layout: the main checkout `/Users/eric/repos/spell-wasm` has no `server/`, `docs/CC-GAME-SERVER.md` or `docs/recon/` (my earlier notes said main; the main checkout is behind or differs). The server lives in the worktree `/Users/eric/repos/spell-wasm-game-server` (branch game-server-phase-a, 5a0027c3) and `spell-wasm-online-rel` (release/online-testflight, 39c816b7). `reports/game-server-phase-e.md` was not found in either. Reconcile before building.

## Resolved 2026-11-01: H2H is turn-based (Eric)

Eric confirmed online Head-to-Head is turn-based. Spec wording to amend before signing (not yet edited in the specs):
- D-O3: "Two engines: Simultaneous (H2H) and Turn" becomes "H2H is already a turn engine (`ModeRules` in `mode.rs`, `h2h.rs`); CC-ONLINE-TURN generalises it to 2-4 seats, phases and hard timers." There is no simultaneous engine online.
- I8 and A8 stand, but now protect H2H's turn_start / word_start / word_result / turn_end wire format.
- F1 "Turn engine" should say whether it extends `ModeRules` or sits beside it. Recommendation: extend, so H2H becomes the first mode on it only if its wire fixtures stay byte-identical; otherwise leave H2H untouched and add the new engine as a second registry entry.
- DECIDED (Eric, 2026-11-01): **extend `ModeRules`**. The turn engine generalises the existing trait. H2H moves onto it only if its wire fixtures replay byte-identical (A8); otherwise H2H stays as is and the new engine is a second registry entry.
- DECIDED (Eric, 2026-11-01): **every seat requires a profile; host must be an adult profile that passed the parent gate** (client-asserted, v1). Amend F6 ("adult account" becomes "adult profile past the parent gate") and I9. Verified adult via the Flask login is a possible later upgrade. Server still needs `profile_kind` validation on create_room/join (kid cannot host) since it trusts the client today. Not checked: whether the app already refuses to go online without a profile, and CC-SECURITY-BASELINE child-data rules.
- DECIDED (Eric, 2026-11-01): **no entitlement enforcement in v1.** No StoreKit exists (CC-GAME-SERVER C7/O2), so online languages = `is_active_lang`, same as local. Amend: F6 and D-O12 drop the entitlement clause; I5 becomes "a seat cannot join a room whose language is not active"; A6 dropped (replace with an inactive-language join test). Reserve the `not_entitled` error code, unused, so adding an entitlement join hook when IAP ships needs no protocol bump. Revisit when IAP ships. §10's purchase-flow stop no longer fires.
- DECIDED (Eric, 2026-11-01): **hold H2H at the old protocol minimum.** New modes and message types ship behind a `PROTOCOL_CURRENT` bump; `SPELL_PROTOCOL_MIN` stays at 1 so shipped H2H clients are unaffected (I8). Mode-level gating: a client below the bumped version may only create/join H2H rooms; the server rejects other modes for it with `update_required`. Needs a test (A8-style) that an old-protocol client plays H2H byte-identically. Not verified: how old clients decode unknown server message types (`src/online_net.rs`, `online_model.rs` not inspected).
- Nothing left open from recon except the unmet match gate.

## 0.1 Locations (paths relative to spell-wasm-game-server unless noted)

Server `server/spell-server/src` (lines): hub.rs 963 (rooms, seats, join L~640-740, attach/resume L462-510, Outbox ring L97-115, host migration), conn.rs 208 (WS handshake, hello, protocol window L55), app.rs 440, protocol.rs 498, mode.rs 281, h2h.rs 516, grader.rs 32, audio.rs 297, store.rs 261, limits.rs 244, names.rs 423, config.rs 201. Tests: h2h_tests 678, hub_tests 652, tests/{server 534, phase_b 483, protocol_golden 57, grader_parity 30}. Also `server/grader-parity`, `server/tools/botload` (main.rs 973, PHASE-E.md 111), `server/deploy/*`. Docs: docs/CC-GAME-SERVER.md 1024, docs/recon/CC-GAME-SERVER-recon.md 515. There are no separate room/join/reconnect/resend files; all are in hub.rs.

H2H engine (`h2h.rs`): alternating turns. Scoreboard is the app's `spell_wasm::native_api::Versus`. Phases: Idle, AwaitNext, Fetching, Live, AfterCorrect, AfterMiss. Snapshot: phase, active_seat, turn_no, word_no, players[{seat,current,best,turns_left}]. Server msgs: turn_start, word_start{seat,word_no,tier,audio_ticket,deadline_ms?}, word_paused, word_result{seat,correct,chain,best,word}, turn_end, match_result, match_ended. Client msgs: next_word, submit{typed}, replay_audio, start_match, emote. Grading: `grader.rs grade()` -> `native_api::grade` -> `src/boardgame_grade.rs` (130 lines). Server never compares itself.

Envelope (`protocol.rs`): `{"v":1,"t":type,"seq":u64 (server->client),"cid":uuid (client->server),"body":{}}`, adjacently tagged serde. ENVELOPE_V=1; PROTOCOL_CURRENT=1 with a min window; mismatch gives `update_required{min,current}` and close 4001. SEAT_HOLD 60 s.

Load harness `server/tools/botload`: subcommands `run` and `tts-stub`; no scenario files, profiles are in main.rs L67 (smoke 4 rooms; load 200 rooms/10 min; soak 100/120 min; chaos 100/120 min/10% drop). 2 bots per room.

Flask/TTS: `backend/app.py:777 /api/speak`; server fetches (3 s timeout, 3 retries, 64 MB LRU).

App (main checkout): rules are Rust only (no TS copy): `src/boardgame/{mod 510, engine 547, rules 209, board 46, balance 365, golden 77, tests 1144}.rs`, `boardgame_grade.rs` 118, `boardgame_pools.rs` 302, `boardgame_ring.rs` 198, `boardgame_input.rs` 486, `boardgame_screen.rs` 1741 (setup L308, render L698, board SVG ~L917-1008, piece() L165). RNG: `crate::spelldoku::rng::Rng` (splitmix64, 43 lines) held in state. Trap enum has **5 kinds** (BackToStart, LongWord, LoseRoll, SwitchTiles, DoubleExpert; `mod.rs:80-86`), not 6; a full board has 6 trap tiles. Variants: Full 84, Sprint 42 (3 traps), Jr 40 (no traps). Boost tiles and streak: **not in `src/boardgame/`** (only a "Phase D" comment at `rules.rs:88`); worktrees `spell-wasm-bgboost`, `bgboost2` exist. Stretch: `Phase::AwaitStretch`, `Action::ChooseStretch`. NPC (D13, `engine.rs:~443`): `npc_acc_milli = clamp(human_acc - delta, 0.30, 0.95)`; human accuracy is `human_acc_milli` (`mod.rs:414`), a Bayesian prior over hits/attempts, computed in the state itself. Timer (D16): only a `timed` flag (`rules.rs:63`), a soft 30 s ring that never auto-fails; the engine has no clock. Pass-and-play: `GameConfig::pass_and_play` (`mod.rs:192`). Flags (`src/flags.rs`): `boardgame()` default ON; `board_stretch()` default OFF; no separate D-P16 list found. Specs CC-BOARD-GAME*.md are **not in the main checkout**; check sibling worktrees.

Randospell: **not found** in the main checkout by grep (`src`, `docs`, `backend`). My notes say builds 280-283 shipped, so it probably sits on a release branch or worktree. Verify before relying on it.

Spelling Bee: `src/bee.rs` 337 (seeded roster, words, elimination_round, reentry_round, placement), `src/bee_screen.rs` 439, flag `bee_sim()` default ON. No timer found in bee.rs.

Spell Off async: `backend/matches.py` 247. Routes: POST /api/match, POST /api/match/<code>/join, POST /api/match/<code>/result, GET /api/match/<code>. Login required. Compare: `_decide_winner` (L220): more correct, then lower elapsed, else tie. Client: `src/online_spelloff.rs` 332.

## 0.2 Answers

- **R1** Server-trusted client values: in online H2H, the client sends only `submit{typed}`, and the server grades it. Nothing else found. In **Spell Off async**, `matches.py submit_result` (L167-205) trusts client `score`, `correct`, `elapsed_ms` with bounds checks only. Also `hello.profile_kind` (adult/kid) is trusted.
- **R2** **No.** `word_start` carries tier and an opaque `audio_ticket` only; the word first appears in `word_result.word` (audio.rs header, G-D12). The botload tts-stub puts the word in clip bytes (test only). Tickets expire 60 s after the word resolves (hub.rs TICKET_AFTER_RESOLVE).
- **R3** **No** for new fields on existing client messages: `ClientMsg`, `RawEnvelope`, `Hello` and `CreateRoom` use `deny_unknown_fields`, and unknown `t` gives UnknownType. **Yes** for a new mode with new message types if `PROTOCOL_CURRENT` is bumped and old clients are held at `SPELL_PROTOCOL_MIN` (the designed gate); `room_state.mode_state` is already an opaque JSON value per mode. Old-client decode of new server messages was not verified (`src/online_net.rs`, `online_model.rs` not inspected in detail). Bumping the protocol would make old H2H clients get update_required, which conflicts with I8 unless H2H stays on the min window.
- **R4** Yes. `new_game(seed: u64, cfg, pools)`; `digest()` FNV hash; host/wasm golden parity (`golden.rs`, `tools/boardgame-golden`). The solo screen seeds from `now_ms ^ Math::random` (`boardgame_screen.rs:401`). Replay from one seed plus actions is supported. Caveat: the roll comes from the internal RNG, not a caller argument.
- **R5** App-only. No server crate imports boardgame code except the grader via `native_api`. `tools/boardgame-golden` is the only other consumer.
- **R6** The 2-seat load test measures (PASS checks, main.rs L856-890): all rooms started, matches > 0, score mismatches 0, seq gaps 0, seq dups 0, duplicated words 0, stalled rooms 0, server idle at start, handling p95 < 50 ms (needs --admin), load: peak connections >= 2x rooms, soak/chaos: RSS growth < 5% after warm-up, fds per connection <= 1.03x, chaos: every dropped bot resumes within 60 s or gets a clean match_ended. **No explicit error-rate threshold.** Last Rocky result: 200 rooms/400 conns, 0 mismatches, p95 0.29 ms.

## BOARD-GAME-ONLINE §0 answers (partial; its own HALT stands)

- R1 The engine is pure by design ("seeded, pure reducer", no DOM/js_sys/wall clock). Couplings: the injected grader `fn(&str,bool,&str,&str,Tier)->bool` (pulls consts, pinyin, jamo, norm, zh_sandhi: deterministic) and `spelldoku::rng`. No impure dependency found that cannot be injected.
- R2 No single `(state, roll, answer)` function. `engine::apply(&mut state, Action)` takes one action at a time: Roll, ChooseStretch, SubmitSpelling, ChooseSwitchTarget, AdvanceNpc. Smallest refactor: a wrapper that drives one full turn from an injected roll, or accept the multi-step actions as the engine's phases.
- R3 Switch Tiles target is a rules call: `Phase::AwaitSwitchTarget` then `Action::ChooseSwitchTarget(Option<u8>)`, validated with `BadTarget`. UI only collects.
- R4 NPC accuracy comes from `human_acc_milli` inside the state, so the server could compute it from verdicts it already sees (no client-reported number needed).
- R5 Yes. `human_roll` stops at `AwaitStretch`, and the word is drawn only after the choice (I-P6).
- RNG call sites: about 9-10 (board.rs:33; rules.rs:191, 199; engine.rs:43, 48, 50, 83, 172, 177, 212). Exact count for I6 needs a careful pass; the agent's own tally was inconsistent.

## Re-recon against origin/main 9812b853 (2026-11-01; supersedes every app-side finding above)

Read only, nothing compiled or run.

**BoardGame.** `src/boardgame/`: mod 610, engine 740, rules 265, board 49, golden 92, tests 1637, balance 480. Spec docs are not in the repo; D-P16 survives only in code comments. RNG: 10 direct draw sites (board.rs:33; rules.rs:213, 217, 222, 247, 255; engine.rs:50, 95, 202, 207), 15 helper calls, all on the state's `Rng`; `boardgame_pools::supply` has a separate word-selection `Rng`. Five trap kinds, three boosts (Tailwind, ExtraRoll, Ward), streak, Stretch and all variants are present. `boardStretch`, `boardBoosts`, `boardStreak` default ON with "set back to false before a public release" comments. Rules ban web_sys, js_sys, clock, floats and storage by test (`tests.rs:891`), so they should run on the server unchanged, not yet built. `with_impossible` reads client state; the server must call `impossible::pool` directly. Turn: Roll, optional ChooseStretch (word drawn after), SubmitSpelling, optional ChooseSwitchTarget (after a correct spelling), AdvanceNpc for a whole NPC turn. `apply` takes no seat, so the server authorizes it. No timeout action. Solo timer is cosmetic.

**Randospell.** `src/randospell/`: mod 1028, table 635, bank 217, payout 140, store 115, daily 32, tests 1693; screen 1796. `randospell` and `randospell_marathon` default ON here; `daily`, `h2h`, `jr`, `heat`, `call`, `stretch`, `doubles`, `ear`, `npc`, `steal`, `lastcall`, `fx` default OFF on main. Pure reducer, method-based, takes a graded bool. No golden or digest. `Session` holds a live player plus one other; `Table` holds one human plus NPCs. No state supports 3+ humans, or 2+ humans with NPCs. Languages listed: en, es. Spec doc not in the repo.

**Server.** `ModeRules` (`mode.rs:71-99`) has on_start, on_next, on_submit, on_replay, on_timer, on_disconnect, on_reconnect, on_audio, next_deadline, snapshot, abort. Missing: on_choice, ranking, restore, engine-driven turn begin. Rooms hold 2-4 seats; no NPC seats exist. No mode-specific client message exists, and `CreateRoom` has nowhere for variant, difficulty or NPC count.

**Conflicts with the specs (need Eric):**
1. **Core-crate law.** CC-GAME-SERVER limits core changes to cfg, visibility and pub-use, enforced by `game-server-core-diff-check`. BoardGame's mixed seats below change core logic.
2. **Humans + NPCs is rejected by the rules.** `engine.rs:29` returns MixedSeats unless exactly one human, pinned by `tests.rs:508`. `human_acc_milli` reads the first human only. D-B2, D-B3 and D-O5 need a rule change; solo output must stay identical (I5).
3. **Snapshot.** No serde in `src/boardgame/`; `Rng` has a private field, state holds `Arc<TierPools>` and a fn pointer, and `digest()` omits queue contents, so it cannot verify a restore. Replay (seed + cfg + pools + action log) needs no engine change and also gives I6/A7. F4 says "not an event replay" to the client; the server can rebuild by replay and still send clients a snapshot built for them.
4. **Secrets in state.** `pending.word`, `Missed.word` and unrevealed traps and boosts are in the state, so per-seat views must redact them (F7).
5. **Randospell needs an N-seat refactor**, and most v1.1 mechanics are dark, so CC-RANDOSPELL-ONLINE must say which rules apply.
6. **Release flags.** D-B9 follows the POLISH flags, but their comments say they go off before public release.

## Stop-and-ask list for Eric

1. Gate not met; continue past §0 only on your say-so.
2. H2H is turn-based, so the "two engines" premise (D-O3) and I8 need rewording; decide whether the turn engine extends the existing `ModeRules` trait.
3. Server-side entitlement does not exist; the spec calls it a check, it is a new feature. Does it touch the purchase flow (§10)? No StoreKit exists today per CC-GAME-SERVER O2.
4. Server-side adult gate does not exist; a trusted `profile_kind` is client-only. Needs an account or token design.
5. Protocol bump versus I8: how old H2H clients are held.
6. D-O16/D-B4: solo timer is a soft ring (D16), online is hard. Not a conflict by itself, but confirm.
7. Spec drift: 5 trap kinds, boost/streak code not on main, Randospell not on main, BOARD-GAME specs not in the main checkout.

## Phase notes

**Phase A (2026-11-01).** `server/spell-server/src/turn.rs` adds `TurnRules` (§5.3) and `TurnEngine`, a `ModeRules` for 2-4 seats that owns seat order, active seat and phase. The protocol is now 2: `turn.*`, `seat.status`, `game.end`, plus `turn.effects` (what a choice changed) and `turn.sync` (F4's hash echo), which the spec implied but did not list. Protocol-1 clients keep H2H, see `protocol: 1` in their welcome, and get `update_required` only for a turn mode; shipped clients also drop message types they don't know (`online_model::decode`). Hello carries `mode_versions`. Flag: `SPELL_ONLINE_TURN`, default off. The only mode is `turn_null`, compiled for tests. A8's fixture, `tests/golden/h2h_wire.txt`, was recorded on the code before this phase; `protocol.txt` gained 10 lines and changed none. Not in this phase: timers and disconnects (B), room rules (C), load (D), client code. `grader_parity` was already red on main. `a9_1` failed once in a full parallel run and passed the next three; cause not found.

**Phase B (2026-11-01).** Every phase has a hard server timer (20 s roll, mode's limit or 35 s spell, 15 s choice, never past 60 s). A drop never moves a deadline. A timeout resolves the phase through `on_auto`; a roll and a word that both time out count as one absent turn. Two absent turns in a row call `on_abandon`, and the mode's `ABANDON` makes the seat an NPC or eliminates it. Past 60 s of grace, the seat's next turn resolves at the next tick. A player can return from NPC while the game lasts (O3); one who chose Leave cannot. The hub keeps absent seats through a turn game (`keeps_absent_seats`) and closes the room only when every human has been gone 60 s; H2H's seat rules are unchanged. New rule method: `on_npc_turn`. Logs (D-O25) hold game_id, seed, event type and state hash.

Decided (Eric): logs leave out typed answers and words; D-O25 and A7 are amended. CC-SECURITY-BASELINE is still not in this repo to check against. Open: CC-BOARD-GAME-ONLINE A1 (relaunch at 40 s, same word, deadline within 1 s) cannot hold with its 35 s hard timer (D-B4): the word has already timed out. Decide at BoardGame Phase B.

**Phase C (2026-11-01).** Invite codes, the 8 fixed reactions and kid knock-to-join already existed and now have turn-mode tests. New for turn modes: a kid creating a room gets `not_adult_host`; an inactive language gets `inactive_language` at create and at join; joining sends `seat.status: connected`. H2H keeps its own refusal codes. A hello with no `profile_kind`, or an adult with no name, is already refused. The kind stays client-asserted, as signed. `not_entitled` is reserved in `turn.rs`, unused. Found while doing this: `turn.verdict` as specified sends a seat's typed answer to everyone, which breaks I9, so in a room with a kid only the seat that typed it gets it back; the others get an empty answer and the same word. Adult display names reach kids, as CC-GAME-SERVER G-D4 signed.
