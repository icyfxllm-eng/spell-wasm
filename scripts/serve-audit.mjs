#!/usr/bin/env node
// Serve dist-audit/ privately so a native speaker can review it.
//
// The audit build carries UNVERIFIED draft content and must not be public, so this
// gates every request behind HTTP basic auth. Flow:
//   bash scripts/build-web-audit.sh        # produce dist-audit/
//   node scripts/serve-audit.mjs           # serve it, prints URL + password
//   # then, to reach a remote auditor, expose the local port over a tunnel, e.g.:
//   #   cloudflared tunnel --url http://localhost:8140
//   #   ngrok http 8140
// Share the tunnel URL AND the password with the auditor. The password keeps the
// unverified build from being stumbled on; the tunnel gives them HTTPS.
//
// Per-language: AUDIT_LANG=fr serves dist-audit-fr/ (built by
// AUDIT_LANG=fr bash scripts/build-web-audit.sh). Give each language its own
// AUDIT_PORT when serving several at once; each run gets its own password, so one
// auditor's password does not open another's build.
//
// Password: set AUDIT_PASS to pin one, else a fresh random token is generated and
// printed each run. Username is "auditor".
import { createServer } from 'node:http';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join, extname, normalize } from 'node:path';
import { randomBytes } from 'node:crypto';

const LANG = process.env.AUDIT_LANG || '';
const DIR = LANG ? `dist-audit-${LANG}` : 'dist-audit';
const ROOT = new URL(`../${DIR}`, import.meta.url).pathname;
if (!existsSync(join(ROOT, 'index.html'))) {
  console.error(`  ${DIR}/ not found — run:  ${LANG ? `AUDIT_LANG=${LANG} ` : ''}bash scripts/build-web-audit.sh`);
  process.exit(1);
}
const PORT = Number(process.env.AUDIT_PORT) || 8140;
const UPSTREAM = 'https://spellgame.net';
const USER = 'auditor';
const PASS = process.env.AUDIT_PASS || randomBytes(6).toString('base64url');
const EXPECT = 'Basic ' + Buffer.from(`${USER}:${PASS}`).toString('base64');

const MIME = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.wasm': 'application/wasm', '.json': 'application/json', '.woff2': 'font/woff2',
  '.woff': 'font/woff', '.png': 'image/png', '.svg': 'image/svg+xml', '.ico': 'image/x-icon',
};

const server = createServer((req, res) => {
  // Constant-ish auth check (length-guarded equality).
  const got = req.headers.authorization || '';
  if (got !== EXPECT) {
    res.writeHead(401, { 'WWW-Authenticate': 'Basic realm="SpellGame audit preview"' });
    res.end('Authentication required — this is an unverified audit build.');
    return;
  }
  let p = decodeURIComponent((req.url || '/').split('?')[0]);
  // The build points SPELL_API_BASE at this origin (build-web-audit.sh), because
  // spellgame.net's CORS refuses a tunnel origin. Telemetry is answered and
  // dropped: an audit session is not a player.
  if (p.startsWith('/telemetry')) {
    res.writeHead(204);
    res.end();
    return;
  }
  if (p.startsWith('/api/') || p.startsWith('/packs/')) return forward(req, res);
  if (p === '/') p = '/index.html';
  const file = normalize(join(ROOT, p));
  if (!file.startsWith(ROOT) || !existsSync(file) || statSync(file).isDirectory()) {
    // Single-page app: unknown paths fall back to index.html.
    return sendFile(res, join(ROOT, 'index.html'));
  }
  sendFile(res, file);
});

// Forward to the production backend as a plain server-side request (no Origin,
// so no CORS). Only the content headers travel: the auditor's basic-auth
// password stays here. Signed-in features (Climb, online play) send their own
// Authorization header, which the password gate refuses; an audit does not need them.
async function forward(req, res) {
  try {
    const chunks = [];
    for await (const c of req) chunks.push(c);
    const headers = {};
    for (const h of ['content-type', 'accept']) {
      if (req.headers[h]) headers[h] = req.headers[h];
    }
    const r = await fetch(UPSTREAM + req.url, {
      method: req.method,
      headers,
      body: ['GET', 'HEAD'].includes(req.method) ? undefined : Buffer.concat(chunks),
    });
    const out = { 'Cache-Control': 'no-store' };
    const ct = r.headers.get('content-type');
    if (ct) out['Content-Type'] = ct;
    res.writeHead(r.status, out);
    res.end(Buffer.from(await r.arrayBuffer()));
  } catch (e) {
    res.writeHead(502);
    res.end('upstream unreachable');
  }
}

function sendFile(res, file) {
  res.writeHead(200, { 'Content-Type': MIME[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
  res.end(readFileSync(file));
}

server.listen(PORT, () => {
  console.log(`\n  ▸ SpellGame AUDIT PREVIEW${LANG ? ` (${LANG} only)` : ''} — private, unverified draft content\n`);
  console.log(`    local:     http://localhost:${PORT}/`);
  console.log(`    username:  ${USER}`);
  console.log(`    password:  ${PASS}\n`);
  console.log(`    To reach a remote auditor, expose this port over a tunnel:`);
  console.log(`      cloudflared tunnel --url http://localhost:${PORT}`);
  console.log(`      ngrok http ${PORT}`);
  console.log(`    then send them the tunnel URL + the password above.\n`);
  console.log(`    (Ctrl-C to stop. Do not leave this exposed — it is not a release.)\n`);
});
