# Phase E on the Rocky host — load, soak, chaos (CC-GAME-SERVER F16)

Run this after `server/deploy/HOST-RUNBOOK.md` and before any build turns
`online_enabled` on. Players can't reach the server yet, so the bots are the
only traffic.

The bots go out through the real tunnel (`wss://play.spellgame.net`) and come
back in through cloudflared, the same path a phone takes. They mint sessions
and read stats on the admin listener, so they run **on the Rocky box itself**.

## 0. Prerequisites
- `verify-host.sh` passes everything (H16 may WARN).
- A Rust toolchain for the spellsrv user, to build the bots:
  `curl https://sh.rustup.rs -sSf | sh` (you install it; CC doesn't).

## 1. Build the bots
```
spellsrv$ cd ~/spell-wasm && git pull
spellsrv$ cargo build --release -p botload
```

## 2. Point the server at the canned-clip stub (test mode)
The load runs must not use the Mac mini or Google: the stub answers every
word instantly. The container reaches the host through
`host.containers.internal`.
```
spellsrv$ ~/spell-wasm/target/release/botload tts-stub --listen 0.0.0.0:9190 &
spellsrv$ sed -i 's|^Environment=SPELL_TTS_BASE=.*|Environment=SPELL_TTS_BASE=http://host.containers.internal:9190|' ~/.config/containers/systemd/spellgame.container
spellsrv$ systemctl --user daemon-reload && systemctl --user restart spellgame.service
spellsrv$ curl -s http://127.0.0.1:8081/admin/readyz        # "tts": true
```
Port 9190, not 9090: Rocky's Cockpit console listens on 9090 by default.
firewalld's public zone has no 9190, so the stub can't be reached from
outside. `verify-host.sh` H5 shows it as an extra listener until step 6.

## 3. A16.1 — load (400 connections, 200 rooms, 10 min)
```
spellsrv$ ulimit -n 16384
spellsrv$ ~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
            --admin http://127.0.0.1:8081 --profile load | tee ~/phase-e-load.txt
```
PASS needs:
- every room started and 400 connections at peak
- handling p95 under 50 ms
- zero score mismatches, gaps, duplicates or stalls

## 4. A16.2 — soak (50%, 2 hours)
```
spellsrv$ ~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
            --admin http://127.0.0.1:8081 --profile soak | tee ~/phase-e-soak.txt
```
PASS needs:
- memory growth under 5% after the 10-minute warm-up (the server's RSS, read
  from /proc)
- a flat file-descriptor count
- everything A16.1 checks

## 5. A16.3 — chaos (the soak again, with failures)
```
spellsrv$ ~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
            --admin http://127.0.0.1:8081 --profile chaos | tee ~/phase-e-chaos.txt
```
While it runs, **once**, somewhere in the middle:
```
$ sudo systemctl restart cloudflared
```
The bots drop 10% of their sockets on their own. PASS needs every dropped
bot to resume within 60 s or receive a clean `match_ended`, with zero score
mismatches.

## 6. Back to the real voice
```
spellsrv$ kill %1                                            # the stub
spellsrv$ sed -i 's|^Environment=SPELL_TTS_BASE=.*|Environment=SPELL_TTS_BASE=http://<MAC_MINI_TAILNET_IP>:8000|' ~/.config/containers/systemd/spellgame.container
spellsrv$ systemctl --user daemon-reload && systemctl --user restart spellgame.service
$ sudo -v && ~spellsrv/spell-wasm/server/deploy/verify-host.sh      # all PASS again (H18 included)
```
The bots' sessions stay in the players table, are never used again, and are
swept after 12 months of inactivity (§9). To remove them sooner, revoke them
or restore the pre-test backup.

Send CC the three `phase-e-*.txt` files. The go/no-go report comes from them.
