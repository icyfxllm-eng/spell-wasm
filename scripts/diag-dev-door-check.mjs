#!/usr/bin/env node
// Instrumentation must never reach a player's screen.
//
// The voice-capture diagnostic (rms, thresholds, buffer counts, request counts)
// is written into #voiceSpellDiag, an overlay fixed across the top of the app.
// Only the raw-transcript trail was ever gated behind the dev door; the readout
// line itself was not, and the element carries its own background and padding
// so even EMPTY it drew a dark bar over the title. It shipped that way to
// TestFlight in build 270 and Eric found it, not a test.
//
// Two laws, because either one alone leaves a visible artefact:
//   1. every write into the diag element is behind the dev door
//   2. the element is `hidden` in the markup, so nothing shows before the first
//      write — and only a dev-door-gated path may unhide it
//
//   node scripts/diag-dev-door-check.mjs            # check the real tree
//   node scripts/diag-dev-door-check.mjs --selftest # prove it bites
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';
export const JS = 'native-language-kit.js';
export const HTML = 'index.html';
const EL = 'voiceSpellDiag';

// The dev-door test, however it is spelled, must appear between the start of a
// handler and the write it guards.
const DOOR = /devDoorOpen\(\)|spell_dev_entitlements/;

export function check(js, html) {
  const bad = [];

  // 2. the element ships hidden
  const tag = (html.match(new RegExp(`<div[^>]*\\bid="${EL}"[^>]*>`)) || [])[0];
  if (!tag) {
    bad.push(`#${EL} is gone from ${HTML} — if the diag moved, this gate must move with it`);
  } else if (!/\shidden(\s|=|>|$)/.test(tag.replace(/aria-hidden/g, ''))) {
    bad.push(`#${EL} is not hidden in the markup — it carries a background and padding, so it draws a bar over the title even when empty`);
  }

  // 1. every write is gated. Walk each handler body that touches the element.
  const lines = js.split('\n');
  const writes = [];
  lines.forEach((l, i) => {
    if (/\bel\.(textContent|innerHTML)\s*=/.test(l) || /\bel\.hidden\s*=\s*false/.test(l)) writes.push(i);
  });
  if (!writes.length) {
    bad.push(`nothing writes the diag in ${JS} — if the readout moved, this gate must move with it`);
  }
  for (const w of writes) {
    // Look back for the enclosing HANDLER — a named function or a sub(...)
    // listener — not merely the nearest `function`, which an inline callback
    // (a .map, a .filter) would otherwise satisfy and hide the real body.
    let start = w;
    while (start > 0 && !/\bsub\s*\(|function\s+[\w$]+\s*\(/.test(lines[start])) start--;
    const body = lines.slice(start, w + 1).join('\n');
    if (!DOOR.test(body)) {
      bad.push(`${JS}:${w + 1} writes the diag with no dev-door check above it (${lines[w].trim().slice(0, 56)})`);
    }
  }
  return { bad };
}

const GOOD_JS = `
      sub('letterDiag', function (d) {
        if (!devDoorOpen()) return;
        var el = document.getElementById('voiceSpellDiag');
        el.hidden = false;
        el.textContent = 'diag: ' + d.info;
      });
      function render(t) {
        if (!devDoorOpen()) return;
        var el = document.getElementById('voiceSpellDiag');
        el.hidden = false;
        el.textContent = t.join('  ');
      }
`;
const GOOD_HTML = `<div class="voice-spell-status" id="voiceSpellDiag" aria-hidden="true" hidden style="position:fixed"></div>`;

if (process.argv.includes('--selftest')) {
  const cases = {
    clean: { r: check(GOOD_JS, GOOD_HTML), want: null },
    // The exact build-270 leak.
    ungated_write: { r: check(GOOD_JS.replace("        if (!devDoorOpen()) return;\n        var el = document.getElementById('voiceSpellDiag');\n        el.hidden = false;\n        el.textContent = 'diag: ' + d.info;", "        var el = document.getElementById('voiceSpellDiag');\n        el.textContent = 'diag: ' + d.info;"), GOOD_HTML), want: /no dev-door check/ },
    not_hidden_in_markup: { r: check(GOOD_JS, GOOD_HTML.replace(' hidden', '')), want: /not hidden in the markup/ },
    element_gone: { r: check(GOOD_JS, '<div id="other"></div>'), want: /is gone from/ },
    readout_gone: { r: check('var x = 1;', GOOD_HTML), want: /nothing writes the diag/ },
    ungated_unhide: { r: check(GOOD_JS.replace('      function render(t) {\n        if (!devDoorOpen()) return;', '      function render(t) {'), GOOD_HTML), want: /no dev-door check/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.r.bad.length === 0 : c.r.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.r.bad[0] ?? '') : (c.r.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.r.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 68) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`diag-dev-door-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('diag-dev-door-check selftest: OK');
  process.exit(0);
}

const { bad } = check(
  fs.readFileSync(path.join(ROOT, JS), 'utf8'),
  fs.readFileSync(path.join(ROOT, HTML), 'utf8'),
);
if (bad.length) {
  console.error('diag-dev-door-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log('diag-dev-door-check: OK — the capture diagnostic is hidden in the markup and every write to it is behind the dev door.');
