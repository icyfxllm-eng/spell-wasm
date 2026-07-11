#!/usr/bin/env node
// i18n key-parity check (CI gate). English (en.json) is the source of truth: every
// other locale must define every key en defines. Fails (exit 1) listing gaps, so a
// new string added only to English can't silently ship as a mixed-language screen.
//
//   node scripts/i18n-parity.mjs
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', 'src', 'i18n', 'locales');
const keys = (f) => new Set(Object.keys(JSON.parse(readFileSync(join(dir, f), 'utf8'))));

const en = keys('en.json');
const locales = readdirSync(dir).filter((f) => f.endsWith('.json') && f !== 'en.json');

let failed = false;
for (const f of locales) {
  const k = keys(f);
  const missing = [...en].filter((x) => !k.has(x));
  const extra = [...k].filter((x) => !en.has(x)); // reported, not fatal
  if (missing.length) {
    failed = true;
    console.error(`✗ ${f}: missing ${missing.length} key(s): ${missing.slice(0, 12).join(', ')}${missing.length > 12 ? ' …' : ''}`);
  }
  if (extra.length) {
    console.warn(`  ${f}: ${extra.length} extra key(s) not in en (ok): ${extra.slice(0, 8).join(', ')}`);
  }
}

if (failed) {
  console.error('\ni18n parity FAILED — every locale must define all English keys.');
  process.exit(1);
}
console.log(`✓ i18n parity OK — ${locales.length} locales all match en (${en.size} keys).`);
