#!/usr/bin/env node
// Production-safety guard: neither the E2E test seam (window.__spelltest) nor
// the review-gated audit build (--features audit) may EVER be present in a
// shipped build. This greps the production dist/ (index.html + the compiled wasm
// + JS glue) for any trace of them and fails if found. Run against the
// PRODUCTION dist (built by scripts/build-web.sh with no --features), not
// dist-test/ or dist-audit/.
//
// The audit needles are the audit build's own identifiers/i18n keys/storage
// keys (Feature 1 + Feature 7). They are specific enough not to collide with
// ordinary content (unlike the bare word "audit"), so a zero count is a hard
// proof that the audit path — the Filipino unlock, the AUDIT badge, the
// preselect and the first-launch banner — left no byte in production.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dist = join(dirname(fileURLToPath(import.meta.url)), '..', process.argv[2] || 'dist');
const NEEDLES = [
  '__spelltest',
  'testseam',
  'auditBanner',
  'audit.banner',
  'audit.badge',
  'spellgame.audit',
  'audit_pinned_options',
];

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
  console.error(`\nseam-absence-check: FAILED — ${hits} test-seam / audit-build trace(s) in the production build.`);
  console.error('Production must be built with NO --features (scripts/build-web.sh, no AUDIT_LANGS).');
  process.exit(1);
}
console.log(`seam-absence-check: OK — no test-seam / audit-build trace in ${process.argv[2] || 'dist'}.`);
