#!/usr/bin/env node
// Self-hosted-fonts regression guard (FIX 1 — the "janky" cold-load fix).
// The site used to pull 3 font resources at runtime from Google Fonts +
// jsdelivr; none were service-worker precached, so every cold load re-hit
// those hosts and text reflowed (FOUT). This lint fails the build if any of
// that regresses. Three assertions, run against the source (and dist-test/ if
// it has been built):
//   1. ZERO external font hosts (googleapis / gstatic / jsdelivr) in the HTML/SW.
//   2. Every ./fonts/* file referenced by an @font-face is in sw.js precache.
//   3. sw.js CACHE_VERSION was bumped to at least v39 (fonts added to cache).
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
let fails = 0;
const fail = (m) => { fails++; console.error('  ✗ ' + m); };

const EXTERNAL = /googleapis|gstatic|jsdelivr/i;
const MIN_CACHE_VERSION = 39;

// Check a given (index.html, sw.js) pair — used for source and built copies.
function checkPair(label, htmlPath, swPath) {
  const html = readFileSync(htmlPath, 'utf8');
  const sw = readFileSync(swPath, 'utf8');

  // 1. No external font hosts anywhere in the shipped HTML or service worker.
  for (const [name, txt] of [['index.html', html], ['sw.js', sw]]) {
    if (EXTERNAL.test(txt)) {
      fail(`${label}: external font host still present in ${name} ` +
        `(${(txt.match(EXTERNAL) || [])[0]}) — fonts must be self-hosted`);
    }
  }

  // 2. Every ./fonts/* referenced by @font-face must be precached by sw.js.
  const referenced = [...html.matchAll(/url\(\s*['"]?(\.\/fonts\/[^'")]+)['"]?\s*\)/g)]
    .map((m) => m[1]);
  if (referenced.length === 0) fail(`${label}: no local ./fonts/* @font-face src found in index.html`);
  for (const ref of referenced) {
    if (!sw.includes(ref.replace('./', './'))) {
      fail(`${label}: font ${ref} referenced in index.html but missing from sw.js precache`);
    }
  }

  // 3. CACHE_VERSION bumped (fonts added → old caches must be invalidated).
  const v = parseInt((sw.match(/CACHE_VERSION\s*=\s*["']v(\d+)["']/) || [])[1] || '0', 10);
  if (v < MIN_CACHE_VERSION) {
    fail(`${label}: sw.js CACHE_VERSION v${v} < v${MIN_CACHE_VERSION} — bump it so font cache refreshes`);
  }
}

checkPair('source', join(ROOT, 'index.html'), join(ROOT, 'sw.js'));

// Also validate a built bundle if one is present (belt-and-suspenders: proves
// the build actually copied fonts/ and kept the rewritten HTML).
for (const dist of ['dist', 'dist-test']) {
  const d = join(ROOT, dist);
  if (existsSync(join(d, 'index.html'))) {
    checkPair(dist, join(d, 'index.html'), join(d, 'sw.js'));
    // The woff2/woff files themselves must exist in the built bundle.
    const html = readFileSync(join(d, 'index.html'), 'utf8');
    for (const m of html.matchAll(/url\(\s*['"]?\.\/(fonts\/[^'")]+)['"]?\s*\)/g)) {
      if (!existsSync(join(d, m[1]))) fail(`${dist}: built bundle missing font file ${m[1]}`);
    }
  }
}

if (fails) {
  console.error(`\nfont-selfhost-check: ${fails} problem(s) — external font stalls / FOUT can regress.`);
  process.exit(1);
}
console.log('font-selfhost-check: OK — 0 external font hosts, all local fonts precached, cache bumped.');
