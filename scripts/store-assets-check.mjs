#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F12 / AT-28: store assets match the submitted build.
//
// After the 2.3.6 rejection, a feature may appear in the App Store screenshots
// only if its flags are on in the exact build being submitted. Run from the
// tree that build is cut from (a release branch): it reads that tree's flag
// defaults from src/flags.rs (the app variant, not the site build's), and the
// scene manifest config/store-scenes.json. It fails when
//   - a scene depends on a flag that is off in this tree;
//   - a screenshot in fastlane/screenshots/en-US maps to no scene;
//   - a scene is missing a screenshot for a device.
//
//   node scripts/store-assets-check.mjs            # the submission lane runs this
//   node scripts/store-assets-check.mjs --selftest
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Each flag's compiled default in the app build: name -> true/false. */
export function flagDefaults(src) {
  const out = {};
  const re = /(#\[cfg\(feature = "web"\)\]\s*)?pub fn (\w+)\(\) -> bool \{\s*(?:\/\/[^\n]*\n\s*)*resolve\(stored\("(\w+)"\)\.as_deref\(\), (true|false)\)/g;
  for (const m of src.matchAll(re)) {
    if (m[1]) continue; // the site build's variant
    out[m[3]] = m[4] === 'true';
  }
  return out;
}

export function check({ flagsSrc, manifest, shots }) {
  const bad = [];
  const defaults = flagDefaults(flagsSrc);
  const ids = new Set(manifest.scenes.map((s) => s.id));
  for (const s of manifest.scenes) {
    for (const f of s.flags) {
      if (!(f in defaults)) bad.push(`${s.id}: depends on "${f}", which src/flags.rs does not define`);
      else if (!defaults[f]) bad.push(`${s.id}: depends on "${f}", which is OFF in this build -- it may not appear in the store`);
    }
  }
  for (const file of shots) {
    const m = file.match(/^([a-z0-9]+)-(\d+)-([a-z0-9-]+)\.png$/);
    if (!m || !manifest.devices.includes(m[1]) || !ids.has(m[3])) bad.push(`${file}: maps to no scene in config/store-scenes.json`);
  }
  for (const d of manifest.devices) {
    manifest.scenes.forEach((s, i) => {
      if (!shots.includes(`${d}-${i + 1}-${s.id}.png`)) bad.push(`${d}-${i + 1}-${s.id}.png: missing`);
    });
  }
  return bad;
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const flagsSrc = readFileSync(join(ROOT, 'src/flags.rs'), 'utf8');
  const manifest = JSON.parse(readFileSync(join(ROOT, 'config/store-scenes.json'), 'utf8'));
  const dir = join(ROOT, 'fastlane/screenshots/en-US');
  const shots = existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith('.png')) : [];
  if (process.argv.includes('--selftest')) {
    const src = 'pub fn on_one() -> bool {\n    resolve(stored("on_one").as_deref(), true)\n}\n#[cfg(feature = "web")]\npub fn off_one() -> bool {\n    resolve(stored("off_one").as_deref(), true)\n}\npub fn off_one() -> bool {\n    // RELEASE BRANCH ONLY: not yet\n    resolve(stored("off_one").as_deref(), false)\n}\n';
    const m = { devices: ['iphone69'], scenes: [{ id: 'a', flags: ['on_one'] }, { id: 'b', flags: ['off_one'] }] };
    const r = check({ flagsSrc: src, manifest: m, shots: ['iphone69-1-a.png', 'iphone69-2-b.png', 'iphone69-9-stray.png'] });
    const ok = r.some((x) => x.includes('"off_one", which is OFF')) && r.some((x) => x.includes('stray')) && !r.some((x) => x.includes('on_one'))
      && check({ flagsSrc: src, manifest: { devices: ['iphone69'], scenes: [{ id: 'a', flags: ['on_one'] }] }, shots: [] }).some((x) => x.includes('missing'));
    if (ok) { console.log('store-assets-check selftest: OK (an off flag, a stray shot and a missing shot are caught; an on flag passes)'); process.exit(0); }
    console.error(`store-assets-check selftest: FAILED\n  ${r.join('\n  ')}`);
    process.exit(1);
  }
  const bad = check({ flagsSrc, manifest, shots });
  if (bad.length) {
    console.error(`store-assets-check: FAILED — store assets must match this build (AT-28):\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`store-assets-check: OK — ${manifest.scenes.length} scenes x ${manifest.devices.length} devices, every flag on in this build`);
}
