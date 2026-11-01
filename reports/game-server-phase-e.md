# CC-GAME-SERVER Phase E: load, soak, chaos on Rocky

Date: 2026-10-31. Host: spellbox (Rocky), behind tunnel spellgame-rocky.
Bots ran on the host itself, with sockets through play.spellgame.net.
Audio came from the canned-clip stub. Real voice restored afterwards.

## Verdict

**Room server: GO. Public launch: NO-GO until backups exist.**

The server held score, order and resume under every test. The host check
still has one FAIL: H12, no backups. The runbook requires every H line to
pass apart from the H16 UPS warning.

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
   used is not recorded.

## Before launch

Blocking:
- H12: nightly backups (O5, runbook step 13).
- App build carrying the resend of an action lost in a drop. The fix is on
  main, tested on the web build only. Builds before it make the player wait
  120 s and lose the turn.

Still open with Eric:
- O4 outside uptime monitor (runbook step 14)
- Ethernet as the default route (H19)
- UPS (H16)
- A13.3 signed kill doc
- Maestro
- App Store id
