#!/usr/bin/env node
// Production-safety guard: the E2E test seam (window.__spelltest) must NEVER be
// present in a shipped build. This greps the production dist/ (index.html + the
// compiled wasm + JS glue) for any trace of the seam and fails if found.
// Run against the PRODUCTION dist (built by scripts/build-web.sh, no
// --features testseam), not dist-test/.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dist = join(dirname(fileURLToPath(import.meta.url)), '..', process.argv[2] || 'dist');
const NEEDLES = ['__spelltest', 'testseam'];

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

let hits = 0;
for (const file of walk(dist)) {
  // Read wasm/js/html as latin1 so the wasm's embedded strings are scannable.
  const buf = readFileSync(file, 'latin1');
  for (const needle of NEEDLES) {
    if (buf.includes(needle)) {
      console.error(`  ✗ "${needle}" found in ${file.replace(dist, '.')}`);
      hits++;
    }
  }
}

if (hits) {
  console.error(`\nseam-absence-check: FAILED — ${hits} test-seam trace(s) in the production build.`);
  console.error('Production must be built WITHOUT --features testseam (scripts/build-web.sh).');
  process.exit(1);
}
console.log(`seam-absence-check: OK — no __spelltest/testseam trace in ${process.argv[2] || 'dist'}.`);
