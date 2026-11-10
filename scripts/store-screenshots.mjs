#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F12: the App Store screenshots, scene by scene, from the
// release tree's own build (config/store-scenes.json is the list, AT-28 checks
// it against the flags). Real interactions, never staged state: a real spoken
// clip drives the orb's corona, a real misspelling the diff, a real ten-word
// perfect run the celebration. The multi-script and themes scenes are 2x2
// composites of real screens.
//
// CC-LAUNCH-EN (2026-11-10): the shots are taken from the RELEASE profile build
// (English only, no network services), with every flag at its release value from
// config/flags.release.json and the rows src/launch_hide.rs hides hidden, so a shot
// shows exactly what the App Store build shows. The player owns SpellGame English
// (the dev-only ownership seam), because the listing shows the whole game.
//
// Usage, from the release branch the build is cut from:
//   SPELL_RELEASE_LANGS=1 bash scripts/build-web-test.sh   # the seam is needed to read the word
//   (cd dist-test-release && python3 -m http.server 8099 &)
//   SHOT_OUT=fastlane/screenshots/en-US node scripts/store-screenshots.mjs
//   node scripts/store-assets-check.mjs
import { chromium } from 'playwright';
import fs from 'node:fs';

const BASE = process.env.SHOT_BASE || 'http://localhost:8099/';
const OUT = process.env.SHOT_OUT || 'fastlane/screenshots/en-US';
const ONLY = (process.env.SHOT_ONLY || '').split(',').filter(Boolean);
const AGE = JSON.stringify({ verdict: 'full', checkedAt: Math.floor(Date.now() / 1000) });
const KID = JSON.stringify({ verdict: 'kid', checkedAt: Math.floor(Date.now() / 1000) });
const DEVICES = [
  { name: 'iphone69', w: 430, h: 932, dpr: 3, mobile: true },   // 1290 x 2796
  { name: 'ipad13', w: 1032, h: 1376, dpr: 2, mobile: false },  // 2064 x 2752
];
const manifest = JSON.parse(fs.readFileSync('config/store-scenes.json', 'utf8'));
// Every flag at its release value. A test build reads every flag from storage, so
// without this a shot would show test-build defaults the App Store build does not
// have. AT-28 separately proves each scene's own flags ship on.
const RELEASE = JSON.parse(fs.readFileSync('config/flags.release.json', 'utf8'));
const FLAGS = Object.entries(RELEASE).filter(([k]) => !k.startsWith('$')).map(([k, v]) => [k, v.ship ? 'on' : 'off']);
// What the launch hides (src/launch_hide.rs), read from the source so the two cannot drift.
const HIDE_SRC = fs.readFileSync('src/launch_hide.rs', 'utf8');
const hideList = (name) => [...HIDE_SRC.match(new RegExp(`${name}: &\\[&str\\] = &\\[([^\\]]*)\\]`))[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
const HIDDEN_ROWS = hideList('HIDDEN_ROWS');
const HIDDEN_DRAWER = hideList('HIDDEN_DRAWER_TARGETS');
if (!HIDDEN_ROWS.length || !HIDDEN_DRAWER.length) throw new Error('could not read the launch_hide lists');

/** A short clean tone, so the corona has a voice to follow. */
function wav() {
  const rate = 22050, n = Math.round(rate * 1.4);
  const b = Buffer.alloc(44 + n * 2);
  b.write('RIFF', 0); b.writeUInt32LE(36 + n * 2, 4); b.write('WAVE', 8); b.write('fmt ', 12);
  b.writeUInt32LE(16, 16); b.writeUInt16LE(1, 20); b.writeUInt16LE(1, 22); b.writeUInt32LE(rate, 24);
  b.writeUInt32LE(rate * 2, 28); b.writeUInt16LE(2, 32); b.writeUInt16LE(16, 34); b.write('data', 36); b.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) { const t = i / rate; const env = Math.min(1, t * 8) * Math.min(1, (1.4 - t) * 8); b.writeInt16LE(Math.round(Math.sin(2 * Math.PI * 330 * t) * env * 0.6 * 32767), 44 + i * 2); }
  return b;
}
const CLIP = wav();

async function open(browser, d, { locale = 'en', init = null, kid = false } = {}) {
  const ctx = await browser.newContext({ viewport: { width: d.w, height: d.h }, deviceScaleFactor: d.dpr, isMobile: d.mobile, colorScheme: 'dark' });
  await ctx.addInitScript(([age, l, flags, rows, drawer]) => {
    for (const [f, v] of flags) localStorage.setItem(`spell_flag_${f}`, v);
    localStorage.setItem('spell_dev_owned', 'net.spellgame.english');
    // The App Store build runs on iOS, and the menu lists the game modes only on
    // a native platform (play_hub::ctx native). No plugins: every native call
    // still falls back as it does in the browser.
    window.Capacitor = { isNativePlatform: () => true, getPlatform: () => 'ios', Plugins: {} };
    // The launch's hidden rows, hidden as a release build hides them
    // (src/launch_hide.rs). Hidden with CSS, never removed: the app panics on a
    // missing element id (dom::el), and the release build keeps them in the DOM too.
    document.addEventListener('DOMContentLoaded', () => {
      const st = document.createElement('style');
      st.textContent = [...rows.map((id) => `#${id}`), ...drawer.map((id) => `.nav-row[data-target="${id}"]`)].join(',') + '{display:none!important}';
      document.head.appendChild(st);
    });
    localStorage.setItem('byear_agegate_v1', age);
    localStorage.setItem('spellgame.locale', l);
    localStorage.setItem('spell_flag_learner_surfaces', 'off');
    Object.defineProperty(window, 'speechSynthesis', { configurable: true, get: () => ({ speak(u) { setTimeout(() => { u.onstart && u.onstart(); setTimeout(() => u.onend && u.onend(), 200); }, 10); }, cancel() {}, pause() {}, resume() {}, getVoices: () => [], speaking: false, pending: false, paused: false, addEventListener() {}, removeEventListener() {} }) });
  }, [kid ? KID : AGE, locale, FLAGS, HIDDEN_ROWS, HIDDEN_DRAWER]);
  if (init) await ctx.addInitScript(init);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/wav', body: CLIP }));
  await ctx.route('**/telemetry/**', (r) => r.fulfill({ status: 404, body: '' }));
  // The real reviewed definitions, from the live server (src=gen1). The page is
  // served from localhost, which the server's CORS rules do not list, so the
  // request is made by Playwright and handed to the page unchanged.
  await ctx.route('**/api/meaning**', async (r) => {
    const resp = await r.fetch();
    await r.fulfill({ response: resp, headers: { ...resp.headers(), 'access-control-allow-origin': '*' } });
  });
  const page = await ctx.newPage();
  await page.goto(BASE, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  if (locale !== 'en') {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', locale).catch(() => {});
    await page.click('#setupDone').catch(() => {});
  }
  await page.evaluate(() => document.fonts.ready);
  return { ctx, page };
}

const word = (page) => page.evaluate(() => window.__spelltest.currentWord());
async function serve(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 8000 });
  return word(page);
}
async function type(page, text) {
  for (const ch of text) { const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`); if (k) await k.click(); }
}

/** Draw four PNG buffers into one device-size 2x2 image, with a label under each. */
async function composite(page, d, panels) {
  const W = d.w * d.dpr, H = d.h * d.dpr;
  const urls = panels.map((p) => ({ src: `data:image/png;base64,${p.png.toString('base64')}`, label: p.label }));
  const data = await page.evaluate(async ({ W, H, urls }) => {
    const c = document.createElement('canvas'); c.width = W; c.height = H;
    const x = c.getContext('2d');
    x.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--bg-color').trim() || '#1c1830';
    x.fillRect(0, 0, W, H);
    const gap = Math.round(W * 0.03), labelH = Math.round(H * 0.035);
    const cw = (W - gap * 3) / 2, ch = (H - gap * 3 - labelH * 2) / 2;
    const load = (s) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = s; });
    for (let k = 0; k < 4; k++) {
      const img = await load(urls[k].src);
      const s = Math.min(cw / img.width, ch / img.height);
      const iw = img.width * s, ih = img.height * s;
      const col = k % 2, row = Math.floor(k / 2);
      const ox = gap + col * (cw + gap) + (cw - iw) / 2, oy = gap + row * (ch + gap + labelH) + (ch - ih) / 2;
      x.save(); x.beginPath(); x.roundRect(ox, oy, iw, ih, Math.round(W * 0.025)); x.clip(); x.drawImage(img, ox, oy, iw, ih); x.restore();
      x.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--ink').trim() || '#efeaf7';
      x.font = `700 ${Math.round(labelH * 0.62)}px 'Bricolage Grotesque', system-ui, sans-serif`;
      x.textAlign = 'center'; x.textBaseline = 'middle';
      x.fillText(urls[k].label, ox + iw / 2, oy + ih + labelH * 0.55);
    }
    return c.toDataURL('image/png');
  }, { W, H, urls });
  return Buffer.from(data.split(',')[1], 'base64');
}

const SCENES = {
  async play(browser, d) {
    const { ctx, page } = await open(browser, d);
    let w = await serve(page);
    for (let i = 0; i < 15 && w.length < 7; i++) { await type(page, w); await page.click('#checkBtn'); await page.waitForFunction(() => /\bgood\b/.test(document.getElementById('feedback').className), null, { timeout: 8000 }); await page.click('#orbWrap'); await page.waitForFunction((p) => { const c = window.__spelltest.currentWord(); return !!c && c !== p; }, w, { timeout: 8000 }); w = await word(page); }
    await type(page, w.slice(0, Math.ceil(w.length / 2)));
    await page.waitForTimeout(500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async meaning(browser, d) {
    // The Definition button on the word being played: the reviewed definition,
    // fetched from the live server exactly as the app fetches it.
    const { ctx, page } = await open(browser, d);
    let ok = false;
    for (let i = 0; i < 12 && !ok; i++) {
      const w = await serve(page);
      await page.click('#defBtn');
      ok = await page.waitForFunction(() => (document.getElementById('meaning').textContent || '').trim().length > 20, null, { timeout: 6000 }).then(() => true, () => false);
      if (!ok) { await type(page, w); await page.click('#checkBtn'); await page.waitForTimeout(1500); }
    }
    if (!ok) throw new Error('meaning: no definition shown in 12 words');
    await page.$eval('#meaning', (e) => e.scrollIntoView({ block: 'nearest' }));
    await page.waitForTimeout(700);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async spelldoku(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.click('#navBurger');
    await page.click('#navRow_spelldoku');
    await page.waitForTimeout(1500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async 'spell-off'(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.evaluate(() => document.getElementById('vsBtn').click());
    await page.waitForSelector('#vsSetupScrim.show, #vsSetupScrim.open, #vsSetupScrim[open]', { timeout: 3000 }).catch(() => {});
    await page.fill('#vsName1', 'Maya');
    await page.fill('#vsName2', 'Dad');
    await page.click('#vsStart');
    const w = await serve(page);
    await type(page, w.slice(0, Math.ceil(w.length / 2)));
    await page.waitForTimeout(500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async 'orb-speaking'(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.click('#orbWrap');
    await page.waitForFunction(() => { const w = document.getElementById('orbWrap'); return w.dataset.orb === 'speaking' && parseFloat(w.style.getPropertyValue('--orb-env')) > 0.7; }, null, { timeout: 8000 });
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async 'wrong-diff'(browser, d) {
    const { ctx, page } = await open(browser, d);
    let w = await serve(page);
    // A short word makes a thin diff: spell it right and take the next until one has six letters.
    for (let i = 0; i < 15 && w.length < 6; i++) {
      await type(page, w);
      await page.click('#checkBtn');
      await page.waitForFunction(() => /\bgood\b/.test(document.getElementById('feedback').className), null, { timeout: 8000 });
      await page.click('#orbWrap');
      await page.waitForFunction((prev) => { const c = window.__spelltest.currentWord(); return !!c && c !== prev; }, w, { timeout: 8000 });
      w = await word(page);
    }
    if (w.length < 6) throw new Error(`wrong-diff: no word of six letters or more in 15 tries (last ${w})`);
    // A natural misspelling: two neighbouring letters swapped, one left out.
    const typed = w.slice(0, 2) + w[3] + w[2] + w.slice(4, -2) + w.slice(-1);
    await type(page, typed);
    await page.click('#checkBtn');
    await page.waitForSelector('#letters .reveal-attempt', { timeout: 8000 });
    await page.waitForTimeout(500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async celebration(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.click('#orbWrap');
    for (let i = 0; i < 40; i++) {
      if (await page.evaluate(() => document.getElementById('celebrate').classList.contains('show'))) break;
      await page.waitForFunction(() => !!window.__spelltest.currentWord() && !/\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 15000 });
      await type(page, await word(page));
      await page.click('#checkBtn');
      await page.waitForFunction(() => /\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
      if (/bad/.test(await page.$eval('#feedback', (e) => e.className))) await page.click('#orbWrap');
    }
    await page.waitForFunction(() => document.getElementById('celebrate').classList.contains('show'), null, { timeout: 8000 });
    await page.waitForTimeout(1600);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async 'hub-icons'(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.click('#navBurger');
    await page.waitForSelector('.nav-row', { timeout: 5000 });
    await page.waitForTimeout(500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async 'multi-script'(browser, d) {
    const panels = [];
    for (const [l, label] of [['ko', '한국어'], ['ar', 'العربية'], ['hi', 'हिन्दी'], ['vi', 'Tiếng Việt']]) {
      const { ctx, page } = await open(browser, d, { locale: l });
      await serve(page);
      await page.waitForTimeout(1200);
      panels.push({ png: await page.screenshot(), label });
      await ctx.close();
    }
    const host = await open(browser, d);
    const png = await composite(host.page, d, panels); await host.ctx.close(); return png;
  },
  async themes(browser, d) {
    const unlocked = () => {
      localStorage.setItem('byear_stats_v1', JSON.stringify({ en: { easy: { seen: 700, correct: 650 }, hard: { seen: 150, correct: 130 } } }));
      localStorage.setItem('spell_daily_v1', JSON.stringify({ last_completed: '2026-10-20', streak: 9, best_streak: 9, history: {} }));
    };
    const { ctx, page } = await open(browser, d, { init: unlocked });
    await serve(page);
    const panels = [];
    for (const t of ['daybreak', 'chalkboard', 'typewriter', 'beestage']) {
      await page.evaluate((t) => document.querySelector(`#themePick [data-theme-choice="${t}"]`).click(), t);
      await page.waitForTimeout(500);
      const label = await page.evaluate((t) => document.querySelector(`#themePick [data-theme-choice="${t}"] b`).textContent, t);
      panels.push({ png: await page.screenshot(), label });
    }
    await page.evaluate(() => document.querySelector('#themePick [data-theme-choice="eclipse"]').click());
    const png = await composite(page, d, panels); await ctx.close(); return png;
  },
  async boardgame(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.evaluate(() => document.getElementById('bgOpenBtn').click());
    await page.waitForTimeout(800);
    await page.evaluate(() => document.getElementById('bgStart') && document.getElementById('bgStart').click());
    await page.waitForTimeout(1500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
  async randospell(browser, d) {
    const { ctx, page } = await open(browser, d);
    await page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await page.waitForTimeout(800);
    await page.evaluate(() => document.getElementById('rsStart').click());
    await page.waitForTimeout(800);
    await page.evaluate(() => document.getElementById('rsOrb').click());
    await page.waitForTimeout(3500);
    const png = await page.screenshot(); await ctx.close(); return png;
  },
};

fs.mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch({ args: ['--mute-audio'] });
try {
  for (const d of DEVICES) {
    for (const [i, s] of manifest.scenes.entries()) {
      if (ONLY.length && !ONLY.includes(s.id)) continue;
      const file = `${OUT}/${d.name}-${i + 1}-${s.id}.png`;
      process.stdout.write(`${file} ... `);
      fs.writeFileSync(file, await SCENES[s.id](browser, d));
      process.stdout.write('ok\n');
    }
  }
} finally {
  await browser.close();
}
