# CC-GAME-SERVER — Phase A report (server core: F1–F6, F11)

**Result: every Phase A acceptance test passes.** Branch `game-server-phase-a`,
worktree `~/repos/spell-wasm-game-server`, based on `origin/main` @ `113606b9`.
Date: 2026-10-24. Phase B (Online Head-to-Head) has not started.

## What exists now

| Path | What |
|---|---|
| `server/spell-server/` | The room server crate: `protocol`, `config`, `store` (SQLite players), `limits`, `hub` (rooms, knock-to-join, seat holds, resume), `mode` (the `ModeRules` trait + a trivial test mode), `conn` (one WebSocket), `app` (public + admin routers), `grader`, `names`, `main` |
| `server/grader-parity/` | The A6.1 fixture (16 languages × 1,000 cases), its golden file, the WASM-side test, and the A6.3 solo comparator |
| `src/native_api.rs` | Re-exports only: the server's whole window into the app core |
| `scripts/game-server-core-diff-check.mjs` | A6.2: prints the core diff and fails on any line that isn't cfg, feature, visibility or `pub use` |

Online Head-to-Head is **not** in this phase. `create_room{mode:"h2h"}`
returns `error{mode_unavailable}` until Phase B. Every room-layer test runs
against a deliberately trivial `test` mode (`--features test-modes`, never in
the deploy image).

## Acceptance tests

Commands, all run from the worktree root:

```
cargo test -p spell-server --all-features
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p grader-parity --target wasm32-unknown-unknown
cargo clippy --no-deps -p spell-server -p grader-parity --all-targets --all-features -- -D warnings
node scripts/game-server-core-diff-check.mjs --selftest
node scripts/game-server-core-diff-check.mjs 113606b9
```

| Test | Where | Result |
|---|---|---|
| A1.1 golden snapshot + round trip, every message type | `tests/protocol_golden.rs`, `src/protocol.rs` tests | PASS |
| A1.2 `protocol:0` → `update_required`, close 4001 | `tests/server.rs` | PASS |
| A1.3 4,097-byte frame → close 1009 (4,096 is still a frame) | `tests/server.rs` | PASS |
| A1.4 frame before hello → close 4002 (also non-JSON) | `tests/server.rs` | PASS |
| A1.5 unknown `t` → `error{unknown_type}`; 5 in 60 s → 4003 | `tests/server.rs` | PASS |
| A1.6 1,000,000 cases on the decoder, no panic (500k arbitrary + 500k mutated valid frames) | `tests/frame_decoder_proptest.rs` | PASS (61 s debug, 10 s release) |
| A1.7 out-of-turn `next_word`/`submit`/`replay_audio` → `not_your_turn`, room state byte-identical | `src/hub_tests.rs` | PASS |
| A2.1 100 sessions: no token bytes in the DB, WAL, a VACUUM copy, or logs | `tests/server.rs` | PASS |
| A2.2 `GET /v1/ws?token=…` → 400 | `tests/server.rs` | PASS |
| A2.3 expired → `session_expired`; revoked → 4010, both live and on reconnect | `tests/server.rs`, `src/store.rs` | PASS |
| A2.4 sixth mint from one IP in an hour → 429 | `tests/server.rs`, `src/limits.rs` | PASS |
| A3.1 1M codes alphabet-only; 2,000 live codes unique; case-insensitive | `src/hub_tests.rs` | PASS |
| A3.2 lifetimes under `tokio::time::pause` (pending 120 s, seat hold, 3 h) | `src/hub_tests.rs` | PASS (see note 6) |
| A3.3 host migration skips kids; no adult left → closes | `src/hub_tests.rs` | PASS |
| A3.4 a kid forces approval with the toggle off (the kid's own knock too) | `src/hub_tests.rs` | PASS |
| A3.5 deny → `denied{host}`; pending at 121 s → `denied{expired}` | `src/hub_tests.rs` | PASS |
| A3.6 kid `create_room` → `not_allowed` | `src/hub_tests.rs` | PASS |
| A3.7 a third join to a 2-seat room → `room_full`; 4-seat mode seats 4, refuses the 5th | `src/hub_tests.rs` | PASS |
| A4.1 11th bad code in 60 s → `rate_limited`, held for 60 s | `src/limits.rs`, `src/hub_tests.rs` | PASS |
| A4.2 non-loopback bind + trust-proxy → the **binary** exits non-zero with the reason | `tests/process.rs`, `src/config.rs` | PASS |
| A4.3 message flood → 4008 | `tests/server.rs` | PASS |
| A4.4 disallowed or missing Origin → 403 (ws and session) | `tests/server.rs` | PASS |
| A4.5 connection cap → 503 at upgrade | `tests/server.rs` | PASS |
| A5.1 30 s drop → same seat, replay gapless from `last_seq + 1`, no duplicates | `src/hub_tests.rs`, `tests/server.rs` (real socket) | PASS |
| A5.2 61 s drop → seat released, room told; match ends `player_left` | `src/hub_tests.rs` | PASS (generic room rule; the H2H turn rules are Phase B) |
| A5.3 the same `cid` ×3 → graded once | `src/hub_tests.rs` | PASS |
| A5.4 600-event gap → snapshot path, equal to the live state | `src/hub_tests.rs` | PASS |
| A6.1 native and WASM verdicts identical: 16,000 cases, one golden, two targets | `tests/grader_parity.rs` + `grader-parity/tests/wasm.rs` | PASS |
| A6.2 every changed core line is cfg/feature/visibility/`pub use` | `scripts/game-server-core-diff-check.mjs` (+ selftest) | PASS |
| A6.3 (informational) shared vs solo disagreements | `tests/grader_differences.rs` | 0 of 16,000 on the fixture; one known difference probed (below) |
| A11.1 public route table is exactly 4 paths; `/admin/*` 404 on 8080; admin routes only on 8081 | `tests/server.rs` | PASS |
| A11.2 a full match at info level logs no token, name, room code or full IP; truncated IPs present | `tests/server.rs` | PASS |
| A11.3 drain: live match finishes, new room refused, serve returns; the **binary** exits 0 on SIGTERM | `tests/server.rs`, `tests/process.rs` | PASS |
| A-DATA.1 exactly `players` (5 columns) + `_probe` | `src/store.rs` | PASS |
| Clippy, `-D warnings`, server crates | | PASS |

**App unaffected:**
- `cargo build -p spell_wasm --lib --target wasm32-unknown-unknown --release`
  passes with and without `--features web`.
- The app wasm still calls `__wbindgen_start` (the `cfg_attr` only drops it
  under `native`).
- `cargo test -p spell_wasm --lib`: **1476 passed**, 0 failed.
- `cargo tree` for every existing crate (spell_wasm, tonal, tracer-core,
  scanlock) is **identical** to `main`. That is why the server pins
  `rusqlite 0.40` and `reqwest 0.12`: newer versions moved `thiserror` and
  `serde`.

## A6.2: the whole core diff

```
Cargo.toml   members += "server/spell-server", "server/grader-parity"
Cargo.toml   + native = []            (with a two-line comment)
src/lib.rs   + #[cfg(feature = "native")]
             + pub mod native_api;
src/lib.rs   - #[wasm_bindgen(start)]
             + #[cfg_attr(not(feature = "native"), wasm_bindgen(start))]
src/native_api.rs  (new) comments + 16 `pub use` lines, no bodies
```

## Findings and choices Eric should know about

1. **Arabic and Persian are active on today's `main`.** `is_active_lang("ar")`
   and `("fa")` are true (Persian landed as the 16th language, Active for
   TestFlight). Recon R5/R8's "ar is RTL-blocked" is now stale, and spec
   A8.3's example (`lang:"ar"` → `lang_unavailable`) is wrong. Phase B will
   test A8.3 with any language where `is_active_lang` is false instead.
   - Online will offer ar and fa exactly as local Head-to-Head does. If that
     isn't wanted for online yet, say so before Phase B.
2. **A real, small online-vs-solo difference (O11, as signed).** Solo English
   sends exact-match answers to Flask, which compares `lower().strip()`, so
   "c at" for "cat" is wrong in solo. The shared grader's fold drops
   whitespace, so online accepts it. Nothing else differed on the 16,000-case
   fixture.
3. **Protocol additions beyond the F1 list.** These are needed to make the
   listed behaviour work:
   - A server `emote{seat,id}` type, so emotes can be relayed.
   - Close codes:
     - 4009: session expired or unknown → re-mint
     - 4011: replaced by a newer connection
     - 4012: no pong in 45 s
     - 4013: shutdown
     - 4014: name rejected
   - `error{session_invalid}` for an unknown token.
   - Direct replies that aren't room events (welcome, errors, join_pending,
     denied) carry `seq: 0`. Room events carry a **per-member** sequence, which
     is what makes every resume gapless.
