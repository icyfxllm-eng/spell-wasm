#!/usr/bin/env node
// CC-LAUNCH-EN v1.1 A15.8 / D-DEF12: no definition or sentence render path may
// bypass the one gate, src/defgate.rs::can_render.
//
// How the gate is made unavoidable, and what this checks:
//   1. /api/meaning and /api/sentence-audio are requested only in src/api.rs.
//   2. api::Meaning keeps its text fields private (no `pub` field), so callers
//      can read them only through definition()/sentence(), which ask the gate.
//   3. play_sentence_audio takes a &Meaning and returns unless its sentence passes.
//   4. /api/defpool is requested only in src/defmatch_screen.rs, and that file
//      drops rows the gate refuses (defgate::can_render ... Field::Definition).
//
//   node scripts/def-render-gate-check.mjs [--selftest]
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;

function rustFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...rustFiles(p));
    else if (name.endsWith('.rs')) out.push(p);
  }
  return out;
}

// Code only: drop // comments so prose that names an endpoint does not count.
const code = (text) => text.split('\n').map((l) => l.replace(/\/\/.*$/, '')).join('\n');

export function scan(files) {
  const bad = [];
  for (const [path, text] of Object.entries(files)) {
    const c = code(text);
    // A request builds the URL with its query string; prose and test messages do not.
    for (const ep of ['/api/meaning', '/api/sentence-audio']) {
      if (c.includes(ep + '?') && path !== 'src/api.rs') bad.push(`${path}: requests ${ep} outside src/api.rs`);
    }
    if (c.includes('/api/defpool?') && path !== 'src/defmatch_screen.rs') bad.push(`${path}: requests /api/defpool outside src/defmatch_screen.rs`);
  }
  const api = code(files['src/api.rs'] || '');
  const m = api.match(/pub struct Meaning\s*\{([^}]*)\}/);
  if (!m) bad.push('src/api.rs: pub struct Meaning not found');
  else if (/\bpub\b/.test(m[1])) bad.push('src/api.rs: Meaning has a pub field, so its text can be read around the gate');
  const play = api.match(/pub fn play_sentence_audio\(([^)]*)\)\s*\{([\s\S]*?)\n\}/);
  if (!play) bad.push('src/api.rs: play_sentence_audio not found');
  else {
    if (!/&Meaning/.test(play[1])) bad.push('src/api.rs: play_sentence_audio does not take a &Meaning');
    if (!/\.sentence\(\)\.is_none\(\)/.test(play[2])) bad.push('src/api.rs: play_sentence_audio does not check the gated sentence first');
  }
  for (const acc of ['definition', 'sentence']) {
    const f = api.match(new RegExp(`pub fn ${acc}\\(&self\\)[^{]*\\{([\\s\\S]*?)\\n    \\}`));
    if (!f || !/defgate::can_render/.test(f[1])) bad.push(`src/api.rs: Meaning::${acc}() does not ask defgate::can_render`);
  }
  const dm = code(files['src/defmatch_screen.rs'] || '');
  if (!/defgate::can_render\([^)]*Field::Definition\)/.test(dm)) bad.push('src/defmatch_screen.rs: pool rows are not filtered through defgate::can_render(.., Field::Definition)');
  return bad;
}

function real() {
  const files = {};
  for (const p of rustFiles(join(ROOT, 'src'))) files[relative(ROOT, p)] = readFileSync(p, 'utf8');
  return files;
}

if (process.argv.includes('--selftest')) {
  const good = real();
  const cases = {
    'an endpoint fetched elsewhere': { ...good, 'src/x.rs': 'fn f(){ let u = "/api/meaning?word=a"; }' },
    'a pub Meaning field': { ...good, 'src/api.rs': good['src/api.rs'].replace('    definition: String,', '    pub definition: String,') },
    'an ungated accessor': { ...good, 'src/api.rs': good['src/api.rs'].replace(/crate::defgate::can_render\(&self\.meta, crate::defgate::Field::Definition\)/, 'true') },
    'an unfiltered Def Match pool': { ...good, 'src/defmatch_screen.rs': good['src/defmatch_screen.rs'].replace('crate::defgate::can_render(&meta, crate::defgate::Field::Definition)', 'true') },
  };
  let ok = scan(good).length === 0;
  for (const [name, files] of Object.entries(cases)) {
    if (scan(files).length === 0) { console.error(`def-render-gate-check selftest: did not catch ${name}`); ok = false; }
  }
  console.log(ok ? 'def-render-gate-check selftest: OK — every planted bypass is caught' : 'def-render-gate-check selftest: FAILED');
  process.exit(ok ? 0 : 1);
}

const bad = scan(real());
if (bad.length) {
  console.error('def-render-gate-check: FAILED — a definition/sentence path can bypass defgate::can_render:');
  for (const b of bad) console.error('  ' + b);
  process.exit(1);
}
console.log('def-render-gate-check: OK — every definition and sentence path goes through defgate::can_render');
