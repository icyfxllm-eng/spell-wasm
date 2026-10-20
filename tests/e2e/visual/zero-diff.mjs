// zero-diff.mjs -- CC-VISUAL-ECLIPSE AT-3: a deterministic screenshot harness
// that proves a CSS token migration (raw colours/durations -> var(--token) at
// identical values) changes ZERO pixels.
//
//   node tests/e2e/visual/zero-diff.mjs capture <outDir>
//   node tests/e2e/visual/zero-diff.mjs compare <baselineDir> <candidateDir>
//
// capture serves whatever dist-test/ holds (see harness.mjs) and writes one PNG
// per (scene x viewport), then re-captures a few scenes in fresh contexts and
// insists they are bit-identical ("determinism: OK").
// compare decodes both sets and counts differing pixels exactly (no
// tolerance, no anti-aliasing allowance), writes <candidateDir>/diff/<name>.png
// for every scene that differs, and exits 1 if any scene differs or is missing.
//
// The workflow for the migration: build the testseam dist at the old commit,
// capture into A; build at the new commit, capture into B; compare A B.
//
// DETERMINISM, which is the whole point:
//   - Math.random is a fixed LCG installed before any app script runs; the
//     word served, every shuffle and every deal follow from it.
//     crypto.getRandomValues / randomUUID are seeded the same way.
//   - Time: Playwright's clock is installed at a fixed instant (FIXED_TIME) and
//     Date is pinned there (setFixedTime), so dates, the daily seed, calendars
//     and any elapsed-time readout are identical on every run. Timers still run.
//     Just before each shot the clock is PAUSED, so no timer (an auto-advance
//     after Check, a countdown) can fire between "state reached" and the pixels.
//   - Animations and transitions: a stylesheet injected at document start sets
//     every animation to paused with delay 0 (so it sits at its 0% keyframe --
//     an animated colour still renders, it is not removed) and every transition
//     to none. Consequence: an element whose 0% keyframe is opacity 0 (the
//     last-typed letter's pop-in, for one) is captured invisible, identically
//     every time. ZD_ANIM_AT (below) moves the freeze point to test durations.
//     reducedMotion is pinned to 'no-preference' so reduced-motion branches do
//     not hide styling.
//   - Network: /api/speak returns empty audio, telemetry returns 404, /api/check
//     is the harness's local grader, and every request that leaves the local
//     server is answered 404 locally. Service workers are blocked.
//   - Fonts: document.fonts.ready, the harness's painted() signal, every <img>
//     decoded, and a DOM quiet window before each shot.
//   - Locale/timezone/colour scheme are pinned on the context.
//
// MASKS: none.
//
// SKIPPED (probed on the testseam build, 2026-10-20):
//   - Guardian dashboard (#gdashOpen): clicking its opener puts nothing new on
//     screen for a fresh profile (no .show surface appears); it sits behind
//     the grown-up gate and a guardian link this harness has no way to make.
//     The grown-up gate itself IS captured (parent_gate).
//   - Online Spell-off (#soBtn): clicking it shows nothing with the network
//     stubbed; it needs a live match server, which a hermetic run cannot have.
//   - Quick Bee Draw outside ja/zh: the opener is a no-op; captured under ja
//     with its flag on instead (qbd_ja, qbd_ja_play).
//   - Sign-in / account past the front door, photo capture past the source
//     picker: need a real auth backend or a camera/file chooser. The front
//     door and the photo source scrim ARE captured.
//   - Def Match past its opening: its round needs served definitions, so with
//     the network stubbed it shows its "couldn't reach the server" state
//     (captured as def_match; deterministic, but not the in-round look).
//   - The daily-result Share sheet, store/purchase sheets: OS-level or
//     network-backed, not stylesheet surfaces.
import { mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { inflateSync, deflateSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------- PNG codec
// Chromium screenshots are 8-bit RGB or RGBA, non-interlaced. That is all this
// decodes; anything else throws rather than guessing.
function decodePng(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG');
  let off = 8, w = 0, h = 0, ct = 0;
  const idat = [];
  while (off < buf.length) {
    const len = buf.readUInt32BE(off);
    const type = buf.toString('ascii', off + 4, off + 8);
    const data = buf.subarray(off + 8, off + 8 + len);
    if (type === 'IHDR') {
      w = data.readUInt32BE(0); h = data.readUInt32BE(4);
      const depth = data[8]; ct = data[9];
      if (depth !== 8 || (ct !== 2 && ct !== 6) || data[12] !== 0) {
        throw new Error(`unsupported PNG (depth ${depth}, colour type ${ct}, interlace ${data[12]})`);
      }
    } else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    off += 12 + len;
  }
  const bpp = ct === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = w * bpp;
  const px = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1;
    const dst = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? px[dst + x - bpp] : 0;
      const b = y > 0 ? px[dst - stride + x] : 0;
      const c = x >= bpp && y > 0 ? px[dst - stride + x - bpp] : 0;
      let v = raw[src + x];
      if (f === 1) v += a;
      else if (f === 2) v += b;
      else if (f === 3) v += (a + b) >> 1;
      else if (f === 4) {
        const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (f !== 0) throw new Error(`bad PNG filter ${f}`);
      px[dst + x] = v & 255;
    }
  }
  // Normalise to RGBA.
  if (bpp === 4) return { w, h, rgba: px };
  const rgba = Buffer.alloc(w * h * 4);
  for (let i = 0, j = 0; i < px.length; i += 3, j += 4) {
    rgba[j] = px[i]; rgba[j + 1] = px[i + 1]; rgba[j + 2] = px[i + 2]; rgba[j + 3] = 255;
  }
  return { w, h, rgba };
}

const CRC = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c; }
  return t;
})();
function crc32(b) { let c = -1; for (let i = 0; i < b.length; i++) c = CRC[(c ^ b[i]) & 255] ^ (c >>> 8); return (c ^ -1) >>> 0; }
function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, 'ascii');
  data.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}
