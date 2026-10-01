#!/usr/bin/env node
// CC-SNAP-CLEAN I8 — one implementation.
//
// The cleanup that turns raw OCR text into word candidates lives in
// src/snap_clean.rs and nowhere else. Before this file there were four
// places doing pieces of it — a Swift line-heal, a Rust whitespace
// tokenizer, an edge trimmer, a per-chip split in the review screen — and
// they disagreed: one dropped every digit-bearing word, another kept them;
// one split "Week 3 Spelling List" into three words, the spec wants it
// flagged whole. A second implementation is how those disagreements come
// back.
//
// What this gate looks for is marker or punctuation stripping applied to OCR
// TEXT outside the module. It is deliberately narrow: `trim()` is everywhere
// and means nothing on its own, so the patterns below want a list marker, a
// bullet, or a non-letter edge trim.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;
const OWNER = 'src/snap_clean.rs';
const SELFTEST = process.argv.includes('--selftest');

/// Each pattern is a way of writing "strip a list marker or an edge".
const PATTERNS = [
  [/trim_matches\s*\(\s*\|?\s*c\s*:?\s*[^)]*is_alphabetic/, 'a non-letter edge trim (the old strip_edges)'],
  [/trim_start_matches\s*\(\s*(?:&?\[)?['"][•·◦▪‣⁃*]/u, 'a bullet strip'],
  [/\bparse_candidates\b/, 'the deleted parse_candidates'],
  [/\bstrip_edges\b/, 'the deleted strip_edges'],
];

/// A table of bullet glyphs is a marker stripper being born. Three or more
/// distinct ones in a file's code is not a coincidence — it is F1's list
/// being retyped somewhere else.
///
/// An earlier version of this gate looked for `split_whitespace` near an NFC
/// call instead, and flagged `norm.rs` (does a definition mention the answer?)
/// and `word_stories.rs` (tokenizing an etymology). Neither is OCR cleanup.
/// A gate that cries wolf gets bypassed, so it asks a narrower question now.
const BULLET_GLYPHS = ['\u{2022}', '\u{00B7}', '\u{25E6}', '\u{25AA}', '\u{2023}', '\u{2043}', '\u{25CB}', '\u{25CF}', '\u{25A0}', '\u{25A1}'];

function bulletTable(code) {
  const seen = BULLET_GLYPHS.filter((g) => code.includes(g));
  return seen.length >= 3;
}

/// Files that legitimately mention OCR text without cleaning it.
const ALLOW = new Set([OWNER, 'tests/snap_clean_golden.rs']);

function rustFiles(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) rustFiles(p, out);
    else if (name.endsWith('.rs')) out.push(p);
  }
  return out;
}

export function scan(files) {
  const problems = [];
  for (const [rel, src] of Object.entries(files)) {
    if (ALLOW.has(rel)) continue;
    // Comments may DISCUSS the old code — the deletion note in
    // native_lang.rs names both functions on purpose — so judge code only.
    const code = src
      .split('\n')
      .filter((l) => !l.trim().startsWith('//'))
      .join('\n');
    for (const [re, what] of PATTERNS) {
      if (re.test(code)) problems.push(`${rel}: ${what} — that belongs in ${OWNER}`);
    }
    if (bulletTable(code)) {
      problems.push(`${rel}: a table of bullet glyphs — that belongs in ${OWNER}`);
    }
  }
  return problems;
}

function loadReal() {
  const files = {};
  for (const p of rustFiles(join(ROOT, 'src'))) {
    files[relative(ROOT, p)] = readFileSync(p, 'utf8');
  }
  for (const p of rustFiles(join(ROOT, 'tests'))) {
    files[relative(ROOT, p)] = readFileSync(p, 'utf8');
  }
  return files;
}

if (!SELFTEST) {
  const problems = scan(loadReal());
  if (problems.length) {
    console.error('snap-clean-one-impl-check: FAILED\n' + problems.map((p) => `  ${p}`).join('\n'));
    process.exit(1);
  }
  console.log(
    `snap-clean-one-impl-check: OK — OCR cleanup lives only in ${OWNER}`,
  );
  process.exit(0);
}

const LESIONS = [
  ['the old edge trimmer, reintroduced', { 'src/other.rs': 'fn f(t: &str) -> &str { t.trim_matches(|c: char| !c.is_alphabetic()) }' }],
  ['a bullet strip somewhere else', { 'src/other.rs': "fn f(s: &str) { s.trim_start_matches('•'); }" }],
  ['a call to the deleted parser', { 'src/other.rs': 'fn f() { parse_candidates(&[]); }' }],
  ['a second bullet table', { 'src/other.rs': "const B: [char; 3] = ['\u{2022}', '\u{00B7}', '\u{25E6}'];" }],
];
let bad = 0;
if (scan(loadReal()).length !== 0) {
  console.error('snap-clean-one-impl-check: the REAL tree already fails; fix that first');
  process.exit(1);
}
// A comment naming the old functions must NOT trip it.
if (scan({ 'src/other.rs': '// parse_candidates and strip_edges used to live here\n' }).length !== 0) {
  console.error('  SURVIVED: a comment about the old code was flagged');
  bad++;
}
for (const [name, files] of LESIONS) {
  if (scan(files).length === 0) {
    console.error(`  SURVIVED: ${name}`);
    bad++;
  }
}
if (bad) {
  console.error(`snap-clean-one-impl-check: FAILED\n  ${bad} lesion(s) were not caught`);
  process.exit(1);
}
console.log(`snap-clean-one-impl-check: selftest OK — ${LESIONS.length} lesions fail the build, comments do not`);
