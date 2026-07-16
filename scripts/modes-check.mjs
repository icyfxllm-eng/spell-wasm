#!/usr/bin/env node
// CC-MODE-HUB I1.2 — schema gate for the mode registry (config/modes.json).
//
//   node scripts/modes-check.mjs            # validate the real registry
//   node scripts/modes-check.mjs --selftest # prove a malformed entry FAILS
//
// A malformed entry fails the build. Beyond shape, this cross-checks the
// registry against the code it describes, because the failure mode that matters
// is not "bad JSON" — it is a registry that has quietly drifted from what the app
// does. Two checks earn their keep:
//
//   * every mode's nameKey/descKey must EXIST in the en locale. F2 forbids new
//     auditable content ("names reuse existing localized mode strings"), so a key
//     that isn't already in the catalog means someone invented copy.
//   * every mode id must have a flag of the same name in src/flags.rs. The
//     registry describes shipped modes; an id with no implementation is a tile
//     that leads nowhere.
//
// Deliberately NOT checked: that `status` matches the flag's compiled-in default.
// They are different concepts — `status` is what the hub SHOWS, the flag is
// whether the feature RUNS — and conflating them would force word_stories
// (hidden, flag off) and online_spelloff (coming_soon, flag off) to lie about one
// or the other.
import { readFileSync, existsSync, writeFileSync, mkdtempSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const STATUSES = new Set(['live', 'hidden', 'coming_soon']);
const PLATFORMS = new Set(['web', 'ios']);
const LEVELS = new Set(['none', 'preview', 'full']);
const PREMIUM = new Set(['photo_ocr', 'multiple_profiles', 'progress_reports', 'custom_lists_unlimited']);

function validate(reg, { locale, flags } = {}) {
  const problems = [];
  if (!Array.isArray(reg.modes) || reg.modes.length === 0) return ['registry has no `modes` array'];

  const seen = new Set();
  for (const m of reg.modes) {
    const at = `mode "${m.id ?? '<no id>'}"`;
    if (!m.id) { problems.push('a mode has no id'); continue; }
    if (seen.has(m.id)) problems.push(`${at}: duplicate id`);
    seen.add(m.id);

    if (!STATUSES.has(m.status)) problems.push(`${at}: status must be one of ${[...STATUSES].join('|')}, got ${JSON.stringify(m.status)}`);
    if (typeof m.kidSafe !== 'boolean') problems.push(`${at}: kidSafe must be a boolean`);
    if (!m.nameKey) problems.push(`${at}: missing nameKey`);
    if (!m.descKey) problems.push(`${at}: missing descKey`);
    if (!m.icon) problems.push(`${at}: missing icon`);

    if (!Array.isArray(m.platforms) || m.platforms.length === 0) problems.push(`${at}: platforms must be a non-empty array`);
    else for (const p of m.platforms) if (!PLATFORMS.has(p)) problems.push(`${at}: unknown platform ${JSON.stringify(p)}`);

    if (!LEVELS.has(m.entitlementLevel)) problems.push(`${at}: entitlementLevel must be one of ${[...LEVELS].join('|')}, got ${JSON.stringify(m.entitlementLevel)}`);

    if (m.requiresPremium !== null && m.requiresPremium !== undefined && !PREMIUM.has(m.requiresPremium)) {
      problems.push(`${at}: requiresPremium must be null or an EntitlementSet boolean (${[...PREMIUM].join(', ')}), got ${JSON.stringify(m.requiresPremium)}`);
    }
    if (m.languages !== null && m.languages !== undefined && !Array.isArray(m.languages)) {
      problems.push(`${at}: languages must be null or an array`);
    }

    // Cross-check: the copy must already exist (F2 — no new auditable content).
    if (locale) {
      for (const k of ['nameKey', 'descKey']) {
        if (m[k] && !(m[k] in locale)) problems.push(`${at}: ${k} "${m[k]}" is not in the en locale — F2 forbids new auditable content`);
      }
    }
    // Cross-check: the mode must actually exist in the app.
    if (flags && m.id && !flags.includes(`pub fn ${m.id}(`)) {
      problems.push(`${at}: no flag "pub fn ${m.id}()" in src/flags.rs — a registry entry with no implementation is a tile leading nowhere`);
    }
  }
  return problems;
}

function loadReal() {
  return {
    reg: JSON.parse(readFileSync(join(ROOT, 'config', 'modes.json'), 'utf8')),
    locale: JSON.parse(readFileSync(join(ROOT, 'src', 'i18n', 'locales', 'en.json'), 'utf8')),
    flags: readFileSync(join(ROOT, 'src', 'flags.rs'), 'utf8'),
  };
}

if (process.argv.includes('--selftest')) {
  // A checker that cannot fail is decoration. Each fixture breaks ONE rule.
  const { locale, flags } = loadReal();
  const base = () => ({
    id: 'ghost_racing', nameKey: 'tools.ghost.name', descKey: 'tools.ghost.desc', icon: '👻',
    status: 'live', kidSafe: true, platforms: ['web'], entitlementLevel: 'full', requiresPremium: null, languages: null,
  });
  const cases = [
    ['bad status', (m) => { m.status = 'enabled'; }],
    ['non-boolean kidSafe', (m) => { m.kidSafe = 'yes'; }],
    ['unknown platform', (m) => { m.platforms = ['android']; }],
    ['bad entitlementLevel', (m) => { m.entitlementLevel = 'premium'; }],
    ['unknown requiresPremium', (m) => { m.requiresPremium = 'unlimited_everything'; }],
    ['invented copy key', (m) => { m.nameKey = 'tools.invented.name'; }],
    ['no implementation', (m) => { m.id = 'teleport_mode'; }],
    ['missing icon', (m) => { delete m.icon; }],
  ];
  const missed = [];
  for (const [name, mutate] of cases) {
    const m = base(); mutate(m);
    if (validate({ modes: [m] }, { locale, flags }).length === 0) missed.push(name);
  }
  // ...and a well-formed entry must PASS, or the checker is just noisy.
  const cleanOk = validate({ modes: [base()] }, { locale, flags }).length === 0;
  if (missed.length || !cleanOk) {
    if (missed.length) console.error(`modes-check --selftest: FAILED — not caught: ${missed.join(', ')}`);
    if (!cleanOk) console.error('modes-check --selftest: FAILED — a valid entry was wrongly rejected');
    process.exit(1);
  }
  console.log(`modes-check --selftest: OK — all ${cases.length} malformed fixtures rejected, valid entry accepted.`);
  process.exit(0);
}

if (!existsSync(join(ROOT, 'config', 'modes.json'))) {
  console.error('modes-check: config/modes.json not found');
  process.exit(1);
}
const { reg, locale, flags } = loadReal();
const problems = validate(reg, { locale, flags });
if (problems.length) {
  console.error(`modes-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ✗ ${p}`);
  process.exit(1);
}
const live = reg.modes.filter((m) => m.status === 'live').length;
const soon = reg.modes.filter((m) => m.status === 'coming_soon').length;
const hidden = reg.modes.filter((m) => m.status === 'hidden').length;
const kid = reg.modes.filter((m) => m.kidSafe && m.status === 'live').length;
console.log(`modes-check: OK — ${reg.modes.length} modes (${live} live, ${soon} coming_soon, ${hidden} hidden; ${kid} kidSafe live).`);
