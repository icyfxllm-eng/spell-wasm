# CC-GAME-SERVER — Phase E report (F16: load, soak, chaos)

**Status: PAUSED, as §7 says, because the host runbook isn't done.**
`play.spellgame.net` has no DNS record yet (checked 2026-10-25: no answer,
healthz unreachable). So none of H0–H18 can pass, and the real-host runs
can't start.

**What's done:** the F16 harness is built, and rehearsed locally at the
spec's full load scale. It's ready to run on Rocky the moment the runbook
is.

## The harness (`server/tools/botload`)

- `botload run --target URL --admin URL --profile load|soak|chaos|smoke`. Each
  room is two bots on two real sockets, driven by one task:
  - The task keeps its **own scoreboard**: the app's `versus::Versus`, fed the
    verdicts the server announces. Every `match_result` is checked against it.
  - Every socket's `seq` must be gapless with no duplicates, and every
    `(seat, word_no)` may appear once per match.
  - In chaos, a share of guests drop their socket at a random moment and
    resume with `hello{resume}`. Each must be back within 60 s or get a clean
    `match_ended`.
  - Soak and chaos sample `/admin/stats` every 30 s for p95, RSS and open
    descriptors.
- **Right answers without a leak.** A bot can't know the word. The test-only
  `botload tts-stub` puts the requested word into the clip's audio payload,
  which the server passes through (it strips only tags), so a bot can answer
  right on purpose. The side channel exists only while the stub is the
  server's `SPELL_TTS_BASE`.
- Server additions for Phase E:
  - **`POST /admin/test-sessions?n=`** (admin listener only, at most 1,000 per
    call): the bots share one IP, which may mint only 5 sessions an hour
    through `/v1/session`.
  - **`rss_bytes` and `open_fds` in `/admin/stats`** (Linux `/proc`).
  - The route-table test covers both: the new route exists on admin only and
    is 404 on public.

## Local rehearsal (this Mac, no tunnel; numbers are indicative only)

| Profile | Scale | Result |
|---|---|---|
| smoke ×5 | 4 rooms, 30 s, guests dropped | all PASS; every drop resumed in 1.2–2.0 s |
| **load** | **200 rooms / 400 connections**, 3 min | **PASS**: 653 matches, 19,376 words (15,448 right), 0 score mismatches, 0 seq gaps or duplicates, 0 duplicated words, 0 stalls, handling p95 0.64 ms, 400 peak connections |
| **chaos** | 50 rooms, 6 min, 30% drops | **PASS** except memory: 302 matches, 9,216 words, 12/12 drops resumed (slowest 4.4 s), 0 mismatches or gaps, p95 0.15 ms. Memory growth: **not measurable on macOS** (no `/proc`) |

Two bugs found while rehearsing, both in the harness:
- It treated the server's first ping as silence.
- A rehearsal hit an older server instance that was still draining after
  SIGTERM, which is correct drain behavior.

## Not done (needs the host)

| Spec test | Needs |
|---|---|
| A16.1 load through `wss://play.spellgame.net` | runbook steps 0–12 (tunnel, DNS, server) |
| A16.2 soak 2 h, memory under 5%, flat fds | the same, on Linux |
| A16.3 chaos with one cloudflared restart | the same, plus you running `sudo systemctl restart cloudflared` once during the run |

**Missing H-checks today: all of H0–H18.** The host hasn't been set up
(`play.spellgame.net` doesn't resolve). The exact procedure, including
switching the server to the canned-clip stub and back, is
`server/tools/botload/PHASE-E.md`.

## Go/no-go (§10) still owed by you
1–6 (verify-host, nmap from cellular, the unplug and reboot tests, the uptime
alert, the restore drill), 7 (this phase on the real host), 8–9 (TestFlight
matches across homes and on cellular, the remote kill), 10 (privacy policy,
App Store label, age rating), and 11 (counsel, for `online_kids` only).

Also still open from Phase C: the signed kill file for A13.3, Maestro, and
the App Store id.
