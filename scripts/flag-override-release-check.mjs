#!/usr/bin/env node
// CC-LAUNCH-EN item 3: in a release bundle, localStorage cannot switch a flag on.
//
// It serves dist/ (the release app bundle from scripts/build-web.sh) on a local
// port, sets the override keys for the default-off Impossible master flag, then
// reloads and checks that the difficulty picker still offers no Impossible
// option. Picker entries exist only when that flag is on
// (impossible::engine::picker_entries), so an "impossible" option means the
// override moved the flag.
//
// Usage: node scripts/flag-override-release-check.mjs [distDir]
// With --expect-override it asserts the opposite, for a dev bundle (dist-dev),
// to prove the check can see a working override.
import { createServer } from 'node:http';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join, extname, resolve } from 'node:path';
import { chromium } from 'playwright';

const args = process.argv.slice(2);
const expectOverride = args.includes('--expect-override');
const DIST = resolve(args.find((a) => !a.startsWith('--')) || 'dist');
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml' };

const server = createServer((req, res) => {
  let p = decodeURIComponent(req.url.split('?')[0]);
  if (p === '/') p = '/index.html';
  const file = join(DIST, p);
  if (!file.startsWith(DIST) || !existsSync(file) || statSync(file).isDirectory()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': MIME[extname(file)] || 'application/octet-stream' });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch();
let failed = false;
try {
  const page = await browser.newPage();
  // Block every non-local request: this check must not reach spellgame.net.
  await page.route('**/*', (route) => (route.request().url().startsWith(base) ? route.continue() : route.abort()));
  await page.addInitScript(() => {
    localStorage.setItem('byear_agegate_v1', JSON.stringify({ verdict: 'full', checkedAt: 1700000000 }));
    for (const f of ['impossible', 'impossible_en', 'impossible_mix']) {
      localStorage.setItem(`spell_flag_${f}`, 'on');
      localStorage.setItem(`spell_flag_override:${f}`, 'on');
    }
  });
  await page.goto(base);
  await page.waitForSelector('#levelSel option', { state: 'attached', timeout: 20000 });
  const values = await page.$$eval('#levelSel option', (os) => os.map((o) => o.value));
  const moved = values.some((v) => v.startsWith('impossible'));
  if (moved !== expectOverride) {
    failed = true;
    console.error(`flag-override-release-check: FAIL. ${DIST} picker offers [${values.join(', ')}]; ` +
      (expectOverride ? 'the dev override did not take' : 'localStorage switched the Impossible flag on'));
  } else {
    console.log(`flag-override-release-check: OK. ${expectOverride ? 'dev override works' : 'release ignores the override'} (picker: ${values.join(', ')})`);
  }
} finally {
  await browser.close();
  server.close();
}
process.exit(failed ? 1 : 0);
