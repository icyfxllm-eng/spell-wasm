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

// AUDITPASS F16 — a key with a `.native` sibling exists BECAUSE the plain
// text is wrong on the wrapped build (the shipped explainer told iPhone
// users to "try Chrome or Edge"). Reaching such a key through `t()` puts
// the browser copy back on a phone, silently. They must go through
// `t_platform`, or through a [data-i18n] attribute, which resolves that
// way for every key.
{
  const src = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');
  const rs = [];
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const q = join(d, e.name);
      if (e.isDirectory()) walk(q);
      else if (e.name.endsWith('.rs')) rs.push([q, readFileSync(q, 'utf8')]);
    }
  };
  walk(src);
  const nativeKeys = Object.keys(en)
    .filter((k) => k.endsWith('.native'))
    .map((k) => k.slice(0, -'.native'.length));
  for (const key of nativeKeys) {
    for (const [file, body] of rs) {
      if (body.includes(`t("${key}")`)) {
        console.log(`i18n-check: ${file} calls t("${key}") — a .native variant exists, use t_platform`);
        problems++;
      }
    }
  }
  // ---- Every key that is USED must be DEFINED -------------------------
  //
  // Parity above only compares the locale files with each other. A key that
  // is referenced but defined NOWHERE is identical in all fifteen and sails
  // straight through -- which is how `placement.title/body/try/skip/done`
  // shipped. `t()` returns the KEY when it cannot resolve one, so the
  // placement card greeted every new player with the literal text
  // "placement.title". Eric's testers hit it on the first tap of the orb;
  // nobody had seen it because it only appears for a player with no
  // placement on record, and every device here had done its placement
  // months earlier.
  //
  // The console warning t() emits was never going to save this: nobody is
  // attached to a console on a TestFlight phone.
  const htmlSrc = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', 'index.html'), 'utf8');
  const used = new Map(); // key -> Set<source>
  const use = (k, where) => { if (!used.has(k)) used.set(k, new Set()); used.get(k).add(where); };
  for (const m of htmlSrc.matchAll(/data-i18n(?:-[a-z]+)?="([^"]+)"/g)) use(m[1], 'index.html');
  for (const [file, body] of rs) {
    for (const m of body.matchAll(/i18n::t\("([^"]+)"\)/g)) use(m[1], file);
    // Bare t("a.b") inside the i18n-importing modules.
    for (const m of body.matchAll(/\bt\("([a-zA-Z][\w]*\.[\w.]+)"\)/g)) use(m[1], file);
  }
  // A `.native` variant is resolved by t_platform at runtime, so the base
  // key standing alone is legitimate.
  const defined = new Set(Object.keys(en));
  const undef = [...used.entries()].filter(
    ([k]) => !defined.has(k) && !defined.has(`${k}.native`)
  );
  for (const [k, where] of undef) {
    console.error(`  [used-but-undefined] ${k} — referenced by ${[...where].join(', ')} but in no locale;`);
    console.error(`      t() will render the key itself to the player`);
    problems++;
  }

  if (problems) process.exit(1);
  console.log(`i18n-check: OK — ${nativeKeys.length} platform-aware keys route through t_platform.`);
  console.log(`i18n-check: OK — all ${used.size} referenced keys are defined.`);
}
