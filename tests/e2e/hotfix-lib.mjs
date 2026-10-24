// CC-ECLIPSE-HOTFIX: shared helpers for the contrast, overlap, glow and diag
// suites. Every screen the suites visit comes from the registry the app itself
// uses (config/modes.json, through the drawer's own rows), plus the chrome
// screens that are not modes, so a screen added later is covered without
// anyone remembering to list it.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openApp, domSettled } from './harness.mjs';

const ROOT = join(fileURLToPath(import.meta.url), '..', '..', '..');

/** The two themes the hotfix is about: Light is Daybreak (F9), dark is Eclipse. */
export const THEMES = ['eclipse', 'daybreak'];

/** Every flag a shipped build (298) turns on, so the suites see what players see. */
export const SHIPPED_FLAGS = [
  'visual_type', 'visual_feedback', 'visual_orb', 'visual_sound', 'visual_haptics',
  'visual_icons', 'visual_celebrate', 'visual_cards', 'visual_themes', 'visual_accents',
  'keyboard_core',
];

/** The registry rows the drawer can open in this build. */
export function registryModes() {
  const reg = JSON.parse(readFileSync(join(ROOT, 'config/modes.json'), 'utf8'));
  return reg.modes.filter((m) => m.status !== 'hidden').map((m) => m.id);
}

/** A player who has been playing a while, the way a device looks after a few
 *  days: learner data (the Learning report's Strong skills and Focus lists),
 *  accuracy stats, misses, and a share sheet (iOS has one; headless Chromium
 *  does not, so the Share your chain pill would never render here). */
export const LIVED_IN = `(() => {
  const day = Math.floor(Date.now() / 86400000), now = Date.now();
  localStorage.setItem('spell_learner_en', JSON.stringify({
    version: 1, lang: 'en', placed: true,
    skills: [
      { id: 'silent_letters', mastery: 0.3, fsrs: { stability: 2, difficulty: 5.2, due_day: day - 9, reps: 6, lapses: 2 } },
      { id: 'doubled_consonant', mastery: 0.9, fsrs: { stability: 30, difficulty: 4.0, due_day: day + 20, reps: 5, lapses: 0 } },
    ],
    log: Array.from({ length: 36 }, (_, i) => ({ day: day - 1, word: 'rabbit', skills: ['doubled_consonant'], correct: i % 3 !== 0, channel: 'typed', typed: i % 3 ? null : 'rabit' })),
  }));
  localStorage.setItem('byear_stats_v1', JSON.stringify({ en: { easy: { seen: 12, correct: 10 }, medium: { seen: 6, correct: 2 } } }));
  localStorage.setItem('byear_misses_v1', JSON.stringify([{ word: 'rhythm', lang: 'en', tier: 'medium', misses: 2, box_: 1, due: now - 60000, ts: now - 60000 }]));
  localStorage.setItem('spell_flag_learner_surfaces', 'on');
  try { Object.defineProperty(navigator, 'share', { configurable: true, value: () => Promise.resolve() }); } catch (e) {}
})();`;

/** The pre-app script: the shipped flags, the chosen theme, test entitlements. */
export function initFor(theme, extra = {}) {
  const seed = { flags: SHIPPED_FLAGS, theme, extra };
  return `(${(({ flags, theme, extra }) => {
    for (const f of flags) localStorage.setItem(`spell_flag_${f}`, 'on');
    localStorage.setItem('spell_themes_v1', JSON.stringify({ unlocked: [], choice: theme, baseline: null, correct_since: 0, hard_since: 0 }));
    localStorage.setItem('spell_dev_entitlements', 'on');
    for (const [k, v] of Object.entries(extra)) localStorage.setItem(k, v);
    // The iOS-only modes tile only on a native platform. A Capacitor that says
    // "native, iOS" but has no plugins: the platform gate opens and every
    // native call reports itself unavailable, as on a device without a model.
    if (!window.Capacitor) window.Capacitor = { isNativePlatform: () => true, getPlatform: () => 'ios', Plugins: {} };
  }).toString()})(${JSON.stringify(seed)});`;
}

/** A page on `theme` with the shipped flags (and, with `lived`, a played-in device). */
export async function openThemed(browser, base, theme, { device = 'se', viewport = null, extra = {}, lived = false } = {}) {
  return openApp(browser, base, { lang: 'en', device, viewport, init: initFor(theme, extra) + (lived ? LIVED_IN : '') });
}

