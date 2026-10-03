#!/usr/bin/env node
// CC-SPELLIT-MIC-FIX G1 — voice spelling needs the microphone, and the thing
// that nearly took it away was a different feature's cleanup.
//
// CC-HUB-DEADROWS removed Say It, and its D4 said to remove the microphone
// permission "only if Say It is its sole consumer". It is not: Spell It Out
// Loud uses the same capture plugin and the same two usage strings, and the
// shipped permission text covers both features in fifteen languages ("so you
// can say or spell a word out loud"). That census resolved against removal —
// this check is what makes the next one not have to notice.
//
// An iOS app whose Info.plist lacks these strings does not degrade: it is
// terminated by the OS the moment it asks for the microphone. So the failure
// this prevents is a crash on tap, in the field, for every language with the
// mic.
//
//   node scripts/voice-spell-deps-check.mjs            # check the real tree
//   node scripts/voice-spell-deps-check.mjs --selftest # prove it bites
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';

/// The two usage strings iOS requires before a capture session may start, and
/// the plugin that owns the capture path.
export const REQUIRED_KEYS = ['NSMicrophoneUsageDescription', 'NSSpeechRecognitionUsageDescription'];
export const CAPTURE_PLUGIN = 'ios/App/App/NativeLanguageKitPlugin.swift';

export function check({ constsSrc, plist, pluginExists }) {
  const bad = [];
  const m = constsSrc.match(/VOICE_SPELL_LANGS:\s*\[&str;\s*(\d+)\]\s*=\s*\[([^\]]*)\]/s);
  if (!m) return { bad: ['could not read VOICE_SPELL_LANGS from src/consts.rs'], langs: [] };
  const langs = m[2].split(',').map((x) => x.trim()).filter(Boolean);
  if (!langs.length) return { bad, langs };   // no language wants the mic: nothing to require

  for (const key of REQUIRED_KEYS) {
    if (!plist.includes(`<key>${key}</key>`)) {
      bad.push(`${langs.length} language(s) have voiceSpell true, but Info.plist has no ${key} — iOS terminates an app that asks for the microphone without it`);
    }
  }
  if (!pluginExists) {
    bad.push(`${langs.length} language(s) have voiceSpell true, but ${CAPTURE_PLUGIN} is gone — nothing captures audio`);
  }
  return { bad, langs };
}

if (process.argv.includes('--selftest')) {
  const consts = 'pub const VOICE_SPELL_LANGS: [&str; 2] =\n    [EN, ES];\n';
  const none = 'pub const VOICE_SPELL_LANGS: [&str; 0] =\n    [];\n';
  const full = REQUIRED_KEYS.map((k) => `<key>${k}</key><string>why</string>`).join('\n');
  const cases = {
    clean: { r: check({ constsSrc: consts, plist: full, pluginExists: true }), want: null },
    no_mic_string: {
      r: check({ constsSrc: consts, plist: `<key>${REQUIRED_KEYS[1]}</key>`, pluginExists: true }),
      want: /no NSMicrophoneUsageDescription/,
    },
    no_speech_string: {
      r: check({ constsSrc: consts, plist: `<key>${REQUIRED_KEYS[0]}</key>`, pluginExists: true }),
      want: /no NSSpeechRecognitionUsageDescription/,
    },
    plugin_deleted: { r: check({ constsSrc: consts, plist: full, pluginExists: false }), want: /nothing captures audio/ },
    // The guard is conditional, as G1 says: with no voiceSpell language there is
    // nothing to protect, and demanding the strings anyway would be noise.
    no_languages_requires_nothing: { r: check({ constsSrc: none, plist: '', pluginExists: false }), want: null },
    unreadable_consts: { r: check({ constsSrc: 'nothing here', plist: full, pluginExists: true }), want: /could not read VOICE_SPELL_LANGS/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.r.bad.length === 0 : c.r.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.r.bad[0] ?? '') : (c.r.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.r.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 70) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`voice-spell-deps-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('voice-spell-deps-check selftest: OK');
  process.exit(0);
}

const { bad, langs } = check({
  constsSrc: fs.readFileSync(`${ROOT}/src/consts.rs`, 'utf8'),
  plist: fs.readFileSync(`${ROOT}/ios/App/App/Info.plist`, 'utf8'),
  pluginExists: fs.existsSync(`${ROOT}/${CAPTURE_PLUGIN}`),
});
if (bad.length) {
  console.error('voice-spell-deps-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log(`voice-spell-deps-check: OK — ${langs.length} voiceSpell language(s), both usage strings present, capture plugin in place.`);
