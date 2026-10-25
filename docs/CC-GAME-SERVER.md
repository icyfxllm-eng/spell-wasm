# CC-GAME-SERVER v1.1 — Online play: room server, Online Head-to-Head, home-host kit

**Status: AWAITING SIGNATURE (§12).** Phase 0 Recon is done
(`docs/recon/CC-GAME-SERVER-recon.md`). v1.1 folds in every recon decision,
each using CC's recommendation as Eric instructed on 2026-10-24. Phases A–E
start only after §12 is signed.

**Release rule.** Nothing in this file may land on any `release/*` branch,
or delay any TestFlight build. Server work and client work behind
`online.enabled` (default off) ship dark on `main`.

**Signature rule.** Every decision in §2 is a recommendation until Eric
signs §12. Where a decision is marked build default, the build may use it
before signature. Eric can change it later, with no rework beyond config.

**Standing rule.** If this file conflicts with another instruction file in a
way that implies a strategy reversal, STOP and ask. Do not infer.

## Changes from v1

| # | Change | Recon ref |
|---|---|---|
| C1 | Online Head-to-Head ports local Head-to-Head **verbatim**: alternating turns, longest chain wins, 2 players. Simultaneous rounds are gone. | R2, D-R2 |
| C2 | G-D17 (think-time speed scoring) is removed, because local Head-to-Head has no speed component. It is replaced by a fair **server-clock deadline** rule for timed rooms. | R2 |
| C3 | Shared grader is the existing pure `boardgame_grade::grade`. Allowed core edits widen to **visibility** (`pub`) changes, plus cfg/feature lines. Still zero logic diff. | R1, D-R1a/b |
| C4 | Room tier is a room setting: `climb` (chain ladder, as local) or one fixed tier. The timer is a room setting, default off as local. | R2, D-R2b |
| C5 | The kid tier ceiling is the **Spell Jr policy** (`experience::serve_tier`), not a hard-coded "Medium". | R2, R5 |
| C6 | Language gate is `consts::is_active_lang`. DEV_PREVIEW does not gate languages. | R5 |
| C7 | O2 resolved: the app has no StoreKit, so online languages = `is_active_lang`, the same as local. | R8 |
| C8 | Token storage uses new keychain methods on the first-party `NativeLanguageKitPlugin`, with no third-party plugin. Online v1 is **iOS only**, because Android has no first-party plugin yet. | R10, D-R10 |
| C9 | TTS comes from the **Mac mini** backend (`/api/speak`) over Tailscale, not a Pi. | §1, D-X2 |
| C10 | Kill flags also refresh on entering the online screen and on app resume (today: launch only). | R3, D-R3 |
| C11 | New strings are machine-drafted, then marked `needs_audit`, following the ar/hi/ru/sw precedent. | R11, D-R11 |
| C12 | The image is built locally on Rocky and pinned by local digest. A registry comes with the VPS move. | R9, D-X4 |
| C13 | `cargo-deny` is new: CC adds `deny.toml`. | R9 |
| C14 | **Spell Off is kept** and gets a 3+ player option, under a separate spec (CC-SPELLOFF-MULTI). | D-X1, D-X1a |
| C15 | Rooms are sized by `ModeRules::max_seats`, so BoardGame and Randospell can later reach 4 players with no protocol change. | Eric 2026-10-24 |
| C16 | `cargo check --target x86_64-unknown-linux-gnu` passes today with 0 errors. | R1 |
| C17 | Online-only additions, needed because players no longer share a device: a `next_word` (orb tap) message, a 120 s untimed-inactivity miss, and the active-seat drop rule (F5). A16.1 is 400 connections in 200 two-seat rooms (was 100 rooms). Load runs use a canned TTS stub. | R2 |

---

## 1. Intent

Players want to tap "Play online," send a code to Mom, and be in a match in
seconds. Families will trust this with their kids, and three failures would
damage that trust more than not having online play at all: a stranger in a
kid's room, a forged score, or a match that dies mid-turn.

Priority order for every judgment call this file doesn't cover:

**kid safety > privacy > scoring correctness > uptime > latency > features**

The server runs on Eric's home machine (Rocky Linux 10) behind Cloudflare
Tunnel. Spelling matches are small turn-based traffic, so home hosting is
viable at current scale. The design must still assume the house's weak
points: power cuts, ISP outages, no inbound ports, and nobody awake at 2 a.m.

### Context

- **Host:** Rocky Linux 10 machine at Eric's home, residential internet.
- **TTS:** the Mac mini runs spellgame.net: Flask, the clip cache, and the
  Google/Azure keys (`~/spellgame-server`, tunnel `spellgame-mac`). The room
  server reaches it over Tailscale (§8 `SPELL_TTS_BASE`).
- **App:**
  - Rust/WASM core (`spell_wasm`) with a Capacitor frontend.
  - Local Head-to-Head is `src/versus.rs` (pure state) plus `game.rs:4192-4400`.
  - The remote kill flags are `src/killflags.rs`.
- **Existing online mode:** the async Spell Off (`src/online_spelloff.rs`,
  `backend/matches.py`). It is adult/account-gated on Flask and **is kept**;
  its multiplayer work lives in CC-SPELLOFF-MULTI (G-D18).
- **Related instruction files:**
  - CC-SECURITY-BASELINE v1:
    - D1 parent-gated accounts, D2 minimal child profile, D4 no third-party
      SDK gets child data.
    - Its O2 (consent mechanism, lawyer decides) blocks kid online play.
    - Its O6 is answered by recon R7: accounts are server-side in
      `climb.db`, and progress is device-local.
  - CC-BOARD-GAME v1 D11: "online later fills empty seats with NPCs." This is
    a later amendment, which consumes F7's trait.
  - CC-IMPOSSIBLE-COMPLETE D-C6: the build turns flags on; the server file can
    only kill. This file reuses that pattern.

## 2. Decisions

All are recommended and unsigned until §12.