/** The drawer rows present for this player, by data-mode. */
export async function drawerRows(page) {
  await page.click('#navBurger');
  await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
  const rows = await page.$$eval('.nav-row[data-mode]', (rs) => rs.filter((r) => r.getClientRects().length).map((r) => r.dataset.mode));
  await page.keyboard.press('Escape').catch(() => {});
  await page.evaluate(() => document.getElementById('navDrawer').classList.remove('show'));
  return rows;
}

/** Open a drawer row, the way a player does. */
export async function openRow(page, mode) {
  await page.click('#navBurger');
  await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
  await page.evaluate((m) => document.querySelector(`.nav-row[data-mode="${m}"]`).click(), mode);
  await domSettled(page);
}

/** The Background picker colours each theme is checked against: the theme's own
 *  (no pick), plus the picker swatch furthest from it. A pick changes only the
 *  page and the ink ON the page, so it is the case where a surface that
 *  inherits its text colour from the page goes white-on-white (F1). */
export const BACKGROUNDS = { eclipse: [null, '#f4f1fb'], daybreak: [null, '#181818'] };

/** Pick a Background colour through the Settings control, as a player does. */
export async function pickBackground(page, color) {
  if (!color) return;
  await page.evaluate((c) => {
    const i = document.getElementById('bgColorInput');
    i.value = c;
    i.dispatchEvent(new Event('input', { bubbles: true }));
    i.dispatchEvent(new Event('change', { bubbles: true }));
  }, color);
  await domSettled(page);
}

// ---------------------------------------------------------------- pixels
import { inflateSync } from 'node:zlib';

/** Decode the 8-bit RGBA, non-interlaced PNGs Playwright writes. */
export function decodePng(buf) {
  let p = 8, w = 0, h = 0, ct = 6;
  const idat = [];
  while (p < buf.length) {
    const len = buf.readUInt32BE(p); const type = buf.toString('ascii', p + 4, p + 8);
    const data = buf.subarray(p + 8, p + 8 + len);
    if (type === 'IHDR') { w = data.readUInt32BE(0); h = data.readUInt32BE(4); ct = data[9]; }
    if (type === 'IDAT') idat.push(data);
    if (type === 'IEND') break;
    p += 12 + len;
  }
  const bpp = ct === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const out = Buffer.alloc(w * h * 4);
  const stride = w * bpp;
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)];
    const line = Buffer.from(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? line[x - bpp] : 0, b = prev[x], c = x >= bpp ? prev[x - bpp] : 0;
      let v = line[x];
      if (f === 1) v += a; else if (f === 2) v += b; else if (f === 3) v += (a + b) >> 1;
      else if (f === 4) { const pp = a + b - c, pa = Math.abs(pp - a), pb = Math.abs(pp - b), pc = Math.abs(pp - c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      line[x] = v & 255;
    }
    for (let x = 0; x < w; x++) {
      const o = (y * w + x) * 4;
      out[o] = line[x * bpp]; out[o + 1] = line[x * bpp + 1]; out[o + 2] = line[x * bpp + 2]; out[o + 3] = bpp === 4 ? line[x * bpp + 3] : 255;
    }
    prev = line;
  }
  return { w, h, px: out };
}

