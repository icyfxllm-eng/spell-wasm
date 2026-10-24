#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F12: the App Store app preview, beat by beat, from the
// release tree's own build (reports/visual-eclipse-phase-f-storyboard.md is
// the signed storyboard). Every beat is a real session: frames come from the
// browser's screencast at the store size, sound is the app's own audio (the
// spoken word through its <audio> element, the game sounds through WebAudio)
// recorded inside the page. The voice is heard in beats 1 and 2 only (Eric,
// 2026-10-24); after that only the game sounds.
//
// Usage, from the release branch the build is cut from:
//   bash scripts/build-web-test.sh
//   (cd dist-test && python3 -m http.server 8099 &)
//   PREVIEW_DEVICE=iphone69 node scripts/store-preview.mjs   # then ipad13
// Needs ffmpeg. Writes fastlane/previews/<device>-preview.mp4.
import { chromium } from 'playwright';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const BASE = process.env.SHOT_BASE || 'http://localhost:8099/';
const OUT = process.env.PREVIEW_OUT || 'fastlane/previews';
const WORK = process.env.PREVIEW_WORK || '/tmp/spell-preview';
const API = 'https://spellgame.net';
const FPS = 30;
const AGE = JSON.stringify({ verdict: 'full', checkedAt: Math.floor(Date.now() / 1000) });
// App preview sizes: the viewport keeps the screenshot geometry, the scale
// lands the screencast exactly on the store's pixels.
const DEVICES = {
  iphone69: { w: 430, h: 932, out: [886, 1920], mobile: true },
  ipad13: { w: 1032, h: 1376, out: [1200, 1600], mobile: false },
};
const d = DEVICES[process.env.PREVIEW_DEVICE || 'iphone69'];
const devName = process.env.PREVIEW_DEVICE || 'iphone69';
if (!d) throw new Error(`unknown PREVIEW_DEVICE ${process.env.PREVIEW_DEVICE}`);
const manifest = JSON.parse(fs.readFileSync('config/store-scenes.json', 'utf8'));
const FLAGS = [...new Set(manifest.scenes.flatMap((s) => s.flags).concat(['visual_sound']))];

/** Runs in the page before the app: routes every sound the app makes into one recorder. */
function recorderInit([age, locale, flags, voice]) {
  for (const f of flags) localStorage.setItem(`spell_flag_${f}`, 'on');
  localStorage.setItem('byear_agegate_v1', age);
  localStorage.setItem('spellgame.locale', locale);
  localStorage.setItem('spell_flag_learner_surfaces', 'off');
  // The word clip comes from the page's own origin (proxied to the server), so
  // the recorder can hear it; a cross-origin clip would play silent into it.
  Object.defineProperty(window, 'SPELL_API_BASE', { configurable: true, get: () => location.origin, set() {} });
  const master = new AudioContext();
  master.__rec = true;
  const out = master.createMediaStreamDestination();
  const voiceGain = master.createGain();
  voiceGain.gain.value = voice ? 1 : 0;
  voiceGain.connect(out);
  const taps = new WeakMap();
  const connect = AudioNode.prototype.connect;
  AudioNode.prototype.connect = function (dest, ...rest) {
    const r = connect.call(this, dest, ...rest);
    const ctx = this.context;
    if (!ctx.__rec && dest === ctx.destination) {
      if (!taps.has(ctx)) {
        const t = ctx.createMediaStreamDestination();
        taps.set(ctx, t);
        connect.call(master.createMediaStreamSource(t.stream), out);
      }
      connect.call(this, taps.get(ctx));
    }
    return r;
  };
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    if (!this.__rec) {
      this.__rec = true;
      try { const s = master.createMediaElementSource(this); connect.call(s, master.destination); connect.call(s, voiceGain); } catch (_) { /* already wired */ }
    }
    return play.call(this);
  };
  window.__rec = {
    start() {
      master.resume();
      this.chunks = [];
      this.mr = new MediaRecorder(out.stream, { mimeType: 'audio/webm;codecs=opus', audioBitsPerSecond: 256000 });
      this.mr.ondataavailable = (e) => this.chunks.push(e.data);
      this.mr.start();
      this.t0 = Date.now();
      return this.t0;
    },
    async stop() {
      const done = new Promise((ok) => { this.mr.onstop = ok; });
      this.mr.stop();
      await done;
      const buf = await new Blob(this.chunks).arrayBuffer();
      let s = '';
      const b = new Uint8Array(buf);
      for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
      return btoa(s);
    },
  };
  Object.defineProperty(window, 'speechSynthesis', { configurable: true, get: () => ({ speak(u) { setTimeout(() => { u.onstart && u.onstart(); setTimeout(() => u.onend && u.onend(), 600); }, 10); }, cancel() {}, pause() {}, resume() {}, getVoices: () => [], speaking: false, pending: false, paused: false, addEventListener() {}, removeEventListener() {} }) });
}

