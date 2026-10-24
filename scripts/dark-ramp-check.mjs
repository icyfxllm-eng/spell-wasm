#!/usr/bin/env node
// CC-ECLIPSE-HOTFIX I9: Eclipse's dark surfaces are neutral near-black.
//
// Every dark surface token Eclipse paints with (the page, its second
// background, the panels, the edges, the scrim, and the raised panel and
// strong edge that are MIXED from the default Controls colour, in the
// standard and the Kid variant) keeps HSL saturation <= 6%, so none can carry
// a violet hue (240-320 deg with saturation > 6%). A Controls colour the player
// picks still tints the raised panels; that is the player's choice and is
// exempt (S1). The ink is not a surface and is not checked here.
//
//   node scripts/dark-ramp-check.mjs            # check design/tokens.json
//   node scripts/dark-ramp-check.mjs --selftest # prove it bites
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const MAX_SAT = 0.06;

const hex = (h) => { h = h.replace('#', ''); if (h.length === 3) h = [...h].map((c) => c + c).join(''); return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)); };
const rgbaOf = (v) => { const m = v.match(/rgba?\(([^)]*)\)/); return m ? m[1].split(',').slice(0, 3).map(Number) : null; };
const parse = (v) => (v.trim().startsWith('#') ? hex(v.trim()) : rgbaOf(v));
const mix = (a, pa, b) => a.map((x, i) => x * pa + b[i] * (1 - pa));

export function hsl([r, g, b]) {
  r /= 255; g /= 255; b /= 255;
  const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2, d = mx - mn;
  if (d === 0) return { h: 0, s: 0, l };
  const s = d / (1 - Math.abs(2 * l - 1));
  let h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h *= 60; if (h < 0) h += 360;
  return { h, s, l };
}

/** The Eclipse surfaces, resolved to sRGB. */
export function surfaces(tokens) {
  const c = tokens.color;
  const bg = parse(c.bg.$value);
  const mixed = (controls) => ({
    'surface-raised': mix(controls, 0.25, bg),
    'edge-strong': mix(controls, 0.7, bg),
  });
  const controls = parse(c.controls.$value);
  const kid = tokens.$overrides?.[':root[data-theme="eclipse"] body.kid']?.['color.controls'];
  const out = {
    bg, 'bg-2': parse(c['bg-2'].$value), surface: parse(c.surface.$value), 'surface-2': parse(c['surface-2'].$value),
    edge: parse(c.edge.$value), 'bg-user': parse(c['bg-user'].$value), scrim: parse(tokens.chrome.scrim.$value),
    ...mixed(controls),
  };
  if (kid) for (const [k, v] of Object.entries(mixed(parse(kid)))) out[`kid ${k}`] = v;
  return out;
}

export function check(tokens) {
  const bad = [];
  for (const [name, rgb] of Object.entries(surfaces(tokens))) {
    if (!rgb) { bad.push(`${name}: not a colour this check can read`); continue; }
    const { h, s } = hsl(rgb);
    const violet = h >= 240 && h <= 320 && s > MAX_SAT;
    if (s > MAX_SAT + 1e-9) bad.push(`${name}: saturation ${(s * 100).toFixed(1)}% > 6%${violet ? ` (violet, hue ${h.toFixed(0)}°)` : ''}`);
  }
  return bad;
}

if (process.argv.includes('--selftest')) {
  const base = JSON.parse(readFileSync(join(ROOT, 'design/tokens.json'), 'utf8'));
  const clone = () => JSON.parse(JSON.stringify(base));
  const cases = [
    ['the real tokens are clean', check(base).length === 0],
    ['the old violet page is caught', (() => { const t = clone(); t.color.bg.$value = '#14111f'; return check(t).some((b) => /^bg: .*violet/.test(b)); })()],
    ['a violet default Controls colour is caught through the raised panel', (() => { const t = clone(); t.color.controls.$value = '#7a66c2'; return check(t).some((b) => b.startsWith('surface-raised')); })()],
    ['the Kid variant is checked too', (() => { const t = clone(); t.$overrides[':root[data-theme="eclipse"] body.kid']['color.controls'] = '#9a84ec'; return check(t).some((b) => b.startsWith('kid ')); })()],
    ['a violet scrim is caught', (() => { const t = clone(); t.chrome.scrim.$value = 'rgba(10,8,18,.72)'; return check(t).some((b) => b.startsWith('scrim')); })()],
  ];
  let failed = 0;
  for (const [n, ok] of cases) { console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${n}`); if (!ok) failed++; }
  if (failed) { console.error(`dark-ramp-check selftest: ${failed} failed`); process.exit(1); }
  console.log('dark-ramp-check selftest: OK');
  process.exit(0);
}

const bad = check(JSON.parse(readFileSync(join(ROOT, 'design/tokens.json'), 'utf8')));
if (bad.length) {
  console.error('dark-ramp-check: FAILED (I9: Eclipse surfaces are neutral, saturation <= 6%)');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log('dark-ramp-check: OK — every Eclipse dark surface is neutral (saturation <= 6%, no violet).');
