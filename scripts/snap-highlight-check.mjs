#!/usr/bin/env node
// CC-SNAP-HIGHLIGHT v1 — one source for the highlight thresholds, and no
// claiming a calibration that has not happened.
//
// Two jobs.
//
// ONE SOURCE. config/snap-highlight.json holds every threshold. Rust bakes it
// in with include_str!, and the Swift shim mirrors the two numbers it needs
// (v_min and sat_buckets), because the V cut and the saturation bucketing
// happen where the pixels are. A mirrored constant is a constant that can
// drift, which is what config/snap-geometry.json already learned; this fails
// the build when they do.
//
// HONEST CALIBRATION. The thresholds are not calibrated: the C2 20-photo
// fixture does not exist, and D-H2 says the fixture is what sets the final ΔS.
// While that is true, min_paper_px stays null and the detector scores nothing,
// which is the Intent's own fall-back -- fail toward showing every word.
//
// The failure this guards against is the tempting one: someone sets
// min_paper_px to a plausible-looking number to "make it work", leaves
// calibrated at false, and the detector silently starts hiding words a reader
// marked, tuned to a page nobody ever measured. So: calibrated and
// min_paper_px must agree. Either both say uncalibrated, or the fixture has
// run and both say so.
//
//   node scripts/snap-highlight-check.mjs
//   node scripts/snap-highlight-check.mjs --selftest

import { readFileSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';

const CONFIG = 'config/snap-highlight.json';
const RUST = 'src/snap_highlight.rs';
const SWIFT = 'ios/App/App/NativeLanguageKitPlugin+PhotoList.swift';
const KEYS = ['calibrated', 'min_paper_px', 'coverage', 'delta_s_camera',
  'delta_s_screenshot', 'v_min', 'hue_buckets', 'sat_buckets'];
// Mirrored into Swift because the shim applies them where the pixels are.
const MIRRORED = ['v_min', 'sat_buckets'];

export function problems(configPath = CONFIG, rustPath = RUST, swiftPath = SWIFT) {
  const out = [];
  if (!existsSync(configPath)) return [`${configPath} is missing — the one source is gone`];

  let cfg;
  try { cfg = JSON.parse(readFileSync(configPath, 'utf8')); }
  catch (e) { return [`${configPath} is not valid JSON: ${e.message}`]; }

  for (const k of KEYS) {
    if (!(k in cfg)) out.push(`${configPath} has no "${k}" — the Rust struct requires it`);
  }

  // Calibration honesty, in both directions.
  if (cfg.calibrated === false && cfg.min_paper_px !== null) {
    out.push(`calibrated is false but min_paper_px is ${cfg.min_paper_px}. A number here `
      + 'starts the detector; if the C2 fixture has not set it, that number was invented '
      + 'and the detector will hide words tuned to a page nobody measured.');
  }
  if (cfg.calibrated === true && (cfg.min_paper_px === null || cfg.min_paper_px === undefined)) {
    out.push('calibrated is true but min_paper_px is null, so the detector still scores '
      + 'nothing. One of the two is wrong.');
  }

  // Ranges. A saturation threshold outside 0..1 is a typo, not a choice.
  for (const k of ['coverage', 'delta_s_camera', 'delta_s_screenshot', 'v_min']) {
    const v = cfg[k];
    if (typeof v !== 'number' || !(v >= 0 && v <= 1)) {
      out.push(`${k} is ${JSON.stringify(v)}; it is a 0..1 fraction`);
    }
  }
  for (const k of ['hue_buckets', 'sat_buckets']) {
    if (!Number.isInteger(cfg[k]) || cfg[k] < 1) out.push(`${k} must be a positive integer`);
  }

  // One source: Rust must read the file, not restate it.
  if (existsSync(rustPath)) {
    const rust = readFileSync(rustPath, 'utf8');
    if (!rust.includes(`include_str!("../${configPath}")`)) {
      out.push(`${rustPath} does not include_str! ${configPath} — it must read the one source`);
    }
    for (const k of ['coverage', 'delta_s_camera', 'delta_s_screenshot', 'v_min']) {
      // A literal of the config's own value, assigned to something named like
      // the key, is the shape of a second copy.
      const re = new RegExp(`${k}\\s*[:=]\\s*${String(cfg[k]).replace('.', '\\.')}\\b`);
      if (re.test(rust.replace(/\/\/.*$/gm, ''))) {
        out.push(`${rustPath} restates ${k} = ${cfg[k]} in code; it belongs only in ${configPath}`);
      }
    }
  }

  // The Swift shim is not written yet (CC-SNAP-BOXES F3). When it appears,
  // its mirrored constants must match. Absence is not a failure; drift is.
  if (existsSync(swiftPath)) {
    const swift = readFileSync(swiftPath, 'utf8');
    for (const k of MIRRORED) {
      const m = swift.match(new RegExp(`${k}\\s*[:=]\\s*([0-9.]+)`, 'i'));
      if (m && Number(m[1]) !== cfg[k]) {
        out.push(`${swiftPath} has ${k} = ${m[1]}, ${configPath} has ${cfg[k]} — they have drifted`);
      }
    }
  }
  return out;
}

function selftest() {
  const good = {
    calibrated: false, min_paper_px: null, coverage: 0.5, delta_s_camera: 0.25,
    delta_s_screenshot: 0.25, v_min: 0.35, hue_buckets: 12, sat_buckets: 32,
  };
  const rust = 'const CONFIG_JSON: &str = include_str!("../config/snap-highlight.json");';
  const cases = [
    ['a clean uncalibrated config passes', good, rust, 0],
    ['a calibrated config with a number passes', { ...good, calibrated: true, min_paper_px: 500 }, rust, 0],
    ['an invented min_paper_px is caught', { ...good, min_paper_px: 500 }, rust, 1],
    ['claiming calibration with no number is caught', { ...good, calibrated: true }, rust, 1],
    ['a coverage above 1 is caught', { ...good, coverage: 50 }, rust, 1],
    ['a missing key is caught', (() => { const c = { ...good }; delete c.v_min; return c; })(), rust, 2],
    ['a Rust copy of a threshold is caught', good, `${rust}\nlet coverage = 0.5;`, 1],
    ['Rust not reading the one source is caught', good, 'let x = 1;', 1],
  ];
  let bad = 0;
  for (const [name, cfg, rustSrc, want] of cases) {
    const dir = mkdtempSync(join(tmpdir(), 'hlcheck-'));
    const c = join(dir, 'c.json');
    const r = join(dir, 'r.rs');
    writeFileSync(c, JSON.stringify(cfg));
    writeFileSync(r, rustSrc.replace('../config/snap-highlight.json', `../${c}`));
    const got = problems(c, r, join(dir, 'absent.swift')).length;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  if (bad) { console.error('snap-highlight-check selftest: FAILED'); process.exit(1); }
  console.log('snap-highlight-check selftest: OK');
}

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = problems();
  if (bad.length) {
    console.error('snap-highlight-check: FAILED');
    for (const b of bad) console.error(`  ${b}`);
    process.exit(1);
  }
  const cfg = JSON.parse(readFileSync(CONFIG, 'utf8'));
  console.log(`snap-highlight-check: OK — one source, ${cfg.calibrated ? 'calibrated' : 'honestly uncalibrated'}`
    + (cfg.calibrated ? '' : ' (the detector scores nothing until the C2 fixture sets min_paper_px)'));
}
