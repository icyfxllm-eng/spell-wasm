#!/usr/bin/env node
// CC-VISUAL-ECLIPSE AT-2 / I5: no raw style value outside design/tokens.json.
//
// COLOURS: zero raw colours anywhere a player's screen is styled from, except
// on a line marked `token-exempt: <reason>` (a CSS /* */ or Rust // comment on
// that line, or inside an HTML <!-- token-exempt:begin: <reason> --> ...
// <!-- token-exempt:end --> block). The number of exempt colours is a ratchet:
// it may only go down (design/token-ratchet.json holds the ceiling).
//
// DURATIONS: raw transition/animation durations are counted against a ceiling
// that may only go down. Phase A4 brings them onto motion tokens and the
// ceiling to zero.
//
// Scanned: index.html (stylesheet outside the generated token block, inline
// style attributes, SVG paint attributes), the standalone notices/privacy
// pages, and every Rust source except the generated src/tokens.rs and
// #[cfg(test)] modules.
//
//   node scripts/token-ratchet.mjs            # check
//   node scripts/token-ratchet.mjs --update   # lower the ceilings to today's counts (never raises)
//   node scripts/token-ratchet.mjs --selftest
import { readFileSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const CEILING_FILE = join(ROOT, 'design/token-ratchet.json');
// A colour literal. `&#123;` HTML entities and `#[cfg(...)]` attributes are
// excluded by the lookbehind / the hex-only body.
const COLOUR = /(?<![&\w])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})(?![\w-])|\b(?:rgba?|hsla?)\(/g;
const DURATION_DECL = /(?:^|[;{\s])(?:transition|animation)(?:-duration|-delay)?\s*:\s*([^;}]*)/g;
const DURATION = /(?<![\w.-])\d*\.?\d+m?s\b/g;
const EXEMPT = /token-exempt:\s*[\w(]/;
const CSS_BLOCK_BEGIN = /\/\*\s*token-exempt:begin:\s*[\w(]/;
const CSS_BLOCK_END = /token-exempt:end\s*\*\//;

const blank = (s) => s.replace(/[^\n]/g, ' ');

/** CSS text (no <style> tags). Returns {raw: [{line, text}], exempt, durations}. */
export function scanCss(css, lineOffset = 0) {
  const out = { raw: [], exempt: 0, durations: 0 };
  const lines = css.split('\n');
  // Comments are blanked for matching, but a line's own comment can exempt it.
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, blank).split('\n');
  let inBlock = false;
  clean.forEach((l, i) => {
    if (CSS_BLOCK_BEGIN.test(lines[i])) inBlock = true;
    // Only declaration values can hold colours: text after a ':' up to ; or }.
    const values = [...l.matchAll(/:([^;{}]*)/g)].map((m) => m[1]).join(' ');
    const n = (values.match(COLOUR) || []).length;
    if (n) {
      if (inBlock || EXEMPT.test(lines[i])) out.exempt += n;
      else out.raw.push({ line: i + 1 + lineOffset, text: lines[i].trim().slice(0, 100) });
    }
    for (const m of l.matchAll(DURATION_DECL)) out.durations += (m[1].match(DURATION) || []).length;
    if (CSS_BLOCK_END.test(lines[i])) inBlock = false;
  });
  return out;
}

/** HTML markup outside <style>: inline style="", SVG paint attributes, swatch data. */
export function scanMarkup(html, lineOffset = 0) {
  const out = { raw: [], exempt: 0, durations: 0 };
  const lines = html.split('\n');
  let inExempt = false;
  lines.forEach((l, i) => {
    if (/<!--\s*token-exempt:begin:\s*[\w(]/.test(l)) inExempt = true;
    const attrs = [...l.matchAll(/\b(?:style|fill|stroke|stop-color|color|data-c|value)\s*=\s*"([^"]*)"/g)].map((m) => m[1]).join(' ');
    const n = (attrs.match(COLOUR) || []).length;
    if (n) {
      if (inExempt || EXEMPT.test(l)) out.exempt += n;
      else out.raw.push({ line: i + 1 + lineOffset, text: l.trim().slice(0, 100) });
    }
    for (const m of attrs.matchAll(DURATION_DECL)) out.durations += (m[1].match(DURATION) || []).length;
    if (/<!--\s*token-exempt:end\s*-->/.test(l)) inExempt = false;
  });
  return out;
}

/** Rust source: string literals only, outside comments and #[cfg(test)] modules. */
export function scanRust(src) {
  const out = { raw: [], exempt: 0, durations: 0 };
  const lines = src.split('\n');
  let skipDepth = -1;
  let depth = 0;
  let pendingTest = false;
  lines.forEach((l, i) => {
    const code = l.replace(/\/\/.*$/, '');
    if (/^\s*#\[cfg\(test\)\]/.test(l)) pendingTest = true;
    const opens = (code.match(/\{/g) || []).length;
    const closes = (code.match(/\}/g) || []).length;
    if (pendingTest && /\bmod\s+\w+/.test(code) && opens) { skipDepth = depth; pendingTest = false; }
    else if (pendingTest && /\S/.test(code) && !/^\s*#\[/.test(l)) pendingTest = false;
    const skipping = skipDepth >= 0;
    if (!skipping) {
      const strings = [...code.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => m[1]).join(' ');
      const n = (strings.match(COLOUR) || []).length;
      if (n) {
        if (EXEMPT.test(l)) out.exempt += n;
        else out.raw.push({ line: i + 1, text: l.trim().slice(0, 100) });
      }
    }
    depth += opens - closes;
    if (skipDepth >= 0 && depth <= skipDepth) skipDepth = -1;
  });
  return out;
}

function walk(dir, ext, acc = []) {
  for (const e of readdirSync(dir)) {
    const f = join(dir, e);
    if (statSync(f).isDirectory()) walk(f, ext, acc);
    else if (f.endsWith(ext)) acc.push(f);
  }
  return acc;
}

export function scanRepo(root = ROOT) {
  const files = [];
  const html = readFileSync(join(root, 'index.html'), 'utf8');
  const s0 = html.indexOf('<style>') + '<style>'.length;
  const s1 = html.indexOf('</style>');
  let css = html.slice(s0, s1);
  // The generated token block is where raw values are SUPPOSED to live.
  const b0 = css.indexOf('/* tokens:begin');
  const b1 = css.indexOf('/* tokens:end */');
  if (b0 >= 0 && b1 > b0) css = css.slice(0, b0) + blank(css.slice(b0, b1)) + css.slice(b1);
  const cssOffset = html.slice(0, s0).split('\n').length - 1;
  files.push(['index.html <style>', scanCss(css, cssOffset)]);
  files.push(['index.html markup', scanMarkup(html.slice(0, s0 - '<style>'.length) + blank(html.slice(s0 - 7, s1 + 8)) + html.slice(s1 + 8))]);
  for (const page of ['notices.html', 'privacy.html']) {
    const p = readFileSync(join(root, page), 'utf8');
    const a = p.indexOf('<style>');
    const b = p.indexOf('</style>');
    const r = a >= 0 ? scanCss(p.slice(a + 7, b), p.slice(0, a + 7).split('\n').length - 1) : { raw: [], exempt: 0, durations: 0 };
    files.push([`${page} <style>`, r]);
  }
  for (const f of walk(join(root, 'src'), '.rs')) {
    const rel = relative(root, f);
    if (rel === 'src/tokens.rs') continue;
    const r = scanRust(readFileSync(f, 'utf8'));
    if (r.raw.length || r.exempt) files.push([rel, r]);
  }
  return files;
}

if (process.argv.includes('--selftest')) {
  const cases = [
    ['a raw CSS colour is caught', scanCss('.a{color:#fff}').raw.length === 1],
    ['a token reference is clean', scanCss('.a{color:var(--x)}').raw.length === 0],
    ['an exempt line counts as exempt', scanCss('.a{color:#fff} /* token-exempt: reason */').exempt === 1],
    ['a bare exempt marker without a reason does not exempt', scanCss('.a{color:#fff} /* token-exempt: */').raw.length === 1],
    ['an id selector is not a colour', scanCss('#fade{opacity:1}').raw.length === 0],
    ['comments are not colours', scanCss('/* was #fff */ .a{color:var(--x)}').raw.length === 0],
    ['a CSS exempt block exempts', scanCss('/* token-exempt:begin: page */\n.a{color:#fff}\n/* token-exempt:end */\n.b{color:#000}').exempt === 1],
    ['durations are counted', scanCss('.a{transition:opacity .2s ease, transform 120ms}').durations === 2],
    ['an HTML entity is not a colour', scanMarkup('<b>&#175;</b>').raw.length === 0],
    ['an inline style colour is caught', scanMarkup('<div style="color:#000"></div>').raw.length === 1],
    ['an HTML exempt block exempts', scanMarkup('<!-- token-exempt:begin: picks -->\n<b data-c="#fff"></b>\n<!-- token-exempt:end -->').exempt === 1],
    ['a Rust literal is caught', scanRust('fn f() { x("#ffffff"); }').raw.length === 1],
    ['a Rust comment is not', scanRust('// "#ffffff"\nfn f() {}').raw.length === 0],
    ['cfg(test) modules are skipped', scanRust('#[cfg(test)]\nmod tests {\n  fn t() { x("#fff"); }\n}\nfn g() { y("#000"); }').raw.length === 1],
    ['a Rust exempt line', scanRust('fn f() { x("#fff"); } // token-exempt: OCR').exempt === 1],
  ];
  let bad = 0;
  for (const [name, ok] of cases) { console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name}`); if (!ok) bad++; }
  if (bad) { console.error(`token-ratchet selftest: ${bad} failed`); process.exit(1); }
  console.log('token-ratchet selftest: OK');
  process.exit(0);
}

const files = scanRepo();
const raw = files.flatMap(([f, r]) => r.raw.map((x) => `${f}:${x.line}  ${x.text}`));
const exempt = files.reduce((n, [, r]) => n + r.exempt, 0);
const durations = files.reduce((n, [, r]) => n + r.durations, 0);
const ceiling = JSON.parse(readFileSync(CEILING_FILE, 'utf8'));

if (process.argv.includes('--update')) {
  const next = { ...ceiling, exemptColours: Math.min(ceiling.exemptColours, exempt), rawDurations: Math.min(ceiling.rawDurations, durations) };
  writeFileSync(CEILING_FILE, JSON.stringify(next, null, 2) + '\n');
  console.log(`token-ratchet: ceilings now exempt colours ${next.exemptColours}, raw durations ${next.rawDurations}`);
  process.exit(0);
}

const problems = [];
if (raw.length) problems.push(`${raw.length} raw colour(s) outside design/tokens.json:\n    ` + raw.slice(0, 40).join('\n    '));
if (exempt > ceiling.exemptColours) problems.push(`token-exempt colours went UP: ${exempt} > ceiling ${ceiling.exemptColours}`);
if (durations > ceiling.rawDurations) problems.push(`raw durations went UP: ${durations} > ceiling ${ceiling.rawDurations} (motion tokens arrive in A4)`);
if (problems.length) {
  console.error('token-ratchet: FAILED');
  for (const p of problems) console.error('  ✗ ' + p);
  console.error('  Fix: add the value to design/tokens.json and reference it, or mark the line `token-exempt: <reason>`.');
  process.exit(1);
}
const lower = exempt < ceiling.exemptColours || durations < ceiling.rawDurations;
console.log(`token-ratchet: OK — 0 raw colours; ${exempt}/${ceiling.exemptColours} exempt colours; ${durations}/${ceiling.rawDurations} raw durations.` +
  (lower ? ' Counts fell: run --update to lower the ceilings.' : ''));