async function open(browser, { locale = 'en', voice = false, init = null } = {}) {
  const ctx = await browser.newContext({
    viewport: { width: d.w, height: d.h }, deviceScaleFactor: d.out[0] / d.w, isMobile: d.mobile, colorScheme: 'dark',
  });
  await ctx.addInitScript(recorderInit, [AGE, locale, FLAGS, voice]);
  if (init) await ctx.addInitScript(init);
  await ctx.route('**/api/speak**', async (r) => {
    const u = new URL(r.request().url());
    try {
      const res = await r.fetch({ url: API + u.pathname + u.search });
      await r.fulfill({ response: res });
    } catch (_) { /* the session closed while the clip was in flight */ }
  });
  await ctx.route('**/telemetry/**', (r) => r.fulfill({ status: 404, body: '' }));
  const page = await ctx.newPage();
  await page.goto(BASE, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  if (locale !== 'en') {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', locale).catch(() => {});
    await page.click('#setupDone').catch(() => {});
  }
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400); // sleep-ok: a recording, not a test
  return { ctx, page };
}

/** Records one take: screencast frames plus the page's audio, cut to `secs`. */
async function take(page, name, secs, act) {
  const dir = path.join(WORK, name);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  const cdp = await page.context().newCDPSession(page);
  const frames = [];
  cdp.on('Page.screencastFrame', (f) => {
    frames.push({ t: f.metadata.timestamp * 1000, data: f.data });
    cdp.send('Page.screencastFrameAck', { sessionId: f.sessionId }).catch(() => {});
  });
  await cdp.send('Page.startScreencast', { format: 'png', maxWidth: d.out[0], maxHeight: d.out[1], everyNthFrame: 1 });
  await page.waitForTimeout(150); // sleep-ok: the first frame
  const t0 = await page.evaluate(() => window.__rec.start());
  await act();
  const left = t0 + secs * 1000 + 200 - Date.now();
  if (left > 0) await page.waitForTimeout(left);
  const audio = await page.evaluate(() => window.__rec.stop());
  await cdp.send('Page.stopScreencast');
  fs.writeFileSync(path.join(dir, 'audio.webm'), Buffer.from(audio, 'base64'));
  // A constant 30 fps from frames that arrive only when the screen changes:
  // each tick shows the newest frame at or before it.
  frames.sort((a, b) => a.t - b.t);
  const n = Math.round(secs * FPS);
  let j = 0;
  for (let i = 0; i < n; i++) {
    const t = t0 + (i * 1000) / FPS;
    while (j + 1 < frames.length && frames[j + 1].t <= t) j++;
    fs.writeFileSync(path.join(dir, `f${String(i).padStart(5, '0')}.png`), Buffer.from(frames[j].data, 'base64'));
  }
  const mp4 = path.join(WORK, `${name}.mp4`);
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(FPS), '-i', path.join(dir, 'f%05d.png'), '-i', path.join(dir, 'audio.webm'),
    '-t', String(secs), '-vf', `scale=${d.out[0]}:${d.out[1]}:flags=lanczos,format=yuv420p`, '-c:v', 'libx264', '-profile:v', 'high', '-crf', '14', '-r', String(FPS),
    '-af', 'apad', '-c:a', 'aac', '-b:a', '256k', '-ar', '48000', '-ac', '2', mp4]);
  console.log(`${name}: ${n} frames from ${frames.length} screencast frames`);
  return mp4;
}

