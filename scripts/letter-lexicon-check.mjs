#!/usr/bin/env node
// CC-SPELL-ALOUD — the letter lexicons are the whole linguistic knowledge of
// voice spelling (Invariant I4: no letter-name literal lives anywhere else), and
// until now nothing checked them. scripts/gen-letter-lexicons.py drafted thirteen
// of the fifteen by machine on 2026-07-27 and they still say "pending native
// review"; the mic renders in all fifteen languages regardless.
//
// This checks the half a machine CAN check: that every letter a player must
// actually produce in a language is reachable by speaking. It cannot check that
// a letter NAME is right — whether a Swahili speaker really says "che" for c is
// a question for a Swahili speaker, and that review is still open.
//
//   node scripts/letter-lexicon-check.mjs            # check the real lexicons
//   node scripts/letter-lexicon-check.mjs --selftest # prove it bites
//
// KNOWN_GAPS is a ratchet, not an excuse. Today's holes are listed with their
// reasons so the build stays green for everyone while the review is pending; a
// NEW hole fails, and a listed hole that someone FIXES also fails until its
// entry is removed, so the list cannot rot into decoration. Same shape as the
// tier-list-check allowlist, and chosen over a hard block because closing these
// needs native speakers, which is weeks, and a red main for weeks helps no one.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';
const TIERS = ['easy', 'medium', 'hard', 'expert'];
const COMMAND_IDS = new Set(['delete', 'clear', 'done']);

/// Languages where the mic is not offered at all. Their lexicon is not checked
/// for coverage, because coverage is a question about a feature the player
/// cannot reach — and more importantly they must not be sent to native review.
export const WITHDRAWN = {
  ko: 'Withdrawn by Eric 2026-10-03. Korean voice spelling could not produce a Korean word: compatibility jamo neither compose nor match, and the obvious swap to conjoining jamo fails too because the same letter is a different codepoint as onset and coda (norm.rs::ko_voice_spelling_needs_positional_jamo_not_a_lexicon_edit pins both). Needs an IME-style composition state machine. The lexicon stays on disk, with correct letter NAMES, ready for the day that lands. Do not pay a speaker to review it before then.',
};

export const KNOWN_GAPS = {
  ar: { chars: 'أؤإئ', why: 'hamza carriers; أ alone appears 586 times. A real hole — the review must add them.' },
  fr: { chars: 'œ', why: 'the oe ligature, 9 occurrences. Needs a spoken name ("o e lie" or similar) from a speaker.' },
  hi: { chars: 'ङञ', why: 'two nasals, 34 occurrences between them. Real, small.' },
  ja: { chars: 'ぁぃぅぇぉゎゔ', why: 'small kana and vu. Spoken as "small a" etc.; the draft has no convention for it.' },
  vi: {
    chars: 'fjwzàáãèéìíòóõùúýĩũạảấầẩẫậắằẳẵặẹẻẽếềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷ',
    why: 'the largest real hole. `diacritics` is EMPTY, so every toned vowel is unspeakable and a Vietnamese player can say base letters only. f/j/w/z are loanword letters the alphabet lacks. This one needs a scheme (letter + tone name), not just entries.',
  },
  zh: { chars: 'ü', why: 'not a hole: players type tone-numbered pinyin with v, and the grader converts v to ü (game.rs). Listed so the arithmetic is honest.' },
};

/// Mirrors spell_aloud::norm_word / norm_phrase. A key that normalizes to the
/// same string as another key silently overwrites it at load, so the check has
/// to normalize the same way the parser does or it would miss exactly that.
const EDGE = /^[^\p{L}\p{N}'-]+|[^\p{L}\p{N}'-]+$/gu;
export function normPhrase(p) {
  return String(p).split(/\s+/).filter(Boolean)
    .map((w) => w.replace(EDGE, '').normalize('NFC').toLowerCase())
    .join(' ');
}

/// What a player must actually PRODUCE in this language. Not the same as the
/// characters in the bank: Mandarin is answered in tone-numbered pinyin, and a
/// Korean syllable is typed as the jamo it decomposes to.
export function requiredChars(bankDir, lang) {
  const out = new Set();
  for (const tier of TIERS) {
    const p = path.join(bankDir, lang, `${tier}.txt`);
    if (!fs.existsSync(p)) continue;
    for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
      let w = line.trim();
      if (!w || w.startsWith('#')) continue;
      if (lang === 'zh') {
        for (const c of w.split('|')[0].toLowerCase()) if (/[\p{L}\p{N}]/u.test(c)) out.add(c);
        continue;
      }
      if (w.includes('|')) w = w.slice(w.lastIndexOf('|') + 1);
      if (lang === 'ko') w = w.normalize('NFD');
      for (const c of w.toLowerCase()) if (/\p{L}/u.test(c)) out.add(c);
    }
  }
  return out;
}

