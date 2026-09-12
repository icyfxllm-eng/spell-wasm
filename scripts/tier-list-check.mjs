#!/usr/bin/env node
// CC-ONBOARD-JR I3 — one tier authority.
//
//   node scripts/tier-list-check.mjs            # scan production Rust under src/
//   node scripts/tier-list-check.mjs --selftest # prove a planted tier list FAILS
//
// Spell Jr drifted because every surface kept its own list of tiers and its own
// idea of the kid cap: the base game capped at Hard, Word Bee at Medium, and the
// selector offered Expert to everyone (inventory §2). src/experience.rs is now
// the one place that decides which tiers a player may be offered or served.
// This gate fails the build when production Rust grows a NEW list of two or
// more tier names anywhere else, unless the list is allowlisted below with the
// reason it is not an access decision — a ladder the resolver already clamps, a
// lookup index, a presentation table, or a mode Spell Jr never sees.
//
// A "list" is one line naming two or more DISTINCT quoted tiers: an array, a
// matches!, a match-arm alternation. Exempt: #[cfg(test)] items, comments, the
// generated word_data.rs, and experience.rs itself. Every allowlist entry must
// still match a line — a stale entry fails too, so the allowlist cannot rot
// into a blanket pass.
//
// Known limit: a list written one tier per line, or through impostor's Tier
// enum, is not seen. Neither shape exists in serving code today.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative, sep } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const EXEMPT = new Set(['src/word_data.rs', 'src/experience.rs']);
const TIER = /"(easy|medium|hard|expert)"/g;

export const ALLOW = [
  { file: 'src/consts.rs', has: 'TIER_ORDER', why: 'ladder order for sorting and display; decides no access' },
  { file: 'src/climb.rs', has: 'DIFFICULTIES', why: 'ranked leaderboard difficulties; Jr Climb never submits (experience::leaderboard_allowed)' },
  { file: 'src/bee.rs', has: 'const LADDER', why: 'round-to-tier ladder; tier_for_round clamps Spell Jr through experience::serve_tier' },
  { file: 'src/daily.rs', has: 'const ARC:', why: 'the standard Daily arc' },
  { file: 'src/daily.rs', has: 'const KID_ARC:', why: 'the Jr Daily arc; jr_daily_tests pins it inside allowed_tiers(Junior, "daily")' },
  { file: 'src/wordpic.rs', has: '("easy", 1,', why: 'picture size bands per tier; kid_ok asks the resolver' },
  { file: 'src/wordpic_layout.rs', has: '=> &["', why: 'slot fallback ladder that only steps DOWN a tier' },
  { file: 'src/wordpic_screen.rs', has: '"easy" | "medium" => (true,', why: 'replay and miss-economics presentation per tier, not access' },
  { file: 'src/wordpic_screen.rs', has: 'for tier in [', why: 'zh_hanzi_for: finds the hanzi for a pinyin already served; serves nothing' },
  { file: 'src/defmatch.rs', has: 'AccessLevel::Full =>', why: 'entitlement depth; defmatch_screen applies the Spell Jr clamp after it' },
  { file: 'src/defmatch_screen.rs', has: 'let ladder = [', why: 'D3 floor ladder that only steps DOWN from the already-clamped tier' },
  { file: 'src/defmatch_screen.rs', has: 'let sway = matches!', why: 'animation timing per tier, not access' },
  { file: 'src/pinyin.rs', has: 'matches!(tier, "easy" | "medium")', why: 'D4 grading rule (surface tones accepted), not access' },
  { file: 'src/learner.rs', has: 'for tier in ["easy", "medium"]', why: 'placement set, already inside the Spell Jr tiers' },
  { file: 'src/say_it.rs', has: 'for tier in ["easy", "medium"]', why: 'Say It pool; Say It is hidden from Spell Jr by juniorPolicy' },
  { file: 'src/online_spelloff.rs', has: '=> s.level.clone()', why: 'match tier from level; Online Spell-Off is hidden from Spell Jr' },
  { file: 'src/word_index.rs', has: 'for tier in [', why: 'whole-bank lookup index for answer checking; serves nothing' },
  { file: 'src/photo_import.rs', has: 'for tier in [', why: 'matches scanned words against the bank; Photo list is hidden from Spell Jr' },
  { file: 'src/ink_probe.rs', has: 'for tier in [', why: 'handwriting-recogniser probe sample; never shown to a player' },
];

// Code with comments and string/char literals blanked, for brace counting.
function stripComment(line) {
  let inStr = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (inStr) { if (c === '\\') i++; else if (c === '"') inStr = false; }
    else if (c === '"') inStr = true;
    else if (c === '/' && line[i + 1] === '/') return line.slice(0, i);
  }
  return line;
}
const bare = (line) =>
  stripComment(line).replace(/"(?:[^"\\]|\\.)*"/g, '""').replace(/'(?:[^'\\]|\\.)'/g, "''");