const word = (page) => page.evaluate(() => window.__spelltest.currentWord());
const fb = (page, re) => page.waitForFunction((re) => new RegExp(re).test(document.getElementById('feedback').className), re, { timeout: 10000 });
async function type(page, text, ms) {
  for (const ch of text) { const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`); if (k) await k.click(); if (ms) await page.waitForTimeout(ms); }
}
async function serve(page, prev = null) {
  await page.click('#orbWrap');
  await page.waitForFunction((p) => { const c = window.__spelltest.currentWord(); return !!c && c !== p; }, prev, { timeout: 10000 });
  return word(page);
}
const BEATS = [
  // 1 + 2: hear it, spell it. One take, voice on.
  async (browser) => {
    const { ctx, page } = await open(browser, { voice: true });
    const mp4 = await take(page, '1-hear-spell', 7.0, async () => {
      await page.waitForTimeout(500);
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 10000 });
      const w = await word(page);
      await page.waitForTimeout(1900);
      await type(page, w, Math.min(150, 2200 / w.length));
      await page.waitForTimeout(250);
      await page.click('#checkBtn');
    });
    await ctx.close(); return mp4;
  },
  // 3: a near miss, held long enough to read.
  async (browser) => {
    // Short words are skipped with Give up, which builds no chain: a miss that
    // breaks a chain of three asks for a leaderboard name over the diff.
    const { ctx, page } = await open(browser);
    let w = await serve(page);
    for (let i = 0; i < 25 && w.length < 7; i++) {
      await page.click('#giveupBtn');
      await page.waitForTimeout(700);
      w = await serve(page, w);
    }
    if (w.length < 7) throw new Error(`3-miss: no word of seven letters in 25 tries (last ${w})`);
    await page.waitForTimeout(600);
    const typed = w.slice(0, 2) + w[3] + w[2] + w.slice(4, -2) + w.slice(-1);
    const mp4 = await take(page, '3-miss', 3.5, async () => {
      await page.waitForTimeout(200);
      await type(page, typed, 70);
      await page.click('#checkBtn');
    });
    await ctx.close(); return mp4;
  },
  // 4: the end of a perfect run, into totality.
  async (browser) => {
    const { ctx, page } = await open(browser);
    let w = await serve(page);
    for (let i = 0; i < 8; i++) { await type(page, w); await page.click('#checkBtn'); await fb(page, '\\bgood\\b'); w = await serve(page, w); }
    await page.waitForTimeout(500);
    const mp4 = await take(page, '4-streak', 4.5, async () => {
      await type(page, w, 45); await page.click('#checkBtn'); await fb(page, '\\bgood\\b');
      w = await serve(page, w); await page.waitForTimeout(250);
      await type(page, w, 45); await page.click('#checkBtn');
    });
    await ctx.close(); return mp4;
  },
  // 5: four languages, one second each.
  ...[['ko', 2], ['ar', 2], ['hi', 2], ['vi', 2]].map(([l]) => async (browser) => {
    const { ctx, page } = await open(browser, { locale: l });
    const w = await serve(page);
    await page.waitForTimeout(900);
    const mp4 = await take(page, `5-${l}`, 1.0, async () => { await type(page, [...w].slice(0, 2).join(''), 300); });
    await ctx.close(); return mp4;
  }),
  // 6: the menu, then Board Game and Randospell.
  async (browser) => {
    const { ctx, page } = await open(browser);
    const mp4 = await take(page, '6-menu', 1.6, async () => { await page.waitForTimeout(200); await page.click('#navBurger'); });
    await ctx.close(); return mp4;
  },
  async (browser) => {
    const { ctx, page } = await open(browser);
    await page.evaluate(() => document.getElementById('bgOpenBtn').click());
    await page.waitForTimeout(800);
    await page.evaluate(() => document.getElementById('bgStart') && document.getElementById('bgStart').click());
    await page.waitForTimeout(600);
    const mp4 = await take(page, '6-board', 1.0, async () => {});
    await ctx.close(); return mp4;
  },
  async (browser) => {
    const { ctx, page } = await open(browser);
    await page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await page.waitForTimeout(800);
    await page.evaluate(() => document.getElementById('rsStart').click());
    await page.waitForTimeout(600);
    const mp4 = await take(page, '6-rando', 1.2, async () => { await page.evaluate(() => document.getElementById('rsOrb').click()); });
    await ctx.close(); return mp4;
  },
  // 7: make it yours. Themes earned the real way's numbers, seeded before load.
  async (browser) => {
    const unlocked = () => {
      localStorage.setItem('byear_stats_v1', JSON.stringify({ en: { easy: { seen: 700, correct: 650 }, hard: { seen: 150, correct: 130 } } }));
      localStorage.setItem('spell_daily_v1', JSON.stringify({ last_completed: '2026-10-20', streak: 9, best_streak: 9, history: {} }));
    };
    const { ctx, page } = await open(browser, { init: unlocked });
    await page.click('#navBurger');
    await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
    await page.click('.nav-row[data-mode="help_settings.title"]');
    await page.waitForTimeout(600);
    await page.evaluate(() => document.getElementById('themePick').scrollIntoView({ block: 'center' }));
    await page.waitForTimeout(400);
    const pick = (t) => page.click(`#themePick [data-theme-choice="${t}"]`);
    const mp4 = await take(page, '7-themes', 4.0, async () => {
      await page.waitForTimeout(500); await pick('daybreak');
      await page.waitForTimeout(900); await pick('chalkboard');
      await page.waitForTimeout(900); await page.click('#setDone');
    });
    await ctx.close(); return mp4;
  },
];

fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(WORK, { recursive: true });
const browser = await chromium.launch({ args: ['--autoplay-policy=no-user-gesture-required'] });
const clips = [];
try {
  for (const b of BEATS) clips.push(await b(browser));
} finally {
  await browser.close();
}
const list = path.join(WORK, 'list.txt');
fs.writeFileSync(list, clips.map((c) => `file '${c}'`).join('\n') + '\n');
const final = path.join(OUT, `${devName}-preview.mp4`);
execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'concat', '-safe', '0', '-i', list,
  '-c:v', 'libx264', '-profile:v', 'high', '-crf', '16', '-r', String(FPS), '-pix_fmt', 'yuv420p',
  '-c:a', 'aac', '-b:a', '256k', '-ar', '48000', '-ac', '2', '-movflags', '+faststart', final]);
console.log(`wrote ${final}`);
