#!/usr/bin/env node
// CC-THEMES-COSMOS static gate, over design/tokens.json and the sources:
//
//   I-T3 / AT-T6  every Cosmos accent (each band, planet, standard and Jr) is
//                 at least ΔE2000 20 from EVERY verdict colour: success and
//                 caution in both pairs, the feedback-line variants on light and
//                 dark, and the wrong-answer coral (Eric, 2026-10-28: "all
//                 verdict colours").
//   I-T2          verdict colours never change per theme, read per lightness
//                 (Eric, 2026-10-28): a dark look keeps the base pair, a bright
//                 look takes exactly the light pair, and no Jr block touches one.
//   I-T4 (tokens) ink and muted text reach 4.5:1 on every stop of a look's own
//                 ground gradient (the metal sheen rule: the darkest stop is the
//                 floor). The rendered pixels are AT-T5b's job (e2e).
//   I-T1 / AT-T15 no theme id in a StoreKit file, a price string or a Complete
//                 entitlement check: themes are earned, never sold.
//
//   node scripts/themes-cosmos-check.mjs
//   node scripts/themes-cosmos-check.mjs --selftest   (planted failures must fail)
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { flatten } from './tokens-build.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

export const COSMOS = ['bronze', 'silver', 'gold', 'diamond', 'sky', 'lunar', 'solar', 'aurora', 'beach', 'pinkmoon', 'bloodmoon', 'greatwall'];
const BANDS = ['sunrise', 'day', 'sunset', 'night'];
const PLANETS = ['mercury', 'venus', 'earth', 'mars', 'jupiter', 'saturn', 'uranus', 'neptune'];
const VERDICT_KEYS = ['color.success', 'color.caution', 'color.teal', 'color.coral'];

// ---------------------------------------------------------------- colour maths
const hex3 = (h) => { h = h.replace('#', ''); return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)); };
const lin = (c) => { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
export function luminance(h) { const [r, g, b] = hex3(h).map(lin); return 0.2126 * r + 0.7152 * g + 0.0722 * b; }
export function contrast(a, b) { const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); }
function lab(h) {
  const [r, g, b] = hex3(h).map(lin);
  const x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047, y = 0.2126 * r + 0.7152 * g + 0.0722 * b, z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
  const f = (t) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  return [116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))];
}
/** CIEDE2000 (Sharma, Wu and Dalal). */
export function deltaE(h1, h2) {
  const [l1, a1, b1] = lab(h1), [l2, a2, b2] = lab(h2);
  const rad = Math.PI / 180;
  const c1 = Math.hypot(a1, b1), c2 = Math.hypot(a2, b2), cm = (c1 + c2) / 2;
  const g = 0.5 * (1 - Math.sqrt(cm ** 7 / (cm ** 7 + 25 ** 7)));
  const a1p = a1 * (1 + g), a2p = a2 * (1 + g);
  const c1p = Math.hypot(a1p, b1), c2p = Math.hypot(a2p, b2);
  const hp = (b, a) => { const v = Math.atan2(b, a) / rad; return v < 0 ? v + 360 : v; };
  const h1p = hp(b1, a1p), h2p = hp(b2, a2p);
  let dh = h2p - h1p;
  if (c1p * c2p === 0) dh = 0; else if (dh > 180) dh -= 360; else if (dh < -180) dh += 360;
  const dH = 2 * Math.sqrt(c1p * c2p) * Math.sin((dh * rad) / 2);
  const lm = (l1 + l2) / 2, cmp = (c1p + c2p) / 2;
  let hm = h1p + h2p;
  if (c1p * c2p !== 0) { if (Math.abs(h1p - h2p) > 180) hm += hm < 360 ? 360 : -360; hm /= 2; }
  const t = 1 - 0.17 * Math.cos((hm - 30) * rad) + 0.24 * Math.cos(2 * hm * rad) + 0.32 * Math.cos((3 * hm + 6) * rad) - 0.2 * Math.cos((4 * hm - 63) * rad);
  const dt = 30 * Math.exp(-(((hm - 275) / 25) ** 2));
  const rc = 2 * Math.sqrt(cmp ** 7 / (cmp ** 7 + 25 ** 7));
  const sl = 1 + (0.015 * (lm - 50) ** 2) / Math.sqrt(20 + (lm - 50) ** 2), sc = 1 + 0.045 * cmp, sh = 1 + 0.015 * cmp * t;
  const rt = -Math.sin(2 * dt * rad) * rc;
  const dl = l2 - l1, dc = c2p - c1p;
  return Math.sqrt((dl / sl) ** 2 + (dc / sc) ** 2 + (dH / sh) ** 2 + rt * (dc / sc) * (dH / sh));
}