const lin = (x) => { x /= 255; return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; };
export const luminance = ([r, g, b]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
export const ratio = (a, b) => { const [x, y] = [luminance(a), luminance(b)].sort((m, n) => n - m); return (x + 0.05) / (y + 0.05); };

/** WCAG contrast of `fg` against what is actually painted behind it.
 *  `pixels` is the region with the foreground hidden; the result is the
 *  contrast at the 2nd-worst percentile, so a stray antialiased pixel at a
 *  gradient's edge does not decide it but a real low-contrast band does. */
export function worstContrast(fg, pixels) {
  const rs = [];
  for (let i = 0; i < pixels.length; i += 4) {
    const a = fg[3] ?? 1;
    const bg = [pixels[i], pixels[i + 1], pixels[i + 2]];
    const f = [0, 1, 2].map((k) => fg[k] * a + bg[k] * (1 - a));
    rs.push(ratio(f, bg));
  }
  if (!rs.length) return Infinity;
  rs.sort((m, n) => m - n);
  return rs[Math.floor(rs.length * 0.02)];
}

const HIDE_CSS = `[data-hf-hide],[data-hf-hide] *{color:transparent!important;-webkit-text-fill-color:transparent!important;text-shadow:none!important;transition:none!important;caret-color:transparent!important}
[data-hf-hide] svg,[data-hf-hide] img{visibility:hidden!important}`;

/** Parse a computed colour (rgb(), rgba() or color(srgb ...)) to [r,g,b,a] 0..255. */
export const PARSE_COLOUR = `(c) => {
  if (!c || c === 'transparent') return null;
  const m = c.match(/[\\d.]+/g); if (!m) return null;
  const v = m.map(Number);
  if (c.startsWith('color(')) return [v[0] * 255, v[1] * 255, v[2] * 255, v[3] ?? 1];
  return [v[0], v[1], v[2], v[3] ?? 1];
}`;

async function shotOf(page, sel, pad = 0) {
  const box = await page.evaluate(([sel, pad]) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    el.scrollIntoView({ block: 'center', inline: 'nearest' });
    const r = el.getBoundingClientRect();
    // Only what the player can see: clip to the viewport and to every
    // ancestor that clips its overflow (a scrolling strip, a card).
    let L = r.left - pad, T = r.top - pad, R = r.right + pad, B = r.bottom + pad;
    // html and body never clip a fixed screen, and nothing above a fixed
    // ancestor clips what is inside it.
    for (let a = el.parentElement; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
      const o = getComputedStyle(a);
      if (o.overflowX !== 'visible' || o.overflowY !== 'visible') {
        const q = a.getBoundingClientRect();
        L = Math.max(L, q.left); T = Math.max(T, q.top); R = Math.min(R, q.right); B = Math.min(B, q.bottom);
      }
      if (o.position === 'fixed') break;
    }
    const x = Math.max(0, L), y = Math.max(0, T);
    const w = Math.min(innerWidth, R) - x, h = Math.min(innerHeight, B) - y;
    return w >= 2 && h >= 2 ? { x, y, width: w, height: h, inner: { x: r.left - x, y: r.top - y, w: r.width, h: r.height } } : null;
  }, [sel, pad]);
  if (!box) return null;
  const png = await page.screenshot({ clip: { x: box.x, y: box.y, width: box.width, height: box.height }, scale: 'css', animations: 'disabled' });
  return { ...decodePng(png), inner: box.inner, origin: { x: box.x, y: box.y } };
}

/** The axe nodes it could not decide, measured against the pixels behind them.
 *  Returns failures as strings. `skipped` collects what was not measurable
 *  and why (emoji are full-colour pictures; text under an open modal is not
 *  what the player is reading). */
