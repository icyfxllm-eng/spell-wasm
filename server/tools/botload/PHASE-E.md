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
# as spellsrv (sudo -iu spellsrv):
cd ~/spell-wasm && git pull
cargo build --release -p botload
```

## 2. Point the server at the canned-clip stub (test mode)
The load runs must not use the Mac mini or Google: the stub answers every
word instantly. The container reaches the host through
`host.containers.internal`.
```
# as spellsrv (sudo -iu spellsrv):
~/spell-wasm/target/release/botload tts-stub --listen 0.0.0.0:9190 &
sed -i 's|^Environment=SPELL_TTS_BASE=.*|Environment=SPELL_TTS_BASE=http://host.containers.internal:9190|' ~/.config/containers/systemd/spellgame.container
systemctl --user daemon-reload && systemctl --user restart spellgame.service
curl -s http://127.0.0.1:8081/admin/readyz        # "tts": true
```
Port 9190, not 9090: Rocky's Cockpit console listens on 9090 by default.
firewalld's public zone has no 9190, so the stub can't be reached from
outside. `verify-host.sh` H5 shows it as an extra listener until step 6.

## 3. A16.1 — load (400 connections, 200 rooms, 10 min)
```
# as spellsrv (sudo -iu spellsrv):
ulimit -n 16384
~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
--admin http://127.0.0.1:8081 --audio-base http://127.0.0.1:8080 --profile load | tee ~/phase-e-load.txt
```
The game sockets go through the tunnel; the clips come over loopback
(`--audio-base`). All 400 bots share one IP, and Cloudflare throttles one IP
fetching a clip every few seconds per bot (the first run drew 429s), which real
players on their own home connections never would. The rooms also start over
90 s (`--ramp`), and a bot that still meets a 429 backs off and retries; the
summary counts those retries.

PASS needs:
- every room started and 400 connections at peak
- handling p95 under 50 ms
- zero score mismatches, gaps, duplicates or stalls

## 4. A16.2 — soak (50%, 2 hours)
```
# as spellsrv (sudo -iu spellsrv):
~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
--admin http://127.0.0.1:8081 --audio-base http://127.0.0.1:8080 --profile soak | tee ~/phase-e-soak.txt
```
PASS needs:
- memory growth under 5% after the 10-minute warm-up (the server's RSS, read
  from /proc)
- a flat file-descriptor count
- everything A16.1 checks

## 5. A16.3 — chaos (the soak again, with failures)
```
# as spellsrv (sudo -iu spellsrv):
~/spell-wasm/target/release/botload run --target https://play.spellgame.net \
--admin http://127.0.0.1:8081 --audio-base http://127.0.0.1:8080 --profile chaos | tee ~/phase-e-chaos.txt
```
While it runs, **once**, somewhere in the middle:
```
# as your admin user (eric):
sudo systemctl restart cloudflared
```
The bots drop 10% of their sockets on their own. PASS needs every dropped
bot to resume within 60 s or receive a clean `match_ended`, with zero score
mismatches.

## 6. Back to the real voice
```
# as spellsrv (sudo -iu spellsrv):
kill %1                                            # the stub
sed -i 's|^Environment=SPELL_TTS_BASE=.*|Environment=SPELL_TTS_BASE=http://<MAC_MINI_TAILNET_IP>:8000|' ~/.config/containers/systemd/spellgame.container
systemctl --user daemon-reload && systemctl --user restart spellgame.service
# as your admin user (eric):
sudo -v && ~spellsrv/spell-wasm/server/deploy/verify-host.sh      # all PASS again (H18 included)
```
The bots' sessions stay in the players table, are never used again, and are
swept after 12 months of inactivity (§9). To remove them sooner, revoke them
or restore the pre-test backup.

Send CC the three `phase-e-*.txt` files. The go/no-go report comes from them.
