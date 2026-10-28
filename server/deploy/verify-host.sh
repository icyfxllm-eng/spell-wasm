#!/usr/bin/env bash
# CC-GAME-SERVER F15 — prove the Rocky host is set up the way HOST-RUNBOOK.md
# says. READ-ONLY: it looks and reports, and changes nothing (A15.2 checks
# this file for any write). It may use sudo, for reads only.
#
#   sudo -v && server/deploy/verify-host.sh
#
# Prints PASS / WARN / FAIL per check and exits non-zero on any FAIL.
set -uo pipefail

SVC_USER="${SPELL_SVC_USER:-spellsrv}"
PUBLIC_HOST="${SPELL_PUBLIC_HOST:-play.spellgame.net}"
fails=0
warns=0

pass() { echo "PASS  $1  $2"; }
warn() { echo "WARN  $1  $2"; warns=$((warns + 1)); }
fail() { echo "FAIL  $1  $2"; fails=$((fails + 1)); }

# The service user's own systemd, asked from here (systemd's machine syntax).
svc_systemctl() {
  sudo systemctl --user -M "${SVC_USER}@" "$@"
}
svc_home="$(getent passwd "${SVC_USER}" | cut -d: -f6)"
[ -n "${svc_home}" ] || svc_home="/home/${SVC_USER}"
unit="${svc_home}/.config/containers/systemd/spellgame.container"

# H0 — a real hostname (it was "localhost").
h="$(hostnamectl --static 2>/dev/null || hostname)"
if [ -n "${h}" ] && [ "${h}" != "localhost" ] && [ "${h}" != "localhost.localdomain" ]; then
  pass H0 "hostname is ${h}"
else
  fail H0 "hostname is '${h}' (runbook step 0)"
fi

# H1 — SELinux enforcing.
if [ "$(getenforce 2>/dev/null)" = "Enforcing" ]; then pass H1 "SELinux Enforcing"; else fail H1 "SELinux is $(getenforce 2>/dev/null || echo unknown)"; fi

# H2 — sshd: no passwords, no root.
sshd_t="$(sudo sshd -T 2>/dev/null)"
if echo "${sshd_t}" | grep -qx "passwordauthentication no" && echo "${sshd_t}" | grep -qx "permitrootlogin no"; then
  pass H2 "sshd: password auth no, root login no"
else
  fail H2 "sshd -T does not show 'passwordauthentication no' and 'permitrootlogin no'"
fi

# H3 — the public zone offers nothing but dhcpv6-client, and no ports.
pub_svcs="$(sudo firewall-cmd --zone=public --list-services 2>/dev/null)"
pub_ports="$(sudo firewall-cmd --zone=public --list-ports 2>/dev/null)"
extra=""
for s in ${pub_svcs}; do
  [ "${s}" = "dhcpv6-client" ] || extra="${extra} ${s}"
done
if [ -z "${extra}" ] && [ -z "${pub_ports}" ]; then
  pass H3 "public zone: ${pub_svcs:-nothing}"
else
  fail H3 "public zone also allows:${extra} ${pub_ports}"
fi

# H4 — tailscale0 sits in a zone that allows ssh.
ts_zone="$(sudo firewall-cmd --get-zone-of-interface=tailscale0 2>/dev/null)"
if [ -n "${ts_zone}" ] && sudo firewall-cmd --zone="${ts_zone}" --list-services 2>/dev/null | grep -qw ssh; then
  pass H4 "tailscale0 in zone '${ts_zone}' with ssh"
else
  fail H4 "tailscale0 is not in a zone that allows ssh (zone: '${ts_zone:-none}')"
fi

# H5 — no TCP listener beyond loopback except sshd (G-D1). tailscaled's own
# listeners on its tailnet addresses are reachable over Tailscale only.
bad=""
while read -r local proc; do
  addr="${local%:*}"
  case "${addr}" in
    127.*|"[::1]"|::1) continue ;;
  esac
  case "${proc}" in
    *sshd*) continue ;;
    *tailscaled*)
      case "${addr}" in
        100.*%tailscale0|"[fd7a:115c:a1e0:"*"]%tailscale0"|100.*|"[fd7a:115c:a1e0:"*"]") continue ;;
      esac ;;
  esac
  bad="${bad} ${local}(${proc})"
done < <(sudo ss -Htlnp 2>/dev/null | awk '{print $4, $6}')
if [ -z "${bad}" ]; then pass H5 "only sshd listens beyond loopback"; else fail H5 "also listening:${bad}"; fi

# H6 — sleep is masked.
unmasked=""
for t in sleep.target suspend.target hibernate.target hybrid-sleep.target; do
  [ "$(systemctl is-enabled "${t}" 2>/dev/null)" = "masked" ] || unmasked="${unmasked} ${t}"
done
if [ -z "${unmasked}" ]; then pass H6 "sleep targets masked"; else fail H6 "not masked:${unmasked}"; fi

# H7 — security updates install themselves.
if systemctl is-active --quiet dnf-automatic.timer && grep -Eq '^\s*upgrade_type\s*=\s*security' /etc/dnf/automatic.conf 2>/dev/null; then
  pass H7 "dnf-automatic.timer active, upgrade_type = security"
