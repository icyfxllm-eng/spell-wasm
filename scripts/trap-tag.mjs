#!/usr/bin/env node
// CC-WORDLIST-RAFU F3 — trap tagger + per-tier coverage reporter.
//
//   node scripts/trap-tag.mjs --validate            # schema-check the registry
//   node scripts/trap-tag.mjs --lang ru             # coverage over shipped ru tiers
//   node scripts/trap-tag.mjs --lang ru --words f   # tag one word per line from f
//   node scripts/trap-tag.mjs --selftest            # fixtures prove each match kind
//
// The registry (config/trap-registry.json) is DATA and this file is the ENGINE.
// The split is the point (F3: "auditors will correct them... editable without
// touching the engine"), so the rule below is absolute:
//
//   THIS FILE MUST CONTAIN NO PER-LANGUAGE KNOWLEDGE.
//
// It implements match KINDS (contains_any, prefix_any, suffix_any,
// final_char_any, regex). It must never learn that ё is Russian or that ة is
// final-only. A new trap class is an entry in the registry; a new match KIND is
// the only reason to edit this file. --validate enforces the kinds; the reviewer
// enforces the spirit.
//
// Tagging honesty (RAFU decision 9). Three confidences, never blurred:
//   auto      — decided from codepoints. Counts toward a quota.
//   assisted  — `match` narrows candidates; a human confirms. Counts only once
//               confirmed, and this tool cannot confirm, so it reports
//               `candidate`, never `tagged`.
//   manual    — the tool cannot decide at all and says so. Never guesses.
// Anything else would let an unconfirmed guess satisfy a quota, which is the
// failure mode decision 9 exists to prevent.

import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REGISTRY_PATH = join(ROOT, 'config', 'trap-registry.json');
const TIERS = ['easy', 'medium', 'hard', 'expert'];
const KINDS = new Set(['contains_any', 'prefix_any', 'suffix_any', 'final_char_any', 'regex', 'none']);
const TAGGINGS = new Set(['auto', 'assisted', 'manual']);

const nfc = (s) => s.normalize('NFC');

function loadRegistry() {
  return JSON.parse(readFileSync(REGISTRY_PATH, 'utf8'));
}

// --- the engine: match kinds only -------------------------------------------

function matches(word, match) {
  if (!match || match.kind === 'none') return false;
  const w = nfc(word);
  switch (match.kind) {
    case 'contains_any':
      return match.values.some((v) => w.includes(nfc(v)));
    case 'prefix_any':
      return match.values.some((v) => w.startsWith(nfc(v)));
    case 'suffix_any':
      return match.values.some((v) => w.endsWith(nfc(v)));
    case 'final_char_any': {
      const chars = [...w];
      const last = chars[chars.length - 1];
      return match.values.some((v) => nfc(v) === last);
    }
    case 'regex':
      return new RegExp(match.pattern, 'u').test(w);
    default:
      throw new Error(`unknown match kind: ${match.kind}`);
  }
}

/** Tag one word. Returns { tagged[], candidates[], manual[] } — kept apart on purpose. */
export function tagWord(word, traps) {
  const out = { tagged: [], candidates: [], manual: [] };
  for (const t of traps) {
    if (t.tagging === 'manual') {
      out.manual.push(t.id);
      continue;
    }
    if (!matches(word, t.match)) continue;
    if (t.tagging === 'assisted') out.candidates.push(t.id);
    else out.tagged.push(t.id);
  }
  return out;
}

// --- validation --------------------------------------------------------------

