#!/usr/bin/env node
// CC-LAUNCH-EN A10.1 / item 5: the feature-flag audit.
//
// Reads every runtime flag in src/flags.rs (each `resolve(stored("name"), default)`)
// and checks it against the committed release table, config/flags.release.json.
// It fails if
//   - a flag exists in code but not in the table, or the table has a flag the
//     code no longer has, so a new flag cannot ship unreviewed;
//   - a flag's compiled default differs from the table's "default";
//   - a flag has no ship/off decision ("ship": null), or is marked
//     "ship": false but defaults on.
//   - the remote kill file can do anything but switch flags off (src/killflags.rs).
//
// `node scripts/flags-audit.mjs --write` regenerates the table from code. It
// keeps every "ship" and "note" field already there. A new default-off flag
// gets "ship": false; a new default-on flag gets "ship": null, which fails
// until Eric decides it.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = join(ROOT, 'src/flags.rs');
const TABLE = join(ROOT, 'config/flags.release.json');

/** Every flag in flags.rs as { name, default }. Parametric flags keep their pattern. */
export function parseFlags(src) {
  const out = new Map();
  const re = /resolve\(stored\((?:"([A-Za-z0-9_]+)"|&format!\("([A-Za-z0-9_]+)\{(\w+)\}"\))\)\.as_deref\(\),\s*([^)]+?)\)/g;
  for (const m of src.matchAll(re)) {
    const name = m[1] ?? `${m[2]}{${m[3]}}`;
    const def = m[4].trim();
    if (out.has(name) && out.get(name) !== def) throw new Error(`flag ${name} has two defaults`);
    out.set(name, def);
  }
  // Quick Bee Draw reads one key per language from its LangCfg table, every one
  // default off (src/flags.rs quickbee_draw_lang / quickbee_draw_any).
  const qbd = readFileSync(join(ROOT, 'src/quickbee_draw/mod.rs'), 'utf8');
  const qbdReads = (src.match(/resolve\(stored\(c\.flag_key\)\.as_deref\(\), false\)/g) || []).length;
  for (const m of qbd.matchAll(/flag_key: "([A-Za-z0-9_]+)"/g)) out.set(m[1], 'false');
  const calls = (src.match(/resolve\(stored\(/g) || []).length;
  const seen = [...src.matchAll(re)].length + qbdReads;
  if (calls !== seen) throw new Error(`flags.rs has ${calls} flag reads but the audit parsed ${seen}; extend the parser`);
  return out;
}

/** Problems as messages; empty means pass. */
export function audit(flags, table, killSrc) {
  const bad = [];
  for (const [name, def] of flags) {
    const row = table[name];
    if (!row) { bad.push(`${name}: in src/flags.rs but not in config/flags.release.json`); continue; }
    if (row.default !== def) bad.push(`${name}: code default is ${def}, table says ${row.default}`);
    if (row.ship === null || row.ship === undefined) bad.push(`${name}: no ship/off decision yet (Eric initials the ship rows, O5)`);
    else if (row.ship === false && def !== 'false') bad.push(`${name}: marked ship:false but its default is ${def}`);
  }
  for (const name of Object.keys(table)) if (!flags.has(name)) bad.push(`${name}: in the table but not in src/flags.rs`);
  // The kill file can only turn flags off: is_killed returns a bool that stored()
  // maps to Some("off") and nothing else.
  if (!/is_killed\(name\)\s*\{\s*return Some\("off"\.to_string\(\)\);/.test(readFileSync(SRC, 'utf8'))) {
    bad.push('flags.rs: the kill-file branch no longer maps to "off" only');
  }
  if (/Some\("on"/.test(killSrc)) bad.push('killflags.rs: the remote file can produce "on"');
  return bad;
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const flags = parseFlags(readFileSync(SRC, 'utf8'));
  const table = existsSync(TABLE) ? JSON.parse(readFileSync(TABLE, 'utf8')) : {};
  if (process.argv.includes('--write')) {
    const next = {};
    for (const [name, def] of [...flags].sort(([a], [b]) => a.localeCompare(b))) {
      const old = table[name] || {};
      // A default-off flag ships off: nothing to decide. A flag that is on (or
      // conditionally on) by default waits for Eric's ship/off decision.
      const ship = old.ship !== undefined && old.ship !== null ? old.ship : (def === 'false' ? false : null);
      next[name] = { default: def, ship, note: old.note ?? '' };
    }
    writeFileSync(TABLE, JSON.stringify(next, null, 2) + '\n');
    console.log(`flags-audit: wrote ${Object.keys(next).length} flags to config/flags.release.json`);
    process.exit(0);
  }
  const bad = audit(flags, table, readFileSync(join(ROOT, 'src/killflags.rs'), 'utf8'));
  if (bad.length) {
    console.error(`flags-audit: ${bad.length} problem(s)`);
    for (const b of bad) console.error('  ' + b);
    process.exit(1);
  }
  console.log(`flags-audit: OK, ${flags.size} flags match config/flags.release.json`);
}