// [lineNumber, text] for every line outside a #[cfg(test)] item.
export function prodLines(text) {
  const lines = text.split('\n');
  const out = [];
  for (let i = 0; i < lines.length; i++) {
    if (!/^\s*#\[cfg\(test\)\]/.test(lines[i])) { out.push([i + 1, lines[i]]); continue; }
    let j = i + 1;
    while (j < lines.length && /^\s*(#\[|\/\/|$)/.test(lines[j])) j++;
    let depth = 0, opened = false;
    for (; j < lines.length; j++) {
      const b = bare(lines[j]);
      for (const c of b) { if (c === '{') { depth++; opened = true; } else if (c === '}') depth--; }
      if (opened && depth <= 0) break;
      if (!opened && b.trim().endsWith(';')) break;
    }
    i = j;
  }
  return out;
}

export function scan(files, allow = ALLOW) {
  const hits = [];
  for (const [file, text] of Object.entries(files)) {
    if (EXEMPT.has(file)) continue;
    for (const [n, raw] of prodLines(text)) {
      const names = new Set([...stripComment(raw).matchAll(TIER)].map((m) => m[1]));
      if (names.size >= 2) hits.push({ file, line: n, text: raw.trim() });
    }
  }
  const used = new Set();
  const unallowed = hits.filter((h) => {
    const i = allow.findIndex((a) => a.file === h.file && h.text.includes(a.has));
    if (i >= 0) { used.add(i); return false; }
    return true;
  });
  const stale = allow.filter((_, i) => !used.has(i));
  return { hits, unallowed, stale };
}

function loadReal() {
  const files = {};
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (name.endsWith('.rs')) files[relative(ROOT, p).split(sep).join('/')] = readFileSync(p, 'utf8');
    }
  };
  walk(join(ROOT, 'src'));
  return files;
}

if (process.argv.includes('--selftest')) {
  const allow = [{ file: 'src/bee.rs', has: 'const LADDER', why: 'fixture' }];
  const files = {
    'src/bee.rs': 'const LADDER: [(u32, &str); 2] = [(1, "easy"), (3, "medium")];\n',
    'src/lesion.rs': [
      '// the ladder runs "easy" then "medium" -- a comment, not a list',
      '#[cfg(test)]',
      'mod tests {',
      '    fn t() { let open = "{"; for x in ["easy", "hard"] {} }',
      '}',
      'fn serve(kid: bool) -> &\'static str { let t = ["easy", "medium", "hard"]; t[2] }',
    ].join('\n'),
  };
  const problems = [];
  const r = scan(files, allow);
  if (!r.unallowed.some((h) => h.file === 'src/lesion.rs' && h.line === 6)) {
    problems.push('a planted tier list after a test module was not caught (a "{" in a test string swallowed it?)');
  }
  const wrong = r.unallowed.filter((h) => !(h.file === 'src/lesion.rs' && h.line === 6));
  if (wrong.length) problems.push(`clean lines flagged (allowlisted, comment, or test code): ${JSON.stringify(wrong)}`);
  if (r.stale.length) problems.push('an allowlist entry in use was reported stale');
  if (scan({ 'src/bee.rs': 'fn nothing() {}\n' }, allow).stale.length !== 1) problems.push('a stale allowlist entry was not caught');
  if (problems.length) {
    console.error(`tier-list-check --selftest: FAILED — ${problems.join('; ')}`);
    process.exit(1);
  }
  console.log('tier-list-check --selftest: OK — planted list caught past a test module; comment, test code and allowlisted list pass; stale entry caught.');
  process.exit(0);
}

const { hits, unallowed, stale } = scan(loadReal());
if (unallowed.length || stale.length) {
  if (unallowed.length) {
    console.error(`tier-list-check: FAILED — ${unallowed.length} tier list(s) outside src/experience.rs:`);
    for (const h of unallowed) console.error(`  ✗ ${h.file}:${h.line}  ${h.text}`);
    console.error('  Ask experience::allowed_tiers / serve_tier instead. If the list truly decides no access, allowlist it with the reason.');
  }
  if (stale.length) {
    console.error(`tier-list-check: FAILED — ${stale.length} stale allowlist entr${stale.length === 1 ? 'y' : 'ies'} (matches nothing; remove):`);
    for (const a of stale) console.error(`  ✗ ${a.file}  has ${JSON.stringify(a.has)}`);
  }
  process.exit(1);
}
console.log(`tier-list-check: OK — ${hits.length} tier list(s) in production Rust, every one allowlisted with a reason; the resolver is the only access authority.`);
