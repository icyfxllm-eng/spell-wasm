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

// Third-party Capacitor plugins also ship Swift, and they can set the session
// without going through the owner. They are outside this repo's control, so the
// law for them is "known and counted": each plugin that mutates the session is
// listed with how many calls it makes and why that is tolerated. An unlisted
// plugin that starts doing it, or a listed one whose call count changes after an
// upgrade, fails the check so a human re-reads it.
export const KNOWN_PLUGINS = {
  'node_modules/@capacitor-community/native-audio/ios/Sources/NativeAudio/Plugin.swift': {
    calls: 4,
    why: 'load() sets .playback and deactivates; configure() sets .playback (focus) or .ambient. ' +
         'audio-native.js calls configure({focus:true}) before every server clip, so the plugin ' +
         'and AudioSessionOwner both set .playback-family categories (found by the ' +
         'CC-VISUAL-ECLIPSE recon, R5). Device-verified behaviour, not changed here.',
  },
};

export function checkPlugins(files) {
  const bad = [];
  const known = [];
  for (const [file, src] of Object.entries(files)) {
    const n = src.split('\n').filter((line) => MUTATORS.test(line) && !line.trim().startsWith('//')).length;
    if (!n) continue;
    const k = KNOWN_PLUGINS[file];
    if (!k) bad.push(`${file} sets the audio session (${n} call(s)) and is not in KNOWN_PLUGINS — review it`);
    else if (k.calls !== n) bad.push(`${file} now makes ${n} session call(s), KNOWN_PLUGINS says ${k.calls} — re-review after the upgrade`);
    else known.push(file);
  }
  return { bad, known };
}

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
    plugin_unlisted: { r: checkPlugins({ 'node_modules/@capacitor/x/ios/X.swift': 'try s.setActive(true)' }), want: /not in KNOWN_PLUGINS/ },
    plugin_count_changed: {
      r: checkPlugins({ [Object.keys(KNOWN_PLUGINS)[0]]: 'try s.setActive(true)' }), want: /re-review after the upgrade/ },
    plugin_known: {
      r: checkPlugins({ [Object.keys(KNOWN_PLUGINS)[0]]: 'try s.setCategory(.a)\ntry s.setActive(false)\ntry s.setCategory(.b)\ntry s.setCategory(.c)' }), want: null },
    plugin_silent_is_fine: { r: checkPlugins({ 'node_modules/@capacitor/y/ios/Y.swift': 'let c = s.category' }), want: null },
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

// Plugins: only where node_modules is installed (a checkout without it has no
// plugin code to build, so there is nothing to scan).
const pluginFiles = {};
const walk = (d) => {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const full = path.join(d, e.name);
    if (e.isDirectory()) walk(full);
    else if (e.name.endsWith('.swift')) pluginFiles[path.relative(ROOT, full)] = fs.readFileSync(full, 'utf8');
  }
};
let pluginNote = 'plugins not scanned (no node_modules)';
const scopes = ['node_modules/@capacitor', 'node_modules/@capacitor-community'].map((s) => path.join(ROOT, s));
if (scopes.some((s) => fs.existsSync(s))) {
  for (const s of scopes) if (fs.existsSync(s)) walk(s);
  const p = checkPlugins(pluginFiles);
  bad.push(...p.bad);
  pluginNote = `${Object.keys(pluginFiles).length} plugin Swift files, ${p.known.length} known session-setter(s)`;
}
if (bad.length) {
  console.error('audio-session-owner-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log(`audio-session-owner-check: OK — ${Object.keys(files).length} Swift files, the session is set in ${OWNER} and nowhere else; ${pluginNote}.`);
