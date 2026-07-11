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

// Resolve var(--x) chains to a concrete color; return null for color-mix/rgba
// (glows/shadows — not checked as text/bg here).
function resolve(map, val, depth = 0) {
  if (!val || depth > 8) return null;
  val = val.trim();
  const v = val.match(/^var\((--[\w-]+)\)$/);
  if (v) return resolve(map, map[v[1]], depth + 1);
  if (/^#([0-9a-f]{6})$/i.test(val)) return val;
  return null; // rgba(), color-mix(), etc. — out of scope for text/bg contrast
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
];

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
    console.log(`[${tag}] ${theme.padEnd(7)} ${fg}(${cf}) on ${bg}(${cb}) = ${r.toFixed(2)}:1 (${kind} ≥ ${min})`);
  }
}
if (failed) {
  console.error('\ncontrast check FAILED — a token pair is below its WCAG threshold (fix the token, do not weaken the check).');
  process.exit(1);
}
console.log('\n✓ contrast OK — all token pairs meet WCAG in every theme.');
