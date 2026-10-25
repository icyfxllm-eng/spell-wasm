#!/usr/bin/env node
// CC-GAME-SERVER Phase D — the deploy kit's static checks.
//
//   A14.6  the tunnel template routes only 8080 and ends in a 404 catch-all
//   A15.2  verify-host.sh contains no write: no state-changing systemctl,
//          firewall-cmd, dnf, rm, tee, sed -i, or output redirection except
//          to /dev/null
//   F14    the Quadlet unit carries every hardening line, publishes on
//          127.0.0.1 only, and never enables auto-update (G-D1, G-D6, G-D16)
//   A14.7  (first pass) no committed file in server/deploy holds anything but
//          <PLACEHOLDER>s where a secret would go; gitleaks is the full scan
//
//   node server/deploy/check-deploy.mjs
//   node server/deploy/check-deploy.mjs --selftest
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = dirname(fileURLToPath(import.meta.url));

/** Code lines only: comments and blank lines removed. */
const code = (text) =>
  text.split('\n').map((l) => l.replace(/(^|\s)#.*$/, '$1')).filter((l) => l.trim() !== '');

export function verifyHostWrites(text) {
  const bad = [];
  const rules = [
    [/\bsystemctl\b[^|;&]*\b(start|stop|enable|disable|restart|reload|mask|unmask|daemon-reload|kill)\b/, 'state-changing systemctl'],
    [/\bfirewall-cmd\b[^|;&]*--(permanent|add-|remove-|set-|reload|change-)/, 'state-changing firewall-cmd'],
    [/\bdnf\b\s+(install|remove|erase|upgrade|update|reinstall)\b/, 'dnf install/remove'],
    [/(^|[\s;|&(])rm(\s|$)/, 'rm'],
    [/\btee\b/, 'tee'],
    [/\bsed\b[^|;&]*\s-i\b/, 'sed -i'],
    [/(^|[\s;|&(])(chmod|chown|mkdir|touch|mv|cp|ln|useradd|usermod)\s|\bhostnamectl\s+set-|\bloginctl\s+enable/, 'a writing command'],
  ];
  for (const line of code(text)) {
    for (const [re, what] of rules) if (re.test(line)) bad.push(`${what}: ${line.trim()}`);
    // Output redirection is allowed only into /dev/null.
    for (const m of line.matchAll(/(\d?)(>>?)\s*([^\s;|&)]*)/g)) {
      const target = m[3];
      if (target === '/dev/null') continue;
      if (/^&\d$/.test(target)) continue; // fd duplication, not a file
      bad.push(`redirection to '${target || '?'}': ${line.trim()}`);
    }
  }
  return bad;
}

export function tunnelTemplate(text) {
  const bad = [];
  if (/8081/.test(text)) bad.push('the tunnel template mentions 8081 (the admin listener)');
  if (!/service:\s*http_status:404/.test(text)) bad.push('the tunnel template has no http_status:404 catch-all');
  if (!/service:\s*http:\/\/127\.0\.0\.1:8080\b/.test(text)) bad.push('the tunnel template does not route to 127.0.0.1:8080');
  return bad;
}

export function quadletUnit(text) {
  const bad = [];
  const need = [
    /^Image=localhost\/spellgame-server@sha256:/m,
    /^ReadOnly=true$/m,
    /^NoNewPrivileges=true$/m,
    /^DropCapability=all$/m,
    /^PodmanArgs=.*--memory=1g.*--pids-limit=256/m,
    /^Volume=spell-data:\/data:Z$/m,
    /^Restart=always$/m,
    /^RestartSec=2$/m,
    /^TimeoutStopSec=960$/m,
    /^WantedBy=default\.target$/m,
  ];
  for (const re of need) if (!re.test(text)) bad.push(`missing ${re.source}`);
  const ports = [...text.matchAll(/^PublishPort=(.*)$/gm)].map((m) => m[1]);
  if (ports.length !== 2) bad.push(`expected two PublishPort lines, found ${ports.length}`);
  for (const p of ports) if (!p.startsWith('127.0.0.1:')) bad.push(`PublishPort beyond loopback: ${p}`);
  if (/^AutoUpdate=/m.test(text)) bad.push('AutoUpdate is set (G-D16)');
  if (/^Network=host/m.test(text) || /--network[= ]host/.test(text)) bad.push('host networking');
  if (/--privileged|^User=root|--user[= ]root/m.test(text)) bad.push('a root or privileged container');
  // The container may listen widely ONLY when its publish lines are loopback.
  if (/SPELL_PUBLISHED_LOOPBACK_ONLY=true/.test(text) && ports.some((p) => !p.startsWith('127.0.0.1:'))) {
    bad.push('SPELL_PUBLISHED_LOOPBACK_ONLY without loopback publishing');
  }
  return bad;
}

const SECRETISH = [
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/,
  /\b(AKIA|ASIA)[A-Z0-9]{16}\b/,
  /\bAccountTag\b|\bTunnelSecret\b/,
  /^\s*(RESTIC_PASSWORD|B2_ACCOUNT_KEY|AWS_SECRET_ACCESS_KEY|TUNNEL_TOKEN)\s*=\s*(?!<)\S+/m,
];

export function secrets(name, text) {
  return SECRETISH.filter((re) => re.test(text)).map((re) => `${name}: looks like a secret (${re.source})`);
}

function run() {
  const read = (f) => readFileSync(join(DIR, f), 'utf8');
  const bad = [
    ...verifyHostWrites(read('verify-host.sh')).map((b) => `verify-host.sh: ${b}`),
    ...tunnelTemplate(read('cloudflared-config.template.yml')).map((b) => `cloudflared template: ${b}`),
    ...quadletUnit(read('spellgame.container')).map((b) => `spellgame.container: ${b}`),
  ];
  for (const f of readdirSync(DIR)) bad.push(...secrets(f, read(f)));
  if (bad.length) {
    console.error(`check-deploy: FAIL\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log('check-deploy: OK — verify-host.sh writes nothing, the tunnel routes only 8080 with a 404 catch-all, the unit is hardened and loopback-only, no secrets');
}

function selftest() {
  const cases = [
    [verifyHostWrites('systemctl is-active foo\n'), 0],
    [verifyHostWrites('systemctl --user restart spellgame\n'), 1],
    [verifyHostWrites('sudo firewall-cmd --permanent --add-service=x\n'), 1],
    [verifyHostWrites('sudo dnf install x\n'), 1],
    [verifyHostWrites('rm -f /tmp/x\n'), 1],
    [verifyHostWrites('echo x | tee /etc/y\n'), 1],
    [verifyHostWrites('echo x > /etc/y\n'), 1],
    [verifyHostWrites('cmd 2>/dev/null\n'), 0],
    [verifyHostWrites('# rm in a comment is fine\n'), 0],
    [verifyHostWrites('[ "$a" -gt 0 ]\n'), 0],
    [verifyHostWrites('h="$(getent passwd x | cut -d: -f6)"\n'), 0],
    [verifyHostWrites('mkdir -p /x\n'), 1],
    [tunnelTemplate('ingress:\n  - service: http://127.0.0.1:8080\n  - service: http_status:404\n'), 0],
    [tunnelTemplate('ingress:\n  - service: http://127.0.0.1:8081\n'), 3],
    [quadletUnit('PublishPort=0.0.0.0:8080:8080\nAutoUpdate=registry\n').filter((b) => /loopback|AutoUpdate/.test(b)), 2],
    [secrets('x', 'RESTIC_PASSWORD=hunter2\n'), 1],
    [secrets('x', 'RESTIC_PASSWORD=<RESTIC_PASSWORD>\n'), 0],
  ];
  let fails = 0;
  cases.forEach(([got, want], i) => {
    if (got.length !== want) {
      console.error(`selftest case ${i}: ${got.length} findings, want ${want}: ${JSON.stringify(got)}`);
      fails++;
    }
  });
  if (fails) process.exit(1);
  console.log('check-deploy selftest: PASS');
}

if (process.argv.includes('--selftest')) selftest();
else run();