export async function residualContrast(page, incomplete, skipped = []) {
  const bad = [];
  await page.evaluate((css) => { if (!document.getElementById('hfHide')) { const s = document.createElement('style'); s.id = 'hfHide'; s.textContent = css; document.head.append(s); } }, HIDE_CSS);
  for (const v of incomplete.filter((x) => x.id === 'color-contrast')) {
    for (const n of v.nodes) {
      const sel = n.target[n.target.length - 1];
      const key = (n.any[0] || {}).data?.messageKey || 'unknown';
      const info = await page.evaluate(([sel, key, parse]) => {
        const P = eval(parse);
        const el = document.querySelector(sel);
        if (!el || !el.getClientRects().length) return { skip: 'not rendered' };
        const cs = getComputedStyle(el);
        if (cs.visibility === 'hidden' || Number(cs.opacity) === 0) return { skip: 'invisible' };
        // WCAG exempts inactive controls: disabled, aria-disabled, inert, or the
        // play keyboard while it is locked between words. Only those.
        if (el.closest(':disabled, [aria-disabled="true"], [inert], .game-kb.locked')) return { skip: 'inactive' };
        el.scrollIntoView({ block: 'center', inline: 'nearest' });
        const r = el.getBoundingClientRect();
        const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        if (top && top !== el && !el.contains(top) && !top.contains(el)) return { skip: 'covered by ' + (top.id || top.className || top.tagName) };
        const svg = el.querySelector('svg');
        let icon = false, fg = P(cs.color);
        if (key === 'nonBmp') {
          if (!svg) return { skip: 'picture glyph (emoji)' };
          icon = true; fg = P(getComputedStyle(svg).stroke !== 'none' ? getComputedStyle(svg).stroke : getComputedStyle(svg).color);
        }
        if (!fg || fg[3] === 0) return { skip: 'no ink' };
        const size = parseFloat(cs.fontSize), weight = Number(cs.fontWeight) || 400;
        const large = size >= 24 || (size >= 18.66 && weight >= 700);
        // Sample only under the ink being judged: the element's own text runs
        // (not a child drawn in another colour, like a Bee rival's avatar), or
        // the icon's own box.
        let rects = [];
        if (icon) rects = [svg.getBoundingClientRect()];
        else {
          const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
          for (let t = w.nextNode(); t; t = w.nextNode()) {
            if (!t.textContent.trim() || getComputedStyle(t.parentElement).color !== cs.color) continue;
            const rg = document.createRange(); rg.selectNodeContents(t);
            rects.push(...rg.getClientRects());
          }
          if (!rects.length) rects = [r];
        }
        el.setAttribute('data-hf-hide', '');
        return { fg, need: icon ? 3 : large ? 3 : 4.5, rects: rects.map((q) => ({ l: q.left, t: q.top, r: q.right, b: q.bottom })) };
      }, [sel, key, PARSE_COLOUR]);
      if (info.skip) { skipped.push(`${sel} (${key}): ${info.skip}`); continue; }
      const shot = await shotOf(page, sel, 0);
      await page.evaluate((sel) => document.querySelector(sel)?.removeAttribute('data-hf-hide'), sel);
      if (!shot) { skipped.push(`${sel}: off screen`); continue; }
      // Keep the pixels inside the text runs (the shot is the element's visible box).
      const keep = [];
      const ox = shot.origin.x, oy = shot.origin.y;
      for (let y = 0; y < shot.h; y++) for (let x = 0; x < shot.w; x++) {
        const vx = x + ox + 0.5, vy = y + oy + 0.5;
        if (info.rects.some((q) => vx >= q.l && vx < q.r && vy >= q.t && vy < q.b)) {
          const o = (y * shot.w + x) * 4; keep.push(shot.px[o], shot.px[o + 1], shot.px[o + 2], 255);
        }
      }
      if (!keep.length) { skipped.push(`${sel}: its text is clipped out of view`); continue; }
      const r = worstContrast(info.fg, keep);
      if (r + 1e-6 < info.need) bad.push(`${sel} ${r.toFixed(2)}:1 < ${info.need}:1 (${key}, ink rgb(${info.fg.slice(0, 3).map(Math.round)}))`);
    }
  }
  return bad;
}

/** I2: what identifies a control must reach 3:1 against what surrounds it.
 *  Fields (their boundary is the control) and unselected option chips (their
 *  outline is all that says "choosable") are checked; a text-only button is
 *  identified by its text, which the text rule already covers. */
