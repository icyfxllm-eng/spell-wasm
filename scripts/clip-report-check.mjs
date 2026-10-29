#!/usr/bin/env node
// CLIP-REPORT: the "Can't tell" skip must stay a VOID, never a miss, and the
// feature must stay dark until Eric turns it on.
//
// The e2e (tests/e2e/specs/cant-tell.mjs) proves the streak survives in a real
// browser; this is the cheap structural half that runs in the gate, plus a
// mutation selftest: each lesion below is the way this feature could start
// costing a player their streak, or ship lit, and the check must go red.
//
//   node scripts/clip-report-check.mjs
//   node scripts/clip-report-check.mjs --selftest

import { readFileSync } from 'node:fs';

const OUTCOMES = ['give_up(', 'skip_word(', 'on_correct(', 'finalize_incorrect', 'bump_streak', 'break_streak', 'stats::record', 'record_miss_routed', 'streak = 0'];

function fnBody(src, header) {
  const i = src.indexOf(header);
  if (i < 0) return null;
  const rest = src.slice(i);
  const end = rest.indexOf('\n}\n');
  return end < 0 ? rest : rest.slice(0, end + 2);
}

export function problems(files) {
  const out = [];
  const body = fnBody(files.game, 'pub fn cant_tell(app: &App) {');
  if (body === null) return ['game.rs: pub fn cant_tell is gone'];
  if (!body.includes('void_round(app, false)')) out.push('cant_tell does not end the round through void_round(app, false)');
  for (const o of OUTCOMES) if (body.includes(o)) out.push(`cant_tell calls ${o} -- a skip must not be an outcome`);
  if (!body.includes('cant_tell_offered(app)')) out.push('cant_tell is not guarded by cant_tell_offered');
  const reveal = fnBody(files.game, 'fn void_round(app: &App, reveal: bool) {');
  if (reveal === null) out.push('game.rs: void_round is gone');
  else for (const o of OUTCOMES.slice(0, 8)) if (reveal.includes(o)) out.push(`void_round calls ${o} -- a void records nothing`);
  const flag = fnBody(files.flags, 'pub fn cant_tell() -> bool {');
  if (!flag || !/resolve\(stored\("cant_tell"\)\.as_deref\(\), false\)/.test(flag)) out.push('flags::cant_tell must default to false');
  if (!/id="rescueCantTell"[^>]*\bhidden\b/.test(files.html)) out.push('index.html: #rescueCantTell must start hidden');
  if (!files.lib.includes('dom::on_click("rescueCantTell"')) out.push('lib.rs does not wire #rescueCantTell');
  if (!/pub const CANT_TELL_SESSION_CAP: u32 = \d+;/.test(files.game)) out.push('the session cap constant is gone');
  return out;
}

const load = () => ({
  game: readFileSync('src/game.rs', 'utf8'),
  flags: readFileSync('src/flags.rs', 'utf8'),
  html: readFileSync('index.html', 'utf8'),
  lib: readFileSync('src/lib.rs', 'utf8'),
});

if (process.argv.includes('--selftest')) {
  const base = load();
  const clean = problems(base);
  if (clean.length) { console.error('clip-report-check selftest: the real tree is already red:\n  ' + clean.join('\n  ')); process.exit(1); }
  const LESIONS = [
    ['the skip becomes a give-up (loses the streak)', (f) => ({ ...f, game: f.game.replace('void_round(app, false);\n}', 'give_up(app);\n}') })],
    ['the skip resets the streak by hand', (f) => ({ ...f, game: f.game.replace('close_rescue();\n    void_round(app, false);', 'close_rescue();\n    app.borrow_mut().streak = 0;\n    void_round(app, false);') })],
    ['the flag ships ON', (f) => ({ ...f, flags: f.flags.replace('resolve(stored("cant_tell").as_deref(), false)', 'resolve(stored("cant_tell").as_deref(), true)') })],
    ['the button starts visible', (f) => ({ ...f, html: f.html.replace(/(id="rescueCantTell"[^>]*?) hidden/, '$1') })],
    ['the handler loses its gate', (f) => ({ ...f, game: f.game.replace('if !cant_tell_offered(app) {\n        return;\n    }\n    let (answered, entry', 'let (answered, entry') })],
  ];
  let bad = 0;
  for (const [name, mutate] of LESIONS) {
    const m = mutate(base);
    const changed = JSON.stringify(m) !== JSON.stringify(base);
    const caught = changed && problems(m).length > 0;
    if (!caught) { bad += 1; console.error(`  NOT CAUGHT: ${name}${changed ? '' : ' (the lesion did not apply -- the check is stale)'}`); }
  }
  if (bad) process.exit(1);
  console.log(`clip-report-check selftest: OK -- ${LESIONS.length} lesions all caught`);
  process.exit(0);
}

const found = problems(load());
if (found.length) { console.error('clip-report-check: FAILED\n  ' + found.join('\n  ')); process.exit(1); }
console.log('clip-report-check: OK -- Can\'t tell is a void, gated, and dark by default');
