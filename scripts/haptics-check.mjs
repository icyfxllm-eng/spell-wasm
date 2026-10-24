#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F6 / AT-17: every haptic goes through src/haptics.rs.
//
// The module reads one mapping table and enforces the toggles and the per-word
// cap, so a second path to the plugin would quietly bypass all three. This
// fails on any reference to the Capacitor Haptics plugin outside the module:
// a `Haptics` property read in Rust or JavaScript, or a `Plugins.Haptics`.
//
//   node scripts/haptics-check.mjs
//   node scripts/haptics-check.mjs --selftest
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SELFTEST = process.argv.includes('--selftest');
const ALLOWED = new Set(['src/haptics.rs']);
// The ways in: the plugin by name, as a string or a property.
const PATTERNS = [/["']Haptics["']/, /Plugins\.Haptics/, /\bHaptics\.(impact|notification|vibrate|selection)/];

function files() {
  const out = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.rs$/.test(name)) out.push(p);
    }
  };
  walk(join(ROOT, 'src'));
  for (const name of readdirSync(ROOT)) if (/\.(js|html)$/.test(name)) out.push(join(ROOT, name));
  return out;
}

function scan(entries) {
  const bad = [];
  for (const [rel, text] of entries) {
    if (ALLOWED.has(rel)) continue;
    text.split('\n').forEach((line, i) => {
      if (PATTERNS.some((re) => re.test(line))) bad.push(`${rel}:${i + 1}: ${line.trim().slice(0, 100)}`);
    });
  }
  return bad;
}

const entries = files().map((p) => [relative(ROOT, p).replace(/\\/g, '/'), readFileSync(p, 'utf8')]);

if (SELFTEST) {
  const planted = scan([['src/game.rs', 'let h = Reflect::get(&plugins, &JsValue::from_str("Haptics"));']]);
  const clean = scan([['src/haptics.rs', 'Reflect::get(&plugins, &JsValue::from_str("Haptics"))'], ['src/game.rs', 'crate::haptics::correct();']]);
  if (planted.length === 1 && clean.length === 0) {
    console.log('haptics-check selftest: OK (a planted bypass is caught; the module and a normal call pass)');
    process.exit(0);
  }
  console.error(`haptics-check selftest: FAILED (planted ${planted.length}, clean ${clean.length})`);
  process.exit(1);
}

const bad = scan(entries);
if (bad.length) {
  console.error(`haptics-check: FAILED — the haptics plugin is reached outside src/haptics.rs:\n  ${bad.join('\n  ')}`);
  process.exit(1);
}
console.log(`haptics-check: OK — ${entries.length} files, every haptic goes through src/haptics.rs`);