| ID | Decision | Why |
|---|---|---|
| G-D1 | Zero inbound ports on the home router. Players reach the server only via Cloudflare Tunnel at `play.spellgame.net`. Admin access only via Tailscale. | Hides the home IP, sidesteps CGNAT, and matches the Mac mini's tunnel pattern. |
| G-D2 | **Authoritative** Rust server (tokio + axum WebSockets), as crate `spell-server` in the existing Cargo workspace. Grading calls `boardgame_grade::grade` from `spell_wasm`, compiled natively. | One grader for every online mode. Clients can't forge results, and server and app can't drift. |
| G-D3 | Private rooms by invite code only. No public matchmaking, lobby browser, friend lists, or player search. | "Friends and family" is the product, and this removes the stranger-meets-kid path entirely. |
| G-D4 | No player-authored free text, except adult display names (≤14 chars, the local Head-to-Head limit), which must pass `profanity::is_blocked`. 8 preset emotes. Kid display names are generated by the server. | No moderation burden, and the largest COPPA surface is gone. |
| G-D5 | Rooms live in memory only. A restart ends live matches with `match_ended{reason:"server_update"}`. No penalty anywhere. | Simple and honest. Persisting live matches is a v2 problem. |
| G-D6 | Rootless Podman + Quadlet under user `spellsrv`. SELinux enforcing. Read-only container, all capabilities dropped, non-root image. | Native to Rocky, and it limits the blast radius of a compromise. |
| G-D7 | Two build flags, `online.enabled` and `online.kids`, both default **off**. Only a build turns them on; the remote kill file can only force them off. Neither flag gets a Tools-hub row, because that row would be a local "on" switch (recon R3 caveat). | Same pattern as D-C6. If the box dies, players see "online play is resting," not errors. |
| G-D8 | Containerized and configured by environment variables. Moving to a VPS means copying the unit, restoring the backup, repointing the tunnel, and moving the image to a registry. | Home hosting is a stage, not the destination. |
| G-D9 | v1 ships exactly one online mode: **Online Head-to-Head, with local Head-to-Head's rules verbatim** (§F7 rules table). The engine exposes a turn-based `ModeRules` trait with `max_seats`, so BoardGame and Randospell (both up to 4) and Rune Duel plug in by later amendment. | One feature fully done beats four half-done. All three candidate modes are turn games, so one protocol serves them. |
| G-D10 | Online results write nothing to existing progress stores: Climb, Daily, shields, achievements, streaks, no-repeat history, spaced repetition, My Words, Misses, learner, word stats. Local versus already writes nothing (`game.rs:4313`). | "Do not touch scoring." No leaderboard can be gamed through online play. |
| G-D11 | Identity is an anonymous random 256-bit token. The server stores only `SHA-256(token)`. The client keeps the token in the **iOS Keychain**, through new methods on the first-party `NativeLanguageKitPlugin`. The token never appears in a URL. | Revocable, no signing key to protect, and nothing useful if the database leaks. No new third-party plugin (CC-SECURITY-BASELINE D4). |
| G-D12 | The answer word's text never leaves the server before that word resolves. Audio is delivered by opaque, room-bound tickets that the server fills from the Mac mini's TTS over Tailscale. | Prevents the obvious cheat: reading the word off the network. |
| G-D13 | Any room containing a kid is **kid-safe**: the host approves every join (knock-to-join), Kid Mode word lists apply, the tier ceiling is the **Spell Jr policy** (`experience::serve_tier`), a kid cannot host, and turns-each is 2 (the local Kid Mode rule). | Safe by construction rather than by moderation, and the same ceiling as local play so the two cannot drift. |
| G-D14 | No definition surfaces online in v1, in any language. This is a deliberate difference from local Head-to-Head, which shows the meaning after a correct word. | Avoids coupling to the per-language definitions registry gate. Can be added later by amendment. |
| G-D15 | Protocol is JSON text frames with a versioned envelope. The server supports protocol versions `[min, current]`, and older clients get `update_required`. | Old App Store builds linger for months, so this must be decided on day one. |
| G-D16 | Deploys always drain first; never auto-update. | Auto-update would kill live matches without warning. |
| G-D17 | **Timed rooms judge deadlines on the server clock.** A submit counts as on time iff `server_recv ≤ deadline + min(median_rtt_last5 / 2, 300 ms)`. Client clocks are never trusted. There is no speed score. | Local Head-to-Head scores chain length only. This keeps a phone on cellular from losing a timed word that a fiber home would have made. |
| G-D18 | **Spell Off is kept** as its own async, account-gated mode on the Flask backend. Its 3+ player option is specified in **CC-SPELLOFF-MULTI** (separate file), not here. This server does not replace it. | Eric 2026-10-24. Different stack (Flask), different identity (Climb accounts), different risks (client-submitted scores). |
| G-D19 | Room size comes from `ModeRules::max_seats` / `min_seats`. Online Head-to-Head is 2 / 2. The room layer and protocol support up to 4 seats from day one. | Eric's direction: Randospell and BoardGame eventually invite up to 3 others. |
| G-D20 | Online v1 is **iOS only**. The Android build keeps `online.enabled` off until Android has a first-party Keystore-backed token store (later amendment). | Android has no first-party native plugin today (recon R10), and a third-party one needs a D4 exception. |

### Open questions, resolved by recommendation (Eric initials in §12)

| ID | Question | v1.1 answer |
|---|---|---|
| O1 | App-only, or also spellgame.net? | **App-only** (iOS, per G-D20) |
| O2 | How is entitlement enforced for the room language? | **There is nothing to enforce.** No StoreKit exists, and local Head-to-Head allows every `is_active_lang` language. Online allows the same set. Revisit when IAP ships. |
| O3 | Can a kid profile host a room? | **No** |
| O4 | Which external uptime monitor? | Eric's choice. It must be outside the house and push to Eric's phone. |
| O5 | Backup target? | Eric's choice of Backblaze B2 or Cloudflare R2. The scripts take a restic repo URL. |
| O6 | Public hostname? | **play.spellgame.net** |
| O7 | Room size? | **2** for Online Head-to-Head (`max_seats`), up to 4 for later modes (G-D19) |
| O8 | Optional "approve joins" toggle for adult-only rooms? | The toggle exists, default off. Kid-safe rooms always require approval. |
| O9 | Room tier mode? | Host picks `climb` (chain ladder, the local default when the Climb level is selected) or one fixed tier. Kid-safe rooms clamp either through the Spell Jr policy. |
| O10 | Room timer? | Host toggle, default **off** (local default: `timed: false`, `model.rs:306`). When on, per-word limits are `consts::tier_time`: easy 12 s, medium 16 s, hard 22 s, expert 32 s. |
| O11 | English grading online? | `boardgame_grade::grade`, which uses the local fold, not Flask's `/api/check`. Solo English is untouched. |
| O12 | New UI strings? | Machine-drafted in all 14 non-English locales, listed under `needs_audit` in `translation-status.json`, with native review as an open audit item, not a gate. |

## 3. Constraints and non-goals

### Constraints (never do)

- **Never change grading, scoring, word banks, definitions gating, or any
  single-player mode.**
  - Allowed `spell_wasm` edits are only these three:
    1. `pub`/`pub(crate)` visibility changes on existing items.
    2. `#[cfg]` lines.
    3. A `native` Cargo feature plus a `pub mod native_api` re-export module
       that contains `pub use` lines only.
  - Every changed line must be one of those (F6, A6.2).
  - Client work for F10/F12/F13 lives in new files plus wiring. It must not
    change any solo code path.
