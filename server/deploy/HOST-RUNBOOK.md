# Rocky host runbook — CC-GAME-SERVER F15

You run every step. CC wrote them and never runs any of them. Each step ends
with a check. `verify-host.sh` runs all the checks at once (H0–H18) and
changes nothing.

Conventions:
- `$` is your admin user with sudo. `spellsrv$` is the service user:
  `sudo -iu spellsrv` (or `machinectl shell spellsrv@`, which gives it a
  proper user session).
- `<LIKE_THIS>` is a value you fill in. None of them belong in git.
- The Rocky box also hosts the Trundlo Bluetooth tests. Nothing here touches
  Bluetooth, but step 8 closes every inbound port except ssh on the tailnet.

---

## 0. A real hostname
Today it is `localhost`, which Tailscale and the logs can't tell apart from
anything else.
```
$ sudo hostnamectl set-hostname spellbox
```
Check: `hostnamectl --static` prints `spellbox`. **H0**

## 1. UPS and NUT
Plug the box (and the router, if the UPS can carry it) into the UPS, USB
cable to the box.
```
$ sudo dnf install -y epel-release && sudo dnf install -y nut
$ sudo nut-scanner -U          # find the UPS; copy its block into /etc/ups/ups.conf
$ sudoedit /etc/ups/ups.conf /etc/ups/upsd.users /etc/ups/upsmon.conf   # MONITOR line, MODE=standalone in nut.conf
$ sudo systemctl enable --now nut-server nut-monitor
```
Check: `upsc <UPS_NAME>@localhost ups.status` shows `OL`; `systemctl is-active nut-monitor`. **H16** (only a WARN until the UPS arrives)

## 2. BIOS: power back on after a power cut
In the firmware setup: *Restore on AC power loss* = **Power On** (sometimes
"Last State").
Check: pull the plug (with no UPS between) and reinsert it. The box boots
without a button press (go/no-go §10.3).

## 3. Wired Ethernet
Use the cable, not Wi-Fi.
Check: `nmcli -t -f DEVICE,TYPE,STATE device | grep ethernet:connected`.

## 4. Never sleep
```
$ sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target
```
Check: `systemctl is-enabled sleep.target` prints `masked`. **H6**

## 5. Security updates, automatically
```
$ sudo dnf install -y dnf-automatic
$ sudoedit /etc/dnf/automatic.conf     # [commands] upgrade_type = security, apply_updates = yes
$ sudo systemctl enable --now dnf-automatic.timer
```
Check: `systemctl is-active dnf-automatic.timer` and `grep upgrade_type /etc/dnf/automatic.conf`. **H7**

## 6. Tailscale, on Rocky **and on the Mac mini**
Rocky:
```
$ sudo dnf config-manager --add-repo https://pkgs.tailscale.com/stable/rhel/10/tailscale.repo
$ sudo dnf install -y tailscale && sudo systemctl enable --now tailscaled
$ sudo tailscale up --ssh=false
```
Mac mini: install Tailscale, sign in to the same tailnet, and note its
`100.x.y.z` address. The room server fetches word audio from the Mac mini's
backend over the tailnet (gunicorn on :8000), so that port has to be
reachable from Rocky over `tailscale0`. You change the Mac mini's services
yourself. Keep the spellgame.net tunnel as it is.

Check from Rocky: `curl -s -o /dev/null -w '%{http_code}\n' http://<MAC_MINI_TAILNET_IP>:8000/api/health` prints `200`. **H18**

## 7. sshd: keys only, no root
Put your key in `~/.ssh/authorized_keys` first, and keep a session open while
you test.
```
$ sudo tee /etc/ssh/sshd_config.d/10-spell.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
EOF
$ sudo sshd -t && sudo systemctl reload sshd
```
Check: `sudo sshd -T | grep -E '^(passwordauthentication|permitrootlogin) '` shows `no` twice. **H2**

## 8. firewalld: nothing in public, ssh on the tailnet only
```
$ sudo firewall-cmd --permanent --zone=public --remove-service=ssh
$ sudo firewall-cmd --permanent --zone=public --remove-service=cockpit
$ sudo firewall-cmd --permanent --new-zone=tailnet
$ sudo firewall-cmd --permanent --zone=tailnet --add-interface=tailscale0
$ sudo firewall-cmd --permanent --zone=tailnet --add-service=ssh
$ sudo firewall-cmd --reload
```
Also stop any desktop service listening on the network, if this is a
workstation install (`sudo ss -tlnp` lists them; usually cups, avahi,
cockpit): `sudo systemctl disable --now cups avahi-daemon cockpit.socket`.
Check: `firewall-cmd --zone=public --list-services` is `dhcpv6-client`;
`firewall-cmd --get-zone-of-interface=tailscale0` is `tailnet`;
`sudo ss -tlnp` shows nothing beyond 127.0.0.1 but sshd. **H3, H4, H5**

## 9. The service user, lingering
```
$ sudo useradd --create-home --shell /bin/bash spellsrv
$ sudo loginctl enable-linger spellsrv
$ sudo dnf install -y podman git sqlite curl
spellsrv$ git clone https://github.com/icyfxllm-eng/spell-wasm.git ~/spell-wasm
```
Check: `loginctl show-user spellsrv -p Linger` prints `Linger=yes`. **H8**