// ---------------------------------------------------------------- looks
/** Every Cosmos look: [name, effective path->value map], standard and Jr. */
export function looks(tokens) {
  const base = Object.fromEntries(flatten(tokens).map((l) => [l.path, l.value]));
  const ov = tokens.$overrides || {};
  const out = [];
  for (const t of COSMOS) {
    const variants = t === 'sky' ? BANDS.map((b) => [`[data-band="${b}"]`, b]) : t === 'solar' ? PLANETS.map((p) => [`[data-planet="${p}"]`, p]) : [['', null]];
    for (const [attr, v] of variants) {
      const std = { ...base, ...(ov[`:root[data-theme="${t}"]`] || {}), ...(ov[`:root[data-theme="${t}"]${attr}`] || {}) };
      const jrSel = [`:root[data-theme="${t}"] body.kid`, `:root[data-theme="${t}"]${attr} body.kid`];
      const jr = { ...std, ...Object.assign({}, ...jrSel.map((s) => ov[s] || {})) };
      const name = v ? `${t}/${v}` : t;
      out.push([name, std, jrSel], [`${name} (Jr)`, jr, jrSel]);
    }
  }
  return { base, out, ov };
}

/** Follow var(--x) to a hex through the effective map (by CSS name). */
function resolver(tokens, map) {
  const byCss = new Map(flatten(tokens).map((l) => [l.css, l.path]));
  const res = (v, d = 0) => {
    if (!v || d > 8) return null;
    v = v.trim();
    const m = v.match(/^var\((--[\w-]+)\)$/);
    if (m) return res(map[byCss.get(m[1])], d + 1);
    const mx = v.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+(?:\.\d+)?)%,\s*(.+)\)$/);
    if (mx) {
      const a = res(mx[1], d + 1), b = res(mx[3], d + 1), p = Number(mx[2]) / 100;
      if (!a || !b) return null;
      const ch = (c, i) => parseInt(c.slice(1 + 2 * i, 3 + 2 * i), 16);
      return '#' + [0, 1, 2].map((i) => Math.round(ch(a, i) * p + ch(b, i) * (1 - p)).toString(16).padStart(2, '0')).join('');
    }
    return /^#[0-9a-f]{6}$/i.test(v) ? v.toLowerCase() : null;
  };
  return res;
}

