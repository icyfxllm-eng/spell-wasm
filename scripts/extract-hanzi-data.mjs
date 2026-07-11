#!/usr/bin/env node
// Extract ONLY the characters SpellGame actually uses from hanzi-writer-data
// into a local, shippable data dir — so we bundle ~KBs, not the full ~40MB set,
// and play fully offline with no CDN (mainland-China reachable, App-Store-safe).
//
// Source of chars: the zh word bank (hanzi after '|') in src/words.rs. Japanese
// is skipped for v1 — the ja pool is all kana (no kanji to write yet).
//
//   node scripts/extract-hanzi-data.mjs
import { readFileSync, writeFileSync, mkdirSync, existsSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const OUT = join(root, 'hanzi-data');            // shipped: copied to dist by build-web.sh
const DATA = join(root, 'node_modules', 'hanzi-writer-data');

// Collect unique CJK ideographs from the zh word bank.
const src = readFileSync(join(root, 'src', 'words.rs'), 'utf8');
const chars = new Set();
for (const m of src.matchAll(/"[^"]*\|([^"]+)"/g)) {
  for (const ch of m[1]) {
    const cp = ch.codePointAt(0);
    if (cp >= 0x4e00 && cp <= 0x9fff) chars.add(ch); // CJK Unified Ideographs
  }
}

if (!existsSync(DATA)) {
  console.error(`hanzi-writer-data not found at ${DATA}. Run: npm install hanzi-writer-data`);
  process.exit(1);
}
mkdirSync(OUT, { recursive: true });

let ok = 0, missing = [], bytes = 0;
for (const ch of [...chars].sort()) {
  const srcFile = join(DATA, `${ch}.json`);
  if (!existsSync(srcFile)) { missing.push(ch); continue; }
  const data = readFileSync(srcFile);
  writeFileSync(join(OUT, `${ch}.json`), data);
  bytes += data.length;
  ok++;
}

console.log(`extracted ${ok}/${chars.size} chars → hanzi-data/ (${(bytes / 1024).toFixed(1)} KB)`);
if (missing.length) console.warn(`missing stroke data (fall back to keyboard): ${missing.join(' ')}`);
