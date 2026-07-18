#!/usr/bin/env node
// Self-hosted-fonts regression guard (FIX 1 — the "janky" cold-load fix).
// The site used to pull 3 font resources at runtime from Google Fonts +
// jsdelivr; none were service-worker precached, so every cold load re-hit
// those hosts and text reflowed (FOUT). This lint fails the build if any of
// that regresses. Three assertions, run against the source (and dist-test/ if
// it has been built):
//   1. ZERO external font hosts (googleapis / gstatic / jsdelivr) in the HTML/SW.
//   2. Every EAGER ./fonts/* file referenced by an @font-face is in sw.js precache.
//   3. sw.js CACHE_VERSION was bumped to at least v39 (fonts added to cache).
//   4. Every LAZY font is NOT precached (see below) and really is unicode-range gated.
//
// Assertion 2 originally read "every referenced font". That was written when
// every font here was a Latin UI face needed on literally every cold load, so
// "referenced" and "needed immediately" were the same thing. CC-RTL D5 broke
// that equivalence: naskh-arabic.woff2 is a 94KB SCRIPT font behind a
// unicode-range, so a browser fetches it only when it actually has to render an
// Arabic glyph — which, while RTL_SUPPORTED is false and ar/fa are rtl_blocked,
// is never. Precaching it would push 94KB onto every English user to prepare for
// a language they cannot select, which is exactly what the unicode-range exists
// to avoid.
//
// Not precaching does NOT mean uncached: sw.js's fetch handler caches any
// successful same-origin GET, so the font is cached on first genuine use and
// offline Arabic works from then on. Precaching buys only the very first render,
// for everyone, forever.
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
let fails = 0;
const fail = (m) => { fails++; console.error('  ✗ ' + m); };

const EXTERNAL = /googleapis|gstatic|jsdelivr/i;
const MIN_CACHE_VERSION = 39;

// Fonts deliberately left OUT of the precache, each with a reason. These are
// asserted to be absent (not merely tolerated) — if someone adds one to
// STATIC_ASSETS to silence a warning, that is a real cold-load regression for
// every user and this check must catch it, which a mere exemption would not.
const LAZY = [
  {
    file: './fonts/naskh-arabic.woff2',
    why: 'Arabic script face (94KB) behind a unicode-range. ar/fa are rtl_blocked, so ' +
         'no LTR user ever renders a glyph in its range; the SW fetch handler caches it ' +
         'on first real use. Revisit if the UI itself is ever localised into Arabic — ' +
         'then it becomes an eager UI font and belongs in the precache.',
  },
  {
    file: './fonts/nastaliq-urdu.woff2',
    why: 'Urdu Nastaliq face (~436KB — Nastaliq carries ~1100 contextual glyphs). Same ' +
         'reasoning as naskh, more so given the size: ur is rtl_blocked, unicode-range ' +
         'gates the download to Urdu Arabic-script text, and precaching it would push ' +
         '436KB onto every user for a language none can select. Cached on first real use.',
  },
];
const isLazy = (ref) => LAZY.find((l) => l.file === ref);

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

  // 2/4. Eager fonts must be precached; lazy ones must NOT be.
  const referenced = [...html.matchAll(/url\(\s*['"]?(\.\/fonts\/[^'")]+)['"]?\s*\)/g)]
    .map((m) => m[1]);
  if (referenced.length === 0) fail(`${label}: no local ./fonts/* @font-face src found in index.html`);
  for (const ref of referenced) {
    const lazy = isLazy(ref);
    const precached = sw.includes(ref);
    if (!lazy && !precached) {
      fail(`${label}: font ${ref} referenced in index.html but missing from sw.js precache`);
    }
    if (lazy && precached) {
      fail(`${label}: font ${ref} is precached but is declared lazy — that pushes it onto ` +
           `EVERY cold load.\n      ${lazy.why}\n      If that reasoning no longer holds, remove it from LAZY in this script.`);
    }
    // A font is only safely lazy if the browser can actually tell when to skip
    // it. Without a unicode-range the face is a candidate for any text and may
    // be fetched on first paint — lazy in intent, eager in fact.
    if (lazy) {
      const face = (html.match(new RegExp(`@font-face\\{[^}]*${ref.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[^}]*\\}`)) || [])[0] || '';
      if (!/unicode-range\s*:/.test(face)) {
        fail(`${label}: font ${ref} is declared lazy but its @font-face has no unicode-range — ` +
             `nothing stops the browser fetching it on first paint`);
      }
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
console.log(`font-selfhost-check: OK — 0 external font hosts, all eager fonts precached, ` +
  `${LAZY.length} lazy font(s) correctly left out (${LAZY.map((l) => l.file.split('/').pop()).join(', ')}), cache bumped.`);
