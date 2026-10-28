#!/usr/bin/env node
// WCAG contrast gate for the design tokens (F: color feedback / per-language
// themes). Parses the :root token block plus each [data-lang="xx"] theme block
// from index.html, resolves var() chains, and checks every declared text pair
// ≥ 4.5:1 and every UI-affordance pair ≥ 3:1 — in EVERY theme. Fails CI (exit 1)
// with a readable message naming the failing pair. No prod deps; plain Node.
//
//   node scripts/check-contrast.mjs   (npm run check:contrast)
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const html = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', 'index.html'), 'utf8');

// Pull "--name:value" pairs out of a CSS block body.
function tokensFrom(body) {
  const out = {};
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;}]+)/g)) out[m[1].trim()] = m[2].trim();
  return out;
}
function block(selector) {
  const re = new RegExp(selector.replace(/[[\]"]/g, '\\$&') + '\\s*\\{([^}]*)\\}');
  const m = html.match(re);
  return m ? tokensFrom(m[1]) : {};
}

const base = block(':root');
const themes = {
  default: base,
  ja: { ...base, ...block(':root[data-lang="ja"]') },
  zh: { ...base, ...block(':root[data-lang="zh"]') },
};
// CC-VISUAL-ECLIPSE F9 (AT-21): every theme, standard and Jr (body.kid), on
// top of the base tokens. Eclipse is the base itself.
for (const t of ['eclipse', 'daybreak', 'chalkboard', 'typewriter', 'beestage']) {
  const std = { ...base, ...block(`:root[data-theme="${t}"]`) };
  if (t !== 'eclipse') themes[t] = std;
  themes[`${t}-jr`] = { ...std, ...block(`:root[data-theme="${t}"] body.kid`) };
}
// CC-THEMES-COSMOS: the twelve earned themes, each Sky band and planet,
// standard and Jr. (Text over their art is measured on pixels, AT-T5b.)
for (const t of ['bronze', 'silver', 'gold', 'diamond', 'sky', 'lunar', 'solar', 'aurora', 'beach', 'pinkmoon', 'bloodmoon', 'greatwall']) {
  const variants = t === 'sky' ? ['sunrise', 'day', 'sunset', 'night'].map((b) => [`[data-band="${b}"]`, b])
    : t === 'solar' ? ['mercury', 'venus', 'earth', 'mars', 'jupiter', 'saturn', 'uranus', 'neptune'].map((p) => [`[data-planet="${p}"]`, p])
    : [['', '']];
  for (const [attr, v] of variants) {
    const std = { ...base, ...block(`:root[data-theme="${t}"]`), ...(attr ? block(`:root[data-theme="${t}"]${attr}`) : {}) };
    const name = v ? `${t}/${v}` : t;
    themes[name] = std;
    themes[`${name}-jr`] = { ...std, ...block(`:root[data-theme="${t}"] body.kid`), ...(attr ? block(`:root[data-theme="${t}"]${attr} body.kid`) : {}) };
  }
}

// Resolve var(--x) chains to a concrete color; return null for color-mix/rgba
// (glows/shadows — not checked as text/bg here).
function resolve(map, val, depth = 0) {
  if (!val || depth > 8) return null;
  val = val.trim();
  const v = val.match(/^var\((--[\w-]+)\)$/);
  if (v) return resolve(map, map[v[1]], depth + 1);
  if (/^#([0-9a-f]{6})$/i.test(val)) return val;
  // color-mix(in srgb, A p%, B): the mixed panels the themes rely on.
  const mix = val && val.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+(?:\.\d+)?)%,\s*(.+)\)$/);
  if (mix) {
    const a = resolve(map, mix[1].trim(), depth + 1), b = resolve(map, mix[3].trim(), depth + 1);
    if (!a || !b) return null;
    const w = parseFloat(mix[2]) / 100;
    const ch = (h, i) => parseInt(h.slice(1 + i * 2, 3 + i * 2), 16);
    return '#' + [0, 1, 2].map((i) => Math.round(ch(a, i) * w + ch(b, i) * (1 - w)).toString(16).padStart(2, '0')).join('');
  }
  return null; // rgba() etc. — out of scope for text/bg contrast
}
function lum(hex) {
  const n = parseInt(hex.slice(1), 16);
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((x) => {
    x /= 255;
    return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
function ratio(a, b) {
  const [l1, l2] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (l1 + 0.05) / (l2 + 0.05);
}

// [foreground, background, minRatio, kind]. Text ≥4.5 (AA), UI affordance ≥3.
const PAIRS = [
  ['--ink', '--bg', 4.5, 'text'],
  ['--ink', '--panel', 4.5, 'text'],
  ['--ink', '--panel-2', 4.5, 'text'],
  ['--ink', '--bg2', 4.5, 'text'],
  ['--ink-on-bg', '--bg-color', 4.5, 'text'],
  ['--amber', '--panel', 3.0, 'ui'],       // focus ring / button border (per-theme accent)
  ['--success', '--panel-2', 3.0, 'ui'],   // correct border on the spell box
  ['--warn', '--panel-2', 3.0, 'ui'],      // wrong border on the spell box
  // CC-VISUAL-ECLIPSE F9: what every theme must also carry.
  ['--muted', '--panel', 4.5, 'text'],     // secondary labels
  ['--ink', '--panel-raised', 4.5, 'text'],// the menu, setup bar, answer box
  ['--button-ink', '--button-color', 4.5, 'text'], // Check spelling, Done
];
// AT-24: a language accent may be a text colour only where it passes. If any
// rule colours text with one, every accent must reach 4.5:1 on the panel.
const css = html.replace(/<!-- tokens:begin[\s\S]*?tokens:end -->/, '');
if (/[^-]color:\s*var\(--lang-accent/.test(css)) {
  for (const k of Object.keys(base).filter((n) => n.startsWith('--lang-accent-'))) PAIRS.push([k, '--panel', 4.5, 'text']);
}

let failed = false;
for (const [theme, map] of Object.entries(themes)) {
  for (const [fg, bg, min, kind] of PAIRS) {
    const cf = resolve(map, map[fg]);
    const cb = resolve(map, map[bg]);
    if (!cf || !cb) continue; // unresolvable (rgba/color-mix) — skip
    const r = ratio(cf, cb);
    const ok = r >= min;
    if (!ok) failed = true;
    const tag = ok ? 'ok ' : 'FAIL';
    console.log(`[${tag}] ${theme.padEnd(14)} ${fg}(${cf}) on ${bg}(${cb}) = ${r.toFixed(2)}:1 (${kind} ≥ ${min})`);
  }
}
if (failed) {
  console.error('\ncontrast check FAILED — a token pair is below its WCAG threshold (fix the token, do not weaken the check).');
  process.exit(1);
}
console.log('\n✓ contrast OK — all token pairs meet WCAG in every theme.');
