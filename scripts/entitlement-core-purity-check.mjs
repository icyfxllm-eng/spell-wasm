#!/usr/bin/env node
// CI purity gate for the single-source-of-truth entitlement doctrine.
//
// The platform strings `storefront`, `CF-IPCountry`, and the Complete product id
// may appear ONLY in the pure Rust core (src/entitlements.rs) and — in later
// phases — the thin StoreKit / Flask ADAPTERS. Anywhere else they signal a
// scattered `if(purchased)` / `if(country==)` check leaking outside the core,
// which is exactly what this project forbids. This scan FAILS the build if any
// needle appears outside the allowlisted core/adapter paths.
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

const NEEDLES = ['storefront', 'CF-IPCountry', 'net.spellgame.complete'];

// Source trees the doctrine governs (product code the app ships).
const SCAN_ROOTS = ['src', 'index.html', 'backend'];

// The ONLY places a needle may legitimately live. src/entitlements.rs is the
// pure core. The adapter paths do not exist yet (later phases) but are reserved
// here so the gate keeps passing when they land.
const ALLOWLIST = [
  'src/entitlements.rs',
  // --- reserved for later-phase adapters (the only other legal homes) ---
  'src/entitlements_adapter.rs',
  'ios/App/App/StoreKit',        // StoreKit purchase adapter (Swift)
  'backend/entitlements.py',     // Flask region adapter (reads CF-IPCountry + the same map)
  // An adapter's own tests must speak the platform's language to test it: the
  // region adapter cannot be tested without sending a CF-IPCountry header, and
  // the purchase adapter cannot be tested without the product id. Allowlisting
  // a test does NOT widen where the doctrine allows logic to live — these files
  // exercise an adapter, they are not a second home for it.
  'backend/test_entitlements.py',
  'src/entitlements_adapter_test.rs',
  'ios/App/AppTests/StoreKit',
];

const IGNORE_DIRS = new Set(['node_modules', 'target', 'dist', 'dist-test', 'pkg', 'pkg-test', '.git']);
const TEXT_EXT = /\.(rs|js|mjs|ts|swift|py|html|json|kt|java|xml)$/i;

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (IGNORE_DIRS.has(name)) continue;
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, out);
    else if (TEXT_EXT.test(name)) out.push(p);
  }
  return out;
}

function allowed(relPath) {
  return ALLOWLIST.some((a) => relPath === a || relPath.startsWith(a + '/') || relPath.startsWith(a));
}

let hits = 0;
for (const rootName of SCAN_ROOTS) {
  const abs = join(ROOT, rootName);
  if (!existsSync(abs)) continue;
  const files = statSync(abs).isDirectory() ? walk(abs) : [abs];
  for (const file of files) {
    const rel = relative(ROOT, file);
    if (allowed(rel)) continue;
    const text = readFileSync(file, 'utf8');
    for (const needle of NEEDLES) {
      if (text.includes(needle)) {
        console.error(`  ✗ "${needle}" found outside the entitlement core: ${rel}`);
        hits++;
      }
    }
  }
}

if (hits) {
  console.error(`\nentitlement-core-purity-check: FAILED — ${hits} leak(s).`);
  console.error('These strings belong ONLY in src/entitlements.rs (or the reserved adapter paths).');
  console.error('Ask the resolver (entitlements::resolve_entitlements) instead of adding a scattered check.');
  process.exit(1);
}
console.log('entitlement-core-purity-check: OK — no storefront/CF-IPCountry/product-id leaks outside the core.');