function encodePng(w, h, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) rgba.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0)),
  ]);
}

/// Exact pixel comparison. Returns the differing-pixel count and, if any, a
/// diff image: the baseline washed out to grey with every differing pixel red.
function diffPng(aBuf, bBuf) {
  const a = decodePng(aBuf), b = decodePng(bBuf);
  if (a.w !== b.w || a.h !== b.h) {
    return { count: Math.max(a.w * a.h, b.w * b.h), note: `size ${a.w}x${a.h} vs ${b.w}x${b.h}`, img: null };
  }
  let count = 0;
  const out = Buffer.alloc(a.rgba.length);
  for (let i = 0; i < a.rgba.length; i += 4) {
    const same = a.rgba[i] === b.rgba[i] && a.rgba[i + 1] === b.rgba[i + 1]
      && a.rgba[i + 2] === b.rgba[i + 2] && a.rgba[i + 3] === b.rgba[i + 3];
    if (same) {
      const g = 160 + ((a.rgba[i] * 0.3 + a.rgba[i + 1] * 0.59 + a.rgba[i + 2] * 0.11) * 0.37) | 0;
      out[i] = out[i + 1] = out[i + 2] = g; out[i + 3] = 255;
    } else {
      count++;
      out[i] = 255; out[i + 1] = 0; out[i + 2] = 0; out[i + 3] = 255;
    }
  }
  return { count, note: '', img: count ? encodePng(a.w, a.h, out) : null };
}

// ---------------------------------------------------------------- compare
function compare(baseDir, candDir) {
  const list = (d) => readdirSync(d).filter((f) => f.endsWith('.png')).sort();
  const names = [...new Set([...list(baseDir), ...list(candDir)])].sort();
  const diffDir = join(candDir, 'diff');
  let bad = 0, total = 0;
  for (const n of names) {
    const pa = join(baseDir, n), pb = join(candDir, n);
    if (!existsSync(pa) || !existsSync(pb)) {
      console.log(`${n.padEnd(44)} MISSING in ${existsSync(pa) ? 'candidate' : 'baseline'}`);
      bad++; continue;
    }
    const r = diffPng(readFileSync(pa), readFileSync(pb));
    total += r.count;
    console.log(`${n.padEnd(44)} ${String(r.count).padStart(8)} differing px${r.note ? '  (' + r.note + ')' : ''}`);
    if (r.count) {
      bad++;
      if (r.img) { mkdirSync(diffDir, { recursive: true }); writeFileSync(join(diffDir, n), r.img); }
    }
  }
  console.log(`\n${names.length} scenes compared, ${bad} differ, ${total} differing pixels in total`);
  if (bad) console.log(`diff images: ${diffDir}`);
  process.exit(bad ? 1 : 0);
}