/// Everything the lexicon can put on screen. A multi-character target (a digraph
/// like "ng") also supplies each of its characters.
/// `named` is what a spoken NAME maps to. `whole` adds the letters only an
/// ambiguity chip can produce. They are kept apart deliberately: coverage may
/// count a chip-only letter as reachable, but an ambiguous entry must not
/// validate itself by being the only thing that mentions its own choices.
export function producible(doc) {
  const named = new Set();
  for (const group of ['letterNames', 'homophones', 'multigraph', 'diacritics']) {
    for (const v of Object.values(doc[group] ?? {})) named.add(String(v).toLowerCase());
  }
  const whole = new Set(named);
  for (const choices of Object.values(doc.ambiguous ?? {})) {
    if (Array.isArray(choices)) for (const c of choices) whole.add(String(c).toLowerCase());
  }
  const chars = new Set();
  for (const t of whole) for (const c of t) chars.add(c);
  return { named, whole, chars };
}

/// The enabled set, read from consts.rs rather than duplicated here: that array
/// IS the capability, and a second copy would be the scattered conditional its
/// own doc comment forbids.
export function enabledLangs(constsSrc) {
  const m = constsSrc.match(/VOICE_SPELL_LANGS:\s*\[&str;\s*\d+\]\s*=\s*\[([^\]]*)\]/s);
  if (!m) return null;
  return new Set(m[1].split(',').map((x) => x.trim().toLowerCase()).filter(Boolean));
}

export function check(lexDir, bankDir, known = KNOWN_GAPS, enabled = null) {
  const bad = [];
  const langs = [];
  if (!fs.existsSync(lexDir)) return { bad: [`${lexDir}: no lexicon directory`], langs };

  for (const f of fs.readdirSync(lexDir).filter((x) => x.endsWith('.json')).sort()) {
    const lang = f.replace(/\.json$/, '');
    if (enabled && !enabled.has(lang)) {
      // Not offered: nothing to cover. Flagged only if it is silently off
      // rather than deliberately withdrawn, because that is a mistake.
      if (!WITHDRAWN[lang]) {
        bad.push(`${lang}: has a lexicon but is not in VOICE_SPELL_LANGS, and is not recorded in WITHDRAWN — say which it is`);
      }
      continue;
    }
    if (WITHDRAWN[lang]) {
      bad.push(`${lang}: recorded as WITHDRAWN but still in VOICE_SPELL_LANGS — the mic is being offered for a language this file says cannot use it`);
    }
    langs.push(lang);
    let doc;
    try {
      doc = JSON.parse(fs.readFileSync(path.join(lexDir, f), 'utf8'));
    } catch (e) {
      bad.push(`${f}: not valid JSON — ${e.message}`);
      continue;
    }

    // L1 — a key that normalizes to nothing is dropped at load, silently.
    // L2 — two keys that normalize alike: the last one wins and the other is
    //      lost, which is a letter name that looks present and never fires.
    const seen = new Map();
    for (const group of ['letterNames', 'homophones', 'multigraph', 'diacritics']) {
      for (const [k, v] of Object.entries(doc[group] ?? {})) {
        const n = normPhrase(k);
        if (!n) { bad.push(`${lang}: ${group} key ${JSON.stringify(k)} normalizes to nothing and is dropped at load`); continue; }
        const prev = seen.get(n);
        if (prev && prev.v !== String(v)) {
          bad.push(`${lang}: ${JSON.stringify(k)} (${group}) and ${JSON.stringify(prev.k)} (${prev.group}) both normalize to ${JSON.stringify(n)} but mean ${JSON.stringify(v)} and ${JSON.stringify(prev.v)} — one is silently lost`);
        }
        seen.set(n, { k, v: String(v), group });
      }
    }

    // L3 — an unknown command id is dropped by Command::from_id, silently.
    for (const [k, id] of Object.entries(doc.commands ?? {})) {
      if (!COMMAND_IDS.has(String(id))) {
        bad.push(`${lang}: command ${JSON.stringify(k)} maps to ${JSON.stringify(id)}, not one of ${[...COMMAND_IDS].join('/')} — dropped at load`);
      }
    }

    const { named, whole, chars } = producible(doc);

    // L4 — an ambiguous name must offer exactly two real letters.
    for (const [k, choices] of Object.entries(doc.ambiguous ?? {})) {
      if (!Array.isArray(choices) || choices.length !== 2) {
        bad.push(`${lang}: ambiguous ${JSON.stringify(k)} must offer exactly two choices, got ${JSON.stringify(choices)}`);
        continue;
      }
      for (const c of choices) {
        if (!named.has(String(c).toLowerCase())) {
          bad.push(`${lang}: ambiguous ${JSON.stringify(k)} offers ${JSON.stringify(c)}, which no name in this lexicon can type`);
        }
      }
    }

    // L5 — the point of the file: every letter the bank needs must be sayable.
    const need = requiredChars(bankDir, lang);
    if (!need.size) continue;                 // no bank for this language here
    const allowed = new Set([...(known[lang]?.chars ?? '')]);
    const missing = [...need].filter((c) => !whole.has(c) && !chars.has(c));
    const fresh = missing.filter((c) => !allowed.has(c));
    if (fresh.length) {
      bad.push(`${lang}: ${fresh.length} character(s) no one can speak — ${fresh.join(' ')}. Add names to lexicons/letters/${lang}.json, or record them in KNOWN_GAPS with a reason.`);
    }
    // L6 — the ratchet. A recorded gap that is no longer missing must leave the
    // list, or the list slowly stops describing anything.
    const fixed = [...allowed].filter((c) => !missing.includes(c));
    if (fixed.length) {
      bad.push(`${lang}: KNOWN_GAPS lists ${fixed.join(' ')}, which ${fixed.length === 1 ? 'is' : 'are'} reachable now — delete ${fixed.length === 1 ? 'it' : 'them'} from the list in this script.`);
    }
  }
  return { bad, langs };
}

