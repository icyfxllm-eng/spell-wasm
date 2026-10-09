#!/usr/bin/env node
// CC-SPELLUZZLE F14: rune glyph uniqueness check.
// Rasterises every <symbol id="rune-N"> in assets/spelluzzle/runes.svg at 64x64
// on white, binarises, and fails if any glyph equals another glyph (or itself)
// under mirror-x, mirror-y, rot90, rot180, rot270.
// Usage: node scripts/spelluzzle-glyph-check.mjs [--selftest]
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const SVG = join(here, '..', 'assets', 'spelluzzle', 'runes.svg');
const N = 64;
const MIN_GLYPHS = 24;
// Jaccard distance (1 - |A&B|/|A|B|) below this counts as "the same glyph".
const SAME = 0.30;

async function loadChromium() {
  const req = createRequire(import.meta.url);
  const tries = [() => req('playwright'), () => req('playwright-core'),
    () => createRequire(join(here, '..', '..', 'spell-wasm', 'package.json'))('playwright')];
  for (const t of tries) { try { return t().chromium; } catch (_) { /* next */ } }
  throw new Error('playwright not found');
}

export function parseSymbols(text) {
  const out = [];
  const re = /<symbol\s+id="rune-(\d+)"[^>]*>([\s\S]*?)<\/symbol>/g;
  let m;
  while ((m = re.exec(text))) out.push({ id: +m[1], inner: m[2] });
  return out;
}