export async function controlBoundaries(page) {
  const sels = await page.evaluate(() => {
    const out = [];
    let i = 0;
    const tag = (el) => { const id = `hf${i++}`; el.setAttribute('data-hf-ctl', id); return `[data-hf-ctl="${id}"]`; };
    // Rendered, and on top: a control under an open screen or modal is not one
    // the player can see (the home glow row sits under every mode screen).
    const vis = (el) => {
      if (!el.getClientRects().length || getComputedStyle(el).visibility === 'hidden') return false;
      el.scrollIntoView({ block: 'center', inline: 'nearest' });
      const r = el.getBoundingClientRect();
      const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return !!top && (top === el || el.contains(top) || top.contains(el));
    };
    for (const el of document.querySelectorAll('input:not([type=hidden]):not([type=range]):not([type=color]):not([type=checkbox]):not([type=radio]):not([type=file]), select, textarea')) {
      if (vis(el)) out.push({ sel: tag(el), what: 'field ' + (el.id || el.name || el.type) });
    }
    const ON = (e) => e.classList.contains('on') || e.classList.contains('active') || e.getAttribute('aria-pressed') === 'true';
    for (const el of document.querySelectorAll('button, [role=button]')) {
      if (!vis(el) || ON(el) || !el.parentElement || el.closest(':disabled, [aria-disabled="true"], [inert]')) continue;
      const cls = [...el.classList][0];
      if (!cls) continue;
      const sibs = [...el.parentElement.children].filter((s) => s !== el && s.classList.contains(cls));
      if (sibs.some(ON)) out.push({ sel: tag(el), what: `unselected .${cls} "${el.textContent.trim().slice(0, 20)}"` });
    }
    return out;
  });
  const bad = [];
  for (const { sel, what } of sels) {
    const info = await page.evaluate(([sel, parse]) => {
      const P = eval(parse);
      const el = document.querySelector(sel); const cs = getComputedStyle(el);
      const bw = parseFloat(cs.borderTopWidth) || 0;
      const inset = (cs.boxShadow.match(/inset[^,]*?(rgba?\([^)]*\)|color\([^)]*\))|(rgba?\([^)]*\)|color\([^)]*\))[^,]*inset/) || []);
      return { border: bw >= 1 && cs.borderTopStyle !== 'none' ? P(cs.borderTopColor) : null, inset: inset[1] || inset[2] ? P(inset[1] || inset[2]) : null, bg: P(cs.backgroundColor), opacity: Number(cs.opacity) };
    }, [sel, PARSE_COLOUR]);
    const shot = await shotOf(page, sel, 4);
    if (!shot) { bad.push(`${what}: could not be measured (no visible box)`); continue; }
    // The surround: the ring of pixels just outside the control.
    const ring = [];
    const { w, h, px, inner } = shot;
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const inside = x >= inner.x - 1 && x < inner.x + inner.w + 1 && y >= inner.y - 1 && y < inner.y + inner.h + 1;
      if (!inside) { const o = (y * w + x) * 4; ring.push([px[o], px[o + 1], px[o + 2]]); }
    }
    if (!ring.length) continue;
    ring.sort((a, b) => luminance(a) - luminance(b));
    const around = ring[Math.floor(ring.length / 2)];
    const over = (c) => c && [0, 1, 2].map((k) => c[k] * c[3] + around[k] * (1 - c[3]));
    const cands = [over(info.border), over(info.inset), over(info.bg)].filter(Boolean).map((c) => ratio(c, around));
    const best = Math.max(0, ...cands);
    if (info.opacity < 1) bad.push(`${what}: opacity ${info.opacity} (an unselected state must not be opacity-only)`);
    if (best + 1e-6 < 3) bad.push(`${what}: boundary ${best.toFixed(2)}:1 < 3:1`);
  }
  return bad;
}

/** Settings > Big text, through its own toggle (the XL text size). */
export async function setBigText(page, on) {
  await page.evaluate((on) => {
    const t = document.getElementById('bigTextToggle');
    if (t.checked === on) return;
    t.checked = on;
    t.dispatchEvent(new Event('change', { bubbles: true }));
  }, on);
  await domSettled(page);
}

/** I4: visible text and interactive elements whose boxes intersect.
 *  Only the layer the player is looking at counts (an open mode screen hides
 *  the home screen under it), and an element never collides with its own
 *  ancestors or descendants. Returns "a × b" strings. */
