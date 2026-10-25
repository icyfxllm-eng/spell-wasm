#!/usr/bin/env node
// CC-GAME-SERVER A6.2: the room server may change the app's core crate by
// visibility, cfg and feature lines ONLY (spec section 3). This prints the
// core diff against a base and fails on any other kind of line.
//
// WHY IT EXISTS. The online grader is the app's own `boardgame_grade::grade`
// compiled natively. The whole fairness argument -- the server judges a word
// exactly as the app does -- holds only while the server's needs never move
// a line of logic in `spell_wasm`. A reviewer cannot be trusted to notice a
// one-character logic change hiding in a "just exposing it" diff; this can.
//
// WHAT COUNTS AS ALLOWED, line by line:
//   - blank lines and `//` comments
//   - `#[cfg(...)]` and `#[cfg_attr(...)]` attributes, and the attribute a
//     cfg_attr replaced (e.g. `#[wasm_bindgen(start)]`)
//   - `pub mod native_api;` and `pub use ...;`
//   - a visibility change: `- fn x` paired with `+ pub fn x` / `+ pub(crate) fn x`
//   - root Cargo.toml: the workspace `members` line, `native = []`,
//     `publish = false` (packaging metadata for cargo-deny), comments
// And src/native_api.rs, whatever the base, may hold only comments and
// `pub use` lines (it must never grow a body).
//
//   node scripts/game-server-core-diff-check.mjs <base-ref>
//   node scripts/game-server-core-diff-check.mjs --selftest

import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';

const CORE_PATHS = ['src', 'build.rs', 'Cargo.toml'];