async function rasterise(page, glyphs) {
  return page.evaluate(async ({ glyphs, N }) => {
    const res = [];
    for (const g of glyphs) {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="${N}" height="${N}" style="color:#000">${g}</svg>`;
      const img = new Image();
      img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg);
      await img.decode();
      const c = document.createElement('canvas'); c.width = c.height = N;
      const x = c.getContext('2d', { willReadFrequently: true });
      x.fillStyle = '#fff'; x.fillRect(0, 0, N, N);
      x.drawImage(img, 0, 0, N, N);
      const d = x.getImageData(0, 0, N, N).data;
      const bits = new Uint8Array(N * N);
      for (let i = 0; i < N * N; i++) bits[i] = d[i * 4] < 128 ? 1 : 0;
      res.push(Array.from(bits));
    }
    return res;
  }, { glyphs, N });
}

const T = {
  'mirror-x': (x, y) => [N - 1 - x, y],
  'mirror-y': (x, y) => [x, N - 1 - y],
  'rot90': (x, y) => [N - 1 - y, x],
  'rot180': (x, y) => [N - 1 - x, N - 1 - y],
  'rot270': (x, y) => [y, N - 1 - x],
};
function transform(b, f) {
  const o = new Uint8Array(N * N);
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    if (b[y * N + x]) { const [a, c] = f(x, y); o[c * N + a] = 1; }
  }
  return o;
}
function dist(a, b) {
  let i = 0, u = 0;
  for (let k = 0; k < N * N; k++) { if (a[k] && b[k]) i++; if (a[k] || b[k]) u++; }
  return u ? 1 - i / u : 1;
}

// returns {pairs:[{a,b,t,d}], selfs:[{a,t,d}], closest}
function analyse(bits, ids) {
  const tr = bits.map((b) => Object.fromEntries(Object.entries(T).map(([k, f]) => [k, transform(b, f)])));
  const pairs = [], selfs = [];
  let closest = { d: 2 };
  for (let i = 0; i < bits.length; i++) {
    for (const [k, tb] of Object.entries(tr[i])) {
      const d = dist(bits[i], tb);
      if (d < SAME) selfs.push({ a: ids[i], t: k, d });
    }
    for (let j = 0; j < bits.length; j++) {
      if (i === j) continue;
      // identity is also forbidden between distinct glyphs (duplicates)
      const dd = dist(bits[i], bits[j]);
      if (j > i) { if (dd < closest.d) closest = { d: dd, a: ids[i], b: ids[j], t: 'identity' }; if (dd < SAME) pairs.push({ a: ids[i], b: ids[j], t: 'identity', d: dd }); }
      for (const [k, tb] of Object.entries(tr[j])) {
        const d = dist(bits[i], tb);
        if (d < closest.d) closest = { d, a: ids[i], b: ids[j], t: k };
        if (d < SAME) pairs.push({ a: ids[i], b: ids[j], t: k, d });
      }
    }
  }
  return { pairs, selfs, closest };
}

async function main() {
  const selftest = process.argv.includes('--selftest');
  const chromium = await loadChromium();
  const browser = await chromium.launch();
  const page = await browser.newPage();
  try {
    const syms = parseSymbols(readFileSync(SVG, 'utf8'));
    if (selftest) {
      const g0 = syms[0].inner;
      const cases = [
        ['verbatim duplicate', [syms[0], syms[1], { id: 100, inner: g0 }], 'pair'],
        ['mirror-x copy', [syms[0], syms[1], { id: 101, inner: `<g transform="translate(24 0) scale(-1 1)">${g0}</g>` }], 'pair'],
        ['rot90 copy', [syms[0], syms[1], { id: 102, inner: `<g transform="rotate(90 12 12)">${g0}</g>` }], 'pair'],
        ['rot180 copy', [syms[0], syms[1], { id: 103, inner: `<g transform="rotate(180 12 12)">${g0}</g>` }], 'pair'],
        ['symmetric glyph', [{ id: 104, inner: '<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 4V20M4 12H20"/></g>' }], 'self'],
        ['mirror-symmetric glyph', [{ id: 105, inner: '<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 18L12 5L20 18"/></g>' }], 'self'],
      ];
      let bad = 0;
      for (const [name, set, kind] of cases) {
        const r = analyse(await rasterise(page, set.map((s) => s.inner)), set.map((s) => s.id));
        const caught = kind === 'pair' ? r.pairs.some((p) => p.a >= 100 || p.b >= 100) : r.selfs.length > 0;
        console.log(`selftest ${caught ? 'OK  rejected' : 'FAIL accepted'}: ${name}`);
        if (!caught) bad++;
      }
      // and the real set must not trip the same detectors on case 1's clean members
      console.log(bad ? `selftest FAILED (${bad})` : 'selftest passed: all bad inputs rejected');
      process.exit(bad ? 1 : 0);
    }
    const ids = syms.map((s) => s.id);
    const bits = await rasterise(page, syms.map((s) => s.inner));
    const r = analyse(bits, ids);
    const ink = bits.map((b) => b.reduce((a, v) => a + v, 0));
    let fail = 0;
    if (syms.length < MIN_GLYPHS) { console.error(`FAIL only ${syms.length} glyphs (< ${MIN_GLYPHS})`); fail++; }
    if (new Set(ids).size !== ids.length) { console.error('FAIL duplicate ids'); fail++; }
    for (const p of r.pairs) { console.error(`FAIL rune-${p.a} ~ rune-${p.b} under ${p.t} (distance ${p.d.toFixed(3)})`); fail++; }
    for (const s of r.selfs) { console.error(`FAIL rune-${s.a} is self-symmetric under ${s.t} (distance ${s.d.toFixed(3)})`); fail++; }
    const empty = ink.map((v, i) => [v, ids[i]]).filter(([v]) => v < 100);
    for (const [v, id] of empty) { console.error(`FAIL rune-${id} nearly empty (ink ${v}px)`); fail++; }
    // closest self-vs-own-transform distance, for the summary
    const minSelf = Math.min(...bits.flatMap((b) => Object.values(T).map((f) => dist(b, transform(b, f)))));
    console.log(`glyphs: ${syms.length}`);
    console.log(`least asymmetric glyph vs its own transforms: Jaccard distance ${minSelf.toFixed(3)}`);
    console.log(`ink px (64x64): min ${Math.min(...ink)}, max ${Math.max(...ink)}`);
    console.log(`closest pair (identity + 5 transforms): rune-${r.closest.a} vs rune-${r.closest.b} via ${r.closest.t}, Jaccard distance ${r.closest.d.toFixed(3)} (threshold ${SAME})`);
    console.log(fail ? `FAILED (${fail} problems)` : 'PASSED');
    process.exit(fail ? 1 : 0);
  } finally { await browser.close(); }
}
main().catch((e) => { console.error(e); process.exit(1); });
