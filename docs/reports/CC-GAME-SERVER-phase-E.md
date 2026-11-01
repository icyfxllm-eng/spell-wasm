# CC-GAME-SERVER — Phase E report (F16: load, soak, chaos)

**Status: host runs done 2026-10-31; see "Host runs on Rocky" at the end.**
The sections before it are the 2026-10-25 harness report, kept as written.

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

# Host runs on Rocky

Date: 2026-10-31. Host: spellbox (Rocky), behind tunnel spellgame-rocky.
Bots ran on the host itself, with sockets through play.spellgame.net.
Audio came from the canned-clip stub. Real voice restored afterwards.

## Verdict

**Room server: GO. Public launch: NO-GO until the items under "Before launch"
are done.**

The server held score, order and resume under every test. The host check is
at 0 FAIL, 1 WARN (H16, UPS on order, which the runbook allows) as of
2026-10-31. Nightly backups to Cloudflare R2 are on, H12 passes, and the
restore drill (go/no-go 10.6) passed, as reported by Eric.

## Results

| Run | Rooms / conns | Matches | Mismatches | Stalls | Drops | Result |
|---|---|---|---|---|---|---|
| A16.1 load | 200 / 400 | 1,841 | 0 | 0 | 0 retries | PASS |
| A16.2 soak 4 (botload 8c98c5a5) | 100 / 200 | 11,273 | 0 | 1 | 253, all resumed | 1 FAIL, not rerun |
| A16.3 chaos, first of two (e7e65435) | 100 / 200 | 11,341 | 0 | 0 | 233, all resumed | PASS* |
| A16.3 chaos, second of two (same build) | 100 / 200 | 11,359 | 0 | 0 | 200, all resumed | PASS* |

Every run: 0 seq gaps, 0 seq duplicates, 0 duplicated words. Handling p95
was at most 0.29 ms against the 50 ms limit.

*The two chaos runs overlapped for 27 minutes by accident, peaking at 400
connections. Each printed a false FAIL: memory +76.91% and fds 121 → 217 on
the first, fds 217 → 117 on the second. All three came from the other run's
connections arriving or leaving. Fds per connection stayed about 1.5-1.6
throughout. botload now compares fds per connection and fails a run that
starts on a busy server. That build has not run on Rocky.

Eric decided on 2026-10-31 not to rerun the soak. The soak's stall note was
lost behind the note cap, so its cause is inferred, not proven: 38 identical
chaos stalls were traced to a lost or refused StartMatch, fixed in a37878f2,
and neither run since has stalled.

## Harness bugs found, none in the server

Bots spinning on a lost socket, lost NextWord/Submit never resent, TurnEnd
without a WordResult ignored, StartMatch never retried, stub killed with its
shell, and post-teardown samples read as leaks. Details in git log for
server/tools/botload.

## Open findings

1. **Memory after a mass reconnect.** Taken alone, the first chaos run went
   65.4 → 74.8 MB (+14%), about 5 MB of it one step at the cloudflared
   restart, then flat for 18 minutes. A local run with 5 mass cuts added
   8 MB on the first, then about 0.5-1.5 MB on each later one. Local means
   macOS, not the Linux container. Mostly levels off; zero growth not shown.
   The soak with no restart was +4.52%, under the 5% limit but close.
2. **Unexplained drops.** One chaos run saw 3,954 socket closes, mostly
   connection reset by peer. Its timeline was overwritten and it did not
   recur (233 and 200 since). Cause unknown.
3. **StartMatch retry unproven on Rocky.** It fired and recovered rooms
   locally, but fired 0 times in both Rocky runs.
4. **Retries by cause unknown.** Counts were 400, 400 and 53. The per-cause
   tally is in the new build only.
5. **Network path.** H19 shows the default route on Wi-Fi at the final host
   check, though soak 4 ran with the cable connected. Wi-Fi saw about 8
   closes a minute, soak 4 on the cable about 2. Which path the chaos runs
   used is not recorded. Fixed 2026-10-31: enp1s0 connected with autoconnect
   on, H19 passes. Not yet tested across a reboot.

## Before launch

Blocking:
- App build carrying the resend of an action lost in a drop. The fix is on
  main, tested on the web build only. Builds before it make the player wait
  120 s and lose the turn.

Still open with Eric:
- O4 outside uptime monitor (runbook step 14)
- UPS (H16)
- A13.3 signed kill doc
- Maestro
- App Store id
