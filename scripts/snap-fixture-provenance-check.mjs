#!/usr/bin/env node
// Every CC-SNAP fixture shot must declare where its pixels came from.
//
// The fixture exists to calibrate thresholds against what a phone sensor
// actually records. A rendered or AI-generated picture of a page is not a
// weaker photo -- it is a different measurement, with no sensor noise, no
// demosaic, no real optics, and shadows that were painted rather than cast.
// Its numbers look exactly like real ones, which is the whole problem: they
// would move a threshold and nothing downstream would ever know.
//
// On 2026-10-06 a generated image of a worksheet on a wooden desk was
// measured and reported before anyone said where it came from. Its "sunlit
// oak" read S 0.757 -- more saturated than every real highlighter in the set
// -- and that figure was briefly set beside genuine camera measurements of
// rows 22 and 23 in an argument about F1. Nothing reached the repo. This
// check is so the next one is declared before it is measured.
//
// NOTHING HERE DETECTS A GENERATED IMAGE, and nothing tries to. There is no
// reliable way to, and a check that claimed to would be worse than none --
// it would launder exactly the images it failed to catch. All this enforces
// is that a human wrote down what they know, in a field that cannot be left
// blank by accident. A sidecar that lies is out of its reach, and that is a
// limit to state rather than paper over.
//
//   node scripts/snap-fixture-provenance-check.mjs
//   node scripts/snap-fixture-provenance-check.mjs --selftest

import { readdirSync, readFileSync, mkdtempSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';

const DIRS = ['tests/fixtures/snap-highlight', 'tests/fixtures/snap-layout'];
// The placeholder the shoot tool writes. Present means nobody filled it in.
const PLACEHOLDER = /camera \| screenshot \| GENERATED/;
// What counts as an answer. "unconfirmed" is deliberately allowed: an honest
// "I do not know yet" is a real state, and it is visible in the file.
const ANSWERED = /\b(camera|screenshot|generated|unconfirmed)\b/i;

export function problems(dirs = DIRS) {
  const out = [];
  for (const dir of dirs) {
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir).filter((n) => n.endsWith('.expected.json'))) {
      const p = join(dir, name);
      let obj;
      try { obj = JSON.parse(readFileSync(p, 'utf8')); }
      catch (e) { out.push(`${p} is not valid JSON: ${e.message}`); continue; }
      const v = obj._provenance;
      if (v === undefined) {
        out.push(`${p} has no _provenance — say where the pixels came from`);
      } else if (typeof v !== 'string' || !v.trim()) {
        out.push(`${p} has an empty _provenance`);
      } else if (PLACEHOLDER.test(v)) {
        out.push(`${p} still carries the placeholder _provenance`);
      } else if (!ANSWERED.test(v)) {
        out.push(`${p} _provenance says "${v}" — it must name one of `
          + 'camera, screenshot, generated or unconfirmed');
      }
    }
  }
  return out;
}

function selftest() {
  const cases = [
    ['a camera shot passes', { _provenance: "camera -- Eric's phone" }, 0],
    ['a screenshot passes', { _provenance: 'screenshot of Kindle' }, 0],
    ['an honest unconfirmed passes', { _provenance: 'UNCONFIRMED -- Eric to say' }, 0],
    ['a declared generated image passes (declared is the point)',
      { _provenance: 'generated -- do not calibrate against this' }, 0],
    ['a missing field fails', { _row: 1 }, 1],
    ['an empty field fails', { _provenance: '   ' }, 1],
    ['the untouched placeholder fails',
      { _provenance: 'camera | screenshot | GENERATED -- say which' }, 1],
    ['prose that answers nothing fails', { _provenance: 'a page from a book' }, 1],
  ];
  let bad = 0;
  for (const [name, obj, want] of cases) {
    const dir = mkdtempSync(join(tmpdir(), 'provcheck-'));
    writeFileSync(join(dir, 'x.expected.json'), JSON.stringify(obj));
    const got = problems([dir]).length;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  if (bad) { console.error('snap-fixture-provenance-check selftest: FAILED'); process.exit(1); }
  console.log('snap-fixture-provenance-check selftest: OK');
}

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = problems();
  if (bad.length) {
    console.error('snap-fixture-provenance-check: FAILED');
    for (const b of bad) console.error(`  ${b}`);
    console.error('\n  A fixture calibrates thresholds against real sensor output.');
    console.error('  An undeclared shot is an uncontrolled variable in every number');
    console.error('  derived from it. Say camera, screenshot, generated or unconfirmed.');
    process.exit(1);
  }
  const n = DIRS.filter(existsSync)
    .reduce((a, d) => a + readdirSync(d).filter((x) => x.endsWith('.expected.json')).length, 0);
  console.log(`snap-fixture-provenance-check: OK — ${n} shots, all declared`);
}