function validate(reg) {
  const problems = [];
  if (!reg.languages || typeof reg.languages !== 'object') return ['registry has no `languages` object'];

  for (const [lang, entry] of Object.entries(reg.languages)) {
    if (!Array.isArray(entry.traps) || entry.traps.length === 0) {
      problems.push(`${lang}: no traps defined`);
      continue;
    }
    const seen = new Set();
    for (const t of entry.traps) {
      const at = `${lang}/${t.id ?? '<no id>'}`;
      if (!t.id) problems.push(`${lang}: a trap has no id`);
      if (seen.has(t.id)) problems.push(`${at}: duplicate trap id`);
      seen.add(t.id);
      if (!t.name) problems.push(`${at}: missing name`);
      if (!t.description) problems.push(`${at}: missing description`);
      if (!TAGGINGS.has(t.tagging)) problems.push(`${at}: tagging must be one of ${[...TAGGINGS].join('|')}, got ${t.tagging}`);
      if (!t.match || !KINDS.has(t.match.kind)) {
        problems.push(`${at}: match.kind must be one of ${[...KINDS].join('|')}, got ${t.match?.kind}`);
        continue;
      }
      // A non-auto class must justify itself — decision 9 in schema form.
      if (t.tagging !== 'auto' && !t.why) problems.push(`${at}: '${t.tagging}' requires \`why\` (RAFU decision 9: don't guess silently)`);
      if (t.tagging !== 'auto' && !t.workflow) problems.push(`${at}: '${t.tagging}' requires a \`workflow\``);
      // Manual classes must NOT carry a live rule — that would invite a silent guess.
      if (t.tagging === 'manual' && t.match.kind !== 'none') problems.push(`${at}: manual class must use match.kind 'none', got '${t.match.kind}'`);
      if (t.tagging !== 'manual' && t.match.kind === 'none') problems.push(`${at}: '${t.tagging}' needs a real match rule`);
      // Rule shape.
      if (['contains_any', 'prefix_any', 'suffix_any', 'final_char_any'].includes(t.match.kind)) {
        if (!Array.isArray(t.match.values) || t.match.values.length === 0) problems.push(`${at}: ${t.match.kind} needs non-empty values[]`);
        else {
          for (const v of t.match.values) {
            if (nfc(v) !== v) problems.push(`${at}: value ${JSON.stringify(v)} is not NFC`);
            if (t.match.kind === 'final_char_any' && [...nfc(v)].length !== 1) problems.push(`${at}: final_char_any value ${JSON.stringify(v)} must be a single character`);
          }
        }
      }
      if (t.match.kind === 'regex') {
        if (!t.match.pattern) problems.push(`${at}: regex needs a pattern`);
        else { try { new RegExp(t.match.pattern, 'u'); } catch (e) { problems.push(`${at}: bad regex — ${e.message}`); } }
      }
      // Alphabet sanity: an auto rule that can never fire is a typo, not a rule.
      if (entry.alphabet && t.match.values) {
        for (const v of t.match.values) {
          for (const ch of nfc(v)) {
            if (!entry.alphabet.includes(ch)) problems.push(`${at}: value char ${JSON.stringify(ch)} is outside ${lang}'s declared alphabet — typo or a stale alphabet`);
          }
        }
      }
    }
  }
  return problems;
}

// --- coverage report ---------------------------------------------------------

function readTier(lang, tier) {
  const p = join(ROOT, 'assets', 'words', lang, `${tier}.txt`);
  if (!existsSync(p)) return null;
  return readFileSync(p, 'utf8').split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
}

function report(reg, lang, wordsFile) {
  const entry = reg.languages[lang];
  if (!entry) { console.error(`no registry entry for language "${lang}" — known: ${Object.keys(reg.languages).join(', ')}`); process.exit(1); }
  const traps = entry.traps;

  const tiers = {};
  if (wordsFile) {
    tiers['(file)'] = readFileSync(wordsFile, 'utf8').split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
  } else {
    for (const t of TIERS) tiers[t] = readTier(lang, t);
  }

  console.log(`\ntrap coverage — ${entry.name} (${entry.variant ?? lang})`);
  const quota = reg.$quotas?.status ?? 'unspecified';
  console.log(`quotas: ${quota}\n`);

  let anyWords = false;
  for (const [tier, words] of Object.entries(tiers)) {
    if (words === null) { console.log(`  ${tier.padEnd(8)} — no source file`); continue; }
    if (!words.length) { console.log(`  ${tier.padEnd(8)} — 0 words`); continue; }
    anyWords = true;
    const counts = new Map(), cands = new Map();
    for (const w of words) {
      const r = tagWord(w, traps);
      for (const id of r.tagged) counts.set(id, (counts.get(id) ?? 0) + 1);
      for (const id of r.candidates) cands.set(id, (cands.get(id) ?? 0) + 1);
    }
    console.log(`  ${tier} (${words.length} words)`);
    for (const t of traps) {
      const n = counts.get(t.id) ?? 0, c = cands.get(t.id) ?? 0;
      const pct = words.length ? ((n / words.length) * 100).toFixed(0) : '0';
      if (t.tagging === 'auto') console.log(`    ${t.id.padEnd(28)} ${String(n).padStart(3)}  (${pct}%)`);
      else if (t.tagging === 'assisted') console.log(`    ${t.id.padEnd(28)} ${String(c).padStart(3)}  CANDIDATES — need auditor confirmation`);
      else console.log(`    ${t.id.padEnd(28)}   ?  MANUAL — tool cannot decide; auditor must tag`);
    }
    console.log('');
  }
  if (!anyWords) {
    console.log(`  ${lang} has no word content yet.`);
    console.log(`  Blocked on a source of record (sources/registry.json has only "es").`);
    console.log(`  See docs/russian-source-options.md.\n`);
  }
}

// --- selftest: fixtures prove each match kind, and prove the honesty rules ----