// ---------------------------------------------------------------- capture
// ZD_ANIM_AT=<seconds> parks every paused animation that far into its
// timeline instead of at 0%. The default (0) is the frozen first frame; a
// non-zero value makes the shot depend on each animation's DURATION and timing
// function, so a duration token that resolves to the wrong value shows up as
// pixels. Both baseline and candidate must use the same value.
const ANIM_AT = Number(process.env.ZD_ANIM_AT || 0);
const FIXED_TIME = new Date('2026-06-15T10:00:00Z').getTime();
const AGE_FULL = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

const VIEWPORTS = {
  v375: { width: 375, height: 667 },
  v440: { width: 440, height: 956 },
  v320: { width: 320, height: 568 },
};
const MAIN_VPS = ['v375', 'v440'];
const SMALL_VPS = ['v375', 'v440', 'v320'];

// Installed before any page script. Seeds every source of randomness the app
// could reach and freezes motion at its first frame.
function pageInit([seed, animAt]) {
  let s = seed >>> 0;
  Math.random = () => { s = (Math.imul(s, 1664525) + 1013904223) >>> 0; return s / 4294967296; };
  let c = (seed ^ 0x9e3779b9) >>> 0;
  const next = () => { c = (Math.imul(c, 1664525) + 1013904223) >>> 0; return c; };
  try {
    crypto.getRandomValues = function (arr) {
      const bytes = new Uint8Array(arr.buffer, arr.byteOffset, arr.byteLength);
      for (let i = 0; i < bytes.length; i++) bytes[i] = next() >>> 24;
      return arr;
    };
    crypto.randomUUID = function () {
      const h = []; for (let i = 0; i < 16; i++) h.push(((next() >>> 24) & 255).toString(16).padStart(2, '0'));
      return `${h.slice(0, 4).join('')}-${h.slice(4, 6).join('')}-4${h[6].slice(1)}${h[7]}-a${h[8].slice(1)}${h[9]}-${h.slice(10).join('')}`;
    };
  } catch (_) { /* crypto is non-writable in some engines; telemetry is stubbed anyway */ }
  try {
    Object.defineProperty(window, 'speechSynthesis', {
      configurable: true,
      get: () => ({
        speak() {}, cancel() {}, pause() {}, resume() {},
        getVoices: () => [], speaking: false, pending: false, paused: false,
        addEventListener() {}, removeEventListener() {},
      }),
    });
  } catch (_) { /* --mute-audio still covers it */ }
  const css = `*,*::before,*::after{animation-play-state:paused!important;animation-delay:${animAt ? '-' + animAt + 's' : '0s'}!important;`
    + 'transition:none!important;caret-color:transparent!important}';
  const add = () => {
    if (document.getElementById('__zeroDiffFreeze')) return true;
    const root = document.head || document.documentElement;
    if (!root) return false;
    const st = document.createElement('style');
    st.id = '__zeroDiffFreeze';
    st.textContent = css;
    root.appendChild(st);
    return true;
  };
  if (!add()) {
    const mo = new MutationObserver(() => { if (add()) mo.disconnect(); });
    mo.observe(document, { childList: true, subtree: true });
  }
  document.addEventListener('DOMContentLoaded', add);
}

const H = await import(join(HERE, '..', 'harness.mjs'));
const { startServer, launch, domSettled, painted, typeOnKeyboard } = H;
const { completePicture, pickTile } = await import(join(HERE, '..', 'specs', 'finale.mjs'));

const need = async (page, fn, msg) => { if (!(await page.evaluate(fn))) throw new Error(msg); };
/// Scroll every scrollable element inside `sel` to its end.
async function scrollEnd(page, sel) {
  const n = await page.evaluate((sel) => {
    let n = 0;
    for (const e of [document.querySelector(sel), ...document.querySelectorAll(sel + ' *')]) {
      if (!e) continue;
      const oy = getComputedStyle(e).overflowY;
      if ((oy === 'auto' || oy === 'scroll') && e.scrollHeight > e.clientHeight + 4) { e.scrollTop = e.scrollHeight; n++; }
    }
    return n;
  }, sel);
  await settle(page);
}

