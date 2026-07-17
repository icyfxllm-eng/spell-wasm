#!/usr/bin/env node
// CC-HINDI-PHASE0 F3 / D4 — reject precomposed nuqta letters in word-list files.
//
//   node scripts/devanagari-check.mjs            # scan the word-list sources
//   node scripts/devanagari-check.mjs --selftest # prove a planted U+0958 FAILS
//
// D4: canonical storage is NFC, and the precomposed nuqta letters U+0958–U+095F
// are COMPOSITION-EXCLUDED — NFC does not produce them, it yields base + U+093C.
// So a precomposed letter in a word list is never NFC output; it is something a
// tool or a person pasted in, and it would compare unequal to the identical word
// typed on a keyboard. `src/akshara.rs` proves the decomposition
// (nfc_decomposes_the_precomposed_nuqta_letters); this stops the shipped data
// disagreeing with it.
//
// GATE ONLY — no list content is authored here (D8 grants no such authority).
// Hindi has no word list yet, so today this scans the existing languages and
// passes trivially. It exists so that when Hindi content lands, the rule is
// already enforced rather than remembered.

import { readFileSync, readdirSync, existsSync, statSync, writeFileSync, mkdtempSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';
import { tmpdir } from 'node:os';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

// U+0958..U+095F — the composition-excluded precomposed nuqta letters.
const PRECOMPOSED = /[क़-य़]/;
const NAMES = {
  'क़': 'क़ QA', 'ख़': 'ख़ KHHA', 'ग़': 'ग़ GHHA', 'ज़': 'ज़ ZA',
  'ड़': 'ड़ DDDHA', 'ढ़': 'ढ़ RHA', 'फ़': 'फ़ FA', 'य़': 'य़ YYA',
};

/** Every shipped word-list source. */
function wordListFiles() {
  const dir = join(ROOT, 'assets', 'words');
  if (!existsSync(dir)) return [];
  const out = [];
  for (const lang of readdirSync(dir)) {
    const p = join(dir, lang);
    if (!statSync(p).isDirectory()) continue;
    for (const f of readdirSync(p)) {
      if (f.endsWith('.txt')) out.push(join(p, f));
    }
  }
  return out;
}

function scan(files) {
  const problems = [];
  for (const file of files) {
    const lines = readFileSync(file, 'utf8').split('\n');
    lines.forEach((line, i) => {
      if (!PRECOMPOSED.test(line)) return;
      for (const ch of line) {
        if (!PRECOMPOSED.test(ch)) continue;
        const cp = 'U+' + ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0');
        const decomposed = ch.normalize('NFC');
        problems.push(
          `${relative(ROOT, file)}:${i + 1}: precomposed nuqta ${cp} (${NAMES[ch] ?? '?'}) in ${JSON.stringify(line.trim())} — ` +
          `D4: store the NFC form (${[...decomposed].map((c) => 'U+' + c.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')).join(' ')})`,
        );
      }
    });
  }
  return problems;
}

if (process.argv.includes('--selftest')) {
  // A gate that cannot fail is decoration. Plant each excluded letter and prove
  // every one is caught, and that a clean file passes.
  const tmp = mkdtempSync(join(tmpdir(), 'deva-'));
  const missed = [];
  for (const ch of Object.keys(NAMES)) {
    writeFileSync(join(tmp, 'w.txt'), `कमल\n${ch}रूरी\n`);
    if (scan([join(tmp, 'w.txt')]).length === 0) missed.push(ch);
  }
  // The DECOMPOSED form is canonical and must pass — the gate rejects the
  // precomposed letter, not the nuqta itself.
  writeFileSync(join(tmp, 'w.txt'), 'ज़रूरी\n'.normalize('NFC'));
  const cleanOk = scan([join(tmp, 'w.txt')]).length === 0;
  if (missed.length || !cleanOk) {
    if (missed.length) console.error(`devanagari-check --selftest: FAILED — not caught: ${missed.map((c) => NAMES[c]).join(', ')}`);
    if (!cleanOk) console.error('devanagari-check --selftest: FAILED — the canonical DECOMPOSED form was wrongly rejected');
    process.exit(1);
  }
  console.log(`devanagari-check --selftest: OK — all ${Object.keys(NAMES).length} precomposed letters rejected; decomposed form accepted.`);
  process.exit(0);
}

const files = wordListFiles();
const problems = scan(files);
if (problems.length) {
  console.error(`devanagari-check: FAILED — ${problems.length} precomposed nuqta letter(s):`);
  for (const p of problems) console.error('  ✗ ' + p);
  console.error('\nD4: NFC is the canonical storage form. U+0958–U+095F are composition-excluded,');
  console.error('so NFC never emits them — a precomposed letter here would compare unequal to the');
  console.error('same word typed on a keyboard.');
  process.exit(1);
}
console.log(`devanagari-check: OK — ${files.length} word-list file(s), no precomposed nuqta letters.`);
