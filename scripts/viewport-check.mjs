#!/usr/bin/env node
// iOS zoom-bug regression guards (see the fix in index.html + FRONTEND README).
// iOS WKWebView auto-zooms on a focused field with font-size < 16px and often
// fails to restore scale. This lint exists because of a real shipped bug —
// DO NOT delete it. Three assertions, run against the source index.html:
//   1. viewport meta locks scale (maximum-scale=1, user-scalable=no, viewport-fit=cover)
//   2. no focusable-element CSS rule sets font-size below 16px
//   3. keyboard keys + buttons opt out of double-tap zoom (touch-action:manipulation)
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const html = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', 'index.html'), 'utf8');
let fails = 0;
const fail = (m) => { fails++; console.error('  ✗ ' + m); };

// 1. viewport meta
const vp = (html.match(/<meta[^>]*name=["']viewport["'][^>]*content=["']([^"']+)["']/i) || [])[1] || '';
for (const req of ['maximum-scale=1', 'user-scalable=no', 'viewport-fit=cover']) {
  if (!vp.includes(req)) fail(`viewport meta missing "${req}" (got: ${vp || 'none'})`);
}

// 2. no sub-16px font-size on a focusable-element selector.
// Scan every CSS rule whose selector mentions input/textarea/select/contenteditable
// and flag a declared font-size in px below 16.
const css = (html.match(/<style>([\s\S]*?)<\/style>/i) || [])[1] || '';
const FOCUSABLE = /\b(input|textarea|select|\[contenteditable\])\b/;
for (const rule of css.split('}')) {
  const [selector, body = ''] = rule.split('{');
  if (!FOCUSABLE.test(selector)) continue;
  const m = body.match(/font-size:\s*(\d+(?:\.\d+)?)px/);
  if (m && parseFloat(m[1]) < 16) {
    fail(`font-size ${m[1]}px < 16 on focusable selector "${selector.trim().slice(0, 60)}" (iOS zoom trigger)`);
  }
}

// 3. touch-action:manipulation covers keyboard keys + buttons.
const hasTouchAction = /(?:\.kb-key|button)[^{]*\{[^}]*touch-action:\s*manipulation/.test(css) ||
  /touch-action:\s*manipulation/.test(css.match(/[^{}]*button[^{}]*\{[^}]*\}/g)?.join('') || '') ||
  /button[^{]*,[^{]*\{[^}]*touch-action:\s*manipulation/.test(css) ||
  css.includes('button, .kb-key');
if (!/touch-action:\s*manipulation/.test(css)) fail('no touch-action:manipulation rule (double-tap zoom guard)');
else if (!/(button|\.kb-key)/.test(css.split('touch-action').slice(0, -1).join('touch-action'))) {
  // best-effort: ensure a rule listing button/.kb-key sets it
  const rules = css.split('}').filter((r) => /touch-action:\s*manipulation/.test(r));
  if (!rules.some((r) => /button|\.kb-key|\.pill|\.btn/.test(r.split('{')[0]))) {
    fail('touch-action:manipulation not applied to keyboard keys / buttons');
  }
}

if (fails) {
  console.error(`\nviewport-check: ${fails} problem(s) — iOS zoom bug can regress.`);
  process.exit(1);
}
console.log('viewport-check: OK — viewport locked, all focusable fields ≥16px, tap-zoom guarded.');
