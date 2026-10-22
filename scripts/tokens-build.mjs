#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F1: design/tokens.json is the single source of style values.
// This script generates the two places that consume it:
//
//   1. the token block in index.html, between the tokens:begin and tokens:end
//      markers (one :root rule plus one rule per $overrides selector), and
//   2. src/tokens.rs, the same values as Rust constants for code that draws
//      outside the stylesheet (canvas share cards, exports, runtime colour
//      pickers).
//
// The spec asks for a typed TS module; this app has no TypeScript, and its
// non-CSS consumers are Rust, so the typed module is Rust.
//
//   node scripts/tokens-build.mjs --write   # regenerate both outputs
//   node scripts/tokens-build.mjs           # check only: fail if either is out of date
//                                           # (pre-push, AT-1; --check is accepted too)
//   node scripts/tokens-build.mjs --selftest
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const BEGIN = '/* tokens:begin';
const END = '/* tokens:end */';

/** Every leaf ({$value}) as {path, css, value}, in file order. */
export function flatten(tree, prefix = []) {
  const out = [];
  for (const [k, v] of Object.entries(tree)) {
    if (k.startsWith('$')) continue;
    const path = [...prefix, k];
    if (v && typeof v === 'object' && '$value' in v) {
      out.push({ path: path.join('.'), css: v.$css || `--${path.join('-')}`, value: String(v.$value) });
    } else if (v && typeof v === 'object') {
      out.push(...flatten(v, path));
    }
  }
  return out;
}

export function validate(tokens) {
  const leaves = flatten(tokens);
  const problems = [];
  const seenCss = new Map();
  for (const l of leaves) {
    if (!/^--[a-z0-9-]+$/.test(l.css)) problems.push(`${l.path}: bad CSS name ${l.css}`);
    if (seenCss.has(l.css)) problems.push(`${l.path}: CSS name ${l.css} already used by ${seenCss.get(l.css)}`);
    seenCss.set(l.css, l.path);
    if (/[;{}]/.test(l.value)) problems.push(`${l.path}: value may not contain ; { or }`);
  }
  const byPath = new Map(leaves.map((l) => [l.path, l]));
  for (const [sel, map] of Object.entries(tokens.$overrides || {})) {
    for (const p of Object.keys(map)) if (!byPath.has(p)) problems.push(`$overrides ${sel}: unknown token ${p}`);
  }
  return { leaves, byPath, problems };
}

// App-only modes are cut from the site build by sentinel comments
// (scripts/build-web.sh). Their tokens are named after them, so each mode's
// tokens go in their own :root block inside that mode's sentinels -- a token
// called --mode-boardgame-* on the site is a Board Game string on the site.
export const WALLED = { boardgame: 'BOARDGAME', randospell: 'RANDOSPELL', spelluzzle: 'SPELLUZZLE', quickbee: 'QUICKBEEDRAW' };
export const wallOf = (path) => {
  const m = path.match(/^(?:motion\.)?mode\.([a-z]+)\./);
  return m && WALLED[m[1]] ? WALLED[m[1]] : null;
};

export function cssBlock(tokens) {
  const { leaves, byPath } = validate(tokens);
  const lines = [
    `  ${BEGIN}: GENERATED from design/tokens.json by scripts/tokens-build.mjs.`,
    '     Edit the JSON and run `node scripts/tokens-build.mjs --write`; never edit this block by hand. */',
    '  :root{',
    ...leaves.filter((l) => !wallOf(l.path)).map((l) => `    ${l.css}:${l.value};`),
    '  }',
  ];
  for (const wall of Object.values(WALLED)) {
    const own = leaves.filter((l) => wallOf(l.path) === wall);
    if (!own.length) continue;
    lines.push(`  /* ${wall}:BEGIN tokens (app only; stripped from the site build) */`);
    lines.push(`  :root{${own.map((l) => `${l.css}:${l.value}`).join(';')}}`);
    lines.push(`  /* ${wall}:END */`);
  }
  for (const [sel, map] of Object.entries(tokens.$overrides || {})) {
    const decls = Object.entries(map).map(([p, v]) => `${byPath.get(p).css}:${v}`).join(';');
    lines.push(`  ${sel}{${decls}}`);
  }
  lines.push(`  ${END}`);
  return lines.join('\n');
}

