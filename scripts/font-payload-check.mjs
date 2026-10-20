#!/usr/bin/env node
// CC-VISUAL-ECLIPSE AT-7 / V-D11: the fonts the app bundles stay within budget.
//
// Sums every file an @font-face in index.html points at (each file once, even
// when several faces share it, as the Vietnamese 'Bricolage Body' alias does)
// and fails above the ceiling. 2 MB for the app (V-D11). The web build serves
// the same files but loads the script faces lazily by unicode-range, so its
// first-load cost is lower than this sum.
//
//   node scripts/font-payload-check.mjs
//   node scripts/font-payload-check.mjs --selftest
import { readFileSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
export const BUDGET = 2 * 1024 * 1024;

export function fontFiles(html) {
  const files = new Set();
  for (const m of html.matchAll(/@font-face\{[^}]*url\('\.\/fonts\/([^']+)'\)/g)) files.add(m[1]);
  return [...files];
}

if (process.argv.includes('--selftest')) {
  const html = "@font-face{font-family:'A';src:url('./fonts/a.woff2')}@font-face{font-family:'B';src:url('./fonts/a.woff2')}@font-face{font-family:'C';src:url('./fonts/c.woff')}";
  const got = fontFiles(html);
  const ok = got.length === 2 && got.includes('a.woff2') && got.includes('c.woff');
  console.log(ok ? 'font-payload-check selftest: OK' : `font-payload-check selftest: FAILED (${got})`);
  process.exit(ok ? 0 : 1);
}

const html = readFileSync(join(ROOT, 'index.html'), 'utf8');
const files = fontFiles(html);
let total = 0;
const rows = files.map((f) => {
  const n = statSync(join(ROOT, 'fonts', f)).size;
  total += n;
  return [f, n];
});
const kb = (n) => `${(n / 1024).toFixed(0)} KB`;
if (total > BUDGET) {
  console.error(`font-payload-check: FAILED, bundled fonts ${kb(total)} exceed the ${kb(BUDGET)} budget (V-D11):`);
  for (const [f, n] of rows.sort((a, b) => b[1] - a[1])) console.error(`  ${kb(n).padStart(7)}  ${f}`);
  process.exit(1);
}
console.log(`font-payload-check: OK, ${files.length} font files, ${kb(total)} of the ${kb(BUDGET)} budget (V-D11).`);
