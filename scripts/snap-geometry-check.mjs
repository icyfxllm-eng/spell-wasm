#!/usr/bin/env node
// CC-SNAP-BOXES I-B6 — one definition of every gap threshold, enforced.
//
// Before Phase B there were four thresholds for one measurement, split across
// two languages: 0.35 in src/snap_clean.rs and 1.1 / 0.45 / 0.25 inside
// healSplitWords on the Swift side. They were not complements, nobody owned
// them together, and a page could in principle be both merged and split.
//
// config/snap-geometry.json is now the one source. Rust bakes it in with
// include_str!, so it cannot drift. Swift CANNOT read it -- the file is not in
// the Xcode target and adding it would mean touching the pbxproj -- so the
// agreement is enforced here instead: the plugin declares named constants with
// a `// config: <key>` marker, and this script fails the build if any of them
// stops matching the JSON.
//
// It also refuses a SECOND threshold appearing anywhere: a bare `* avgChar`
// or `* glyph` multiplier that is not one of the named constants is exactly
// the regression this exists to stop.

import { readFileSync } from 'node:fs';

const CONFIG = 'config/snap-geometry.json';
const SWIFT = 'ios/App/App/NativeLanguageKitPlugin+PhotoList.swift';
const RUST = 'src/snap_clean.rs';

const bad = [];
const read = (p) => {
  try {
    return readFileSync(p, 'utf8');
  } catch {
    bad.push(`${p} is missing — the geometry contract has lost one of its three halves.`);
    return '';
  }
};

const cfgRaw = read(CONFIG);
const swift = read(SWIFT);
const rust = read(RUST);

let cfg = {};
try {
  cfg = JSON.parse(cfgRaw || '{}');
} catch (e) {
  bad.push(`${CONFIG} is not valid JSON: ${e.message}`);
}
const keys = Object.keys(cfg).filter((k) => !k.startsWith('_'));
if (keys.length === 0) bad.push(`${CONFIG} declares no thresholds.`);

// --- Swift mirrors every key, with the right value -------------------------
for (const key of keys) {
  const re = new RegExp(
    `static let (\\w+)\\s*:\\s*CGFloat\\s*=\\s*([0-9.]+)\\s*//\\s*config:\\s*${key}\\b`,
  );
  const m = swift.match(re);
  if (!m) {
    bad.push(
      `${SWIFT}: no constant carries the marker "// config: ${key}". ` +
        `Every key in ${CONFIG} needs a mirrored Swift constant, or the two can drift unseen.`,
    );
    continue;
  }
  if (Number(m[2]) !== Number(cfg[key])) {
    bad.push(
      `${key}: ${CONFIG} says ${cfg[key]}, ${SWIFT} says ${m[2]} (as ${m[1]}). ` +
        `Change the JSON, then this file to match — never the other way round.`,
    );
  }
}

// --- Rust reads the config rather than carrying its own number -------------
if (rust && !rust.includes('include_str!("../config/snap-geometry.json")')) {
  bad.push(
    `${RUST} no longer bakes in ${CONFIG}. The core must read the one source, ` +
      `not keep a second copy of it.`,
  );
}
const rustLiteral = rust.match(/const\s+GAP_RATIO\s*:\s*f32\s*=/);
if (rustLiteral) {
  bad.push(`${RUST} has reintroduced a literal GAP_RATIO. The value belongs in ${CONFIG}.`);
}

// --- no unnamed multiplier anywhere ---------------------------------------
// A gap test written as `gap <= 0.4 * avgChar` is a fifth threshold nobody
// owns. Named constants are allowed; bare numbers are not.
const stray = [];
for (const [file, src, unit] of [
  [SWIFT, swift, 'avgChar'],
  [RUST, rust, 'glyph'],
]) {
  const re = new RegExp(`([0-9]*\\.?[0-9]+)\\s*\\*\\s*${unit}\\b`, 'g');
  for (const m of src.matchAll(re)) {
    const line = src.slice(0, m.index).split('\n').length;
    stray.push(`${file}:${line}: bare multiplier ${m[1]} * ${unit}`);
  }
}
if (stray.length) {
  bad.push(
    `a gap threshold is written as a bare number instead of a named constant:\n    ` +
      stray.join('\n    '),
  );
}

if (bad.length) {
  console.error('snap-geometry-check: FAILED');
  for (const b of bad) console.error(`  ${b}`);
  process.exit(1);
}
console.log(
  `snap-geometry-check: OK — ${keys.length} threshold(s), one source, Rust and Swift agree`,
);
