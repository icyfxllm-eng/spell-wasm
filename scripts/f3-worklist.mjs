#!/usr/bin/env node
// CC-AUDIO-CLARITY v1.1 F3 — the auditor's worklist.
//
// F3 rows may only come from a native-speaker auditor (see
// audio/lexicon/README.md); nothing here writes an IPA value, and nothing here
// ever should. What it does is turn F2's verdicts into a short, specific
// worklist so the ask is "please read these 9 clips and write what they should
// say" rather than "please audit the audio".
//
//   node scripts/f3-worklist.mjs --lang en
//
// Output is a TSV with the lexicon's own header and the `ipa`, `auditor` and
// `signed_at` columns left EMPTY, so a completed worklist is already the file
// it needs to become -- minus the two columns only a person can fill.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';

const args = Object.fromEntries(
  process.argv.slice(2).flatMap((a, i, all) => (a.startsWith('--') ? [[a.slice(2), all[i + 1]?.startsWith('--') === false ? all[i + 1] : true]] : [])),
);
const lang = args.lang || 'en';
const STORE = 'config/audio-verdicts.json';
// The transcripts are the whole point of this file -- an auditor needs to see
// what a machine heard -- and they live beside the verdicts rather than in
// them, because config/ is compiled into the app. See audio-verdicts.mjs.
const TRANSCRIPTS = 'audio_clarity/transcripts.json';
if (!existsSync(STORE)) throw new Error(`${STORE} does not exist — run scripts/audio-verdicts.mjs first`);
const store = JSON.parse(readFileSync(STORE, 'utf8'));
const heardBy = existsSync(TRANSCRIPTS) ? JSON.parse(readFileSync(TRANSCRIPTS, 'utf8')) : {};

/// A HEURISTIC triage hint, and labelled as one everywhere it appears. Most
/// of en's Fails are not synthesis faults at all: a recognizer writing
/// "12" for `twelve` or "M" for `am` is following a transcription convention,
/// and one that does not know `onomatopoeia` fails on vocabulary rather than
/// on the clip. An auditor's time should go to the words where the audio is
/// genuinely in question, so the ones that are probably not get named --
/// as a guess to check, never as a verdict.
function guessWhy(word, heard) {
  const h = heard.map((x) => String(x).toLowerCase().replace(/[^a-z0-9']/g, ''));
  if (h.some((x) => /^\d+$/.test(x))) return 'guess: number form, not a mishearing';
  // A LETTER name is one letter, optionally possessive -- "M", "C's". Not
  // "as", which is a word; an earlier version matched it and mislabelled
  // `aims`.
  if (word.length <= 4 && h.some((x) => /^[a-z]('s)?$/.test(x))) {
    return 'guess: recognizer wrote a letter name';
  }
  // Long AND nothing like the word: `discussed` heard as "disgust" is a near
  // homophone, not a vocabulary miss, and sharing an opening is the cheapest
  // way to tell the two apart.
  if (word.length >= 9 && !h.some((x) => x.slice(0, 3) === word.slice(0, 3))) {
    return 'guess: rare word, may be outside the recognizer vocabulary';
  }
  if (h.some((x) => x && Math.abs(x.length - word.length) <= 2 && x[0] !== word[0])) {
    return 'guess: onset heard as a different consonant';
  }
  return '';
}

const rows = [];
for (const [k, e] of Object.entries(store.clips)) {
  const [l, word, variant] = k.split('|');
  if (l !== lang) continue;
  // Fail is the whole point: I1 withholds the clip, so today the player hears
  // the platform voice instead, and the platform voice is measurably WORSE on
  // the one class this all started with (see audio_clarity/bakeoff/en.md).
  // Weak is included because one recognizer missing a word is where `half`
  // lived -- the word a tester actually reported.
  // Fail by default. Weak is one recognizer missing the word, which is where
  // `half` lived -- the case a tester actually reported -- but there are 141
  // of them in en against 35 Fails, and handing an auditor 176 clips is how a
  // worklist gets put down and never picked up. --include-weak asks for them.
  if (e.v === 'Weak' && !args['include-weak']) continue;
  if (e.v !== 'Fail' && e.v !== 'Weak') continue;
  const heard = heardBy[k] || e.heard || [];
  rows.push({ word, variant, verdict: e.v, heard, guess: guessWhy(word, heard) });
}
rows.sort((a, b) => (a.verdict === b.verdict ? a.word.localeCompare(b.word) : a.verdict === 'Fail' ? -1 : 1));

const tsv = ['word\tipa\tprovider\tauditor\tsigned_at\t# verdict\t# heard\t# triage (heuristic)'];
for (const r of rows) {
  tsv.push(`${r.word}\t\tgoogle\t\t\t${r.verdict}\t${r.heard.map((h) => JSON.stringify(h)).join(' / ')}\t${r.guess}`);
}
const out = `audio/lexicon/${lang}.worklist.tsv`;
writeFileSync(out, `${tsv.join('\n')}\n`);

const fails = rows.filter((r) => r.verdict === 'Fail').length;
console.log(
  `f3-worklist: ${rows.length} words for ${lang} (${fails} Fail, ${rows.length - fails} Weak) -> ${out}\n` +
  '  The ipa, auditor and signed_at columns are empty on purpose. An unsigned\n' +
  '  row is a guess about how a language sounds, and guesses are what broke the\n' +
  '  audio in the first place.',
);