if (process.argv.includes('--selftest')) {
  const base = () => ({
    letterNames: { a: 'a', be: 'b' },
    homophones: {}, multigraph: {}, diacritics: {},
    commands: { futa: 'delete' }, ambiguous: {},
  });
  const bank = (dir, lang, words) => {
    fs.mkdirSync(path.join(dir, 'bank', lang), { recursive: true });
    fs.writeFileSync(path.join(dir, 'bank', lang, 'easy.txt'), `# header\n${words.join('\n')}\n`);
  };
  const run = (doc, words, known = {}, lang = 'sw', enabled = null) => {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), 'lexcheck-'));
    fs.mkdirSync(path.join(d, 'lex'));
    fs.writeFileSync(path.join(d, 'lex', `${lang}.json`), JSON.stringify(doc));
    bank(d, lang, words);
    const r = check(path.join(d, 'lex'), path.join(d, 'bank'), known, enabled);
    fs.rmSync(d, { recursive: true });
    return r.bad;
  };

  const cases = {
    clean: { bad: run(base(), ['ab']), want: null },
    unreachable_letter: { bad: run(base(), ['abc']), want: /no one can speak/ },
    gap_allowed: { bad: run(base(), ['abc'], { sw: { chars: 'c', why: 'test' } }), want: null },
    stale_allowlist: { bad: run(base(), ['ab'], { sw: { chars: 'c', why: 'test' } }), want: /reachable now/ },
    empty_key: { bad: run({ ...base(), homophones: { '!!': 'z' } }, ['ab']), want: /normalizes to nothing/ },
    normalized_collision: { bad: run({ ...base(), homophones: { ' BE. ': 'v' } }, ['ab']), want: /silently lost/ },
    same_value_collision_ok: { bad: run({ ...base(), homophones: { ' BE. ': 'b' } }, ['ab']), want: null },
    bad_command_id: { bad: run({ ...base(), commands: { futa: 'remove' } }, ['ab']), want: /not one of/ },
    ambiguous_wrong_arity: { bad: run({ ...base(), ambiguous: { x: ['a'] } }, ['ab']), want: /exactly two choices/ },
    ambiguous_unknown_letter: { bad: run({ ...base(), ambiguous: { x: ['a', 'q'] } }, ['ab']), want: /no name in this lexicon can type/ },
    digraph_supplies_its_chars: { bad: run({ ...base(), multigraph: { 'en gee': 'ng' } }, ['ang']), want: null },
    zh_reads_the_pinyin_half: { bad: run({ ...base(), letterNames: { yi: 'a', er: 'i', san: '4' } }, ['ai4|爱'], {}, 'zh'), want: null },
    // A language that is off must say WHY it is off. Silently disabled is the
    // state ko was in for months while the mic was still being offered.
    disabled_without_a_reason: { bad: run(base(), ['abc'], {}, 'sw', new Set(['en'])), want: /not recorded in WITHDRAWN/ },
    // ...and a withdrawn language must not still be offered.
    withdrawn_but_still_offered: { bad: run(base(), ['ab'], {}, 'ko', new Set(['ko'])), want: /still in VOICE_SPELL_LANGS/ },
    // A withdrawn language that is properly off is simply skipped, holes and all.
    withdrawn_and_off_is_skipped: { bad: run(base(), ['abc'], {}, 'ko', new Set(['en'])), want: null },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.bad.length === 0 : c.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.bad[0] ?? '') : (c.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 72) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`letter-lexicon-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('letter-lexicon-check selftest: OK');
  process.exit(0);
}

const enabled = enabledLangs(fs.readFileSync(`${ROOT}/src/consts.rs`, 'utf8'));
if (!enabled) {
  console.error('letter-lexicon-check: FAILED — could not read VOICE_SPELL_LANGS from src/consts.rs');
  process.exit(1);
}
const { bad, langs } = check(`${ROOT}/lexicons/letters`, `${ROOT}/assets/words`, KNOWN_GAPS, enabled);
if (bad.length) {
  console.error('letter-lexicon-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
const pending = Object.keys(KNOWN_GAPS).length;
const withdrawn = Object.keys(WITHDRAWN).join(', ');
console.log(`letter-lexicon-check: OK — ${langs.length} lexicons offered, every letter their banks need is sayable except ${pending} recorded gap(s) awaiting the native review; withdrawn: ${withdrawn || 'none'}.`);