export function check(tokens, files = sourceFiles()) {
  const problems = [];
  const { base, out, ov } = looks(tokens);
  const darkPair = VERDICT_KEYS.map((k) => base[k].toLowerCase());
  const lightPair = ov[':root[data-theme="daybreak"]'] ? VERDICT_KEYS.map((k) => ov[':root[data-theme="daybreak"]'][k].toLowerCase()) : null;
  // Every verdict colour a player can see, in any theme.
  const verdicts = [...new Set([...darkPair, ...(lightPair || []),
    ...['color.good-on-light', 'color.bad-on-light', 'color.warn-on-light', 'color.good-on-dark', 'color.bad-on-dark', 'color.warn-on-dark'].map((k) => base[k].toLowerCase())])];
  for (const [name, map, jrSel] of out) {
    const res = resolver(tokens, map);
    const accent = res(map['color.accent']);
    if (!accent) { problems.push(`${name}: accent does not resolve to a colour`); continue; }
    // I-T3
    for (const v of verdicts) {
      const d = deltaE(accent, v);
      if (d < 20) problems.push(`I-T3 ${name}: accent ${accent} is only ΔE ${d.toFixed(1)} from verdict ${v}`);
    }
    // I-T2, per lightness
    const bg = res(map['color.bg']);
    const pair = VERDICT_KEYS.map((k) => (res(map[k]) || '').toLowerCase());
    const bright = luminance(bg) > 0.18;
    const want = bright ? lightPair : darkPair;
    if (want && pair.join() !== want.join()) problems.push(`I-T2 ${name}: a ${bright ? 'bright' : 'dark'} look must use the ${bright ? 'light' : 'base'} verdict pair, has ${pair.join(' ')}`);
    for (const s of jrSel) for (const k of VERDICT_KEYS) if (ov[s] && k in ov[s]) problems.push(`I-T2 ${s}: a Jr block may not change ${k}`);
    // I-T4 on the look's own ground
    const page = map['theme.page-image'] || 'none';
    const stops = [...page.matchAll(/#[0-9a-f]{6}/gi)].map((m) => m[0]);
    for (const ground of [bg, res(map['color.bg-2']), ...stops].filter(Boolean)) {
      for (const [k, min] of [['color.text', 4.5], ['color.text-muted', 4.5], ['color.text-muted-on-bg', 4.5]]) {
        const fg = res(map[k]);
        const r = contrast(fg, ground);
        if (r < min) problems.push(`I-T4 ${name}: ${k} ${fg} on ground ${ground} is ${r.toFixed(2)}:1 (< ${min})`);
      }
    }
  }
  // I-T1: no theme id where things are sold.
  const ids = [...COSMOS, 'chalkboard', 'typewriter', 'beestage'];
  for (const [file, text] of files) {
    for (const id of ids) {
      if (new RegExp(`["'\`]${id}["'\`]|theme[._-]?${id}\\b`, 'i').test(text)) problems.push(`I-T1 ${file}: theme id "${id}" appears where purchases and entitlements live`);
    }
  }
  return problems;
}

/** I-T11 / I-T10: the Lab's preview (and with it its clock) is read in one
 *  place, themes.rs, which hands the clock only to the look resolver. No other
 *  module may reach into the Lab, so Daily seeding, streaks, days played, the
 *  Climb, the no-repeat history and timers cannot see the override. */
export function labScope(rs) {
  const problems = [];
  for (const [file, text] of rs) {
    if (file.endsWith('theme_lab.rs') || file.endsWith('lib.rs')) continue;
    if (/theme_lab::/.test(text) && !file.endsWith('themes.rs')) problems.push(`I-T11 ${file}: reaches into the Theme Lab (only themes.rs may)`);
    if (file.endsWith('themes.rs')) {
      const uses = [...text.matchAll(/theme_lab::(\w+)/g)].map((m) => m[1]);
      if (uses.some((u) => u !== 'preview')) problems.push(`I-T11 themes.rs: uses theme_lab::${uses.find((u) => u !== 'preview')}`);
    }
  }
  return problems;
}
const rustFiles = () => readdirSync(join(ROOT, 'src')).filter((n) => n.endsWith('.rs')).map((n) => [`src/${n}`, readFileSync(join(ROOT, 'src', n), 'utf8')]);

/** Where things are sold or entitled: StoreKit configs, the entitlement
 *  resolver, the Complete checks, and the store metadata's price lines. */
function sourceFiles() {
  const out = [];
  const walk = (dir, pick) => {
    if (!existsSync(dir)) return;
    for (const n of readdirSync(dir)) {
      if (n === 'node_modules' || n === 'build' || n === 'Pods' || n.startsWith('.')) continue;
      const p = join(dir, n);
      if (statSync(p).isDirectory()) walk(p, pick);
      else if (pick(p)) out.push([p.slice(ROOT.length + 1), readFileSync(p, 'utf8')]);
    }
  };
  walk(join(ROOT, 'ios'), (p) => /\.storekit$/.test(p) || (/\.swift$/.test(p) && /StoreKit|Product\.products|SKProduct/.test(readFileSync(p, 'utf8'))));
  walk(join(ROOT, 'src'), (p) => /entitlement|purchase|paywall|store_kit|iap/i.test(p));
  walk(join(ROOT, 'fastlane', 'metadata'), (p) => /price|promotional|subscription/i.test(p));
  return out;
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const tokens = JSON.parse(readFileSync(join(ROOT, 'design/tokens.json'), 'utf8'));
  if (process.argv.includes('--selftest')) {
    const clone = () => JSON.parse(JSON.stringify(tokens));
    const caution = flatten(tokens).find((l) => l.path === 'color.caution').value;
    const cases = [
      ['the real tokens pass', () => check(tokens).length === 0],
      ['an accent equal to caution fails (AT-T6)', () => { const t = clone(); t.$overrides[':root[data-theme="gold"]']['color.accent'] = caution; return check(t, []).some((p) => p.startsWith('I-T3 gold')); }],
      ['a planet accent near success fails', () => { const t = clone(); t.$overrides[':root[data-theme="solar"][data-planet="uranus"]']['color.accent'] = '#8ed6d8'; return check(t, []).some((p) => p.startsWith('I-T3 solar/uranus')); }],
      ['a bright look on the dark pair fails (I-T2)', () => { const t = clone(); for (const k of VERDICT_KEYS) delete t.$overrides[':root[data-theme="beach"]'][k]; return check(t, []).some((p) => p.startsWith('I-T2 beach')); }],
      ['a Jr block changing a verdict fails', () => { const t = clone(); t.$overrides[':root[data-theme="lunar"] body.kid']['color.success'] = '#00ff00'; return check(t, []).some((p) => p.includes('Jr block')); }],
      ['muted-on-bg too faint on the sand fails (I-T4)', () => { const t = clone(); t.$overrides[':root[data-theme="beach"]']['color.text-muted-on-bg'] = 'color-mix(in srgb, var(--ink-on-bg) 62%, var(--bg-color))'; return check(t, []).some((p) => p.startsWith('I-T4 beach') && p.includes('text-muted-on-bg')); }],
      ['muted text on a too-dark sheen stop fails (I-T4)', () => { const t = clone(); t.$overrides[':root[data-theme="bronze"]']['theme.page-image'] = 'linear-gradient(115deg,#3a2414 0%,#e2ac80 50%)'; return check(t, []).some((p) => p.startsWith('I-T4 bronze')); }],
      ['a theme id in a StoreKit file fails (I-T1)', () => check(tokens, [['ios/Products.storekit', '{"productID":"net.spellgame.theme.gold"}']]).some((p) => p.startsWith('I-T1'))],
      ['a theme id in an entitlement check fails', () => check(tokens, [['src/entitlements.rs', 'if id == "diamond" { grant() }']]).some((p) => p.startsWith('I-T1'))],
      ['the real sources keep the Lab in themes.rs', () => labScope(rustFiles()).length === 0],
      ['a Daily reading the Lab clock fails (I-T11)', () => labScope([['src/daily.rs', 'let now = crate::theme_lab::preview();']]).length === 1],
    ];
    let bad = 0;
    for (const [n, f] of cases) { const ok = f(); if (!ok) bad++; console.log(`  ${ok ? 'ok  ' : 'FAIL'}   ${n}`); }
    console.log(bad ? `themes-cosmos-check selftest: ${bad} FAILED` : 'themes-cosmos-check selftest: OK');
    process.exit(bad ? 1 : 0);
  }
  const problems = [...check(tokens), ...labScope(rustFiles())];
  for (const p of problems) console.error(`  ✗ ${p}`);
  if (problems.length) { console.error(`themes-cosmos-check: FAILED (${problems.length})`); process.exit(1); }
  console.log(`themes-cosmos-check: OK, ${looks(tokens).out.length} looks: accents ΔE >= 20 from every verdict, verdicts per lightness, text on every ground stop, no theme id where things are sold.`);
}
