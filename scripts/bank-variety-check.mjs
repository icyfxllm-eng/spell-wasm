#!/usr/bin/env node
// CC-AUDIO-CLARITY v1.1 C10 / D17 — one variety per bank.
//
// Census C10 (2026-09-22): no bank declared a spelling variety, and English
// shipped BOTH members of three pairs — center/centre, gray/grey,
// behavior/behaviour — while Portuguese shipped desporto beside esporte.
//
// What this is NOT: an unwinnable item. Those English pairs are already
// accept-all sets in the collision table, and game.rs runs homophones::accepts
// on every English answer (CC-SENSE-CUE F2(a)), so a player typing either
// spelling is accepted. I claimed otherwise before checking, and it was wrong.
//
// What it IS: variety drift. A US voice asking for "theatre", and a Brazilian
// player being taught "desporto". D17 requires a voice's variety to match the
// variety its bank grades, and a bank that teaches two varieties teaches the
// wrong one to somebody.
//
// Two laws:
//   1. A bank never contains both members of a known variety pair.
//   2. Where the two differ, the bank keeps the member that matches the variety
//      config/bank-variety.json declares for that language.
//
// The pair list is CURATED, deliberately. An earlier draft derived pairs from
// suffix rules (-our/-or, -re/-er) and "proved" that absence should be spelled
// absense. Rules of that shape do not know which words are words.
import { existsSync, readFileSync } from 'node:fs';

const TIERS = ['easy', 'medium', 'hard', 'expert'];
const registry = JSON.parse(readFileSync('config/bank-variety.json', 'utf8')).varieties;

/// Each entry: the form each variety uses. A language's declared variety picks
/// the column; every other column is an error if it appears.
const PAIRS = {
  en: [
    // -our / -or
    ['color', 'colour'], ['favor', 'favour'], ['honor', 'honour'], ['labor', 'labour'],
    ['behavior', 'behaviour'], ['neighbor', 'neighbour'], ['humor', 'humour'],
    ['rumor', 'rumour'], ['armor', 'armour'], ['harbor', 'harbour'], ['vapor', 'vapour'],
    ['flavor', 'flavour'], ['odor', 'odour'], ['savor', 'savour'], ['splendor', 'splendour'],
    // -er / -re
    ['center', 'centre'], ['theater', 'theatre'], ['meter', 'metre'], ['liter', 'litre'],
    ['fiber', 'fibre'], ['somber', 'sombre'], ['specter', 'spectre'],
    // -ize / -ise and friends
    ['organize', 'organise'], ['realize', 'realise'], ['recognize', 'recognise'],
    ['apologize', 'apologise'], ['analyze', 'analyse'], ['paralyze', 'paralyse'],
    ['organization', 'organisation'], ['civilization', 'civilisation'],
    // -se / -ce
    ['defense', 'defence'], ['offense', 'offence'], ['pretense', 'pretence'], ['license', 'licence'],
    // doubled l
    ['traveled', 'travelled'], ['traveling', 'travelling'], ['traveler', 'traveller'],
    ['canceled', 'cancelled'], ['modeling', 'modelling'], ['labeled', 'labelled'],
    ['jewelry', 'jewellery'], ['counselor', 'counsellor'],
    // the rest
    ['gray', 'grey'], ['plow', 'plough'], ['program', 'programme'], ['catalog', 'catalogue'],
    ['dialog', 'dialogue'], ['aluminum', 'aluminium'], ['tire', 'tyre'], ['curb', 'kerb'],
    ['mold', 'mould'], ['smolder', 'smoulder'], ['draft', 'draught'], ['pajamas', 'pyjamas'],
  ],
  pt: [
    // Brazilian / European, same meaning.
    ['esporte', 'desporto'], ['equipe', 'equipa'],
    ['ônibus', 'autocarro'], ['celular', 'telemóvel'],
    // NOT pairs, though they look like them: "comboio" is ordinary Brazilian
    // for a convoy and "elétrico" is the everyday adjective, so listing either
    // as the European form of trem/tram would delete good Brazilian words. A
    // first draft of this list did exactly that.
    ['banheiro', 'casa de banho'], ['açougue', 'talho'], ['geladeira', 'frigorífico'],
    ['xícara', 'chávena'], ['suco', 'sumo'], ['grama', 'relva'],
  ],
};
/// Which column each variety uses: 0 is the first form, 1 the second.
const COLUMN = { 'en-US': 0, 'en-GB': 1, 'pt-BR': 0, 'pt-PT': 1 };

function bank(lang) {
  const out = new Set();
  for (const t of TIERS) {
    const p = `assets/words/${lang}/${t}.txt`;
    if (!existsSync(p)) continue;
    for (const line of readFileSync(p, 'utf8').split('\n')) {
      const w = line.trim();
      if (w && !w.startsWith('#')) out.add(w.toLowerCase());
    }
  }
  return out;
}

const problems = [];
for (const [lang, pairs] of Object.entries(PAIRS)) {
  const variety = registry[lang];
  const col = COLUMN[variety];
  if (col === undefined) {
    problems.push(`${lang}: config/bank-variety.json says ${variety}, which this check has no column for`);
    continue;
  }
  const words = bank(lang);
  for (const pair of pairs) {
    const keep = pair[col];
    const drop = pair[1 - col];
    if (words.has(keep) && words.has(drop)) {
      problems.push(`${lang}: the bank holds both "${keep}" and "${drop}" — ${variety} should ask for one variety`);
    } else if (!words.has(keep) && words.has(drop)) {
      problems.push(`${lang}: the bank asks for "${drop}" but ${variety} grades "${keep}"`);
    }
  }
}

if (problems.length) {
  console.error('bank-variety-check: FAILED');
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}
const counted = Object.entries(PAIRS).map(([l, p]) => `${l} ${p.length}`).join(', ');
console.log(`bank-variety-check: OK — one variety per bank (${counted} pairs checked)`);
