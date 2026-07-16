#!/usr/bin/env node
// CI gate for the country -> language grant map (Feature 2).
//
// Validates config/country-language-map.json against four rules and FAILS the
// build (exit 1) on any violation:
//   1. every KEY is a valid ISO 3166-1 alpha-2 country code;
//   2. every VALUE code is one of the 16 shipped SpellGame language codes;
//   3. every NON-English shipped language appears in >= 1 country (a language
//      with no home country is a bug);
//   4. the JSON is sorted by key (deterministic single source of truth).
//
// Usage:
//   node scripts/entitlement-map-check.mjs            # checks the real map
//   node scripts/entitlement-map-check.mjs <path>     # checks an arbitrary file
//   node scripts/entitlement-map-check.mjs --selftest # proves the check FAILS
//                                                       on the broken fixture
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');

// The 16 shipped SpellGame language codes (must match src/consts.rs BUILTIN_LANGS).
// CC-LINEUP-SWAP (2026-07-16): it/nl/sv/nb cut; ru/ar/fa/ur added; tr reinstated
// per D7. Thai stays cut (5fc69ff). ar/fa/ur are registered and granted here but
// hard-gated from activation by rtl_required until RTL_SUPPORTED — a regional
// grant is not an activation, so the map lists them like any other language.
const SHIPPED_LANGS = new Set([
  'en', 'es', 'fr', 'de', 'pt', 'pl', 'tr', 'vi', 'ko', 'ja', 'fil', 'zh',
  'ru', 'ar', 'fa', 'ur',
]);

// Full ISO 3166-1 alpha-2 officially-assigned code set.
const ISO_ALPHA2 = new Set(
  ('AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ ' +
   'BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR ' +
   'CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR ' +
   'GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ' +
   'ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ ' +
   'LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ ' +
   'MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF ' +
   'PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI ' +
   'SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR ' +
   'TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW').split(' ')
);

function validate(path) {
  const problems = [];
  const raw = readFileSync(path, 'utf8');
  let map;
  try {
    map = JSON.parse(raw);
  } catch (e) {
    return [`invalid JSON: ${e.message}`];
  }
  if (map === null || typeof map !== 'object' || Array.isArray(map)) {
    return ['top level must be an object of country -> [lang, ...]'];
  }

  const keys = Object.keys(map);
  const granted = new Set();

  for (const key of keys) {
    // Rule 1: valid ISO 3166-1 alpha-2.
    if (!ISO_ALPHA2.has(key)) problems.push(`invalid ISO 3166-1 alpha-2 country code: "${key}"`);
    const langs = map[key];
    if (!Array.isArray(langs) || langs.length === 0) {
      problems.push(`"${key}" must map to a non-empty array of language codes`);
      continue;
    }
    // Rule 2: values are shipped languages. Also flag dupes.
    const seen = new Set();
    for (const lang of langs) {
      if (!SHIPPED_LANGS.has(lang)) problems.push(`"${key}" grants unshipped language code: "${lang}"`);
      if (seen.has(lang)) problems.push(`"${key}" lists "${lang}" more than once`);
      seen.add(lang);
      granted.add(lang);
    }
    // Value arrays should be sorted (determinism).
    const sortedVals = [...langs].sort();
    if (JSON.stringify(langs) !== JSON.stringify(sortedVals)) {
      problems.push(`"${key}" language array must be sorted: got ${JSON.stringify(langs)}`);
    }
  }

  // Rule 3: every non-English shipped language has >= 1 home country.
  for (const lang of SHIPPED_LANGS) {
    if (lang === 'en') continue; // English is free everywhere; exempt.
    if (!granted.has(lang)) problems.push(`non-English language "${lang}" has no home country (bug)`);
  }

  // Rule 4: JSON sorted by key.
  const sortedKeys = [...keys].sort();
  if (JSON.stringify(keys) !== JSON.stringify(sortedKeys)) {
    problems.push('map is not sorted by country code (keys out of order)');
  }

  return problems;
}

function report(label, problems) {
  if (problems.length) {
    console.error(`entitlement-map-check: FAILED (${label}) — ${problems.length} problem(s):`);
    for (const p of problems) console.error(`  ✗ ${p}`);
    return false;
  }
  console.log(`entitlement-map-check: OK (${label})`);
  return true;
}

const arg = process.argv[2];

if (arg === '--selftest') {
  // Prove the checker actually rejects a broken map (negative test). Exit 0 iff
  // the fixture correctly FAILS; exit 1 if the broken fixture wrongly passed.
  const fixture = join(HERE, 'fixtures', 'country-language-map.broken.json');
  const problems = validate(fixture);
  if (problems.length === 0) {
    console.error('entitlement-map-check --selftest: FAILED — the broken fixture unexpectedly PASSED.');
    process.exit(1);
  }
  console.log(`entitlement-map-check --selftest: OK — broken fixture correctly rejected with ${problems.length} problem(s):`);
  for (const p of problems) console.log(`  ✓ caught: ${p}`);
  process.exit(0);
}

const target = arg || join(ROOT, 'config', 'country-language-map.json');
process.exit(report(target.replace(ROOT + '/', ''), validate(target)) ? 0 : 1);
