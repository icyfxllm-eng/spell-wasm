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
// CC-ECLIPSE-HOTFIX F4 adds three more. In 298 the readout, once shown, stayed
// up across every mode, and TestFlight and the App Store ship one binary:
//   3. the element, its dev-menu switch and every line that paints it sit
//      inside SPELL-DIAG sentinels, so a release build carries none of it (I7)
//   4. every release build (app, education, auditor) runs scripts/cut-diag.py,
//      and the Dockerfile cuts SPELL-DIAG for the site
//   5. the capture teardown hides it, so it never outlives the mic
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

const BEGIN = /SPELL-DIAG:BEGIN/;
const END = /SPELL-DIAG:END/;

/** Line indexes that sit inside SPELL-DIAG sentinels. */
function inRegion(lines) {
  const inside = new Set();
  let open = false;
  lines.forEach((l, i) => {
    if (BEGIN.test(l)) open = true;
    if (open) inside.add(i);
    if (END.test(l)) open = false;
  });
  return inside;
}

export function check(js, html, builds = null) {
  const bad = [];

  // 2. the element ships hidden
  const tag = (html.match(new RegExp(`<div[^>]*\\bid="${EL}"[^>]*>`)) || [])[0];
  if (!tag) {
    bad.push(`#${EL} is gone from ${HTML} — if the diag moved, this gate must move with it`);
  } else if (!/\shidden(\s|=|>|$)/.test(tag.replace(/aria-hidden/g, ''))) {
    bad.push(`#${EL} is not hidden in the markup — it carries a background and padding, so it draws a bar over the title even when empty`);
  }
  // 3. the markup (element and switch) is inside the sentinels
  const hl = html.split('\n');
  const hIn = inRegion(hl);
  hl.forEach((l, i) => {
    if ((l.includes(`id="${EL}"`) || l.includes('id="devDiagBar"')) && !hIn.has(i)) {
      bad.push(`${HTML}:${i + 1} is outside the SPELL-DIAG sentinels, so a release build would carry it`);
    }
  });

  // 1. every write is gated; 3. and inside the sentinels.
  const lines = js.split('\n');
  const jIn = inRegion(lines);
  const writes = [];
  lines.forEach((l, i) => {
    if (/\b(el|e)\.(textContent|innerHTML)\s*=(?!\s*'')/.test(l) || /\b(el|e)\.hidden\s*=\s*false/.test(l)) writes.push(i);
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
    if (!jIn.has(w)) {
      bad.push(`${JS}:${w + 1} paints the diag outside the SPELL-DIAG sentinels, so a release build would carry it`);
    }
  }
  lines.forEach((l, i) => {
    if ((l.includes(EL) || /'diag: '/.test(l) || l.includes('spell_diag_bar')) && !jIn.has(i)) {
      bad.push(`${JS}:${i + 1} names the diag outside the SPELL-DIAG sentinels (${l.trim().slice(0, 56)})`);
    }
  });
  // 5. the teardown hides it
  // The letter capture's own teardown: the cleanup() that follows `diagBar`.
  const after = js.slice(Math.max(0, js.indexOf('var diagBar')));
  const cleanup = (after.match(/function cleanup\(\) \{[\s\S]*?\n {6}\}/) || [''])[0];
  if (!/diagBar\.hide\(\)/.test(cleanup)) {
    bad.push(`${JS}: cleanup() does not hide the diag, so once shown it stays up across modes (the 298 bug)`);
  }

  // 4. every release build cuts it
  if (builds) {
    for (const [name, text] of Object.entries(builds.scripts)) {
      if (!/python3 scripts\/cut-diag\.py "\$DIST"/.test(text)) bad.push(`${name} never runs scripts/cut-diag.py, so its build carries the diag`);
    }
    if ((builds.dockerfile.match(/SPELL-DIAG/g) || []).length < 3) bad.push('the Dockerfile does not cut SPELL-DIAG for the site');
  }
  return { bad };
}

const GOOD_JS = `
      var diagBar = null;
      function cleanup() {
        handles = [];
        if (diagBar) diagBar.hide();
      }
      /* SPELL-DIAG:BEGIN */
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
      /* SPELL-DIAG:END */
`;
const GOOD_HTML = `<!-- SPELL-DIAG:BEGIN -->
<div class="voice-spell-status" id="voiceSpellDiag" aria-hidden="true" hidden style="position:fixed"></div>
<!-- SPELL-DIAG:END -->`;
const GOOD_BUILDS = { scripts: { 'build-web.sh': 'python3 scripts/cut-diag.py "$DIST"' }, dockerfile: 'SPELL-DIAG SPELL-DIAG SPELL-DIAG' };

if (process.argv.includes('--selftest')) {
  const cases = {
    clean: { r: check(GOOD_JS, GOOD_HTML, GOOD_BUILDS), want: null },
    // The exact build-270 leak.
    ungated_write: { r: check(GOOD_JS.replace("        if (!devDoorOpen()) return;\n        var el = document.getElementById('voiceSpellDiag');\n        el.hidden = false;\n        el.textContent = 'diag: ' + d.info;", "        var el = document.getElementById('voiceSpellDiag');\n        el.textContent = 'diag: ' + d.info;"), GOOD_HTML), want: /no dev-door check/ },
    not_hidden_in_markup: { r: check(GOOD_JS, GOOD_HTML.replace(' hidden', '')), want: /not hidden in the markup/ },
    element_gone: { r: check(GOOD_JS, '<div id="other"></div>'), want: /is gone from/ },
    readout_gone: { r: check('var x = 1;', GOOD_HTML), want: /nothing writes the diag/ },
    // CC-ECLIPSE-HOTFIX F4
    js_outside_sentinels: { r: check(GOOD_JS.replace('      /* SPELL-DIAG:BEGIN */\n', ''), GOOD_HTML), want: /outside the SPELL-DIAG sentinels/ },
    markup_outside_sentinels: { r: check(GOOD_JS, GOOD_HTML.replace('<!-- SPELL-DIAG:BEGIN -->\n', '')), want: /outside the SPELL-DIAG sentinels/ },
    never_rehidden: { r: check(GOOD_JS.replace('        if (diagBar) diagBar.hide();\n', ''), GOOD_HTML), want: /stays up across modes/ },
    build_skips_cut: { r: check(GOOD_JS, GOOD_HTML, { ...GOOD_BUILDS, scripts: { 'build-web-edu.sh': 'cp index.html "$DIST/"' } }), want: /never runs scripts\/cut-diag/ },
    site_skips_cut: { r: check(GOOD_JS, GOOD_HTML, { ...GOOD_BUILDS, dockerfile: '' }), want: /Dockerfile does not cut/ },
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

const read = (f) => fs.readFileSync(path.join(ROOT, f), 'utf8');
const { bad } = check(read(JS), read(HTML), {
  scripts: Object.fromEntries(['build-web.sh', 'build-web-edu.sh', 'build-web-audit.sh'].map((f) => [f, read(`scripts/${f}`)])),
  dockerfile: read('Dockerfile'),
});
if (bad.length) {
  console.error('diag-dev-door-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log('diag-dev-door-check: OK — the capture diagnostic is hidden, dev-door gated, cut from every release build, and hidden again at mic teardown.');