export async function overlaps(page) {
  return page.evaluate(() => {
    const vw = innerWidth, vh = innerHeight;
    // The layer: the outermost fixed ancestor of what sits at the screen's
    // centre and upper third; the document when nothing fixed covers it.
    const layerOf = (x, y) => {
      let e = document.elementFromPoint(x, y), root = null;
      for (; e && e !== document.body; e = e.parentElement) {
        const c = getComputedStyle(e), r = e.getBoundingClientRect();
        if (c.position === 'fixed' && r.width >= vw * 0.8 && r.height >= vh * 0.6) root = e;
      }
      return root;
    };
    const layer = layerOf(vw / 2, vh / 2) || layerOf(vw / 2, vh / 3) || document.body;
    const ownText = (el) => [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    const INTERACTIVE = 'button, a[href], input:not([type=hidden]), select, textarea, [role=button], [role=switch], [role=tab]';
    const visible = (el) => {
      const c = getComputedStyle(el);
      if (c.visibility === 'hidden' || Number(c.opacity) === 0 || c.display === 'contents') return false;
      for (let a = el; a && a !== layer; a = a.parentElement) {
        const ac = getComputedStyle(a);
        if (Number(ac.opacity) === 0 || ac.visibility === 'hidden') return false;
        if (a !== layer && a.parentElement && getComputedStyle(a).position === 'fixed' && layer === document.body) {
          // A fixed overlay that is not the layer: on the home layer, only count
          // it if it is actually showing on top somewhere.
        }
      }
      const r = el.getBoundingClientRect();
      if (r.width < 2 || r.height < 2) return false;
      // Under a backdrop inside the layer (a card's scrim): not what is seen.
      // Covered by another control or another piece of text is exactly an
      // overlap, so only a covering element that is neither counts.
      const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
      if (cx >= 0 && cx < vw && cy >= 0 && cy < vh) {
        const hit = document.elementFromPoint(cx, cy);
        if (hit && hit !== el && !el.contains(hit) && !hit.contains(el)) {
          // Inside an overlay that spans the layer (a card on its backdrop):
          // covered. Otherwise the covering thing is a neighbour: an overlap.
          const lr = layer.getBoundingClientRect();
          for (let t = hit; t && t !== layer; t = t.parentElement) {
            if (t.contains(el)) break;
            const tc = getComputedStyle(t), tr = t.getBoundingClientRect();
            if (/(absolute|fixed)/.test(tc.position) && tr.width >= lr.width * 0.8 && tr.height >= Math.min(lr.height, vh) * 0.6) return false;
          }
          let t = hit, content = false;
          for (; t && !content; t = t.parentElement) { if (t.matches(INTERACTIVE) || ownText(t)) content = true; if (t === layer) break; }
          if (!content) return false;
        }
      }
      return true;
    };
    const items = [];
    for (const el of layer.querySelectorAll('*')) {
      if (el.closest('svg') && el.tagName.toLowerCase() !== 'svg') continue;
      if (el.closest('[aria-hidden="true"]') && !el.matches(INTERACTIVE)) continue;
      if (!(el.matches(INTERACTIVE) || ownText(el))) continue;
      if (!visible(el)) continue;
      // Hidden by an ancestor's clipping (a scrolled strip): only its visible part counts.
      let r = el.getBoundingClientRect();
      let L = r.left, T = r.top, R = r.right, B = r.bottom;
      for (let a = el.parentElement; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
        const o = getComputedStyle(a);
        if (o.overflowX !== 'visible' || o.overflowY !== 'visible') {
          const q = a.getBoundingClientRect();
          L = Math.max(L, q.left); T = Math.max(T, q.top); R = Math.min(R, q.right); B = Math.min(B, q.bottom);
        }
        if (o.position === 'fixed') break;
      }
      if (R - L < 2 || B - T < 2) continue;
      // An inline run that wraps is several line boxes, not their union.
      const frags = getComputedStyle(el).display === 'inline'
        ? [...el.getClientRects()].map((q) => ({ L: Math.max(L, q.left), T: Math.max(T, q.top), R: Math.min(R, q.right), B: Math.min(B, q.bottom) }))
        : [{ L, T, R, B }];
      items.push({ el, frags });
    }
    const name = (e) => {
      const t = (e.textContent || e.getAttribute('aria-label') || '').trim().replace(/\s+/g, ' ').slice(0, 24);
      return `${e.tagName.toLowerCase()}${e.id ? '#' + e.id : ''}${e.classList.length ? '.' + [...e.classList].slice(0, 2).join('.') : ''} "${t}"`;
    };
    const out = [];
    // A control outside the viewport must be reachable by scrolling something.
    const scrolls = (e) => { const c = getComputedStyle(e); return /(auto|scroll)/.test(c.overflowY) && e.scrollHeight > e.clientHeight + 1; };
    for (const { el, frags } of items) {
      if (!el.matches(INTERACTIVE)) continue;
      const f = frags[0];
      if (f.B <= vh + 1 && f.T >= -1) continue;
      let ok = layer === document.body && document.documentElement.scrollHeight > vh + 1;
      for (let a = el.parentElement; !ok && a; a = a.parentElement) { if (scrolls(a)) ok = true; if (a === layer) break; }
      if (!ok) out.push(`${name(el)} is outside the screen and nothing scrolls to it`);
    }
    for (let i = 0; i < items.length; i++) for (let j = i + 1; j < items.length; j++) {
      const a = items[i], b = items[j];
      if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
      let worst = null;
      for (const f of a.frags) for (const g of b.frags) {
        const w = Math.min(f.R, g.R) - Math.max(f.L, g.L), h = Math.min(f.B, g.B) - Math.max(f.T, g.T);
        if (w > 1 && h > 1 && (!worst || w * h > worst[0] * worst[1])) worst = [w, h];
      }
      if (worst) out.push(`${name(a.el)} × ${name(b.el)} (${Math.round(worst[0])}×${Math.round(worst[1])}px)`);
    }
    return out;
  });
}