## 10. journald keeps 14 days
```
$ sudo install -D -m 0644 ~spellsrv/spell-wasm/server/deploy/journald-spell.conf /etc/systemd/journald.conf.d/spell.conf
$ sudo systemctl restart systemd-journald
```
Check: `systemd-analyze cat-config systemd/journald.conf | grep MaxRetentionSec`. **H15**

## 11. The tunnel and DNS (you, in Cloudflare)
A **new** tunnel for Rocky. Leave `spellgame-mac` alone.
```
$ sudo dnf install -y cloudflared          # from Cloudflare's repo
$ cloudflared tunnel login
$ cloudflared tunnel create spellgame-rocky        # prints <TUNNEL_ID>, writes the credentials JSON
$ cloudflared tunnel route dns spellgame-rocky play.spellgame.net
$ sudo install -D -m 0644 ~spellsrv/spell-wasm/server/deploy/cloudflared-config.template.yml /etc/cloudflared/config.yml
$ sudoedit /etc/cloudflared/config.yml     # <TUNNEL_ID>, <CREDENTIALS_FILE>, <HOSTNAME>=play.spellgame.net
$ sudo cloudflared service install && sudo systemctl enable --now cloudflared
```
In the Cloudflare dashboard, **WebSockets** must be on for the zone (they are
on by default).
Check: `systemctl is-active cloudflared`; after step 12,
`curl https://play.spellgame.net/healthz` prints `ok`. **H10, H11**

## 12. Build and install the server
```
spellsrv$ cd ~/spell-wasm && git pull
spellsrv$ server/deploy/build.sh                      # prints sha256:<hex>
spellsrv$ mkdir -p ~/.config/containers/systemd
spellsrv$ cp server/deploy/spellgame.container ~/.config/containers/systemd/
spellsrv$ sed -i 's|<MAC_MINI_TAILNET_IP>|100.x.y.z|' ~/.config/containers/systemd/spellgame.container
spellsrv$ /usr/libexec/podman/quadlet -dryrun -user    # must print the generated unit, no errors (A14.4)
spellsrv$ server/deploy/deploy.sh sha256:<hex>
```
Check: `systemctl --user is-active spellgame.service`;
`curl -s http://127.0.0.1:8081/admin/readyz` shows `"ready":true`. **H9, H11**

**Before go-live, settle the loopback question (Phase D finding 1).**
Podman delivers a published port to the container's own interface. That is
why the unit sets `SPELL_PUBLISHED_LOOPBACK_ONLY=true` and the image listens
on `0.0.0.0` inside its own namespace. Run this once:
`sudo ss -tlnp | grep -E ':(8080|8081)'`. Both must show `127.0.0.1` only.

## 13. Nightly backups (O5: B2 or R2)
```
spellsrv$ mkdir -p ~/.config/spellgame && chmod 700 ~/.config/spellgame
spellsrv$ install -m 600 /dev/null ~/.config/spellgame/restic-password && $EDITOR ~/.config/spellgame/restic-password
spellsrv$ install -m 600 /dev/null ~/.config/spellgame/backup.env && $EDITOR ~/.config/spellgame/backup.env
     # RESTIC_REPOSITORY=<b2:bucket:spellgame  or  s3:https://<ACCOUNT>.r2.cloudflarestorage.com/<bucket>>
     # RESTIC_PASSWORD_FILE=/home/spellsrv/.config/spellgame/restic-password
     # B2_ACCOUNT_ID=... B2_ACCOUNT_KEY=...   (or AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY for R2)
$ sudo dnf install -y restic
spellsrv$ set -a; . ~/.config/spellgame/backup.env; set +a; restic init
spellsrv$ mkdir -p ~/.config/systemd/user
spellsrv$ cp ~/spell-wasm/server/deploy/spell-backup.{service,timer} ~/.config/systemd/user/
spellsrv$ systemctl --user daemon-reload && systemctl --user enable --now spell-backup.timer
spellsrv$ systemctl --user start spell-backup.service      # one now, to prove it
spellsrv$ ~/spell-wasm/server/deploy/restore-drill.sh      # PASS (go/no-go §10.6)
```
Keep a copy of the restic password somewhere off this box. Without it, the
backups are unreadable.
Check: `systemctl --user list-timers spell-backup.timer`;
`cat ~/.local/state/spellgame/last-backup`. **H12**

## 14. An uptime monitor outside the house (O4)
Any service that checks `https://play.spellgame.net/healthz` every minute and
pushes to your phone (Better Stack, UptimeRobot, Healthchecks.io, and so on).
Check: stop the server (`systemctl --user stop spellgame.service`). The alert
arrives within 5 minutes (go/no-go §10.5). Start it again.

---

## Then
```
$ sudo -v && ~spellsrv/spell-wasm/server/deploy/verify-host.sh
```
Every line must PASS. H16 may WARN only while the UPS is on order. Then work
through the go/no-go list in `docs/CC-GAME-SERVER.md` §10. Phase E's load,
soak and chaos runs need everything above in place.

## Deploying a new version later
```
spellsrv$ cd ~/spell-wasm && git pull && server/deploy/build.sh
spellsrv$ server/deploy/deploy.sh sha256:<hex>     # drains first; never auto-update
```
