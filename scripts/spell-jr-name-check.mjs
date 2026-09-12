#!/usr/bin/env node
// CC-ONBOARD-JR F0 / I11 — the junior experience has ONE player-facing name: Spell Jr.
//
//   node scripts/spell-jr-name-check.mjs            # check locales, web pages, store metadata
//   node scripts/spell-jr-name-check.mjs --selftest # prove planted lesions FAIL
//
// The Step 0 inventory found the in-app text already clean: all 15 locales say
// "Spell Jr", and "Kid Mode" / "Little Speller" survived only in code comments,
// persisted field names, the App Store description and the privacy page. This
// gate keeps every player-facing surface that way. It is deliberately NOT a
// grep over the source tree: `AppState.kid`, `byear_prefs_v1` and comments are
// internal names, and renaming persisted fields needs a migration that buys a
// player nothing (inventory §1, signed). What a player or parent can read is:
//   * the locale catalogs (every value, and no retired key names),
//   * the renderable part of the tracked web pages (index.html, privacy.html),
//   * the store listing text under fastlane/metadata (reviewer notes excluded).
//
// Known limit: a retired name TRANSLATED into another script (a Korean phrase
// for "kid mode") is not detectable by pattern. The positive check — every
// locale's settings.kid is literally "Spell Jr" — covers the one place such a
// translation would have lived.
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative, sep } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const NAME = 'Spell Jr';
const RETIRED = /\b(kids?[ -]?mode|little[ -]?speller)\b/i;
const RETIRED_KEY = /(little_?speller|kids?_?mode)/i;
const PAGES = ['index.html', 'privacy.html'];

// Comments, scripts and styles never reach a reader. Attribute values
// (aria-label, title, placeholder) do, so they stay in.
const renderable = (html) => html
  .replace(/<!--[\s\S]*?-->/g, '')
  .replace(/<script\b[\s\S]*?<\/script>/gi, '')
  .replace(/<style\b[\s\S]*?<\/style>/gi, '');

export function check({ locales, pages = {}, store = {} }) {
  const problems = [];
  for (const [code, cat] of Object.entries(locales)) {
    if (cat['settings.kid'] !== NAME) {
      problems.push(`${code}: settings.kid is ${JSON.stringify(cat['settings.kid'])}, not "${NAME}"`);
    }
    for (const [k, v] of Object.entries(cat)) {
      if (RETIRED_KEY.test(k)) problems.push(`${code}: key "${k}" carries a retired name`);
      if (typeof v === 'string' && RETIRED.test(v)) {
        problems.push(`${code}: "${k}" says ${JSON.stringify(v.match(RETIRED)[0])} — the player-facing name is ${NAME}`);
      }
    }
  }
  const scan = (label, text) => {
    for (const m of text.matchAll(new RegExp(RETIRED.source, 'gi'))) {
      const around = text.slice(Math.max(0, m.index - 30), m.index + m[0].length + 30).replace(/\s+/g, ' ');
      problems.push(`${label}: says ${JSON.stringify(m[0])} (…${around}…)`);
    }
  };
  for (const [name, html] of Object.entries(pages)) scan(name, renderable(html));
  for (const [name, text] of Object.entries(store)) scan(name, text);
  return problems;
}

function loadReal() {
  const dir = join(ROOT, 'src', 'i18n', 'locales');
  const locales = {};
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
    locales[f.replace(/\.json$/, '')] = JSON.parse(readFileSync(join(dir, f), 'utf8'));
  }
  const pages = {};
  for (const p of PAGES) if (existsSync(join(ROOT, p))) pages[p] = readFileSync(join(ROOT, p), 'utf8');
  const store = {};
  const walk = (d) => {
    if (!existsSync(d)) return;
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      if (statSync(p).isDirectory()) { if (name !== 'review_information') walk(p); }
      else if (name.endsWith('.txt')) store[relative(ROOT, p).split(sep).join('/')] = readFileSync(p, 'utf8');
    }
  };
  walk(join(ROOT, 'fastlane', 'metadata'));
  return { locales, pages, store };
}

if (process.argv.includes('--selftest')) {
  const clean = () => ({
    locales: { en: { 'settings.kid': 'Spell Jr', 'settings.kidSmall': 'Bigger text, friendly words' } },
    // A comment or script mentioning the old name is NOT player-facing and must pass.
    pages: { 'index.html': '<!-- Kid Mode hides climbBtn --><label>Spell Jr</label><script>// Little Speller</script>' },
    store: { 'fastlane/metadata/en-US/description.txt': '• Spell Jr and a Big Text option.' },
  });
  const cases = [
    ['retired name in a locale value', (f) => { f.locales.en['home.hint'] = 'Turn on Kid Mode for friendlier words'; }],
    ['retired name in a locale key', (f) => { f.locales.en['littleSpeller.title'] = 'Spell Jr'; }],
    ['settings.kid renamed', (f) => { f.locales.en['settings.kid'] = 'Kids'; }],
    ['settings.kid missing in a second locale', (f) => { f.locales.es = { 'other.key': 'Hola' }; }],
    ['retired name in visible html', (f) => { f.pages['index.html'] += '<p>Little Speller</p>'; }],
    ['retired name in an aria-label', (f) => { f.pages['index.html'] += '<button aria-label="Kid mode on"></button>'; }],
    ['retired name on the privacy page', (f) => { f.pages['privacy.html'] = '<p>Children playing in Kid Mode</p>'; }],
    ['retired name in the store listing', (f) => { f.store['fastlane/metadata/en-US/description.txt'] = '• Kid Mode for younger players.'; }],
  ];
  const missed = [];
  for (const [name, mutate] of cases) {
    const f = clean(); mutate(f);
    if (check(f).length === 0) missed.push(name);
  }
  const cleanProblems = check(clean());
  if (missed.length || cleanProblems.length) {
    if (missed.length) console.error(`spell-jr-name-check --selftest: FAILED — not caught: ${missed.join(', ')}`);
    if (cleanProblems.length) console.error(`spell-jr-name-check --selftest: FAILED — clean fixture rejected: ${cleanProblems.join('; ')}`);
    process.exit(1);
  }
  console.log(`spell-jr-name-check --selftest: OK — all ${cases.length} planted lesions caught, clean fixture (with comment-only mentions) accepted.`);
  process.exit(0);
}

const real = loadReal();
const problems = check(real);
if (problems.length) {
  console.error(`spell-jr-name-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ✗ ${p}`);
  process.exit(1);
}
console.log(`spell-jr-name-check: OK — ${Object.keys(real.locales).length} locales name it "${NAME}"; ` +
  `${Object.keys(real.pages).length} web pages and ${Object.keys(real.store).length} store texts carry no retired name.`);
