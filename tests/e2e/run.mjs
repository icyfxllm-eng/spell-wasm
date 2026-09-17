#!/usr/bin/env node
// E2E runner: serves the seam-enabled dist-test/, runs every spec, and writes a
// per-build TEST-REPORT-{app,site}.md (pass/fail by area). Built on the plain `playwright`
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
import * as finale from './specs/finale.mjs';
import * as platform from './specs/platform.mjs';
import * as pictureWall from './specs/picture-wall.mjs';
import * as gallery from './specs/gallery.mjs';
import * as gameplay from './specs/gameplay.mjs';
import * as placement from './specs/placement.mjs';
import * as modes from './specs/modes.mjs';
import * as menu from './specs/menu.mjs';
import * as coming from './specs/coming.mjs';
import * as sayit from './specs/sayit.mjs';
import * as ghost from './specs/ghost.mjs';
import * as spellaloud from './specs/spellaloud.mjs';
import * as submitAdvance from './specs/submit-advance.mjs';
import * as attemptsShields from './specs/attempts_shields.mjs';
import * as toolsHub from './specs/tools-hub.mjs';
import * as playHub from './specs/playhub.mjs';
import * as hubRow from './specs/hub-row.mjs';
import * as economics from './specs/economics.mjs';
import * as finalePixels from './specs/finale-pixels.mjs';
import * as audioGate from './specs/audio-gate.mjs';
import * as finaleRelaunch from './specs/finale-relaunch.mjs';
import * as metadataAudit from './specs/metadata-audit.mjs';
import * as kidReveal from './specs/kid-reveal.mjs';
import * as pickerContinue from './specs/picker-continue.mjs';
import * as frontDoor from './specs/front-door.mjs';
import * as jrClimb from './specs/jr-climb.mjs';
import * as myWordsLists from './specs/mywords-lists.mjs';
import * as translateScreen from './specs/translate-screen.mjs';
import * as settingsEffects from './specs/settings-effects.mjs';

const SPECS = [
  ['playhub', playHub, placement],
  ['hub-row', hubRow],
  ['keyboard', keyboard],
  ['gameplay', gameplay],
  ['modes', modes],
  ['menu', menu],
  ['settings-effects', settingsEffects],
  ['front-door', frontDoor],
  ['jr-climb', jrClimb],
  ['mywords-lists', myWordsLists],
  ['translate-screen', translateScreen],
  ['coming', coming],
  ['sayit', sayit],
  ['ghost', ghost],
  ['spellaloud', spellaloud],
  ['submit-advance', submitAdvance],
  ['attempts-shields', attemptsShields],
  ['tools-hub', toolsHub],
  ['finale', finale],
  ['economics', economics],
  ['audio-gate', audioGate],
  ['finale-relaunch', finaleRelaunch],
  ['metadata-audit', metadataAudit],
  ['kid-reveal', kidReveal],
  ['picker-continue', pickerContinue],
  ['finale-pixels', finalePixels],
  ['platform', platform],
  ['picture-wall', pictureWall],
  ['gallery', gallery],
];

const ROOT = join(fileURLToPath(import.meta.url), '..', '..', '..');

// Specs that assert PLATFORM behaviour. The site is English-only and the app
// ships every language (Eric, 2026-07-31), so "Spanish is coming-soon gated"
// is true of one build and false of the other. Running these against the app
// build asserted nothing and sat red; they run under SPELL_WEB=1 instead.
const WEB_ONLY_SPECS = new Set(['coming', 'picture-wall']);
// Specs that must run in BOTH configurations -- a platform claim checked on
// one side only is half a check.
const BOTH_SPECS = new Set(['platform']);
const IS_WEB = process.env.SPELL_WEB === '1';

const { server, base } = await startServer();
const browser = await launch();
const suites = [];
try {
  // `[name, ...mods]` — an area may bundle several spec modules. This
  // used to destructure `[name, mod]`, so any module after the first was
  // registered, imported, and NEVER RUN. placement.mjs sat dead that way
  // while the orb it was meant to protect died on device.
  for (const [name, ...mods] of SPECS) {
    if (!BOTH_SPECS.has(name) && WEB_ONLY_SPECS.has(name) !== IS_WEB) {
      // Announced, never silent. A spec that vanishes without a word is how
      // the twelve-day language regression stayed invisible.
      process.stdout.write(`\n▶ ${name} — skipped (runs in the ${WEB_ONLY_SPECS.has(name) ? 'site' : 'app'} build)\n`);
      continue;
    }
    const suite = new Suite(name);
    process.stdout.write(`\n▶ ${name}\n`);
    for (const mod of mods) await mod.run(browser, base, suite);
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
const lines = [`# TEST-REPORT — Web E2E (${IS_WEB ? 'site' : 'app'} build)`, '', `**${total - failed}/${total} passed** across ${suites.length} areas.`, ''];
for (const s of suites) {
  const f = s.results.filter((r) => !r.ok).length;
  lines.push(`## ${s.name} — ${s.results.length - f}/${s.results.length}`);
  for (const r of s.results) lines.push(`- ${r.ok ? '✅' : '❌'} ${r.title}${r.ok ? '' : `\n  - ${r.err}`}`);
  lines.push('');
}
// One report per configuration. Both runs used to write TEST-REPORT.md,
// so the site's 11 results silently overwrote the app's 80 and nobody
// could see WHICH app specs ran — which is how a placement spec
// asserting an invisible card stayed invisible through the Aug 6 audit.
const REPORT = `TEST-REPORT-${IS_WEB ? 'site' : 'app'}.md`;
writeFileSync(join(ROOT, 'tests', 'e2e', REPORT), lines.join('\n'));

process.stdout.write(`\n${failed ? '❌' : '✅'} E2E: ${total - failed}/${total} passed. Report → tests/e2e/${REPORT}\n`);
process.exit(failed ? 1 : 0);