- **Never execute host-changing commands.** CC writes scripts and runbooks;
  Eric runs them. CC never runs `sudo`, `firewall-cmd --permanent`,
  `dnf install`, `systemctl enable/start/stop/restart`, or
  `cloudflared tunnel create/route`. CC never touches the Cloudflare
  dashboard, DNS, the router, or the Mac mini's services.
- **Never commit secrets:** tunnel credentials, restic passwords, Tailscale
  auth keys, the kill-flag signing key. Templates use `<PLACEHOLDER>` only.
- **No port forwarding.** Never disable SELinux. No root containers. No host
  networking.
- **No new third-party SDK or Capacitor plugin in the app**
  (CC-SECURITY-BASELINE D4). Extending `NativeLanguageKitPlugin` (first-party)
  is allowed.
- **No silent fallbacks** (doctrine). Missing TTS, a missing bank, or an
  unavailable language must surface as explicit errors or states. Online audio
  never falls back to packs, human recordings, or native TTS (F9).

### Non-goals for v1 (do not build)

- Public matchmaking, chat of any kind, spectators, or persisting live
  matches across restarts.
- Online BoardGame, Randospell, or Rune Duel. These are separate amendments
  that consume F7's trait.
- Spell Off changes of any kind (G-D18, CC-SPELLOFF-MULTI).
- Android online (G-D20) and web online (O1).
- Deep links, Universal Links, push notifications, friend lists, or match
  history.
- Live keystrokes: watchers do not see the active speller's letters as they
  type.
- Writing to any existing progress store (G-D10).
- Definitions online (G-D14).

## 4. Stop-and-ask triggers

STOP and report if any of these happen:

1. `boardgame_grade::grade`, `versus`, `words`, `kid_filter`, `experience` or
   `profanity` cannot be called from a native binary without an edit outside
   the three allowed kinds in §3, or a native **link** (not only `check`)
   fails.
2. A local Head-to-Head rule in the code contradicts the §F7 rules table.
3. A ticket URL cannot be played through `native_audio::play_word(asset_id, url)`
   / `SpellAudio.playWord` with a ticket-derived asset id.
4. iOS Keychain access cannot be added to `NativeLanguageKitPlugin` without a
   new dependency or entitlement beyond `keychain-access-groups`.
5. Any field would need to be stored or logged that is not in §9 Data
   inventory.
6. A new string cannot be machine-drafted. Examples: legal text, or anything
   CC-SECURITY-BASELINE marks as counsel-reviewed. List the keys and stop.
7. Any instruction-file conflict that implies a strategy reversal.

## 5. Phase 0 — Recon: DONE

The report is at `docs/recon/CC-GAME-SERVER-recon.md`. Its decision table is
resolved by this file.

## 6. Features

Each feature lists Intent, Behavior, Invariants, and Acceptance tests.
Acceptance test IDs are `A<feature>.<n>`.

### F1 — Protocol v1

**Intent:** Every client, including two-year-old App Store builds, gets a
clear, versioned contract. Bad input fails loudly and cheaply.

**Behavior:**
- **Envelope.** Text frames only, at most 4 KiB:
  ```
  {"v":1,"t":"<type>","seq":<u64, server→client only>,"cid":"<uuid, client→server only>","body":{...}}
  ```
- **Client message types:**
  - `hello`
  - `create_room`
  - `join_room`
  - `approve_join`
  - `deny_join`
  - `leave`
  - `start_match`
  - `next_word`: the orb tap; active seat only
  - `submit`: active seat only
  - `replay_audio{variant: "normal"|"slow"}`
  - `emote`
- **Server message types:**
  - `welcome`
  - `update_required`
  - `room_state`
  - `join_request`
  - `join_pending`
  - `joined`
  - `denied`
  - `turn_start{seat, turn_no, turns_left}`
  - `word_start{seat, word_no, tier, audio_ticket, deadline_ms?}`
  - `word_paused{reason}`
  - `word_result{seat, correct, chain, best, word}`: the word text appears
    only here, after resolution
  - `turn_end{seat}`
  - `match_result{standings, winner_seat|null}`
  - `match_ended{reason}`
  - `server_draining{deadline}`
  - `error{code}`
- **`hello` is the first frame:**
  `{protocol, app_build, platform, profile_kind: "adult"|"kid", ui_lang, display_name?, token, resume?}`.
  The WebSocket upgrade itself carries no credentials.
- **`word_start` contents:** no word text and no length. Local Head-to-Head
  shows neither, since its letters row holds only what was typed (recon R2,
  rule 14).
- **Turn enforcement:** `submit` or `next_word` from a non-active seat →
  `error{not_your_turn}`, with no state change.

**Invariants:**
- Every frame is validated against the envelope version.
- Unknown, out-of-turn, or out-of-order messages never mutate room state.

**Acceptance:**
- A1.1 Golden snapshot round-trip test for every message type.
- A1.2 `hello{protocol:0}` → `update_required`, then close code 4001.
- A1.3 A 4,097-byte frame → close code 1009.
- A1.4 Any frame before `hello` → close code 4002.
- A1.5 Unknown `t` → `error{unknown_type}`. Five such frames within 60 s →
  close code 4003.
- A1.6 Proptest, 1M cases, on the frame decoder: no panics.
- A1.7 `submit` from the idle seat → `error{not_your_turn}`, and a room-state
  hash is unchanged.

### F2 — Sessions and identity

**Intent:** Players have a stable, anonymous identity with nothing worth
stealing.

**Behavior:**
- `POST /v1/session` with no body returns `{player_id, token}`.
  - `player_id`: 128-bit random.
  - `token`: 256-bit random, base64url.
- The server stores `SHA-256(token)` only.
- Tokens expire after 90 days, sliding on use. On expiry the server sends
  `error{session_expired}` and the client re-mints.
- Revocation goes through the admin listener (F11).
- **Client side:** new `NativeLanguageKitPlugin` methods `keychainGet`,
  `keychainSet` and `keychainDelete`.
  - Service `net.spellgame.app.online`.
  - Accessibility `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`.
  - No iCloud sync.
  - The token is never written to localStorage.

**Invariants:**
- Plaintext tokens never touch disk or logs on the server, or localStorage on
  the client.
- Tokens are never accepted from a URL.

**Acceptance:**
- A2.1 After 100 sessions, a byte-search of the SQLite file and logs finds no
  token bytes.
- A2.2 `GET /v1/ws?token=…` → 400.
- A2.3 An expired token → `session_expired`. A revoked token → close code 4010.
- A2.4 The sixth session mint from one IP within an hour → 429.
- A2.5 Client: after minting, a scan of every localStorage value finds no
  token. A Keychain round-trip test runs on the simulator.

### F3 — Rooms, codes, knock-to-join

**Intent:** Joining family takes one code. Joining a stranger's kid's room is
impossible without the host saying yes.

