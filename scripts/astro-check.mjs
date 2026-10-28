#!/usr/bin/env node
// CC-THEMES-COSMOS F4: the Moon and the sky need no network and no location.
//
// AT-T14 (lint, fails): src/astro.rs and src/astro_events.rs import none of
//   web_sys, js_sys, crate::api, crate::storage, crate::telemetry, reqwest, fetch;
//   and ios/App/App/Info.plist asks for no NSLocation* permission.
// AT-T8 (expiry, warns): the event table ends at EVENTS_LAST_YEAR; within two
//   years of its end this prints a WARNING (exit 0) to regenerate it with
//   tools/build_astro_events.py.
//
//   node scripts/astro-check.mjs
//   node scripts/astro-check.mjs --today 2039-01-01   # pretend it is that date
//   node scripts/astro-check.mjs --selftest           # prove the warning fires and the lint bites
import { readFileSync, writeFileSync, mkdtempSync, rmSync, existsSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SELFTEST = process.argv.includes('--selftest');
const todayArg = (() => {
  const i = process.argv.indexOf('--today');
  return i >= 0 ? process.argv[i + 1] : null;
})();

const SOURCES = ['src/astro.rs', 'src/astro_events.rs'];
const PLIST = 'ios/App/App/Info.plist';
const FORBIDDEN = [
  ['web_sys', /\bweb_sys\b/],
  ['js_sys', /\bjs_sys\b/],
  ['crate::api', /\bcrate::api\b/],
  ['crate::storage', /\bcrate::storage\b/],
  ['crate::telemetry', /\bcrate::telemetry\b/],
  ['reqwest', /\breqwest\b/],
  ['fetch', /\bfetch\b/],
];

// Lint over a root directory; returns a list of problems.
function lint(root) {
  const bad = [];
  for (const rel of SOURCES) {
    const p = join(root, rel);
    if (!existsSync(p)) {
      bad.push(`${rel}: missing`);
      continue;
    }
    readFileSync(p, 'utf8').split('\n').forEach((line, i) => {
      for (const [name, re] of FORBIDDEN) if (re.test(line)) bad.push(`${rel}:${i + 1}: forbidden ${name}: ${line.trim().slice(0, 100)}`);
    });
  }
  const plist = join(root, PLIST);
  if (existsSync(plist)) {
    readFileSync(plist, 'utf8').split('\n').forEach((line, i) => {
      if (/<key>\s*NSLocation/.test(line)) bad.push(`${PLIST}:${i + 1}: location permission key: ${line.trim()}`);
    });
  }
  return bad;
}

function lastYear(root) {
  const m = readFileSync(join(root, 'src/astro_events.rs'), 'utf8').match(/pub const EVENTS_LAST_YEAR: i32 = (\d{4});/);
  if (!m) throw new Error('astro-check: EVENTS_LAST_YEAR not found in src/astro_events.rs');
  return Number(m[1]);
}

// The table runs through 31 Dec of its last year; warn from two years before that.
function expiryWarning(last, today) {
  const end = Date.UTC(last, 11, 31);
  const warnFrom = Date.UTC(last - 2, 11, 31);
  const t = Date.parse(`${today}T00:00:00Z`);
  if (Number.isNaN(t)) throw new Error(`astro-check: bad --today ${today}`);
  if (t < warnFrom) return null;
  return t > end
    ? `WARNING: astro event table ended ${last}-12-31 (today ${today}); full-moon/eclipse windows fall back or stop. Run python3 tools/build_astro_events.py with a later LAST_YEAR.`
    : `WARNING: astro event table ends ${last}-12-31, within 2 years of ${today}. Run python3 tools/build_astro_events.py with a later LAST_YEAR.`;
}

if (SELFTEST) {
  const fails = [];
  const last = lastYear(ROOT);
  if (!expiryWarning(last, `${last - 1}-01-01`)) fails.push(`no warning on ${last - 1}-01-01`);
  if (last === 2040 && !expiryWarning(last, '2039-01-01')) fails.push('no warning on 2039-01-01');
  if (expiryWarning(last, '2026-10-28')) fails.push('warning fired on 2026-10-28');
  // Plant a forbidden import (and a location key) in a temp copy; the lint must bite.
  const tmp = mkdtempSync(join(tmpdir(), 'astro-check-'));
  try {
    mkdirSync(join(tmp, 'src'));
    mkdirSync(join(tmp, 'ios/App/App'), { recursive: true });
    for (const rel of SOURCES) writeFileSync(join(tmp, rel), readFileSync(join(ROOT, rel), 'utf8'));
    writeFileSync(join(tmp, PLIST), '<plist><dict></dict></plist>\n');
    if (lint(tmp).length) fails.push(`clean copy flagged: ${lint(tmp).join('; ')}`);
    writeFileSync(join(tmp, 'src/astro.rs'), 'use web_sys::window;\n' + readFileSync(join(ROOT, 'src/astro.rs'), 'utf8'));
    if (!lint(tmp).some((b) => b.includes('web_sys'))) fails.push('planted web_sys import not caught');
    writeFileSync(join(tmp, PLIST), '<plist><dict><key>NSLocationWhenInUseUsageDescription</key><string>x</string></dict></plist>\n');
    if (!lint(tmp).some((b) => b.includes('NSLocation'))) fails.push('planted NSLocation key not caught');
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
  if (fails.length) {
    console.error(`astro-check selftest: FAILED\n  ${fails.join('\n  ')}`);
    process.exit(1);
  }
  console.log('astro-check selftest: OK (expiry warning fires on 2039-01-01, not today; a planted web_sys import and an NSLocation key are caught)');
  process.exit(0);
}

const bad = lint(ROOT);
if (bad.length) {
  console.error(`astro-check: FAILED — the astro module must stay offline and location-free:\n  ${bad.join('\n  ')}`);
  process.exit(1);
}
const today = todayArg ?? new Date().toISOString().slice(0, 10);
const last = lastYear(ROOT);
const warn = expiryWarning(last, today);
if (warn) console.warn(`astro-check: ${warn}`);
console.log(`astro-check: OK — ${SOURCES.join(', ')} import nothing networked; ${PLIST} has no NSLocation keys; table runs to ${last}`);
