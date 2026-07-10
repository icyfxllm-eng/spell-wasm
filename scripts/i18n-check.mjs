#!/usr/bin/env node
// Key-parity gate for the UI locale tables (src/i18n/locales/*.json).
// en.json is canonical: every other locale must define exactly the same key
// set — no missing keys (would fall back to English at runtime) and no extra
// keys (dead strings). Also flags empty values and `{param}` placeholder drift.
// Exits non-zero on any violation so it can gate CI.
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', 'src', 'i18n', 'locales');
const files = readdirSync(dir).filter((f) => f.endsWith('.json'));
const load = (f) => JSON.parse(readFileSync(join(dir, f), 'utf8'));

const en = load('en.json');
const enKeys = new Set(Object.keys(en));
const params = (s) => new Set([...String(s).matchAll(/\{(\w+)\}/g)].map((m) => m[1]));
const enParams = Object.fromEntries(Object.entries(en).map(([k, v]) => [k, params(v)]));

let problems = 0;
const report = (f, msg) => {
  problems++;
  console.error(`  [${f}] ${msg}`);
};

for (const f of files) {
  if (f === 'en.json') continue;
  const t = load(f);
  const keys = new Set(Object.keys(t));
  for (const k of enKeys) if (!keys.has(k)) report(f, `missing key: ${k}`);
  for (const k of keys) if (!enKeys.has(k)) report(f, `extra key not in en.json: ${k}`);
  for (const [k, v] of Object.entries(t)) {
    if (typeof v !== 'string' || v.trim() === '') report(f, `empty value: ${k}`);
    if (enKeys.has(k)) {
      const want = enParams[k];
      const got = params(v);
      for (const p of want) if (!got.has(p)) report(f, `key ${k} missing placeholder {${p}}`);
      for (const p of got) if (!want.has(p)) report(f, `key ${k} has stray placeholder {${p}}`);
    }
  }
}

if (problems) {
  console.error(`\ni18n-check: ${problems} problem(s) across ${files.length} locale file(s).`);
  process.exit(1);
}
console.log(`i18n-check: OK — ${enKeys.size} keys parity across ${files.length} locales.`);