**Behavior:**
- **`create_room{mode:"h2h", lang, tier_mode:"climb"|"fixed", tier?, timed}`**
  returns a 6-character code.
  - Alphabet: `ACDEFGHJKMNPQRTUVWXY34679` (25 characters).
  - Case-insensitive, unique among live rooms.
- `lang` must satisfy `is_active_lang` (F8).
- The creator is host. Each player may have one live room at a time.
- **Seats:** `ModeRules::max_seats` caps joins, so Online Head-to-Head is
  full at 2 and a join beyond that → `error{room_full}`.
- **Joining.** If the room requires approval (always for kid-safe rooms;
  optional for others per O8):
  - the joiner gets `join_pending`, and the host gets
    `join_request{name, kind}`;
  - pending requests expire after 120 s.
- **Lifetimes:** a room closes 10 minutes after becoming empty. The hard
  lifetime is 3 hours.
- **Host leaves:** host passes to the earliest-joined adult. If there is none,
  the room closes.
- **Starting:** only the host can start, and only with at least
  `ModeRules::min_seats` seated.

**Invariants:**
- No path adds a player to a kid-safe room without an explicit `approve_join`
  from the host.
- A kid never holds host (O3).
- Seats never exceed `max_seats`.

**Acceptance:**
- A3.1 One million generated codes contain only alphabet characters.
  Collisions with live rooms retry.
- A3.2 Expiry and lifetime tests run under `tokio::time::pause`.
- A3.3 Host migration skips kids. A room with no adult left closes.
- A3.4 A kid in the room forces approval even if the host's O8 toggle is off.
- A3.5 Deny → `denied`. A request still pending at 121 s → `denied{expired}`.
- A3.6 A kid sending `create_room` → `error{not_allowed}`.
- A3.7 A third join to an H2H room → `room_full`. A test mode with
  `max_seats = 4` seats 4 and refuses the 5th.

### F4 — Abuse limits

**Intent:** A bored teenager with a script can't brute-force room codes or
exhaust the box.

**Behavior:**
- **Client IP:** taken from `CF-Connecting-IP`. It is trusted only because the
  public listener binds `127.0.0.1`. At startup, the server refuses to run if
  the public bind is non-loopback while `SPELL_TRUST_PROXY=true`.
- **Join failures:** 10 per minute per IP and 10 per minute per player →
  `error{rate_limited}` for 60 s.
- **`create_room`:** 5 per minute per player.
- **Message rate:** 20 per second sustained, burst 40 per connection → close
  code 4008.
- **Global connection cap:** `SPELL_MAX_CONNS` (default 2000) → 503 at
  upgrade.
- **Origin allowlist:** `capacitor://localhost` only (iOS; recon R10). Any
  other origin → 403. `https://localhost` (Android) is added with G-D20's
  amendment.

**Invariants:**
- Every limit is enforced server-side. Cloudflare rules are an extra outer
  layer, never the only one.

**Acceptance:**
- A4.1 The 11th bad code in 60 s → `rate_limited`.
- A4.2 A non-loopback bind with trust-proxy on → the process exits non-zero
  with a clear message.
- A4.3 A message flood → close code 4008.
- A4.4 A disallowed Origin → 403.
- A4.5 Connection cap reached → 503.

### F5 — Reconnect and resume

**Intent:** Phones drop off Wi-Fi in elevators. A 20-second blip must not
cost a match or duplicate an answer.

**Behavior:**
- **Keepalive:** the server pings every 15 s. No pong within 45 s means the
  player is disconnected.
- **Seat hold:** 60 s.
- **Idle seat drops:** the match continues. They miss nothing that is theirs.
- **Active seat drops:**
  - In a **timed** room, the word deadline keeps running. A timeout ends the
    turn exactly as a local miss does.
  - In an **untimed** room, the turn waits for the seat hold. If the player
    is not back at 60 s, the live word resolves as a miss, the turn ends, and
    the seat is released.
- **Seat released:** if fewer than `min_seats` remain, the match ends with
  `match_ended{player_left}`. There is no penalty.
- **Untimed inactivity:** an active seat that is connected but sends nothing
  for 120 s after `word_start` resolves that word as a miss. This is an
  online-only rule; local play has no cap because both players share a
  device.
- **Resume:** `hello{resume:{room, last_seq}}` makes the server replay events
  after `last_seq` from a per-room ring buffer (512 events). If the gap is
  larger than the buffer, the server sends a full `room_state` snapshot
  instead.
- **Idempotent submits:** `submit` carries a `cid`. A duplicate `cid` within a
  match is acknowledged but not re-graded.

**Invariants:**
- A word resolves at most once.
- A reconnecting client never sees an event twice or misses one.

**Acceptance:**
- A5.1 Drop for 30 s and resume → same seat, same chain and best, and the
  event sequence is gapless with no duplicates.
- A5.2 Active seat drops for 61 s in an untimed room → the word resolves as a
  miss, the seat is released, and `match_ended{player_left}` follows.
- A5.3 The same `cid` submitted 3 times → graded once.
- A5.4 Missing 600 events → snapshot path, and the resulting state is
  identical.
- A5.5 An untimed room's active seat idle for 120 s → miss, and the turn
  passes.

### F6 — Shared grader

**Intent:** Every online mode judges "K-A-T" with one function, the one
BoardGame already uses, compiled natively. A reimplementation would drift.

**Behavior:**
- The server calls `boardgame_grade::grade(lang, kid, typed, entry, tier)`
  (`src/boardgame_grade.rs:25`) through `spell_wasm::native_api`.
- `kid` is true for every word in a kid-safe room. This matches local Kid
  Mode, which is accent-lenient for Spell Jr.
- NFC normalization happens inside the existing primitives only.
- **Known, accepted differences from local Head-to-Head** (recon R1 addendum):
  - English is graded by the local fold, not Flask `/api/check`.
  - Impossible-word accepted alternates are accepted for every word.
  - The zh sandhi entry is looked up from the bank entry.
  - These are listed, not fixed. Solo play is untouched.

**Invariants:**
- Native and WASM verdicts are byte-identical for every input.

**Acceptance:**
- A6.1 **Parity fixture:**
  - 1,000 deterministic `(entry, typed, kid, tier)` cases per language × all
    `is_active_lang` languages, drawn from the bank with seeded perturbations:
    typos, NFC/NFD forms, case, homophones, variants, pinyin tones, jamo.
  - The WASM and native verdict JSON are identical.
  - The WASM side runs under `wasm-bindgen-test` or the Playwright test seam.
- A6.2 CI prints the `spell_wasm` diff. The phase report includes it, and
  every changed line is a `pub`, `#[cfg]`, feature, or `pub use` line.
- A6.3 **Informational, not gating:** a report counts where
  `boardgame_grade::grade` and the solo `submit_guess` ladder disagree on the
  A6.1 fixture, per language, for Eric's review.

