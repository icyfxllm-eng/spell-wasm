#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F10 / AT-25: no flags and no national symbols anywhere.
// A language is a "room" with an accent colour, never a country's flag.
//
// Fails on (1) any regional-indicator character (the code points every flag
// emoji is built from) in the app's sources, strings, config or assets, and
// (2) any image file whose name says flag. Feature flags and the Board Game's
// finish line are words, not images, and do not trip it.
//
//   node scripts/flags-check.mjs
//   node scripts/flags-check.mjs --selftest
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, relative, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REGIONAL = /[\u{1F1E6}-\u{1F1FF}]/u;
const TEXT = new Set(['.rs', '.js', '.mjs', '.html', '.json', '.css', '.svg', '.md']);
const IMAGE = new Set(['.png', '.svg', '.webp', '.jpg', '.jpeg', '.gif']);
const SCAN = ['src', 'config', 'design', 'index.html', 'audio-native.js', 'icons', 'fonts', 'img', 'images', 'assets'];
const SKIP = /node_modules|target|dist|\.git|tests\/e2e\/visual/;

function walk(p, out) {
  let st;
  try { st = statSync(p); } catch { return; }
  if (SKIP.test(p)) return;
  if (st.isDirectory()) for (const n of readdirSync(p)) walk(join(p, n), out);
  else out.push(p);
}

export function check(files) {
  const bad = [];
  for (const [rel, read] of files) {
    const ext = extname(rel).toLowerCase();
    if (IMAGE.has(ext) && /flag/i.test(rel.split('/').pop())) bad.push(`${rel}: a flag image`);
    if (TEXT.has(ext)) {
      const text = read();
      const lines = text.split('\n');
      lines.forEach((l, i) => { if (REGIONAL.test(l)) bad.push(`${rel}:${i + 1}: a flag emoji`); });
    }
  }
  return bad;
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const paths = [];
  for (const s of SCAN) walk(join(ROOT, s), paths);
  const files = paths.map((p) => [relative(ROOT, p).replace(/\\/g, '/'), () => readFileSync(p, 'utf8')]);
  if (process.argv.includes('--selftest')) {
    const planted = check([['src/x.rs', () => 'let s = "🇫🇷 French";'], ['icons/flag-de.svg', () => '<svg/>']]);
    const clean = check([['src/flags.rs', () => 'pub fn visual_icons() -> bool'], ['src/board.rs', () => '<text class="bg-flag">']]);
    if (planted.length === 2 && clean.length === 0) {
      console.log('flags-check selftest: OK (a flag emoji and a flag image are caught; feature flags and the finish line pass)');
      process.exit(0);
    }
    console.error(`flags-check selftest: FAILED (planted ${planted.length}, clean ${clean.length})`);
    process.exit(1);
  }
  const bad = check(files);
  if (bad.length) {
    console.error(`flags-check: FAILED — no flags or national symbols (AT-25):\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`flags-check: OK — ${files.length} files, no flag emoji or flag images`);
}