const openHidden = async (page, id) => {
  await page.evaluate((id) => document.getElementById(id).click(), id);
  await domSettled(page, { quietMs: 200 });
};
const shown = (page, sel) => page.waitForSelector(sel, { state: 'attached', timeout: 10000 });
const settle = (page) => domSettled(page, { quietMs: 200 });
const word = (page) => page.evaluate(() => window.__spelltest.currentWord() || '');

async function serveWord(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 8000 });
  await settle(page);
  return word(page);
}
async function check(page) {
  await page.click('#checkBtn');
  await page.waitForFunction(() => document.getElementById('feedback').className.trim().length > 0, null, { timeout: 8000 });
  // A verdict is drawn in two beats: the mark, then (after a timer of several
  // hundred ms) the orb's "tap for next word" label. A 250ms quiet window
  // races that second beat -- it was the one nondeterminism found -- so a
  // verdict waits for a full second of quiet.
  await domSettled(page, { quietMs: 1200, timeout: 8000 });
}
async function settingToggle(page, id) {
  await H.openSettings(page);
  await page.click(`label:has(#${id})`);
  await settle(page);
  await page.click('#setDone').catch(async () => { await page.keyboard.press('Escape'); });
  await settle(page);
}
async function setupSheet(page) {
  await page.click('#setupChip');
  await shown(page, '#setupScrim.show');
  await settle(page);
}

const bigText = async (p) => {
  await settingToggle(p, 'bigTextToggle');
  await need(p, () => document.body.classList.contains('big-text'), 'Big text toggle did not set body.big-text');
};
const readable = async (p) => {
  await settingToggle(p, 'readToggle');
  await need(p, () => document.body.classList.contains('readable'), 'Readable toggle did not set body.readable');
};

const ALL_FLAGS = { boardgame: 'on', randospell: 'on', spelluzzle: 'on', translate: 'on', sd_tier_mode: 'on', ghost_racing: 'on' };

