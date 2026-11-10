#!/usr/bin/env node
// Every full-screen mode keeps clear of the status bar and the home indicator.
//
// THE BUG THIS EXISTS FOR. Build 317 on Eric's iPhone: Word Chain's screen is
// `position:fixed; inset:0` with a flat 12px padding, so its top bar, and the
// Exit button in it, sat under the clock and the battery. The mode had an exit
// that nobody could tap. Impostor, Bee and Letter Forge had the same shape.
//
// WHAT IT CHECKS. In index.html's CSS, every `.<name>-screen` class with a
// `position:fixed;inset:0` rule must have some rule for that class that reads
// env(safe-area-inset-top). It does not judge the amount, only that the inset
// is read at all, which is the mistake that keeps recurring.
//
//   node scripts/safe-area-check.mjs
//   node scripts/safe-area-check.mjs --selftest

import { readFileSync } from 'node:fs';

export function problems(css) {
  const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({ sel: m[1], body: m[2].replace(/\s+/g, '') }));
  const screens = new Set();
  for (const r of rules) {
    if (!r.body.includes('position:fixed;inset:0')) continue;
    for (const m of r.sel.matchAll(/\.([a-z]+-screen)(?![\w-])/g)) screens.add(m[1]);
  }
  const bad = [];
  for (const s of [...screens].sort()) {
    const re = new RegExp(`\\.${s}(?![\\w-])`);
    const ok = rules.some((r) => re.test(r.sel) && r.body.includes('safe-area-inset-top'));
    if (!ok) bad.push(s);
  }
  return bad;
}

function selftest() {
  const good = '.a-screen{position:fixed;inset:0;padding:12px}\n.a-screen{padding:max(12px,env(safe-area-inset-top))}';
  const bad = '.b-screen{position:fixed;inset:0;padding:12px}';
  const ok = problems(good).length === 0 && problems(bad).join() === 'b-screen';
  if (!ok) { console.error('safe-area-check selftest: FAILED'); process.exit(1); }
  console.log('safe-area-check selftest: OK');
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
  const css = [...html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]).join('\n').replace(/\/\*[\s\S]*?\*\//g, '');
  const bad = problems(css);
  if (bad.length) {
    console.error(`safe-area-check: these full-screen modes never read env(safe-area-inset-top), so their top bar sits under the status bar: ${bad.join(', ')}`);
    process.exit(1);
  }
  console.log('safe-area-check: OK');
}
