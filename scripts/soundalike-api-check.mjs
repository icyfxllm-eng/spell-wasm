#!/usr/bin/env node
// CC-SENSE-CUE v1.1 I1 / F7 -- one table, one API.
//
// Fails if any code file other than the three API modules and the generator
// names a sound-alike table file (assets/words/<lang>/soundalikes.json) or the
// retired plain-text format (homophones.txt). Consumers go through
// soundalikes(): src/soundalikes.rs, tools/soundalikes.py,
// scripts/lib/soundalikes.mjs.
//
//   node scripts/soundalike-api-check.mjs
//   node scripts/soundalike-api-check.mjs --selftest
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const ALLOW = new Set([
  'src/soundalikes.rs',
  'tools/soundalikes.py',
  'scripts/lib/soundalikes.mjs',
  'tools/build_collisions.py',          // the generator writes the tables
  'scripts/soundalike-api-check.mjs',   // names the patterns it forbids
]);
const CODE = /\.(rs|py|mjs|js|cjs|sh|swift|kt|ts)$/;
const PATTERN = /soundalikes\.json|homophones\.txt/;

export function scan(files, read) {
  const bad = [];
  for (const f of files) {
    if (!CODE.test(f) || ALLOW.has(f)) continue;
    if (f.startsWith('node_modules/') || f.startsWith('dist') || f.startsWith('pkg') || f.startsWith('target/')) continue;
    const lines = read(f).split('\n');
    lines.forEach((l, i) => { if (PATTERN.test(l)) bad.push(`${f}:${i + 1}: ${l.trim().slice(0, 120)}`); });
  }
  return bad;
}

function selftest() {
  const dir = mkdtempSync(join(tmpdir(), 'sa-api-'));
  mkdirSync(join(dir, 'src'), { recursive: true });
  writeFileSync(join(dir, 'src/rogue.rs'), 'const T: &str = include_str!("../assets/words/en/homophones.txt");\n');
  writeFileSync(join(dir, 'src/soundalikes.rs'), 'include_str!("../assets/words/en/soundalikes.json")\n');
  writeFileSync(join(dir, 'src/ok.rs'), 'fn f() { crate::soundalikes::soundalikes("en"); }\n');
  const files = ['src/rogue.rs', 'src/soundalikes.rs', 'src/ok.rs'];
  const bad = scan(files, (f) => readFileSync(join(dir, f), 'utf8'));
  if (bad.length !== 1 || !bad[0].startsWith('src/rogue.rs')) {
    console.error('soundalike-api-check selftest FAILED: expected exactly the rogue reader, got', bad);
    process.exit(1);
  }
  console.log('soundalike-api-check selftest: OK (a direct table read outside the API is caught)');
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const files = execFileSync('git', ['ls-files'], { encoding: 'utf8' }).split('\n').filter(Boolean);
  const bad = scan(files, (f) => { try { return readFileSync(f, 'utf8'); } catch { return ''; } });
  if (bad.length) {
    console.error('soundalike-api-check: FAILED -- read sound-alike tables through soundalikes(), not the raw files:');
    for (const b of bad) console.error('  ' + b);
    process.exit(1);
  }
  console.log('soundalike-api-check: OK -- every reader goes through the soundalikes() API');
}
