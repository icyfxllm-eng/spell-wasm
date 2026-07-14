#!/usr/bin/env node
// E2E runner: serves the seam-enabled dist-test/, runs every spec, and writes a
// merged TEST-REPORT.md (pass/fail by area). Built on the plain `playwright`
// library (no @playwright/test dependency), so it runs in CI with just
// `npm ci && npx playwright install chromium`.
//
//   bash scripts/build-web-test.sh   # produces dist-test/ with __spelltest
//   node tests/e2e/run.mjs
import { writeFileSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startServer, launch, Suite } from './harness.mjs';

import * as keyboard from './specs/keyboard.mjs';
import * as gameplay from './specs/gameplay.mjs';
import * as modes from './specs/modes.mjs';
import * as menu from './specs/menu.mjs';
import * as coming from './specs/coming.mjs';
import * as audit from './specs/audit.mjs';

const SPECS = [
  ['keyboard', keyboard],
  ['gameplay', gameplay],
  ['modes', modes],
  ['menu', menu],
  ['coming', coming],
];

const ROOT = join(fileURLToPath(import.meta.url), '..', '..', '..');

const { server, base } = await startServer();
// The audit spec needs BOTH the production-parity bundle (dist-test) and the
// audit bundle (dist-test-audit, the seam + `--features audit` twin). Build the
// latter on demand if it's missing, so the canonical gate
// `bash scripts/build-web-test.sh && node tests/e2e/run.mjs` (which only builds
// dist-test) still exercises the audit spec.
const auditDist = join(ROOT, 'dist-test-audit');
if (!existsSync(auditDist)) {
  process.stdout.write('\n▶ building dist-test-audit (AUDIT_LANGS=fil) for the audit spec…\n');
  try {
    execSync('AUDIT_LANGS=fil bash scripts/build-web-test.sh', { cwd: ROOT, stdio: 'inherit' });
  } catch (e) {
    process.stderr.write(`  ⚠ could not build dist-test-audit: ${e.message}\n`);
  }
}
let auditServer = null;
let auditBase = null;
if (existsSync(auditDist)) {
  ({ server: auditServer, base: auditBase } = await startServer(8130, 'dist-test-audit'));
}
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
  {
    const suite = new Suite('audit');
    process.stdout.write(`\n▶ audit\n`);
    await audit.run(browser, base, suite, auditBase);
    for (const r of suite.results) process.stdout.write(`  ${r.ok ? '✓' : '✗'} ${r.title}${r.ok ? '' : ' — ' + r.err}\n`);
    suites.push(suite);
  }
} finally {
  await browser.close();
  server.close();
  if (auditServer) auditServer.close();
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