### F7 — Online Head-to-Head rules engine

**Intent:** Online Head-to-Head must play exactly like local Head-to-Head.
Future modes plug in without touching the room layer.

**Behavior:**
- The trait:
  ```rust
  trait ModeRules {
      fn max_seats(&self) -> u8;
      fn min_seats(&self) -> u8;
      fn on_start(..) -> Vec<Event>;
      fn on_next(..) -> Vec<Event>;
      fn on_submit(..) -> Vec<Event>;
      fn on_timer(..) -> Vec<Event>;
      fn on_disconnect(..) -> Vec<Event>;
  }
  ```
  Implementations are deterministic given (seed, inputs, server timestamps).
- The H2H implementation **wraps `crate::versus::Versus` unchanged**: it calls
  `start`, `record_correct`, `end_turn` and `winner`. No rule is
  re-implemented.
- **Rules table** (from recon R2; one A7.1 test per row):

| # | Rule | Online value | Local source |
|---|---|---|---|
| 1 | Players | Exactly 2 | `versus.rs:39-45` |
| 2 | Names | Adult: typed, ≤14 chars, profanity-checked. Kid: generated (F10) | `index.html:4333` |
| 3 | Structure | Alternating turns; a turn lasts until the first miss | `versus.rs:88-106` |
| 4 | Turns each | 3; **2 if the room is kid-safe** | `game.rs:4212` |
| 5 | Attempts per word | 1, with no extra attempt and no shields | `game.rs:2910, 2930` |
| 6 | Correct | `current += 1`, `best = max(best, current)` | `versus.rs:78-85` |
| 7 | Miss / timeout | Turn ends: `current = 0`, `turns_left -= 1`, hand off | `versus.rs:90-106` |
| 8 | Turn order | Strict alternation while both have turns; else the side with turns plays out | `versus.rs:97-105` |
| 9 | End | Neither side has turns left | `versus.rs:102` |
| 10 | Winner | Higher `best`; equal → tie | `versus.rs:110-121` |
| 11 | Tier | `climb`: `tier_for_streak(current)` (0-2 easy, 3-6 medium, 7-11 hard, 12+ expert). `fixed`: the room tier. Both pass through `clamp_base_tier` for kid-safe rooms | `game.rs:230-277, 1876` |
| 12 | Timer | Room setting (O10). When on, `tier_time(tier)` per word, judged per G-D17 | `game.rs:3980`, `consts.rs:3` |
| 13 | Speed | None | `versus.rs` |
| 14 | Word display | No length, no slots | `game.rs:355-380` |
| 15 | After a correct word | Next word after `CORRECT_DELAY_MS` (2200 ms). **No definition** (G-D14) | `game.rs:3864`, `consts.rs:448` |
| 16 | After a miss | Next turn after 1300 ms | `game.rs:4349-4351` |
| 17 | Turn start | Waits for the active seat's `next_word` (the orb tap) | `game.rs:4281-4305` |
| 18 | Persistence | None (G-D10) | `game.rs:4313` |

- `tier_for_streak` and `clamp_base_tier` are private in `game.rs`, a
  DOM-bound file. `native_api` re-exports them. If re-exporting requires
  moving them out of `game.rs`, that is a logic move: **STOP (trigger 1)**.
- **Rematch** (local rule 18): host sends `start_match` again, with the same
  seats and a fresh seed.

**Invariants:**
- The room layer contains no mode-specific logic.
- Replaying a recorded event log reproduces the identical result.

**Acceptance:**
- A7.1 One test per rules-table row.
- A7.2 1,000 recorded matches replay to identical results.
- A7.3 Timed room: two simulated clients at 40 ms and 250 ms RTT each submit
  50 ms before the deadline as measured on their own clock. Both are on time.
  A submit 400 ms late at 40 ms RTT is a timeout.
- A7.4 A property test drives `Versus` locally and through the server with the
  same verdict sequence, and gets an identical scoreboard.

### F8 — Word selection

**Intent:** Online words come from the same audited, gated banks as the app.
Kid rooms never serve a word above the Spell Jr ceiling or on an exclusion
list.

**Behavior:**
- Words come from `words::tier_for(lang, tier)` (compiled-in banks, PURITY
  already enforced at build time; recon R5). For kid-safe rooms they then pass
  through `kid_filter::filter_kid(lang, pool)`.
- The server passes the junior/kid state explicitly. It never relies on the
  storage-backed `junior_profile()` path, which returns false natively.
- **Language gate:** `consts::is_active_lang(lang)` as compiled for the app
  (not the `web` feature). Today this means all Active languages except
  RTL-blocked ones.
- The per-match seed comes from a CSPRNG. No word repeats within a match.
- Online play does not read or write the player's no-repeat history (G-D10).

**Invariants:**
- No gating rule is added or relaxed relative to local Head-to-Head.

**Acceptance:**
- A8.1 10,000 simulated matches per language: zero in-match repeats.
- A8.2 Kid-safe simulations: zero words above the Spell Jr ceiling, and zero
  words on `kid-exclude` lists.
- A8.3 `create_room{lang:"ar"}` (RTL-blocked) → `error{lang_unavailable}`. So
  does any lang for which `is_active_lang` is false.

### F9 — Audio tickets (no answer leakage)

**Intent:** The player hears the word but cannot read it off the wire.

**Behavior:**
- **Tickets.** `word_start.audio_ticket` is 128-bit random, valid until the
  word resolves + 60 s. Each variant (normal/slow) gets its own ticket.
  `replay_audio{variant}` returns the same ticket for the same variant.
- **Audio route.** `GET /v1/audio/{ticket}` requires
  `Authorization: Bearer <token>` from a member of that room.
- **Upstream fetch.** The server fetches
  `GET {SPELL_TTS_BASE}/api/speak?word=&variant=&lang=[&py=]`, using exactly
  the app's request (recon R4). For zh, `py` is the bank entry's pinyin half.
  The voice table and cache live on the Mac mini, so they are inherited, not
  copied.
- In-memory LRU cache, capped at 64 MB.
- **Response headers:** `Cache-Control: no-store`, no `Content-Disposition`,
  and no `ETag` derived from the word. ID3 metadata is stripped before
  serving.
- **TTS failure:** 3 retries within 5 s, then `word_paused{audio_unavailable}`.
  Three consecutive paused words → `match_ended{audio_unavailable}`.
- **Client.** A new `api::play_ticket(url, variant)` plays through
  `native_audio::play_word(asset_id, url)`.
  - The asset id is `t:<ticket>`, never derived from the word.
  - `audio-native.js` `ensureCached` gains an optional Authorization header.
  - Online play **never** calls the Human / Pack / ServerCache / NativeTts
    chain or `audio_verdict::servable`. Audio failure is the explicit paused
    state (no silent fallbacks).