// One entry per scene. `vps` defaults to the two main viewports.
// `run(page)` drives the real UI to the state and leaves it there.
const SCENES = [
  { name: 'home', vps: SMALL_VPS, run: async () => {} },
  { name: 'play_typed', vps: SMALL_VPS, run: async (p) => {
    const w = await serveWord(p);
    await typeOnKeyboard(p, w.toLowerCase().slice(0, 3));
    await settle(p);
  } },
  { name: 'result_correct', vps: SMALL_VPS, run: async (p) => {
    const w = await serveWord(p);
    await typeOnKeyboard(p, w.toLowerCase());
    await check(p);
  } },
  { name: 'result_wrong', vps: SMALL_VPS, run: async (p) => {
    await serveWord(p);
    await typeOnKeyboard(p, 'zzzz');
    await check(p);
  } },
  { name: 'drawer', run: async (p) => {
    await p.click('#navBurger');
    await shown(p, '#navDrawer.show');
    await settle(p);
  } },
  { name: 'settings', run: async (p) => { await H.openSettings(p); await shown(p, '#setScrim.show'); } },
  { name: 'setup_sheet', run: setupSheet },
  { name: 'kid_home', age: AGE_KID, run: async (p) => {
    await need(p, () => document.body.classList.contains('kid'), 'agegate kid verdict did not set body.kid');
  } },
  { name: 'kid_play', age: AGE_KID, run: async (p) => {
    const w = await serveWord(p);
    await typeOnKeyboard(p, w.toLowerCase().slice(0, 2));
    await settle(p);
  } },
  { name: 'bigtext_home', run: (p) => bigText(p) },
  { name: 'bigtext_play', run: async (p) => { await bigText(p); await serveWord(p); } },
  { name: 'bigtext_settings', run: async (p) => { await bigText(p); await H.openSettings(p); await shown(p, '#setScrim.show'); } },
  { name: 'readable_home', run: (p) => readable(p) },
  { name: 'readable_play', run: async (p) => { await readable(p); await serveWord(p); } },
  { name: 'readable_settings', run: async (p) => { await readable(p); await H.openSettings(p); await shown(p, '#setScrim.show'); } },
  { name: 'ja_home', lang: 'ja', run: async (p) => {
    await need(p, () => document.documentElement.dataset.lang === 'ja', 'ja did not set :root[data-lang]');
  } },
  { name: 'ja_play', lang: 'ja', run: async (p) => { await serveWord(p); } },
  { name: 'zh_home', lang: 'zh', run: async (p) => {
    await need(p, () => document.documentElement.dataset.lang === 'zh', 'zh did not set :root[data-lang]');
  } },
  { name: 'zh_play', lang: 'zh', run: async (p) => { await serveWord(p); } },
  { name: 'ar_home', locale: 'ar', run: async (p) => {
    await need(p, () => document.documentElement.lang === 'ar', 'ar locale did not apply');
  } },
  { name: 'ar_drawer', locale: 'ar', run: async (p) => {
    await p.click('#navBurger'); await shown(p, '#navDrawer.show'); await settle(p);
  } },
  { name: 'ar_play', locale: 'ar', run: async (p) => { await serveWord(p); } },
  // Arabic as the WORD language too: the RTL key grid and answer box (dom::set_dir).
  { name: 'ar_word_play', lang: 'ar', run: async (p) => {
    await serveWord(p);
    await p.click('#gameKeyboard .kb-key >> nth=0'); await p.click('#gameKeyboard .kb-key >> nth=1');
    await settle(p);
  } },
  { name: 'ar_settings', locale: 'ar', run: async (p) => { await H.openSettings(p); await shown(p, '#setScrim.show'); } },
  { name: 'versus_setup', run: async (p) => { await openHidden(p, 'vsBtn'); await shown(p, '#vsSetupScrim.show'); } },
  { name: 'versus_play', run: async (p) => {
    await openHidden(p, 'vsBtn'); await shown(p, '#vsSetupScrim.show');
    await p.click('#vsStart'); await settle(p);
  } },
  { name: 'climb_board', run: async (p) => { await openHidden(p, 'climbBtn'); await shown(p, '#climbScrim.show'); } },
  { name: 'climb_play', run: async (p) => {
    await p.click('#navBurger'); await shown(p, '#navDrawer.show');
    await p.click('.nav-row[data-mode="climb"]'); await settle(p);
    await serveWord(p);
  } },
  { name: 'daily', run: async (p) => { await openHidden(p, 'dailyBtn'); await serveWord(p); } },
  { name: 'boardgame_setup', run: async (p) => { await openHidden(p, 'bgOpenBtn'); await shown(p, '#bgScreen.show'); } },
  { name: 'boardgame_play', run: async (p) => {
    await openHidden(p, 'bgOpenBtn'); await shown(p, '#bgScreen.show');
    await p.click('[data-bg="mode:pass"]'); await p.click('[data-bg="count:2"]');
    await p.click('#bgStart');
    await p.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
    await settle(p);
  } },
  { name: 'randospell_setup', run: async (p) => { await openHidden(p, 'rsOpenBtn'); await shown(p, '#rsScreen.show'); } },
  { name: 'randospell_play', run: async (p) => {
    await openHidden(p, 'rsOpenBtn'); await shown(p, '#rsScreen.show');
    await p.click('#rsStart'); await shown(p, '#rsPlay:not([hidden])'); await settle(p);
  } },
  { name: 'spelluzzle_setup', run: async (p) => { await openHidden(p, 'szOpenBtn'); await shown(p, '#szScreen.show'); } },
  { name: 'spelluzzle_play', run: async (p) => {
    await openHidden(p, 'szOpenBtn'); await shown(p, '#szScreen.show');
    await p.click('#szStart'); await settle(p);
    if (await p.evaluate(() => !document.getElementById('szHow').hidden)) { await p.click('#szHowOk'); await settle(p); }
  } },
  { name: 'spelldoku', run: async (p) => { await openHidden(p, 'sdOpenBtn'); await shown(p, '#sdScreen.show'); } },
  { name: 'spell_search', run: async (p) => { await openHidden(p, 'wsOpenBtn'); await shown(p, '#wsScreen.show'); } },
  { name: 'spell_cross', run: async (p) => { await openHidden(p, 'xwOpenBtn'); await shown(p, '#xwScreen.show'); } },
  { name: 'practice', run: async (p) => { await openHidden(p, 'practiceOpen'); await shown(p, '#practiceScreen.show'); } },
  { name: 'word_picture', run: async (p) => { await openHidden(p, 'wordPicOpen'); await shown(p, '#wpPicker.show'); } },
  { name: 'def_match', run: async (p) => { await openHidden(p, 'defMatchOpen'); await shown(p, '#defMatch.show'); } },
  { name: 'impostor', run: async (p) => { await openHidden(p, 'impostorOpen'); await shown(p, '#impostor.show'); } },
  { name: 'bee', run: async (p) => { await openHidden(p, 'beeOpen'); await shown(p, '#bee.show'); } },
  { name: 'letter_forge', run: async (p) => { await openHidden(p, 'forgeOpen'); await shown(p, '#forge.show'); } },
  { name: 'word_chains', run: async (p) => { await openHidden(p, 'chainsOpen'); await shown(p, '#chains.show'); } },
  { name: 'translate', run: async (p) => { await openHidden(p, 'trOpenBtn'); await shown(p, '#trScreen.show'); } },
  { name: 'ghost_racing', run: async (p) => { await openHidden(p, 'ghostOpenBtn'); await shown(p, '#ghostScreen.show'); } },
  { name: 'spell_racing', run: async (p) => { await openHidden(p, 'spellRacingOpen'); await shown(p, '#spellRacing.show'); } },
  { name: 'lists', run: async (p) => { await H.openMyWords(p); await shown(p, '#listsScreen.show'); } },
  { name: 'calendar', run: async (p) => { await openHidden(p, 'calOpenBtn'); await shown(p, '#calScrim.show'); } },
  { name: 'reports', run: async (p) => { await openHidden(p, 'repOpenBtn'); await shown(p, '#repScrim.show'); } },
  { name: 'credits', run: async (p) => { await openHidden(p, 'creditsBtn'); await shown(p, '#creditsScrim.show'); } },
  { name: 'front_door', run: async (p) => { await openHidden(p, 'accountBtn'); await shown(p, '#frontDoor.show'); } },
  { name: 'photo_src', run: async (p) => { await openHidden(p, 'photoBtn'); await shown(p, '#photoSrcScrim.show'); } },
  { name: 'spellit_guide', run: async (p) => { await openHidden(p, 'spellItBtn'); await shown(p, '#spellItGuide.show'); } },
  { name: 'age_gate', age: null, run: async (p) => { await shown(p, '#ageScrim.show'); } },
  { name: 'qbd_ja', lang: 'ja', flags: { quickbee_draw_ja: 'on' }, run: async (p) => {
    await openHidden(p, 'qbdOpenBtn'); await shown(p, '#qbdScreen.show');
  } },
  { name: 'qbd_ja_play', lang: 'ja', flags: { quickbee_draw_ja: 'on' }, run: async (p) => {
    await openHidden(p, 'qbdOpenBtn'); await shown(p, '#qbdScreen.show');
    await p.click('#qbdStart'); await settle(p);
  } },

  // ---- in-round surfaces and the scrims reachable from them
  { name: 'hint', run: async (p) => { await serveWord(p); await p.click('#hintBtn'); await settle(p); } },
  { name: 'giveup', run: async (p) => { await serveWord(p); await p.click('#giveupBtn'); await settle(p); } },
  { name: 'kid_result_correct', age: AGE_KID, run: async (p) => {
    const w = await serveWord(p); await typeOnKeyboard(p, w.toLowerCase()); await check(p);
  } },
  { name: 'kid_result_wrong', age: AGE_KID, run: async (p) => {
    await serveWord(p); await typeOnKeyboard(p, 'zzzz'); await check(p);
  } },
  { name: 'ja_result_wrong', lang: 'ja', run: async (p) => {
    await serveWord(p);
    await p.click('#gameKeyboard .kb-key >> nth=0'); await p.click('#gameKeyboard .kb-key >> nth=1');
    await check(p);
  } },
  { name: 'zh_result_wrong', lang: 'zh', run: async (p) => {
    await serveWord(p);
    await p.click('#gameKeyboard .kb-key >> nth=0'); await p.click('#gameKeyboard .kb-key >> nth=1');
    await check(p);
  } },
  { name: 'vs_quit', run: async (p) => {
    await openHidden(p, 'vsBtn'); await shown(p, '#vsSetupScrim.show');
    await p.click('#vsStart'); await settle(p);
    await p.click('#vsExit'); await shown(p, '#vsQuitScrim.show'); await settle(p);
  } },
  { name: 'daily_result', run: async (p) => {
    await openHidden(p, 'dailyBtn');
    for (let i = 0; i < 40; i++) {
      if (await p.evaluate(() => document.getElementById('dailyResScrim').classList.contains('show'))) break;
      const w = await serveWord(p).catch(() => '');
      await typeOnKeyboard(p, w ? 'zzzz' : '');
      await p.click('#checkBtn').catch(() => {});
      await p.waitForFunction((prev) => window.__spelltest.currentWord() !== prev
        || document.getElementById('dailyResScrim').classList.contains('show')
        || /good|bad/.test(document.getElementById('feedback').className), w, { timeout: 8000 }).catch(() => {});
      await settle(p);
      // A miss may hold for a reveal; move on the way a player would.
      if (!(await p.evaluate(() => document.getElementById('dailyResScrim').classList.contains('show')))) {
        await p.click('#orbWrap').catch(() => {});
        await settle(p);
      }
    }
    await shown(p, '#dailyResScrim.show');
  } },
  { name: 'parent_gate', age: AGE_KID, run: async (p) => {
    await H.openSettings(p); await p.click('#kidGrownups'); await shown(p, '#parentScrim.show'); await settle(p);
  } },
  { name: 'import_sheet', run: async (p) => {
    await H.openMyWords(p); await shown(p, '#listsScreen.show');
    await p.click('[data-l-add]'); await shown(p, '#importScrim.show'); await settle(p);
  } },
  { name: 'practice_play', run: async (p) => {
    await openHidden(p, 'practiceOpen'); await shown(p, '#practiceScreen.show');
    await p.click('#prIntro button'); await settle(p);
  } },
  { name: 'word_picture_play', run: async (p) => {
    await openHidden(p, 'wordPicOpen'); await shown(p, '#wpPicker.show');
    await pickTile(p, 'star');
    await shown(p, '#wpPlay.show');
    await p.waitForSelector('#wpHow.show', { timeout: 4000 }).catch(() => {});
    await settle(p);
  } },
  { name: 'word_picture_reveal', run: async (p) => {
    await completePicture(p, 'star');
    await shown(p, '#wpReveal.show');
    await p.click('#wpRevealStage'); await settle(p);
  } },
  { name: 'kid_word_picture_reveal', age: AGE_KID, run: async (p) => {
    await completePicture(p, 'star');
    await shown(p, '#wpReveal.show');
    await p.click('#wpRevealStage'); await settle(p);
  } },

  // ---- the far end of screens that scroll inside themselves (fullPage only
  // reaches the document's own scroll). The other full-screen modes were
  // probed: their overlays do not scroll inside themselves at these sizes.
  { name: 'settings_end', run: async (p) => { await H.openSettings(p); await shown(p, '#setScrim.show'); await scrollEnd(p, '#setScrim'); } },
  { name: 'word_picture_end', run: async (p) => { await openHidden(p, 'wordPicOpen'); await shown(p, '#wpPicker.show'); await scrollEnd(p, '#wpPicker'); } },
];