function selftest(reg) {
  const fails = [];
  const check = (name, got, want) => { if (got !== want) fails.push(`${name}: expected ${want}, got ${got}`); };
  const trapsOf = (lang) => reg.languages[lang].traps;
  const has = (lang, word, id) => tagWord(word, trapsOf(lang)).tagged.includes(id);
  const cand = (lang, word, id) => tagWord(word, trapsOf(lang)).candidates.includes(id);

  // Russian — one positive and one NEGATIVE per auto rule. The negatives are the
  // real test: a rule that fires on everything is worthless.
  check('ru ь in мышь', has('ru', 'мышь', 'soft-hard-sign'), true);
  check('ru no ь in дом', has('ru', 'дом', 'soft-hard-sign'), false);
  check('ru -ться in учиться', has('ru', 'учиться', 'tsya-tsa'), true);
  check('ru -тся in учится', has('ru', 'учится', 'tsya-tsa'), true);
  check('ru no тся in читать', has('ru', 'читать', 'tsya-tsa'), false);
  check('ru ё in лёд', has('ru', 'лёд', 'yo'), true);
  check('ru no ё in лед', has('ru', 'лед', 'yo'), false);
  check('ru doubled in класс', has('ru', 'класс', 'doubled-consonant'), true);
  check('ru not doubled in дом', has('ru', 'дом', 'doubled-consonant'), false);
  check('ru final devoice друг', has('ru', 'друг', 'final-devoicing'), true);
  check('ru no final devoice дом', has('ru', 'дом', 'final-devoicing'), false);
  check('ru hushing is CANDIDATE not tagged', cand('ru', 'щука', 'hushing-confusion'), true);
  check('ru hushing never auto-tags', has('ru', 'щука', 'hushing-confusion'), false);

  // Arabic — hamza must be candidate-only (decision 9), never asserted correct.
  check('ar hamza is CANDIDATE', cand('ar', 'مأمور', 'hamza-seat'), true);
  check('ar hamza never auto-tags', has('ar', 'مأمور', 'hamza-seat'), false);
  check('ar final ة', has('ar', 'مدرسة', 'taa-marbuta-vs-haa'), true);
  check('ar ة non-final does not fire', has('ar', 'مدرسةx', 'taa-marbuta-vs-haa'), false);
  check('ar madda آ', has('ar', 'آخر', 'madda'), true);
  check('ar dagger-alif is MANUAL', tagWord('هذا', trapsOf('ar')).manual.includes('dagger-alif'), true);
  check('ar dagger-alif never tagged', has('ar', 'هذا', 'dagger-alif'), false);

  // Persian — marked members only; unmarked default must NOT fire.
  check('fa ص fires', has('fa', 'صبح', 'homophone-s'), true);
  check('fa plain س does NOT fire', has('fa', 'سلام', 'homophone-s'), false);
  check('fa silent vav خواهر', has('fa', 'خواهر', 'silent-vav'), true);
  check('fa no silent vav in خانه', has('fa', 'خانه', 'silent-vav'), false);
  check('fa ZWNJ detected', has('fa', 'می‌رود', 'zwnj-compound'), true);
  check('fa no ZWNJ in ساده', has('fa', 'ساده', 'zwnj-compound'), false);

  // Urdu — distinct codepoints, easily confused with their Arabic lookalikes.
  check('ur do-chashmi ھ', has('ur', 'بھائی', 'do-chashmi-he'), true);
  check('ur nun ghunna ں', has('ur', 'ہاں', 'nun-ghunna'), true);
  check('ur retroflex ٹ', has('ur', 'ٹماٹر', 'retroflex'), true);
  check('ur bari ye ے', has('ur', 'کیے', 'bari-ye'), true);
  check('ur ain-initial', has('ur', 'عالم', 'ain-initial'), true);
  check('ur ain non-initial does not fire', has('ur', 'شمع', 'ain-initial'), false);

  // The honesty invariant, asserted structurally rather than trusted.
  for (const [lang, entry] of Object.entries(reg.languages)) {
    for (const t of entry.traps) {
      if (t.tagging !== 'manual') continue;
      const r = tagWord('xyz', entry.traps);
      if (r.tagged.includes(t.id)) fails.push(`${lang}/${t.id}: a manual class must never appear in tagged[]`);
    }
  }

  if (fails.length) {
    console.error(`trap-tag --selftest: FAILED — ${fails.length} problem(s):`);
    for (const f of fails) console.error(`  ✗ ${f}`);
    process.exit(1);
  }
  console.log('trap-tag --selftest: OK — every match kind verified with positive AND negative fixtures.');
}

// --- cli ---------------------------------------------------------------------

const args = process.argv.slice(2);
const reg = loadRegistry();
const argOf = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : null; };

if (args.includes('--validate')) {
  const problems = validate(reg);
  if (problems.length) {
    console.error(`trap-registry: FAILED — ${problems.length} problem(s):`);
    for (const p of problems) console.error(`  ✗ ${p}`);
    process.exit(1);
  }
  const n = Object.values(reg.languages).reduce((a, e) => a + e.traps.length, 0);
  console.log(`trap-registry: OK — ${n} trap classes across ${Object.keys(reg.languages).length} languages.`);
} else if (args.includes('--selftest')) {
  selftest(reg);
} else if (args.includes('--lang')) {
  report(reg, argOf('--lang'), argOf('--words'));
} else {
  console.log('usage: trap-tag.mjs [--validate | --selftest | --lang <code> [--words <file>]]');
  process.exit(1);
}