const isComment = (l) => /^\s*(\/\/.*)?$/.test(l);
const isCfg = (l) => /^\s*#\[cfg(_attr)?\(.*\)\]\s*(\/\/.*)?$/.test(l);
const isReexport = (l) => /^\s*pub (mod native_api;|use [^;]+;)\s*(\/\/.*)?$/.test(l);
// `pub fn x` / `pub(crate) fn x` / `fn x` all reduce to `fn x`.
const unVis = (l) => l.trim().replace(/^pub(\((crate|super)\))? /, '');
const isCargoAllowed = (l) =>
  /^\s*(#.*)?$/.test(l) || /^members = \[.*\]$/.test(l.trim()) || /^native = \[\]$/.test(l.trim()) || /^publish = false$/.test(l.trim());

/** Classifies one file's -/+ lines; returns the offending lines. */
export function offending(file, removed, added) {
  const bad = [];
  const cargo = file === 'Cargo.toml';
  const addedSet = new Set(added.map((l) => l.trim()));
  const addedUnVis = new Set(added.map(unVis));
  const addedCfgAttr = added.filter((l) => /#\[cfg_attr\(/.test(l)).join('\n');
  for (const l of removed) {
    const t = l.trim();
    if (cargo ? isCargoAllowed(l) : isComment(l) || isCfg(l)) continue;
    if (addedUnVis.has(unVis(l)) && !addedSet.has(t)) continue; // only the visibility changed
    const attr = t.match(/^#\[(.*)\]$/);
    if (attr && addedCfgAttr.includes(attr[1])) continue; // attribute moved under cfg_attr
    bad.push(`- ${l}`);
  }
  const removedUnVis = new Set(removed.map(unVis));
  const removedSet = new Set(removed.map((l) => l.trim()));
  for (const l of added) {
    const t = l.trim();
    if (cargo ? isCargoAllowed(l) : isComment(l) || isCfg(l) || isReexport(l)) continue;
    if (removedUnVis.has(unVis(l)) && !removedSet.has(t)) continue; // only the visibility changed
    bad.push(`+ ${l}`);
  }
  return bad;
}

/** native_api.rs may hold only comments and `pub use` lines. */
export function nativeApiOffending(text) {
  return text
    .split('\n')
    .filter((l) => !(isComment(l) || /^\s*\/\/!/.test(l) || /^\s*pub use [^;]+;\s*$/.test(l)))
    .map((l) => `native_api.rs: ${l}`);
}

function parseDiff(diff) {
  const files = new Map();
  let cur = null;
  for (const line of diff.split('\n')) {
    if (line.startsWith('+++ ')) {
      cur = line.slice(4).replace(/^b\//, '');
      if (cur === '/dev/null') cur = null;
      if (cur && !files.has(cur)) files.set(cur, { removed: [], added: [] });
    } else if (line.startsWith('--- ')) {
      const from = line.slice(4).replace(/^a\//, '');
      if (from !== '/dev/null') {
        cur = from;
        if (!files.has(cur)) files.set(cur, { removed: [], added: [] });
      }
    } else if (cur && line.startsWith('+')) files.get(cur).added.push(line.slice(1));
    else if (cur && line.startsWith('-')) files.get(cur).removed.push(line.slice(1));
  }
  return files;
}

function run(base) {
  const diff = execFileSync('git', ['diff', '-U0', base, '--', ...CORE_PATHS], { encoding: 'utf8' });
  console.log(`# spell_wasm core diff vs ${base}\n${diff || '(empty)'}`);
  const bad = [];
  for (const [file, { removed, added }] of parseDiff(diff)) {
    if (file === 'src/native_api.rs') continue; // checked whole, below
    for (const b of offending(file, removed, added)) bad.push(`${file}: ${b}`);
  }
  if (existsSync('src/native_api.rs')) bad.push(...nativeApiOffending(readFileSync('src/native_api.rs', 'utf8')));
  if (bad.length) {
    console.error(`\ngame-server-core-diff-check: FAIL, ${bad.length} line(s) are not cfg/feature/visibility:`);
    for (const b of bad) console.error(`  ${b}`);
    process.exit(1);
  }
  console.log('\ngame-server-core-diff-check: PASS (every changed core line is cfg, feature, visibility or pub use)');
}

function selftest() {
  const cases = [
    ['src/lib.rs', ['#[wasm_bindgen(start)]'], ['#[cfg_attr(not(feature = "native"), wasm_bindgen(start))]'], 0],
    ['src/lib.rs', [], ['#[cfg(feature = "native")]', 'pub mod native_api; // re-exports'], 0],
    ['src/words.rs', ['fn tier_for(lang: &str) -> u8 {'], ['pub fn tier_for(lang: &str) -> u8 {'], 0],
    ['src/words.rs', ['fn tier_for(lang: &str) -> u8 {'], ['pub fn tier_for(lang: &str) -> u16 {'], 2],
    ['src/game.rs', ['pub(crate) fn voice(a: &str) {'], ['pub fn voice(a: &str) {'], 0],
    ['src/game.rs', ['pub(crate) fn voice(a: &str) {'], ['pub fn voice(a: &str, b: u8) {'], 2],
    ['src/game.rs', ['    if a > b {'], ['    if a >= b {'], 2],
    ['src/game.rs', [], ['    let x = 1;'], 1],
    ['Cargo.toml', ['members = ["."]'], ['members = [".", "server/spell-server"]', 'native = []', '# why'], 0],
    ['Cargo.toml', [], ['tokio = "1"'], 1],
  ];
  let fails = 0;
  for (const [file, rm, add, want] of cases) {
    const got = offending(file, rm, add).length;
    if (got !== want) {
      console.error(`selftest: ${file} ${JSON.stringify(add)} -> ${got} offending, want ${want}`);
      fails++;
    }
  }
  const api = nativeApiOffending('//! doc\n\npub use crate::a::b;\npub fn x() {}\n');
  if (api.length !== 1) {
    console.error(`selftest: native_api body not caught: ${api}`);
    fails++;
  }
  if (fails) process.exit(1);
  console.log('game-server-core-diff-check selftest: PASS');
}

const arg = process.argv[2];
if (arg === '--selftest') selftest();
else if (!arg) {
  console.error('usage: node scripts/game-server-core-diff-check.mjs <base-ref> | --selftest');
  process.exit(2);
} else run(arg);
