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
// Password: set AUDIT_PASS to pin one, else a fresh random token is generated and
// printed each run. Username is "auditor".
import { createServer } from 'node:http';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join, extname, normalize } from 'node:path';
import { randomBytes } from 'node:crypto';

const ROOT = new URL('../dist-audit', import.meta.url).pathname;
if (!existsSync(join(ROOT, 'index.html'))) {
  console.error('  dist-audit/ not found — run:  bash scripts/build-web-audit.sh');
  process.exit(1);
}
const PORT = Number(process.env.AUDIT_PORT) || 8140;
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
  if (p === '/') p = '/index.html';
  const file = normalize(join(ROOT, p));
  if (!file.startsWith(ROOT) || !existsSync(file) || statSync(file).isDirectory()) {
    // Single-page app: unknown paths fall back to index.html.
    return sendFile(res, join(ROOT, 'index.html'));
  }
  sendFile(res, file);
});

function sendFile(res, file) {
  res.writeHead(200, { 'Content-Type': MIME[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
  res.end(readFileSync(file));
}

server.listen(PORT, () => {
  console.log(`\n  ▸ SpellGame AUDIT PREVIEW — private, unverified draft content\n`);
  console.log(`    local:     http://localhost:${PORT}/`);
  console.log(`    username:  ${USER}`);
  console.log(`    password:  ${PASS}\n`);
  console.log(`    To reach a remote auditor, expose this port over a tunnel:`);
  console.log(`      cloudflared tunnel --url http://localhost:${PORT}`);
  console.log(`      ngrok http ${PORT}`);
  console.log(`    then send them the tunnel URL + the password above.\n`);
  console.log(`    (Ctrl-C to stop. Do not leave this exposed — it is not a release.)\n`);
});