else
  fail H7 "dnf-automatic.timer inactive or upgrade_type is not security"
fi

# H8 — the service user lingers (its units run with nobody logged in).
if [ "$(loginctl show-user "${SVC_USER}" -p Linger --value 2>/dev/null)" = "yes" ]; then
  pass H8 "linger on for ${SVC_USER}"
else
  fail H8 "linger is off for ${SVC_USER}"
fi

# H9 — the room server unit is running.
svc_state="$(svc_systemctl is-active spellgame.service 2>/dev/null)"
if [ "${svc_state}" = "active" ]; then
  pass H9 "spellgame.service active"
else
  fail H9 "spellgame.service for ${SVC_USER} is '${svc_state:-unknown}'"
fi

# H10 — the tunnel is running.
if systemctl is-active --quiet cloudflared; then pass H10 "cloudflared active"; else fail H10 "cloudflared is not active"; fi

# H11 — healthz answers locally and through the tunnel.
local_hz="$(curl -s -o /dev/null -w '%{http_code}' -m 5 http://127.0.0.1:8080/healthz)"
public_hz="$(curl -s -o /dev/null -w '%{http_code}' -m 10 "https://${PUBLIC_HOST}/healthz")"
if [ "${local_hz}" = "200" ] && [ "${public_hz}" = "200" ]; then
  pass H11 "healthz 200 locally and via ${PUBLIC_HOST}"
else
  fail H11 "healthz local=${local_hz} public=${public_hz}"
fi

# H12 — a backup in the last 26 hours.
stamp="$(sudo cat "${svc_home}/.local/state/spellgame/last-backup" 2>/dev/null)"
now="$(date -u +%s)"
if [ -n "${stamp}" ] && [ $((now - stamp)) -lt $((26 * 3600)) ]; then
  pass H12 "last backup $(((now - stamp) / 3600)) h ago"
else
  fail H12 "no backup stamp under 26 h old"
fi

# H13 — disk below 80%.
use="$(df --output=pcent / | tail -n 1 | tr -dc '0-9')"
if [ -n "${use}" ] && [ "${use}" -lt 80 ]; then pass H13 "disk ${use}% used"; else fail H13 "disk ${use:-?}% used"; fi

# H14 — the clock is synchronized (deadlines are server-clock, G-D17).
if [ "$(timedatectl show -p NTPSynchronized --value 2>/dev/null)" = "yes" ]; then
  pass H14 "clock synchronized"
else
  fail H14 "clock is not NTP-synchronized (chronyd)"
fi

# H15 — journald keeps 14 days.
if systemd-analyze cat-config systemd/journald.conf 2>/dev/null | grep -Eq '^\s*MaxRetentionSec=14day'; then
  pass H15 "journald MaxRetentionSec=14day"
else
  fail H15 "journald retention is not 14day"
fi

# H16 — the UPS monitor (WARN only: acceptable while a UPS is on order).
if systemctl is-active --quiet nut-monitor 2>/dev/null; then pass H16 "upsmon active"; else warn H16 "upsmon (nut-monitor) is not active"; fi

# H17 — podman auto-update is never on (G-D16).
au_sys="$(systemctl is-enabled podman-auto-update.timer 2>/dev/null)"
au_usr="$(svc_systemctl is-enabled podman-auto-update.timer 2>/dev/null)"
if [ "${au_sys}" != "enabled" ] && [ "${au_usr}" != "enabled" ]; then
  pass H17 "podman-auto-update.timer not enabled"
else
  fail H17 "podman-auto-update.timer is enabled (system=${au_sys} user=${au_usr})"
fi

# H18 — the Mac mini's TTS answers from here over the tailnet.
tts="$(sudo grep -E '^Environment=SPELL_TTS_BASE=' "${unit}" 2>/dev/null | cut -d= -f3-)"
if [ -n "${tts}" ] && [ "$(curl -s -o /dev/null -w '%{http_code}' -m 5 "${tts}/api/health")" = "200" ]; then
  pass H18 "TTS reachable at ${tts}"
else
  fail H18 "TTS not reachable (SPELL_TTS_BASE='${tts:-unset}', read from ${unit})"
fi

# H19 — on the cable (runbook step 3); on Wi-Fi, at least no power saving.
def_if="$(ip route show default 2>/dev/null | awk '{for (i = 1; i < NF; i++) if ($i == "dev") { print $(i + 1); exit }}')"
if [ -z "${def_if}" ]; then
  fail H19 "no default route"
elif [ -d "/sys/class/net/${def_if}/wireless" ]; then
  ps="$(iw dev "${def_if}" get power_save 2>/dev/null | awk '{print $NF}')"
  if [ "${ps}" = "off" ]; then
    warn H19 "default route is Wi-Fi (${def_if}), power save off; the cable is better"
  else
    warn H19 "default route is Wi-Fi (${def_if}) with power save '${ps:-unknown}' (runbook step 3)"
  fi
else
  pass H19 "default route on ${def_if} (wired)"
fi

echo
echo "${fails} FAIL, ${warns} WARN"
[ "${fails}" -eq 0 ]