async function newScenePage(browser, base, scene, vp) {
  const v = VIEWPORTS[vp];
  const ctx = await browser.newContext({
    viewport: v, deviceScaleFactor: 2, isMobile: true, hasTouch: false,
    locale: 'en-US', timezoneId: 'UTC', colorScheme: 'light', reducedMotion: 'no-preference',
    serviceWorkers: 'block',
  });
  const age = scene.age === undefined ? AGE_FULL : scene.age;
  const flags = { ...ALL_FLAGS, ...(scene.flags || {}) };
  await ctx.addInitScript(pageInit, [0x5eed1234, ANIM_AT]);
  await ctx.addInitScript(([age, locale, flags]) => {
    if (age) localStorage.setItem('byear_agegate_v1', age);
    if (locale) localStorage.setItem('spellgame.locale', locale);
    localStorage.setItem('spell_flag_learner_surfaces', 'off');
    localStorage.setItem('spell_flag_learner_select', 'off');
    for (const [k, v] of Object.entries(flags)) localStorage.setItem('spell_flag_' + k, v);
  }, [age, scene.locale || scene.lang || 'en', flags]);
  // Hermetic network: local files, the local /api/check grader, and nothing else.
  await ctx.route('**/*', (r) => {
    const u = r.request().url();
    if (u.includes('/api/speak')) return r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.alloc(0) });
    if (u.includes('/telemetry/')) return r.fulfill({ status: 404, body: '' });
    if (u.startsWith(base) || u.startsWith('data:') || u.startsWith('blob:')) return r.continue();
    return r.fulfill({ status: 404, body: '' });
  });
  const page = await ctx.newPage();
  await page.clock.install({ time: FIXED_TIME });
  await page.clock.setFixedTime(FIXED_TIME);
  await page.emulateMedia({ reducedMotion: 'no-preference', colorScheme: 'light' });
  await page.goto(base, { waitUntil: 'load' });
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
  if (age) {
    await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
    await painted(page);
  } else {
    await page.waitForFunction(() => !!window.__spelltest, null, { timeout: 30000 });
  }
  if (scene.lang && scene.lang !== 'en') {
    await page.click('#setupChip');
    await page.selectOption('#langSel', scene.lang);
    await page.click('#setupDone');
    await settle(page);
  }
  await settle(page);
  return { ctx, page };
}