4. **A duplicate `submit` cid is "acknowledged" by silence.** It is never
   re-graded and nothing is sent; the original result is already in the
   member's stream.
5. **Drain waits for live matches, not lobby rooms,** to reach zero. Taken
   literally, an idle lobby would hold every deploy for the full 15 minutes.
6. **The 10-minute empty-room rule can't trigger.** When the last seat is
   released, the host is gone too, and "no adult left → close" fires first. It
   stays as a safety net.
7. **Revocation is immediate.** A revoked player who is connected is closed
   with 4010 at once, not on their next connect.
8. **Origin is required.** A request with no Origin gets 403 on `/v1/*`, so
   the Phase E bot must send `capacitor://localhost`.
9. **Not covered by an automated test yet: keepalive** (ping every 15 s, close
   after 45 s without a pong). It is implemented; the soak/chaos run in Phase E
   is where it gets exercised.
10. **Kid names are English-only** until Phase B localizes them (O12).
11. **Command differences from §11:**
    - Clippy needs `--no-deps`, because an existing workspace crate
      (`scanlock`) fails `-D warnings` on its own `unnecessary_cast`. That
      crate is not CC-GAME-SERVER's to change.
    - The WASM parity run needs `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`.
12. **Toolchain changes on the Mac:** `rustup target add x86_64-unknown-linux-gnu`
    (recon) and `rustup component add clippy`. Nothing was installed anywhere
    else.

## Flagged for Phase D (needs a decision there, not now)

**The A4.2 rule and rootless Podman collide.** A4.2 makes the server refuse to
trust `CF-Connecting-IP` unless its public listener binds loopback. In a
container, though, a published port is delivered to the container's own
interface, not its loopback. So `127.0.0.1:8080` inside the container is
probably unreachable through `PublishPort=127.0.0.1:8080:8080`. The loopback
guarantee really lives at the publish layer on the host.

Options for Phase D:
- (a) Prove that pasta's forwarding reaches the container's loopback, and
  keep A4.2 as written.
- (b) Add an explicit, logged `SPELL_PUBLISHED_LOOPBACK_ONLY=true` that the
  Quadlet sets alongside a host-side `127.0.0.1` publish, and have
  `verify-host.sh` prove the host side.

I'll test (a) first and stop to ask if it fails.
