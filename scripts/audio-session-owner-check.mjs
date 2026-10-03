#!/usr/bin/env node
// CC-SPELLIT-MIC-FIX A2 / I-M5 — exactly one audio-session owner.
//
// AVAudioSession is one object per process. Five call sites across two files
// used to set the category and activate it with no coordination, and that is
// the whole of the build-219 bug: `.playback` is output-only, so a spoken word
// starting while the mic listened removed the mic's input route and the tap
// received nothing for the rest of the session (diag buf=0).
//
// A second owner would restore that bug silently, so this refuses one.
//
//   node scripts/audio-session-owner-check.mjs            # check the real tree
//   node scripts/audio-session-owner-check.mjs --selftest # prove it bites
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';
export const OWNER = 'ios/App/App/AudioSessionOwner.swift';
// Mutating calls only. Reading `session.category` or `currentRoute` is fine
// anywhere; it is setting and activating that must have one home.
const MUTATORS = /\.(setCategory|setActive)\s*\(/;

export function check(files) {
  const bad = [];
  let ownerSeen = false;
  for (const [file, src] of Object.entries(files)) {
    const isOwner = file === OWNER;
    if (isOwner) ownerSeen = true;
    const hits = src.split('\n')
      .map((line, i) => [i + 1, line])
      .filter(([, line]) => MUTATORS.test(line) && !line.trim().startsWith('//'));
    if (!isOwner && hits.length) {
      for (const [n, line] of hits) {
        bad.push(`${file}:${n} sets the audio session directly — route it through AudioSessionOwner (${line.trim().slice(0, 60)})`);
      }
    }
    if (isOwner && !hits.length) {
      bad.push(`${OWNER} no longer sets the session at all — the owner has been hollowed out`);
    }
  }
  if (!ownerSeen) bad.push(`${OWNER} is missing — the single owner this law is about`);
  return { bad };
}

if (process.argv.includes('--selftest')) {
  const owner = 'try session.setCategory(.playAndRecord)\ntry session.setActive(true)';
  const cases = {
    clean: { r: check({ [OWNER]: owner, 'ios/App/App/Other.swift': 'let c = session.category' }), want: null },
    second_owner: { r: check({ [OWNER]: owner, 'ios/App/App/Other.swift': 'try? session.setCategory(.playback)' }), want: /sets the audio session directly/ },
    second_activator: { r: check({ [OWNER]: owner, 'ios/App/App/Other.swift': 'try? session.setActive(true)' }), want: /sets the audio session directly/ },
    reads_are_fine: { r: check({ [OWNER]: owner, 'ios/App/App/Other.swift': 'session.currentRoute.inputs.first' }), want: null },
    commented_out_is_fine: { r: check({ [OWNER]: owner, 'ios/App/App/Other.swift': '// try session.setActive(true)' }), want: null },
    owner_hollowed: { r: check({ [OWNER]: 'let x = 1' }), want: /hollowed out/ },
    owner_missing: { r: check({ 'ios/App/App/Other.swift': 'let x = 1' }), want: /is missing/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.r.bad.length === 0 : c.r.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.r.bad[0] ?? '') : (c.r.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.r.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 66) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`audio-session-owner-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('audio-session-owner-check selftest: OK');
  process.exit(0);
}

const dir = `${ROOT}/ios/App/App`;
const files = {};
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith('.swift'))) {
  files[`ios/App/App/${f}`] = fs.readFileSync(path.join(dir, f), 'utf8');
}
const { bad } = check(files);
if (bad.length) {
  console.error('audio-session-owner-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log(`audio-session-owner-check: OK — ${Object.keys(files).length} Swift files, the session is set in ${OWNER} and nowhere else.`);
