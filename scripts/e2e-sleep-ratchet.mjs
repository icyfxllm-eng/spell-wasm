#!/usr/bin/env node
// A ratchet on fixed sleeps in the e2e suite.
//
// Six gate runs on 2026-10-03 failed on four different tests, each passing on
// an immediate retry. All four were waiting on the clock instead of on the
// app: `waitForTimeout(400)` is a guess about how long something takes, and
// the guess was being made on a machine that was also running a cache warm
// and a gate. Fixing those four left a few hundred more.
//
// This does NOT ban the pattern. A sleep is occasionally the honest tool --
// chiefly when asserting that something does NOT happen, where there is no
// positive signal to wait for. It caps the count instead, so the number can
// only go down. A test that needs one documents it and the cap absorbs it.
//
// WHY A RATCHET AND NOT A REWRITE. Converting several hundred sleeps in one
// change would be unreviewable, and most of them have never failed. The cost
// of the flakes is not the waiting -- it is that after the third retry "just
// run it again" becomes the reflex, and that is how a real regression gets
// waved through. Stopping the number growing protects that, today, for the
// price of one constant.
//
//   node scripts/e2e-sleep-ratchet.mjs
//   node scripts/e2e-sleep-ratchet.mjs --selftest
//
// To exempt a genuinely necessary sleep, put `sleep-ok:` and a reason in the
// comment block directly above it -- anywhere in that block, so a reason may
// run to several lines. Exempt sleeps are not counted, so adding one does not
// force the cap up, but it does leave the reason in the diff.

import { readdirSync, readFileSync, statSync, mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';

const ROOT = 'tests/e2e';
const BASELINE_FILE = 'config/e2e-sleep-baseline.json';
const CALL = /\.waitForTimeout\s*\(/g;
const EXEMPT = /sleep-ok:/;
const COMMENT = /^\s*(\/\/|\*|\/\*)/;

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (p.endsWith('.mjs') || p.endsWith('.js')) out.push(p);
  }
  return out;
}

/** Counted sleeps per file, ignoring exempted ones and comment lines. */
export function countSleeps(root = ROOT) {
  const per = {};
  let total = 0;
  let exempt = 0;
  for (const file of walk(root)) {
    const lines = readFileSync(file, 'utf8').split('\n');
    let n = 0;
    for (let i = 0; i < lines.length; i += 1) {
      const line = lines[i];
      // A mention inside a comment is prose, not a sleep.
      if (COMMENT.test(line)) continue;
      const hits = (line.match(CALL) || []).length;
      if (!hits) continue;
      // Scan up through the contiguous comment block above. A reason worth
      // writing rarely fits on one line, and a checker that only reads the
      // line directly above quietly ignores the second half of every one.
      let marked = EXEMPT.test(line);
      for (let j = i - 1; j >= 0 && !marked && COMMENT.test(lines[j]); j -= 1) {
        if (EXEMPT.test(lines[j])) marked = true;
      }
      if (marked) { exempt += hits; continue; }
      n += hits;
    }
    if (n) per[file] = n;
    total += n;
  }
  return { total, exempt, per };
}

function selftest() {
  // Four lesions, so a change that defeats the check fails loudly here rather
  // than quietly in six months.
  const cases = [
    ['a bare call counts', 'await page.waitForTimeout(400);', 1],
    ['a mention in a comment does not', '// we used to waitForTimeout(400) here', 0],
    ['an exempted call does not', '// sleep-ok: proving nothing renders\nawait page.waitForTimeout(400);', 0],
    ['a reason may run to several lines',
      '// sleep-ok: proving nothing renders --\n// there is no event to wait for.\nawait page.waitForTimeout(400);', 0],
    ['the block must be contiguous with the call',
      '// sleep-ok: stale reason\n\nawait page.waitForTimeout(400);', 1],
    ['two on one line count twice', 'await a.waitForTimeout(1); await b.waitForTimeout(2);', 2],
  ];
  let bad = 0;
  for (const [name, body, want] of cases) {
    const dir = mkdtempSync(join(tmpdir(), 'ratchet-'));
    mkdirSync(join(dir, 'specs'), { recursive: true });
    writeFileSync(join(dir, 'specs', 't.mjs'), body);
    const got = countSleeps(dir).total;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  if (bad) { console.error('e2e-sleep-ratchet selftest: FAILED'); process.exit(1); }
  console.log('e2e-sleep-ratchet selftest: OK');
}

// Only act when run as a command. The counter is exported so a test can
// import it without the ratchet firing on load.
const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;

if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const baseline = JSON.parse(readFileSync(BASELINE_FILE, 'utf8')).max_fixed_sleeps;
  const { total, exempt, per } = countSleeps();

  if (total > baseline) {
    console.error('e2e-sleep-ratchet: FAILED');
    console.error(`  ${total} fixed sleeps in ${ROOT}, and the cap is ${baseline}.`);
    console.error('  A new waitForTimeout is a new flake waiting for a loaded machine.');
    console.error('  Wait for the thing you are about to assert on instead -- a state, a');
    console.error('  count, a text that stops changing. If the sleep is genuinely the only');
    console.error('  option, put "sleep-ok: <reason>" in a comment on the line above it.');
    console.error('\n  highest files:');
    for (const [f, n] of Object.entries(per).sort((a, b) => b[1] - a[1]).slice(0, 5)) {
      console.error(`    ${n.toString().padStart(3)}  ${f}`);
    }
    process.exit(1);
  }

  if (total < baseline) {
    // The ratchet tightens. One number, one line, and it can never slide back.
    console.error('e2e-sleep-ratchet: FAILED (in the good direction)');
    console.error(`  ${total} fixed sleeps now, down from ${baseline}. Lower the cap so it holds:`);
    console.error(`    ${BASELINE_FILE}  ->  "max_fixed_sleeps": ${total}`);
    process.exit(1);
  }

  console.log(
    `e2e-sleep-ratchet: OK — ${total} fixed sleeps, at the cap`
      + (exempt ? `, plus ${exempt} documented as sleep-ok` : ''),
  );
}
