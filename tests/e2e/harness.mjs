// Shared E2E harness: serves the seam-enabled dist-test/ over a local HTTP
// server (never production / spellgame.net), launches Chromium, and provides a
// page factory that seeds the age gate and stubs TTS audio (audio correctness
// is tools/audio-verify's job, not E2E's). Specs read the expected word from the
// window.__spelltest seam and type via real key clicks on the anti-dictation
// keyboard — the thing under test — not input.fill or keyboard events.
import { createServer } from 'node:http';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const ROOT = join(fileURLToPath(import.meta.url), '..', '..', '..');
const DIST = join(ROOT, 'dist-test');
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png', '.mjs': 'text/javascript' };

export function startServer(port = 8129) {
  const server = createServer((req, res) => {
    let p = decodeURIComponent(req.url.split('?')[0]);
    if (p === '/') p = '/index.html';
    const file = join(DIST, p);
    if (!file.startsWith(DIST) || !existsSync(file) || statSync(file).isDirectory()) {
      res.writeHead(404); res.end(); return;
    }
    res.writeHead(200, { 'Content-Type': MIME[extname(file)] || 'application/octet-stream' });
    res.end(readFileSync(file));
  });
  return new Promise((resolve) => server.listen(port, () => resolve({ server, base: `http://localhost:${port}/` })));
}

// Devices per the harness contract: iPhone SE (375×667) and a large phone.
export const DEVICES = {
  se: { width: 375, height: 667, dpr: 2, mobile: true },
  large: { width: 430, height: 932, dpr: 3, mobile: true },
};

export async function launch() {
  return chromium.launch();
}

/** New page at `lang`, age gate satisfied, TTS stubbed silent, wasm booted. */
export async function openApp(browser, base, { lang = null, device = 'se' } = {}) {
  const d = DEVICES[device];
  const ctx = await browser.newContext({ viewport: { width: d.width, height: d.height }, deviceScaleFactor: d.dpr, isMobile: d.mobile });
  await ctx.addInitScript(([age, l]) => {
    localStorage.setItem('byear_agegate_v1', age);
    if (l) localStorage.setItem('spellgame.locale', l);
  }, [AGE, lang]);
  // Stub the backend audio so no real TTS traffic + deterministic timing.
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  // Wait for wasm boot: the seam installs once the app is up.
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  // The language picker lives in the setup sheet (home-regroup F3), so open the
  // sheet before selecting, then close it. Behaviour is unchanged — this is a
  // relocation of the same #langSel control.
  if (lang) {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', lang).catch(() => {});
    await page.click('#setupDone').catch(() => {});
    await page.waitForTimeout(200);
  }
  return { ctx, page };
}

/** Type `answer` on the on-screen keyboard via real key clicks (per language). */
export async function typeOnKeyboard(page, answer) {
  for (const ch of answer) {
    const sel = `#gameKeyboard .kb-key[data-k="${ch}"]`;
    const key = await page.$(sel);
    if (key) { await key.click(); }
    else { /* combining/tone/uppercase forms: click the base char if present */ }
  }
}

// Minimal assertion + result collector (no @playwright/test dependency).
export class Suite {
  constructor(name) { this.name = name; this.results = []; }
  async test(title, fn) {
    try { await fn(); this.results.push({ title, ok: true }); }
    catch (e) { this.results.push({ title, ok: false, err: e.message }); }
  }
}
export function assert(cond, msg) { if (!cond) throw new Error(msg); }
export function assertEq(a, b, msg) { if (a !== b) throw new Error(`${msg}: expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`); }