**Invariants:**
- No byte representing the target word reaches a client before that word's
  `word_result`.

**Acceptance:**
- A9.1 **Leak test.**
  - Capture every server→client byte (WebSocket and audio HTTP) for 500 words
    across all active languages.
  - Search for the target in four forms: NFC, NFD, lowercase, and the Mac
    mini cache-key md5 hex.
  - Expect zero hits before each word's `word_result`.
- A9.2 A non-member token or an expired ticket → 404, not 403.
- A9.3 With a mocked TTS outage, pause and end behavior happen exactly per the
  rule.
- A9.4 Client network log for an online match: zero requests to
  `/api/speak`, packs, or native TTS.

### F10 — Kid safety layer

**Intent:** A child can play with Grandma, and nobody else can get near them.

**Behavior:**
- **Client gating.** The online entry appears for a kid (`AppState.kid ||
  age_locked`) only if all three hold:
  1. The `online.kids` build flag is on and not remotely killed.
  2. A parent enabled "Play online with family" behind `parent_gate_then`.
     This is a new device setting, default off.
  3. `online.enabled` is on.
- **Server side:**
  - A kid `hello` marks the room kid-safe.
  - The kid's display name is generated by the server as
    `<Color> <Piece> <NN>`, from localized fixed lists. The pieces reuse
    BoardGame's wizard hat, wand, bee, and book.
  - The lists are machine-drafted per O12.
- The server never stores `profile_kind` (§9).
- `online.kids` stays off until CC-SECURITY-BASELINE O2 is resolved by
  counsel (§10.11).

**Invariants:**
- A kid never types anything other than spellings, and never sees
  player-authored text.
- No kid-safe room exists without approval-gated joins.

**Acceptance:**
- A10.1 A kid without the parent toggle → no online entry, and no request to
  `play.spellgame.net` (network log assertion).
- A10.2 Toggle on but `online.kids` off → no entry.
- A10.3 A kid's `display_name` in `hello` is ignored, and the server-generated
  name is used.
- A10.4 A3.4 and A8.2 both pass.

### F11 — Health, admin listener, logs

**Intent:** Eric can see and steer the server from his phone or laptop over
the tailnet, and nothing private leaks into logs.

**Behavior:**
- **Public listener `127.0.0.1:8080`** routes only:
  - `GET /healthz`
  - `POST /v1/session`
  - `GET /v1/ws`
  - `GET /v1/audio/{ticket}`
  - Everything else → 404.
- **Admin listener `127.0.0.1:8081`**, reached via SSH over Tailscale and never
  in the tunnel:
  - `GET /admin/stats`
  - `GET /admin/readyz`: healthz + not draining + TTS reachable
  - `POST /admin/drain`
  - `POST /admin/snapshot`
  - `POST /admin/revoke/{player_id}`
- **Logs:** tracing JSON to stdout, which lands in journald.
  - IPs are truncated to /24 (IPv4) and /48 (IPv6).
  - Never log tokens, tickets, display names, or word text at info level or
    above.
- **Drain**, on SIGTERM or `/admin/drain`:
  - `readyz` goes false and `server_draining{deadline}` is broadcast.
  - New rooms are refused.
  - The server waits until rooms reach 0 or `SPELL_DRAIN_SECS` passes, sends
    `match_ended{server_update}` to any remaining, and exits 0.

**Invariants:**
- The admin surface is unreachable from the internet.

**Acceptance:**
- A11.1 Route-table test: every unlisted path on 8080 → 404, and every
  `/admin/*` path on 8080 → 404.
- A11.2 Log-scrape test: run a full bot match at info level, then grep for
  the token, ticket, display names, word text, and full IPs → zero hits.
- A11.3 Drain test: a live match finishes, a new room is refused, and the
  process exits 0.

### F12 — Client: online entry and room UX (Capacitor, iOS)

**Intent:** Create, share, join, play, and see results in under a minute, with
no account screens.

**Behavior:**
- "Play online" lives inside the Head-to-Head setup sheet (`vsSetupScrim`),
  next to the local Start.
- **Create:** pick a language (`is_active_lang` list), tier mode (O9), and
  timer (O10).
- **Join:** a 6-character input that uppercases automatically and rejects
  characters outside the alphabet with an inline hint.
- **Lobby:**
  - player list, host badge, kid-safe badge
  - Share button (`@capacitor/share`, already present; plain text containing
    the code)
  - host Start, and approval prompts
- **Match:**
  - Reuses the Head-to-Head bar (`vsBar`) and the orb.
  - The active seat taps the orb (`next_word`), hears ticket audio, and types.
  - The idle seat sees "<name> is spelling", the chain count, and each word's
    result as it resolves.
  - Emote bar.
- Results screen, with Rematch and Leave. Leave is the word `aria.exit`,
  reachable at every point including the end screen (every-mode-has-an-exit
  law, `mode-exit-check.mjs`).
- The token lives in the Keychain (F2).
- New strings follow O12, so they pass `i18n-check` and
  `i18n-translated-check`.

**Invariants:**
- The client never computes or displays a score the server didn't send.
- Every element id the Rust side touches exists (`dom-id-live-check.mjs`).

**Acceptance:**
- A12.1 Playwright (web test build, with a test seam supplying the token) and
  Maestro (iOS): create → share → join from a second device → start → full
  match → results.
- A12.2 A wrong code shows an inline error with no crash.
- A12.3 After 10 bad codes, the rate-limit message shows.
- A12.4 Emote round trip under 1 s on LAN.
- A12.5 `mode-exit-check.mjs` and `dom-id-live-check.mjs` pass with the online
  screens included.

### F13 — Client: flags, kill switch, failure states

**Intent:** When anything is wrong, players see a calm, specific message, and
local play always works.

**Behavior:**
- `killflags::refresh()` additionally runs on entering the online screen and
  on app resume. Today it runs at launch only (recon R3).
- States:

| State | What the player sees |
|---|---|
| Build flag off | Entry hidden; zero network calls |
| Remote kill | "Online play is resting — local Head-to-Head still works" card |
| Server unreachable | Same resting card, after 2 failed attempts with backoff |
| `update_required` | "Update Spell to play online" with App Store link |
| Draining | Banner with countdown |
| Mid-match drop | Reconnect overlay with a 60 s countdown |
| `match_ended{reason}` | Reason-specific message, no penalty language |

**Invariants:**
- Local Head-to-Head is reachable in every state.

**Acceptance:**
- A13.1 Component tests: every server state fixture renders the right screen.
- A13.2 Airplane mode for 20 s mid-match → overlay → resume.
- A13.3 Remote kill published while a player sits in the lobby → leaving and
  re-entering the online screen shows the resting card, with no relaunch.
- A13.4 Build flag off → the network log shows zero requests to
  `play.spellgame.net`.

### F14 — Deploy kit (`server/deploy/`)

