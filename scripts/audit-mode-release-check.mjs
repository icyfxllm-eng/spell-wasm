#!/usr/bin/env node
// CI assertion for Feature 9 (audit bypass): a RELEASE build must NEVER carry
// the audit bypass. `AUDIT_MODE=1` builds pass `audit_override=true` into the
// resolver and expose EVERY entitlement with no purchase surface — that is a
// review/App-Store-audit tool only. A release lane shipping it is broken.
//
// This scans the release lanes (CI workflows + fastlane) and FAILS if any of
// them sets AUDIT_MODE=1 / AUDIT_MODE: 1 / AUDIT_MODE = "1". It also warns if a
// release lane never pins AUDIT_MODE=0 explicitly (defence in depth).
import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

// Files that define release/publish lanes.
const LANE_SOURCES = ['.github/workflows', 'fastlane'];
const LANE_EXT = /\.(yml|yaml|rb)$/i;

// AUDIT_MODE set to a truthy value (1/true/on/yes), tolerating : or = and quotes.
const BYPASS_ON = /AUDIT_MODE\s*[:=]\s*['"]?(1|true|on|yes)['"]?/i;
const PINNED_OFF = /AUDIT_MODE\s*[:=]\s*['"]?(0|false|off|no)['"]?/i;

function collect(dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) collect(p, out);
    else if (LANE_EXT.test(name)) out.push(p);
  }
  return out;
}

let files = [];
for (const s of LANE_SOURCES) files = collect(join(ROOT, s), files);

let hits = 0;
let anyPinnedOff = false;
for (const file of files) {
  const rel = relative(ROOT, file);
  const text = readFileSync(file, 'utf8');
  if (BYPASS_ON.test(text)) {
    console.error(`  ✗ release lane enables the audit bypass (AUDIT_MODE truthy): ${rel}`);
    hits++;
  }
  if (PINNED_OFF.test(text)) anyPinnedOff = true;
}

if (hits) {
  console.error(`\naudit-mode-release-check: FAILED — ${hits} release lane(s) carry the audit bypass.`);
  console.error('Release lanes must build with AUDIT_MODE=0 (audit_override=false).');
  process.exit(1);
}
if (!anyPinnedOff) {
  console.log('audit-mode-release-check: OK — no release lane enables AUDIT_MODE (none pins it explicitly yet).');
} else {
  console.log('audit-mode-release-check: OK — release lanes pin AUDIT_MODE=0 and none enable the bypass.');
}
