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
  // Not just a bare numeral: `ones` comes back as "1s" and `tend` as "10.",
  // and an inflected or punctuated digit form is the same convention.
  if (h.some((x) => /^\d+'?s?$/.test(x))) return 'guess: number form, not a mishearing';
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

/// Is this a question an auditor can answer at all?
///
/// A signed IPA row changes how the clip is SPOKEN. It cannot change how a
/// recognizer chooses to WRITE what it heard. When a machine renders `for` as
/// "4", `am` as "M" or `patients` as "patience", the audio may be perfect --
/// the transcript is a convention or a homophone, and no pronunciation an
/// auditor writes will ever make that row pass F2. Those rows sat in the
/// worklist looking like outstanding work, and the first thing an auditor
/// would have done is waste an hour on them.
///
/// Splitting them out is triage, not a verdict, and the reason travels with
/// the row so a person can overrule it.
///
/// cmudict is used ONLY to recognise that two spellings sound identical. It
/// is never a source of IPA: F3 rows come from a native-speaker auditor and
/// from nobody else, this script has never written one, and it still does not.
const CMUDICT = 'tools/wordpipe/sources/cmudict.dict';
let homophones = null;   // null = unavailable, so the test is skipped, not failed
if (existsSync(CMUDICT)) {
  homophones = new Map();
  for (const line of readFileSync(CMUDICT, 'utf8').split('\n')) {
    if (!line || line.startsWith(';;;')) continue;
    const sp = line.indexOf(' ');
    if (sp < 1) continue;
    const w = line.slice(0, sp).replace(/\(\d+\)$/, '').toLowerCase();
    const phones = line.slice(sp + 1).trim().replace(/\d/g, '');
    if (!homophones.has(w)) homophones.set(w, new Set());
    homophones.get(w).add(phones);
  }
}
const soundsTheSame = (a, b) => {
  if (!homophones) return false;
  const A = homophones.get(a); const B = homophones.get(b);
  if (!A || !B) return false;
  for (const x of A) if (B.has(x)) return true;
  return false;
};

function auditable(word, heard, guess) {
  const h = heard.map((x) => String(x).toLowerCase().replace(/[^a-z0-9']/g, '')).filter(Boolean);
  if (guess.includes('number form')) return [false, 'a transcription convention: the machine wrote a numeral'];
  if (guess.includes('letter name')) return [false, 'a transcription convention: the machine wrote a letter name'];
  if (guess.includes('outside the recognizer vocabulary')) return [false, 'vocabulary, not audio: the word is not one the recognizer knows'];
  if (h.length && h.every((x) => x === word || soundsTheSame(word, x))) {
    return [false, 'a true homophone by cmudict: the clip is right and the spelling is the machine\'s choice'];
  }
  return [true, ''];
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
  const guess = guessWhy(word, heard);
  const [ask, why] = auditable(word, heard, guess);
  rows.push({ word, variant, verdict: e.v, heard, guess, ask, why });
}
rows.sort((a, b) => (a.verdict === b.verdict ? a.word.localeCompare(b.word) : a.verdict === 'Fail' ? -1 : 1));

const ask = rows.filter((r) => r.ask);
const skip = rows.filter((r) => !r.ask);

const tsv = ['word\tipa\tprovider\tauditor\tsigned_at\t# verdict\t# heard\t# triage (heuristic)'];
for (const r of ask) {
  tsv.push(`${r.word}\t\tgoogle\t\t\t${r.verdict}\t${r.heard.map((h) => JSON.stringify(h)).join(' / ')}\t${r.guess}`);
}
const out = `audio/lexicon/${lang}.worklist.tsv`;
writeFileSync(out, `${tsv.join('\n')}\n`);

// Deliberately NOT in audio/lexicon/: that directory is for files the server
// loads, and this one must never be mistaken for a lexicon awaiting signature.
const skipped = ['word\t# verdict\t# heard\t# why no IPA will fix this'];
for (const r of skip) {
  skipped.push(`${r.word}\t${r.verdict}\t${r.heard.map((h) => JSON.stringify(h)).join(' / ')}\t${r.why}`);
}
const skipOut = `audio_clarity/f3-not-auditable-${lang}.tsv`;
writeFileSync(skipOut, `${skipped.join('\n')}\n`);

const fails = ask.filter((r) => r.verdict === 'Fail').length;
console.log(
  `f3-worklist: ${rows.length} failing clip(s) for ${lang}; ${ask.length} are worth an auditor's time ` +
  `(${fails} Fail, ${ask.length - fails} Weak) -> ${out}\n` +
  `  ${skip.length} set aside as unfixable by any IPA -> ${skipOut}\n` +
  (homophones ? '' : `  (cmudict absent at ${CMUDICT}, so the homophone test was SKIPPED and some\n   unfixable rows may still be in the worklist)\n`) +
  '  The ipa, auditor and signed_at columns are empty on purpose. An unsigned\n' +
  '  row is a guess about how a language sounds, and guesses are what broke the\n' +
  '  audio in the first place.',
);
