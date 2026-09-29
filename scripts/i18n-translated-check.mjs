#!/usr/bin/env node
// CC-HUB-GROUP-L10N F3 — the completeness gate, and why it is not the one
// the spec asked for.
//
// F3 says "each key has a non-empty value for each language". Every one of
// the three drawer headers already satisfied that: all fifteen locales
// carried nav.spellIt, and all fifteen held the string "Spell it". The
// existing parity gate (i18n-check) checks presence and emptiness, so a key
// whose value is the English string is invisible to it — which is how three
// English headers reached fourteen languages with nothing going red.
//
// So the rule here is SAMENESS, not emptiness: for a key on the `translated`
// list, a non-English locale whose value equals English is a failure. That
// is the actual defect, stated as a test.
//
// The list is explicit, not every key, and that is deliberate. Plenty of
// strings are legitimately identical across languages — "OK", emoji, proper
// nouns — and a gate that flagged those would be muted within a week.
import { readFileSync, readdirSync, existsSync } from 'node:fs';

const DIR = 'src/i18n/locales';
const STATUS = 'src/i18n/translation-status.json';
const SELFTEST = process.argv.includes('--selftest');

function load() {
  const locales = {};
  for (const f of readdirSync(DIR)) {
    if (f.endsWith('.json')) locales[f.replace(/\.json$/, '')] = JSON.parse(readFileSync(`${DIR}/${f}`, 'utf8'));
  }
  return { locales, status: JSON.parse(readFileSync(STATUS, 'utf8')) };
}

const nfc = (s) => String(s).normalize('NFC');

export function evaluate({ locales, status }) {
  const fails = [];
  const en = locales.en;
  if (!en) return ['no en locale — nothing to compare against'];
  const dnt = new Set(Object.keys(status.do_not_translate || {}));

  for (const key of status.translated || []) {
    if (!(key in en)) { fails.push(`${key}: not in the en table at all`); continue; }
    for (const [lang, table] of Object.entries(locales)) {
      if (lang === 'en') continue;
      const v = table[key];
      // I2 — still an emptiness check, because that failure exists too.
      if (typeof v !== 'string' || v.trim() === '') { fails.push(`${lang}: ${key} is missing or empty`); continue; }
      // ...and the one the old gate could not see.
      if (!dnt.has(key) && nfc(v) === nfc(en[key])) {
        fails.push(`${lang}: ${key} is still the English string ${JSON.stringify(en[key])}`);
      }
      // I5 — stored strings equal their NFC form.
      if (v !== nfc(v)) fails.push(`${lang}: ${key} is not NFC-normalized`);
    }
  }

  // I9 — a do_not_translate key is IDENTICAL everywhere. The opposite
  // promise, and it needs saying: without it a later localization pass would
  // "fix" SpellDoku into fifteen different words.
  for (const key of dnt) {
    if (!(key in en)) { fails.push(`${key}: marked do_not_translate but not in en`); continue; }
    for (const [lang, table] of Object.entries(locales)) {
      if (lang === 'en') continue;
      if (nfc(table[key] ?? '') !== nfc(en[key])) {
        fails.push(`${lang}: ${key} is ${JSON.stringify(table[key])}, but it is a brand name and must read ${JSON.stringify(en[key])}`);
      }
    }
  }

  // The audit ledger has to cover what it claims to. A key that is on the
  // translated list and in no audit row is a draft nobody will ever check.
  const audited = status.needs_audit || {};
  for (const [lang, keys] of Object.entries(audited)) {
    if (!(lang in locales)) fails.push(`needs_audit names ${lang}, which is not a shipped locale`);
    for (const k of keys) if (!(status.translated || []).includes(k)) {
      fails.push(`needs_audit lists ${lang}/${k}, which is not on the translated list`);
    }
  }
  return fails;
}

if (!SELFTEST) {
  const fails = evaluate(load());
  if (fails.length) {
    console.error('i18n-translated-check: FAILED\n' + fails.map((f) => `  ${f}`).join('\n'));
    process.exit(1);
  }
  const { status, locales } = load();
  console.log(
    `i18n-translated-check: OK — ${(status.translated || []).length} key(s) translated across ` +
    `${Object.keys(locales).length} locales, ${Object.keys(status.do_not_translate || {}).length} brand name(s) held identical`,
  );
  process.exit(0);
}

// A gate that cannot fail is decoration. Each lesion is a way this bug could
// come back.
const world = load();
const LESIONS = [
  {
    name: 'a header left as the English string (the original bug)',
    mutate: (w) => {
      const l = JSON.parse(JSON.stringify(w));
      l.locales.es['hub.group.spell_it'] = l.locales.en['hub.group.spell_it'];
      return l;
    },
  },
  {
    name: 'a header present but empty',
    mutate: (w) => { const l = JSON.parse(JSON.stringify(w)); l.locales.ru['hub.group.meaning'] = '   '; return l; },
  },
  {
    name: 'a brand name "helpfully" localized',
    mutate: (w) => { const l = JSON.parse(JSON.stringify(w)); l.locales.zh['sd.name'] = '拼字数独'; return l; },
  },
  {
    name: 'a string stored in NFD rather than NFC',
    mutate: (w) => {
      const l = JSON.parse(JSON.stringify(w));
      l.locales.fr['hub.group.spell_it'] = 'Épellation'; // decomposed
      return l;
    },
  },
  {
    name: 'an audit row for a key nobody tracks',
    mutate: (w) => { const l = JSON.parse(JSON.stringify(w)); l.status.needs_audit.es.push('some.other.key'); return l; },
  },
];

if (evaluate(world).length !== 0) {
  console.error('i18n-translated-check: the REAL tables already fail; fix that before trusting the selftest');
  process.exit(1);
}
let bad = 0;
for (const { name, mutate } of LESIONS) {
  if (evaluate(mutate(world)).length === 0) { console.error(`  SURVIVED: ${name}`); bad++; }
}
if (bad) {
  console.error(`i18n-translated-check: FAILED\n  ${bad} of ${LESIONS.length} lesions were not caught`);
  process.exit(1);
}
console.log(`i18n-translated-check: selftest OK — all ${LESIONS.length} lesions fail the build`);
