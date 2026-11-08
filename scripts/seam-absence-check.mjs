#!/usr/bin/env node
// Production-safety guard: the E2E test seam (window.__spelltest) must NEVER be
// present in a shipped build. This greps the production dist/ (index.html + the
// compiled wasm + JS glue) for any trace of the seam and fails if found.
// Run against the PRODUCTION dist (built by scripts/build-web.sh, no
// --features testseam), not dist-test/.
import { readdirSync, readFileSync, statSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
// CC-LEARNING-ENGINE-L0 R4 (acceptance 12): the dev-only learner inspector is
// never in a player's build either. Its marker is the panel id it creates.
// CC-ECLIPSE-HOTFIX F4 (I7): nor is the mic diag readout, its switch, or its
// 'diag: ' prefix. TestFlight and the App Store ship this same dist.
const NEEDLES = ['__spelltest', 'testseam', 'spellLearnerInspector', 'dev_preview',
  'voiceSpellDiag', 'devDiagBar', 'spell_diag_bar', 'diag: ',
  // CC-THEMES-COSMOS census: nor the Test entitlements switch, which granted
  // Complete to any player who tapped the logo five times.
  'devEntitlements', 'spell_dev_entitlements',
  // CC-LAUNCH-EN e2e: the dev-only owned-products list (entitlements_adapter::dev_owned).
  'spell_dev_owned',
  // CC-LAUNCH-EN item 3: nor the local flag-override reader, which let
  // localStorage switch on any flag. Release reads only defaults, the kill file
  // and the Tools screen's player flags (src/flags.rs PLAYER_FLAGS).
  'spell_flag_override:'];
// CC-THEMES-COSMOS T-D10 / AT-T2: nor is the Theme Lab -- its menu row, its
// stored preview, its stylesheet and its chip. Otherwise any player could tap
// the logo five times and wear every earned theme. The one build allowed to
// carry it is the internal-only Lab TestFlight build (SPELL_THEME_LAB=1),
// which fastlane refuses to submit (fastlane/lab-builds.txt).
export const LAB_NEEDLES = ['devOpenThemeLab', 'spell_dev_theme_lab', 'themeLabStyle', 'labChip'];

// CC-LAUNCH-EN Phase B item 2: the four network paths the App Store build
// compiles out (Cargo features accounts_net, server_check, telemetry,
// server_stt; net_services turns on all four). Each needle is a string only the
// compiled-in path owns. The site build (SPELL_WEB=1, --features web) carries
// net_services, so it is the one release build allowed them.
// `--services-present <dist>` is the other half of the proof: run it on a
// dev or e2e build, where every one of these must be found.
export const SERVICE_NEEDLES = {
  accounts_net: ['/api/auth/', '/api/climb/'],
  server_check: ['malformed /api/check response'],
  telemetry: ['/v1/events', '/v1/flags'],
  server_stt: ['/api/stt'],
};
const ALL_SERVICE_NEEDLES = Object.values(SERVICE_NEEDLES).flat();

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

/** Every needle found in `dist`, as messages. */
export function scan(dist, needles) {
  const hits = [];
  for (const file of walk(dist)) {
    // Read wasm/js/html as latin1 so the wasm's embedded strings are scannable.
    const buf = readFileSync(file, 'latin1');
    for (const needle of needles) if (buf.includes(needle)) hits.push(`"${needle}" found in ${file.replace(dist, '.')}`);
  }
  return hits;
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN && process.argv.includes('--selftest')) {
  // AT-T2's deliberate failure: a dist holding the Lab must fail; a clean one must pass.
  const tmp = mkdtempSync(join(tmpdir(), 'seam-absence-'));
  writeFileSync(join(tmp, 'index.html'), '<html><body>a player build</body></html>');
  const clean = scan(tmp, [...NEEDLES, ...LAB_NEEDLES]).length === 0;
  writeFileSync(join(tmp, 'spell_wasm_bg.wasm'), Buffer.from('\0\0asm...spell_dev_theme_lab...devOpenThemeLab', 'latin1'));
  const caught = scan(tmp, [...NEEDLES, ...LAB_NEEDLES]).length === 2;
  // Item 2: a dist carrying one compiled-out path fails, once per needle it holds.
  writeFileSync(join(tmp, 'spell_wasm_bg.wasm'), Buffer.from('\0\0asm...{}/v1/events...', 'latin1'));
  const svc = scan(tmp, ALL_SERVICE_NEEDLES).length === 1;
  rmSync(tmp, { recursive: true, force: true });
  console.log(`  ${clean ? 'ok  ' : 'FAIL'}   a clean dist passes`);
  console.log(`  ${caught ? 'ok  ' : 'FAIL'}   a dist carrying the Theme Lab fails`);
  console.log(`  ${svc ? 'ok  ' : 'FAIL'}   a dist carrying the telemetry sender fails`);
  const all = clean && caught && svc;
  console.log(all ? 'seam-absence-check selftest: OK' : 'seam-absence-check selftest: FAILED');
  process.exit(all ? 0 : 1);
}
if (MAIN && process.argv[2] === '--services-present') {
  const dist = join(ROOT, process.argv[3] || 'dist-dev');
  const found = new Set(scan(dist, ALL_SERVICE_NEEDLES).map((h) => h.split('"')[1]));
  let ok = true;
  for (const [feature, needles] of Object.entries(SERVICE_NEEDLES)) {
    for (const n of needles) {
      const hit = found.has(n);
      ok &&= hit;
      console.log(`  ${hit ? 'ok  ' : 'FAIL'}   ${feature}: "${n}" ${hit ? 'found' : 'MISSING'} in ${process.argv[3] || 'dist-dev'}`);
    }
  }
  console.log(ok ? 'seam-absence-check --services-present: OK' : 'seam-absence-check --services-present: FAILED (a needle no longer marks its path; fix the needle)');
  process.exit(ok ? 0 : 1);
}
if (MAIN) {
  const dist = join(ROOT, process.argv[2] || 'dist');
  const lab = process.env.SPELL_THEME_LAB === '1';
  const web = process.env.SPELL_WEB === '1';
  if (lab) console.log('seam-absence-check: SPELL_THEME_LAB=1, the internal Lab build -- the Theme Lab is allowed, everything else is not.');
  if (web) console.log('seam-absence-check: SPELL_WEB=1, the site build -- the account, check, telemetry and server-STT paths are allowed.');
  const hits = scan(dist, [...NEEDLES, ...(lab ? [] : LAB_NEEDLES), ...(web ? [] : ALL_SERVICE_NEEDLES)]);
  for (const h of hits) console.error(`  ✗ ${h}`);
  if (hits.length) {
    console.error(`\nseam-absence-check: FAILED — ${hits.length} dev-only trace(s) in the production build.`);
    console.error('Production must be built WITHOUT --features testseam, dev_preview, theme_lab or net_services (scripts/build-web.sh).');
    process.exit(1);
  }
  console.log(`seam-absence-check: OK — no test seam, dev door extra, Theme Lab or compiled-out network path in ${process.argv[2] || 'dist'}.`);
}