**Intent:** Eric deploys with one command, and moving to a VPS later is a copy.

**Files:**
- **`Containerfile`:** multi-stage build. The runtime image is
  `gcr.io/distroless/cc-debian12:nonroot`, with no shell.
- **`build.sh`:** runs `podman build` on Rocky as `spellsrv` and prints the
  local image digest.
- **`spellgame.container`** (user Quadlet):
  - `Image=localhost/spellgame-server@sha256:<DIGEST>`
  - `PublishPort=127.0.0.1:8080:8080` and `PublishPort=127.0.0.1:8081:8081`
  - `Volume=spell-data:/data:Z`
  - `ReadOnly=true`, `NoNewPrivileges=true`, `DropCapability=all`
  - `PodmanArgs=--memory=1g --pids-limit=256`
  - `[Service] Restart=always`, `RestartSec=2`, `TimeoutStopSec=960`
  - `[Install] WantedBy=default.target`
- **`cloudflared-config.template.yml`:** ingress `<HOSTNAME>` →
  `http://127.0.0.1:8080`, catch-all `http_status:404`, and placeholders only
  (`<TUNNEL_ID>`, `<CREDENTIALS_FILE>`). This is a **new tunnel on Rocky**, not
  `spellgame-mac`.
- **`deploy.sh <digest>`:** drain → set the digest in the unit → restart →
  wait for healthz (timeout 120 s) → print status.
- **Backup:** `backup.sh`, `spell-backup.service` and `spell-backup.timer`
  (daily 04:17). Steps: `/admin/snapshot` → read the snapshot via
  `podman unshare` → restic backup → write a success stamp file.
- **`restore-drill.sh`:** restore the latest snapshot to a scratch directory,
  compare row counts with live, and print PASS/FAIL.
- **`journald-spell.conf`:** `MaxRetentionSec=14day`.
- **`deny.toml`** at the repo root, for cargo-deny.

**Invariants:**
- No secrets in any file.
- Port 8081 never appears in the tunnel template.
- The podman-auto-update timer is never enabled (G-D16).

**Acceptance:**
- A14.1 `podman build` succeeds in CI (ubuntu runner).
- A14.2 `podman inspect` shows a non-root user.
- A14.3 `podman run --entrypoint sh` fails.
- A14.4 `/usr/libexec/podman/quadlet -dryrun -user` passes on the unit.
- A14.5 shellcheck is clean on all scripts.
- A14.6 Grep proves the tunnel template has no 8081 and does have the 404
  catch-all.
- A14.7 A gitleaks scan is clean.
- A14.8 `cargo deny check` passes.

### F15 — Host runbook and `verify-host.sh`

**Intent:** Eric hardens the box himself, step by step, and one read-only
script proves it's right.

**Behavior:** `server/deploy/HOST-RUNBOOK.md` gives these steps in order, each
with its verification command:

0. Set a real hostname. Today it is `localhost`.
1. UPS and NUT
2. BIOS "restore on AC power loss"
3. Wired Ethernet
4. Mask sleep targets
5. `dnf-automatic` with security-only updates
6. Tailscale on **Rocky and the Mac mini**. Make the Mac mini backend
   reachable from Rocky over the tailnet. Eric changes Mac mini services
   himself.
7. sshd hardening drop-in
8. firewalld: an empty public zone, and a tailnet zone with ssh only. Disable
   any desktop listeners that H5 flags (cups, avahi, …).
9. `spellsrv` user and linger
10. journald retention
11. Tunnel creation and DNS for `play.spellgame.net` (Eric, in Cloudflare)
12. Image build and Quadlet install
13. Backup timer
14. External monitor (O4)

`verify-host.sh` is read-only. It may use sudo for reads only. It prints
PASS/WARN/FAIL per check and exits non-zero on any FAIL:

| ID | Check |
|---|---|
| H0 | Hostname is not `localhost` |
| H1 | SELinux is Enforcing |
| H2 | `sshd -T` shows password auth no and root login no |
| H3 | Public zone services ⊆ {dhcpv6-client} |
| H4 | `tailscale0` is in a zone that allows ssh |
| H5 | No non-loopback TCP listener other than sshd |
| H6 | Sleep targets are masked |
| H7 | `dnf-automatic.timer` is active with `upgrade_type=security` |
| H8 | Linger is on for `spellsrv` |
| H9 | Quadlet service is active |
| H10 | cloudflared is active |
| H11 | healthz is 200 on localhost and via the public hostname |
| H12 | Last backup stamp is less than 26 h old |
| H13 | Disk usage is below 80% |
| H14 | chrony is synchronized |
| H15 | journald retention is set |
| H16 | upsmon is active (WARN if no UPS) |
| H17 | podman-auto-update timer is not enabled |
| H18 | `SPELL_TTS_BASE/api/health` is 200 from Rocky over the tailnet |

**Invariants:**
- CC never executes the runbook.
- `verify-host.sh` contains no write operations.

**Acceptance:**
- A15.1 shellcheck is clean.
- A15.2 A static check finds none of the following:
  - `systemctl (start|stop|enable|disable|restart)`
  - `firewall-cmd --permanent`
  - `dnf (install|remove)`
  - `rm`
  - `tee`
  - file redirection other than to `/dev/null`

### F16 — Load, soak, and chaos harness (`server/tools/botload`)

**Intent:** Prove capacity and correctness with numbers before any real family
connects.

**Behavior:** A Rust bot that:
- Creates and joins rooms and plays full matches with seeded right and wrong
  answers.
- Randomly disconnects and resumes.
- Independently computes each bot's expected scoreboard by driving its own
  `versus::Versus` with the same verdicts.
- Runs through the real tunnel against `play.spellgame.net` before any build
  turns `online.enabled` on, so no real players can reach it during testing.
- Uses a test TTS mode. The server env `SPELL_TTS_BASE` points at a canned-clip
  stub on Rocky, so load runs do not hammer the Mac mini or Google billing. One
  final soak uses the real Mac mini.

**Acceptance:**
- A16.1 **Load:** 400 connections / 200 rooms. Server handling p95 under
  50 ms (from `/admin/stats`), zero lost or duplicated words, and zero
  scoreboard mismatches against bot expectations.
- A16.2 **Soak:** 2 h at 50% load. Memory grows less than 5% after warm-up, and
  the file-descriptor count is flat.
- A16.3 **Chaos:** during the soak, restart cloudflared once and drop 10% of
  sockets. Every affected bot resumes within 60 s or gets a clean
  `match_ended`.

## 7. Phases

| Phase | Features | Notes |
|---|---|---|
| 0 Recon | R1–R11 | **Done** |
| A | Server core: F1–F6, F11 | Includes the `native_api` visibility diff (A6.2) |
| B | Online Head-to-Head: F7, F8, F9, F10 (server side) | |
| C | Client (iOS): F2 client, F9 client, F10 client, F12, F13 | Includes the `NativeLanguageKitPlugin` keychain methods |
| D | Deploy and host: F14, F15 | |
| E | Proof: F16 | Then a go/no-go report, then STOP |