export function rustModule(tokens) {
  const { leaves } = validate(tokens);
  const ident = (p) => p.toUpperCase().replace(/[^A-Z0-9]+/g, '_');
  const esc = (s) => s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return [
    '//! GENERATED from design/tokens.json by scripts/tokens-build.mjs. Do not edit;',
    '//! edit the JSON and regenerate (the pre-push hook checks this file is current).',
    '//!',
    '//! Style values for Rust code that draws outside the stylesheet. `css` is the',
    '//! custom property the stylesheet uses, `value` its default (before any',
    '//! per-language override or runtime colour pick).',
    '#![allow(dead_code)]',
    '',
    '#[derive(Clone, Copy, Debug, PartialEq, Eq)]',
    'pub struct Token {',
    '    pub css: &\'static str,',
    '    pub value: &\'static str,',
    '}',
    '',
    ...leaves.map((l) => `pub const ${ident(l.path)}: Token = Token { css: "${l.css}", value: "${esc(l.value)}" };`),
    '',
  ].join('\n');
}

export function spliceBlock(html, block) {
  const a = html.indexOf(BEGIN);
  const b = html.indexOf(END);
  if (a < 0 || b < 0 || b < a) throw new Error('index.html has no tokens:begin ... tokens:end markers');
  const lineStart = html.lastIndexOf('\n', a) + 1;
  return html.slice(0, lineStart) + block + html.slice(b + END.length);
}

// Run only when invoked directly: other checks import flatten() from here.
const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN && process.argv.includes('--selftest')) {
  const t = {
    color: { bg: { $value: '#000', $css: '--bg' }, x: { y: { $value: 'rgba(1,2,3,.5)' } } },
    $overrides: { ':root[data-lang="ja"]': { 'color.bg': '#111' } },
  };
  const cases = [
    ['flattens with $css and path names', () => {
      const l = flatten(t);
      return l.length === 2 && l[0].css === '--bg' && l[1].css === '--color-x-y';
    }],
    ['emits the overrides rule', () => cssBlock(t).includes(':root[data-lang="ja"]{--bg:#111}')],
    ['rejects an unknown override', () => validate({ ...t, $overrides: { ':root': { 'color.nope': '#1' } } }).problems.length === 1],
    ['rejects a duplicate CSS name', () => validate({ a: { $value: '#1', $css: '--x' }, b: { $value: '#2', $css: '--x' } }).problems.length === 1],
    ['rust consts', () => rustModule(t).includes('pub const COLOR_X_Y: Token = Token { css: "--color-x-y", value: "rgba(1,2,3,.5)" };')],
    ['splice replaces only the block', () => {
      const h = 'a\n  /* tokens:begin old */\nold\n  /* tokens:end */\nb';
      const out = spliceBlock(h, '  NEW');
      return out === 'a\n  NEW\nb';
    }],
  ];
  let bad = 0;
  for (const [name, f] of cases) {
    const ok = f();
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name}`);
    if (!ok) bad++;
  }
  if (bad) { console.error(`tokens-build selftest: ${bad} failed`); process.exit(1); }
  console.log('tokens-build selftest: OK');
  process.exit(0);
}

if (MAIN) main();

function main() {
const tokens = JSON.parse(readFileSync(join(ROOT, 'design/tokens.json'), 'utf8'));
const { leaves, problems } = validate(tokens);
if (problems.length) {
  console.error('tokens-build: design/tokens.json is invalid');
  for (const p of problems) console.error('  ✗ ' + p);
  process.exit(1);
}
const htmlPath = join(ROOT, 'index.html');
const rsPath = join(ROOT, 'src/tokens.rs');
const html = readFileSync(htmlPath, 'utf8');
const wantHtml = spliceBlock(html, cssBlock(tokens));
const wantRs = rustModule(tokens);
let haveRs = '';
try { haveRs = readFileSync(rsPath, 'utf8'); } catch { /* first run */ }

if (!process.argv.includes('--write')) {
  const stale = [];
  if (wantHtml !== html) stale.push('index.html token block');
  if (wantRs !== haveRs) stale.push('src/tokens.rs');
  if (stale.length) {
    console.error(`tokens-build: out of date with design/tokens.json: ${stale.join(', ')}. Run: node scripts/tokens-build.mjs --write`);
    process.exit(1);
  }
  console.log(`tokens-build: OK, ${leaves.length} tokens, index.html block and src/tokens.rs are current.`);
  process.exit(0);
}
writeFileSync(htmlPath, wantHtml);
writeFileSync(rsPath, wantRs);
console.log(`tokens-build: wrote ${leaves.length} tokens to index.html and src/tokens.rs`);
}