async function finalSettle(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all([...document.images].map((i) => (i.complete ? null : i.decode().catch(() => {}))));
  });
  await domSettled(page, { quietMs: 250 });
  // Freeze: from here no timer can fire, so nothing moves between the state we
  // reached and the pixels we take.
  await page.clock.pauseAt(FIXED_TIME + 1);
  await page.evaluate(() => document.fonts.ready);
}

async function shoot(browser, base, scene, vp) {
  const { ctx, page } = await newScenePage(browser, base, scene, vp);
  try {
    await scene.run(page);
    await finalSettle(page);
    return await page.screenshot({ animations: 'allow', caret: 'hide', scale: 'device', fullPage: true });
  } finally {
    await ctx.close();
  }
}

async function capture(outDir) {
  const t0 = Date.now();
  mkdirSync(outDir, { recursive: true });
  const only = process.env.ZD_ONLY ? new RegExp(process.env.ZD_ONLY) : null;
  const jobs = [];
  for (const s of SCENES) {
    if (only && !only.test(s.name)) continue;
    for (const vp of s.vps || MAIN_VPS) jobs.push({ s, vp });
  }
  const { server, base } = await startServer(0);
  const browser = await launch();
  const failures = [];
  let i = 0;
  const workers = Number(process.env.ZD_WORKERS || 4);
  const worker = async () => {
    while (i < jobs.length) {
      const { s, vp } = jobs[i++];
      const name = `${s.name}@${vp}.png`;
      try {
        writeFileSync(join(outDir, name), await shoot(browser, base, s, vp));
        console.log(`captured ${name}`);
      } catch (e) {
        failures.push(`${name}: ${e.message.split('\n')[0]}`);
        console.log(`FAILED   ${name}: ${e.message.split('\n')[0]}`);
      }
    }
  };
  await Promise.all(Array.from({ length: workers }, worker));

  // Determinism self-check: the same scene, captured again in a fresh context,
  // must be bit-identical.
  const selfScenes = ['home', 'play_typed', 'result_correct', 'result_wrong', 'randospell_play', 'daily_result']
    .map((n) => SCENES.find((s) => s.name === n)).filter((s) => s && (!only || only.test(s.name)));
  let nondet = 0;
  for (const s of selfScenes) {
    const name = `${s.name}@v375.png`;
    if (!existsSync(join(outDir, name))) continue;
    const again = await shoot(browser, base, s, 'v375');
    const r = diffPng(readFileSync(join(outDir, name)), again);
    if (r.count) {
      nondet++;
      console.log(`determinism: ${name} differs by ${r.count} px on re-capture`);
      mkdirSync(join(outDir, 'nondet'), { recursive: true });
      writeFileSync(join(outDir, 'nondet', name), again);
      if (r.img) writeFileSync(join(outDir, 'nondet', 'diff-' + name), r.img);
    }
  }
  await browser.close();
  server.close();
  console.log(nondet ? `determinism: FAILED (${nondet} scene(s))` : `determinism: OK (${selfScenes.length} scenes re-captured, 0 differing px)`);
  console.log(`${jobs.length - failures.length}/${jobs.length} shots in ${((Date.now() - t0) / 1000).toFixed(1)}s -> ${outDir}`);
  if (failures.length) { console.log('failures:\n  ' + failures.join('\n  ')); }
  process.exit(nondet || failures.length ? 1 : 0);
}

const [cmd, a, b] = process.argv.slice(2);
if (cmd === 'capture' && a) await capture(a);
else if (cmd === 'compare' && a && b) compare(a, b);
else {
  console.error('usage: zero-diff.mjs capture <outDir> | compare <baselineDir> <candidateDir>');
  process.exit(2);
}