- Each phase ends with `docs/reports/CC-GAME-SERVER-phase-<X>.md`. It lists
  every acceptance test, the command run, and its result.
- Advance to the next phase only when all of the phase's tests pass. If a test
  is red and the fix is in scope, fix it. Otherwise STOP.
- Work happens in worktrees off `origin/main`. Concurrent sessions share
  spell-wasm, so check `git log` before every push.
- Phase E runs against the real host, which requires Eric to have completed
  the F15 runbook. If the runbook isn't done, Phase E pauses with a checklist
  of the missing H-checks.

## 8. Configuration

All via environment variables, set in the Quadlet unit.

| Variable | Default |
|---|---|
| `SPELL_BIND_PUBLIC` | `127.0.0.1:8080` |
| `SPELL_BIND_ADMIN` | `127.0.0.1:8081` |
| `SPELL_TRUST_PROXY` | `true` |
| `SPELL_DB_PATH` | `/data/spell.db` |
| `SPELL_TTS_BASE` | none; required. The tailnet URL of the Mac mini backend. Startup fails if unset. |
| `SPELL_ALLOWED_ORIGINS` | `capacitor://localhost` |
| `SPELL_MAX_CONNS` | `2000` |
| `SPELL_PROTOCOL_MIN` | `1` |
| `SPELL_DRAIN_SECS` | `900` |
| `SPELL_LOG` | `info` |

App side: a new `window.SPELL_PLAY_BASE` (`wss://play.spellgame.net`) sits
beside `SPELL_API_BASE`.

## 9. Data inventory

The server may store or log only what is listed here. Anything else triggers
stop-and-ask #5.

| Where | Fields | Retention |
|---|---|---|
| SQLite `players` | `player_id`, `token_hash`, `created_day`, `last_seen_day`, `revoked` | Until revoked, or 12 months inactive |
| Memory only | Rooms, display names, `profile_kind`, scoreboards, emotes, audio cache, tickets | Ends at room close |
| Logs (journald) | Timestamp, `player_id`, event type, error code, truncated IP | 14 days |
| Client | Token (iOS Keychain), `player_id` | Until app deletion or re-mint |

**Acceptance A-DATA.1:** a schema test asserts that the `players` table has
exactly these columns, and that no other table exists except the `_probe`
table used by the health check.

## 10. Go/no-go

Eric owns this, before any build turns `online.enabled` on:

1. `verify-host.sh` shows all PASS (an H16 WARN is acceptable only if the UPS
   is on order).
2. External `nmap -Pn` against the home IP, run from cellular, shows 0 open
   ports.
3. Unplug test: the box returns to a healthy state unattended within 5
   minutes.
4. `kill -9` of the server → healthy within 10 s. `sudo reboot` → healthy
   within 5 minutes.
5. The uptime alert arrives on Eric's phone within 5 minutes of stopping the
   container.
6. `restore-drill.sh` → PASS.
7. The Phase E report shows A16.1–A16.3 green.
8. TestFlight: a real match across two different homes, plus one player on
   cellular.
9. TestFlight: remote kill → resting card. Local Head-to-Head still works.
10. Privacy policy, App Store privacy label (identifier used for app
    functionality), and age-rating questionnaire, all re-answered for online
    play. Eric reviews them before submission (build 48's 2.3.6 rejection was
    an age-rating metadata mismatch).
11. `online.kids` only: CC-SECURITY-BASELINE O2 resolved by counsel. Not
    required for `online.enabled`.
12. Mac mini TTS reachable from Rocky over the tailnet (H18), and the Mac mini
    still serves spellgame.net normally afterwards.

## 11. Definition of done (CC)

CC is done when all of the following pass and every phase report exists:

```
cargo test -p spell-server --all-features
cargo test -p spell-server --test grader_parity        # plus the WASM-side runner (A6.1)
cargo clippy -p spell-server -p botload --all-targets -- -D warnings
cargo deny check
cargo test -p spell-server frame_decoder_proptest       # 1M cases (A1.6)
podman build -f server/deploy/Containerfile .
/usr/libexec/podman/quadlet -dryrun -user               # with the unit in place
shellcheck server/deploy/*.sh
gitleaks detect
bash scripts/gate.sh                                    # app gates still green
<Playwright (web test build) + Maestro (iOS) suites for F12/F13>
cargo run -p botload --release -- --target wss://play.spellgame.net --profile load|soak|chaos
```

Eric's go/no-go (§10) is separate and comes after.

## 12. Signature

Eric initials each decision or writes a change.

- [ ] G-D1 zero inbound ports / tunnel / Tailscale admin
- [ ] G-D2 authoritative Rust server; grader = `boardgame_grade::grade`, native
- [ ] G-D3 private invite-code rooms only
- [ ] G-D4 no free text except profanity-checked adult names; preset emotes; generated kid names
- [ ] G-D5 in-memory rooms; restart ends matches without penalty
- [ ] G-D6 rootless Podman + Quadlet, SELinux enforcing
- [ ] G-D7 build-on / remote-kill-only flags, both default off, no Tools-hub row
- [ ] G-D8 containerized, env-configured, VPS-portable
- [ ] G-D9 Online Head-to-Head only in v1, local rules verbatim; turn-based `ModeRules`
- [ ] G-D10 online writes nothing to existing progress stores
- [ ] G-D11 anonymous hashed tokens; iOS Keychain via `NativeLanguageKitPlugin`
- [ ] G-D12 audio tickets, no word text before resolve
- [ ] G-D13 kid-safe rooms (approval, Kid lists, Spell Jr ceiling, kid can't host, 2 turns)
- [ ] G-D14 no definitions online in v1
- [ ] G-D15 JSON protocol with version window
- [ ] G-D16 drain deploys, no auto-update
- [ ] G-D17 server-clock deadlines for timed rooms, RTT grace capped at 300 ms, no speed score
- [ ] G-D18 Spell Off kept; its 3+ player option goes in CC-SPELLOFF-MULTI
- [ ] G-D19 rooms sized by `ModeRules::max_seats`, up to 4 supported
- [ ] G-D20 online v1 is iOS only
- [ ] §3 allowed core edits: visibility + cfg + `native` feature / `pub use` only
- Open (initial to accept the v1.1 answer):
  - O1 ___
  - O2 ___
  - O3 ___
  - O4 ___ (monitor: ______)
  - O5 ___ (B2 / R2: ______)
  - O6 ___
  - O7 ___
  - O8 ___
  - O9 ___
  - O10 ___
  - O11 ___
  - O12 ___

Signed: ________ Date: ________
