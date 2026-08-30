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
// Which build this pass is exercising. The app config ships every language;
// the site config ships English. Specs that assert platform behaviour must
// run against the matching build or they assert nothing.
const DIST = join(ROOT, process.env.SPELL_WEB === '1' ? 'dist-test-web' : 'dist-test');
export const IS_WEB_BUILD = process.env.SPELL_WEB === '1';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png', '.mjs': 'text/javascript' };

export function startServer(port = 8129) {
  const server = createServer((req, res) => {
    let p = decodeURIComponent(req.url.split('?')[0]);

    // STUB /api/check LOCALLY.
    //
    // English grades through an async POST to /api/check. Against the real
    // backend that request is CORS-blocked from this harness -- the allowlist
    // covers :3000, :5173, capacitor://localhost and the production domain,
    // never the port we serve on -- so it FAILS, and the app falls back to a
    // local comparison. Two consequences, both bad:
    //
    //   1. Every English grading assertion in this suite has been timing a
    //      doomed network round trip. It took under 500ms for a long time and
    //      now takes ~2.7s, which broke A9 and both attempts-shields tests in
    //      one day -- three "regressions" that were fixed sleeps drifting past
    //      a call nobody meant to make.
    //   2. backend_verify's SUCCESS path has never been exercised. The suite
    //      only ever ran its error fallback.
    //
    // Serving it here makes English grading instant, deterministic, offline,
    // and -- for the first time -- actually the path the player takes.
    if (p === '/api/check' && req.method === 'POST') {
      let body = '';
      req.on('data', (c) => { body += c; });
      req.on('end', () => {
        let correct = false;
        try {
          const d = JSON.parse(body || '{}');
          const norm = (x) => String(x || '').trim().toLowerCase();
          correct = norm(d.word) === norm(d.answer);
        } catch (_) { /* malformed body grades wrong, same as the server */ }
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ correct }));
      });
      return;
    }

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
  // The suite must be SILENT (Eric, 2026-08-05: "Whats with all the
  // voiced out words in the background"). Headless Chromium routes the
  // Web Speech API to the host's own voices, so a test run narrates
  // itself out loud through the Mac's speakers. Local + free, but
  // maddening — mute at the browser, and neuter speechSynthesis in the
  // page (below) so nothing queues either.
  return chromium.launch({ args: ['--mute-audio'] });
}

/** New page at `lang`, age gate satisfied, TTS stubbed silent, wasm booted.
 *  `viewport: {width,height}` overrides the named device's dimensions (used by
 *  the submit-control-per-width sweep); DPR/mobile flags come from `device`.
 *  Feature flags are set by each spec via its own `ctx.addInitScript`
 *  (`localStorage['spell_flag_<name>']`), mirroring the FP2 specs (sayit /
 *  spellaloud / ghost) — the harness stays flag-agnostic. */
/// Every context — openApp's and the specs that build their own — gets
/// the same BASELINE: learner surfaces off (the placement card would
/// pause a first serve), and speechSynthesis muted. A spec that is
/// TESTING one of these turns it on explicitly, which is the only way
/// the flag state under test is ever ambiguous-free.
export async function pinBaseline(ctx) {
  await ctx.addInitScript(() => {
    try {
      Object.defineProperty(window, 'speechSynthesis', {
        configurable: true,
        get: () => ({
          speak() {}, cancel() {}, pause() {}, resume() {},
          getVoices: () => [], speaking: false, pending: false, paused: false,
          addEventListener() {}, removeEventListener() {},
        }),
      });
    } catch (_) { /* the --mute-audio flag still covers audio */ }
    if (!localStorage.getItem('spell_flag_learner_surfaces')) {
      localStorage.setItem('spell_flag_learner_surfaces', 'off');
      localStorage.setItem('spell_flag_learner_select', 'off');
    }
  });
  return ctx;
}

export async function openApp(browser, base, { lang = null, device = 'se', viewport = null } = {}) {
  const d = viewport ? { ...DEVICES[device], ...viewport } : DEVICES[device];
  const ctx = await browser.newContext({ viewport: { width: d.width, height: d.height }, deviceScaleFactor: d.dpr, isMobile: d.mobile });
  await ctx.addInitScript(([age, l]) => {
    localStorage.setItem('byear_agegate_v1', age);
    if (l) localStorage.setItem('spellgame.locale', l);
    // Learner surfaces default ON in the app (L1/L2 QA pass); the e2e
    // BASELINE pins them off so every legacy spec's first solo serve
    // stays deterministic. placement.spec turns them on explicitly —
    // the flag state under test is always the one the spec declares.
    // Silence the Web Speech API for the whole run — see launch().
    try {
      Object.defineProperty(window, 'speechSynthesis', {
        configurable: true,
        get: () => ({
          speak() {}, cancel() {}, pause() {}, resume() {},
          getVoices: () => [], speaking: false, pending: false, paused: false,
          addEventListener() {}, removeEventListener() {},
        }),
      });
    } catch (_) { /* older engines: the mute flag still covers audio */ }
    if (!localStorage.getItem('spell_flag_learner_surfaces')) {
      localStorage.setItem('spell_flag_learner_surfaces', 'off');
      localStorage.setItem('spell_flag_learner_select', 'off');
    }
  }, [AGE, lang]);
  // Stub the backend audio so no real TTS traffic + deterministic timing.
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  // Point the app's API base at THIS server, so /api/check hits the local stub
  // above instead of a cross-origin request that is guaranteed to fail. Set
  // AFTER load on purpose: index.html assigns SPELL_API_BASE in a script tag,
  // so an addInitScript would be overwritten. api_base() re-reads window on
  // every call, so a late assignment is honoured.
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
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
