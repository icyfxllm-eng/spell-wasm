#!/usr/bin/env node
// E2E runner: serves the seam-enabled dist-test/, runs every spec, and writes a
// merged TEST-REPORT.md (pass/fail by area). Built on the plain `playwright`
// library (no @playwright/test dependency), so it runs in CI with just
// `npm ci && npx playwright install chromium`.
//
//   bash scripts/build-web-test.sh   # produces dist-test/ with __spelltest
//   node tests/e2e/run.mjs
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startServer, launch, Suite } from './harness.mjs';

import * as keyboard from './specs/keyboard.mjs';
import * as gameplay from './specs/gameplay.mjs';
import * as modes from './specs/modes.mjs';
import * as menu from './specs/menu.mjs';
import * as coming from './specs/coming.mjs';
import * as sayit from './specs/sayit.mjs';
import * as ghost from './specs/ghost.mjs';
import * as spellaloud from './specs/spellaloud.mjs';
import * as submitAdvance from './specs/submit-advance.mjs';
import * as attemptsShields from './specs/attempts_shields.mjs';

const SPECS = [
  ['keyboard', keyboard],
  ['gameplay', gameplay],
  ['modes', modes],
  ['menu', menu],
  ['coming', coming],
  ['sayit', sayit],
  ['ghost', ghost],
  ['spellaloud', spellaloud],
  ['submit-advance', submitAdvance],
  ['attempts-shields', attemptsShields],
];

const ROOT = join(fileURLToPath(import.meta.url), '..', '..', '..');

const { server, base } = await startServer();
const browser = await launch();
const suites = [];
try {
  for (const [name, mod] of SPECS) {
    const suite = new Suite(name);
    process.stdout.write(`\n▶ ${name}\n`);
    await mod.run(browser, base, suite);
    for (const r of suite.results) process.stdout.write(`  ${r.ok ? '✓' : '✗'} ${r.title}${r.ok ? '' : ' — ' + r.err}\n`);
    suites.push(suite);
  }
} finally {
  await browser.close();
  server.close();
}

const total = suites.reduce((n, s) => n + s.results.length, 0);
const failed = suites.reduce((n, s) => n + s.results.filter((r) => !r.ok).length, 0);

// Merged report artifact.
const lines = ['# TEST-REPORT — Web E2E', '', `**${total - failed}/${total} passed** across ${suites.length} areas.`, ''];
for (const s of suites) {
  const f = s.results.filter((r) => !r.ok).length;
  lines.push(`## ${s.name} — ${s.results.length - f}/${s.results.length}`);
  for (const r of s.results) lines.push(`- ${r.ok ? '✅' : '❌'} ${r.title}${r.ok ? '' : `\n  - ${r.err}`}`);
  lines.push('');
}
writeFileSync(join(ROOT, 'tests', 'e2e', 'TEST-REPORT.md'), lines.join('\n'));

process.stdout.write(`\n${failed ? '❌' : '✅'} E2E: ${total - failed}/${total} passed. Report → tests/e2e/TEST-REPORT.md\n`);
process.exit(failed ? 1 : 0);
