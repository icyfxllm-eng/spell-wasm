#!/usr/bin/env node
// CC-EDITIONS F3/D4 — the education artifact must contain ZERO purchase surface.
//
//   node scripts/edu-no-purchase-check.mjs <dist-dir>     # scan an education build
//   node scripts/edu-no-purchase-check.mjs --selftest     # prove it catches a leak
//
// D4 is the Little Speller invariant generalized: a school's build renders no
// paywall, no price strings, no StoreKit init, no "upgrade" copy. F3 asks for it
// as machine-checked fact rather than review-time diligence, and for the plugin
// to be EXCLUDED at build time, not stubbed.
//
// So this scans for the strings a purchase surface cannot exist without. It is a
// backstop, not the mechanism: the mechanism is `#[cfg(not(feature =
// "education"))]` on COMPLETE_PRODUCT_ID (and, when it lands, on the StoreKit
// adapter), which keeps the bytes out of the binary in the first place. This
// catches the day someone adds a price string without thinking about editions.
//
// Modelled on scripts/seam-absence-check.mjs, which does the same job for the
// E2E test seam — same shape, so the two stay comparable.
//
// NOTE ON WHAT THIS PROVES TODAY: the app currently has NO purchase surface in
// EITHER edition — no paywall, no price strings, no StoreKit, no IAP anywhere in
// the Rust tree. A pass here is therefore not yet evidence that the education
// exclusion WORKS; it is evidence that nothing has regressed. The gate earns its
// keep the moment the purchase surface is built, which is exactly when it is too
// late to add it.
import { readdirSync, readFileSync, statSync, existsSync, writeFileSync, mkdtempSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';
import { tmpdir } from 'node:os';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

// Strings that cannot appear in a build with no purchase surface.
//   * the product id is THE literal a purchase needs (cfg'd out in education);
//   * StoreKit/SKProduct are the iOS purchase API's own symbols;
//   * the price/paywall copy keys are what a paywall would render.
const NEEDLES = [
  'net.spellgame.complete',
  'StoreKit',
  'SKProduct',
  'SKPayment',
  'paywall',
  'restorePurchases',
  'price.',
  'iap.',
  'upgrade.',
];

const SKIP_EXT = /\.(map|md|txt|png|jpg|jpeg|svg|webp|woff2?|ttf|mp3|wav)$/i;

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else if (!SKIP_EXT.test(name)) out.push(p);
  }
  return out;
}

/** Scan a directory tree. Returns [{file, needle}]. Binary-safe: wasm is read as latin1 so string literals still match. */
function scan(dir) {
  const hits = [];
  for (const file of walk(dir)) {
    const text = readFileSync(file, 'latin1');
    for (const needle of NEEDLES) {
      if (text.includes(needle)) hits.push({ file: relative(dir, file), needle });
    }
  }
  return hits;
}

if (process.argv.includes('--selftest')) {
  // Negative test: a checker that cannot fail is decoration. Plant each needle in
  // a fixture and prove every one is caught.
  const tmp = mkdtempSync(join(tmpdir(), 'edu-check-'));
  let missed = [];
  for (const needle of NEEDLES) {
    writeFileSync(join(tmp, 'app.js'), `const x = "${needle}";`);
    if (scan(tmp).length === 0) missed.push(needle);
  }
  writeFileSync(join(tmp, 'app.js'), 'const x = "nothing to see";');
  const clean = scan(tmp).length === 0;
  if (missed.length || !clean) {
    if (missed.length) console.error(`edu-no-purchase-check --selftest: FAILED — needles not caught: ${missed.join(', ')}`);
    if (!clean) console.error('edu-no-purchase-check --selftest: FAILED — a clean fixture wrongly reported a hit');
    process.exit(1);
  }
  console.log(`edu-no-purchase-check --selftest: OK — all ${NEEDLES.length} needles caught, clean fixture passes.`);
  process.exit(0);
}

const target = process.argv[2];
if (!target) {
  console.error('usage: edu-no-purchase-check.mjs <dist-dir> | --selftest');
  process.exit(1);
}
const dir = join(ROOT, target);
if (!existsSync(dir)) {
  console.error(`edu-no-purchase-check: no such directory: ${target}`);
  process.exit(1);
}

const hits = scan(dir);
if (hits.length) {
  console.error(`edu-no-purchase-check: FAILED — ${hits.length} purchase-surface trace(s) in the EDUCATION artifact:`);
  for (const h of hits) console.error(`  ✗ "${h.needle}" in ${h.file}`);
  console.error('\nCC-EDITIONS D4: an education build must render ZERO purchase surfaces.');
  console.error('F3: exclude at BUILD TIME with #[cfg(not(feature = "education"))] — do not stub it out.');
  process.exit(1);
}
console.log(`edu-no-purchase-check: OK — no purchase-surface traces in ${target}.`);
