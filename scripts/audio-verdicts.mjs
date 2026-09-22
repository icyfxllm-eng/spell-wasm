#!/usr/bin/env node
// CC-AUDIO-CLARITY v1.1 F2 — the intelligibility harness.
//
// Transcribes served clips with TWO recognizers and writes config/audio-verdicts.json.
// Neither recognizer is ever shown the bank text (I3): they are handed audio and
// nothing else, which is the whole point — a recognizer told the answer would
// agree with it.
//
//   node scripts/audio-verdicts.mjs --lang en --limit 200
//   node scripts/audio-verdicts.mjs --lang en --dry-run     (what it WOULD do)
//
// Requirements, and why this refuses rather than guesses when they are missing:
//   * whisper.cpp + a multilingual model  (WHISPER_MODEL=/path/ggml-*.bin)
//   * a second, independent recognizer    (census D2: Google STT, already wired)
//   * the clips themselves. Regeneration is lazy, so a clip exists only once it
//     has been played or fetched.
//
// A verdict is evidence. Running this with a recognizer missing would produce
// confident-looking nonsense, so it stops instead.
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const OUT = 'config/audio-verdicts.json';
const args = Object.fromEntries(
  process.argv.slice(2).flatMap((a, i, all) => (a.startsWith('--') ? [[a.slice(2), all[i + 1]?.startsWith('--') === false ? all[i + 1] : true]] : [])),
);

/// Census C8: what a recognizer's output is compared against, per language.
/// zh is the odd one — the player types pinyin, but a recognizer hears and
/// returns hanzi, so hanzi is the key. ja is unresolved: a recognizer may
/// return kanji for a kana entry, which would read as a false FAIL, so it is
/// UNSCORABLE until Eric decides (the spec's own mechanism for exactly this).
const KEY = {
  en: 'surface', es: 'surface', fr: 'surface', de: 'surface', pt: 'surface',
  pl: 'surface', sw: 'surface', fil: 'surface', vi: 'surface',
  ru: 'surface-yo',      // ё/е equivalence, per CC-PLAYER-CONTRACT D1
  hi: 'surface', ko: 'surface',
  ar: 'surface-bare',    // diacritics stripped from the recognizer side
  zh: 'hanzi',
  ja: 'unscorable',
};

/// I9: every comparison runs on NFC, case-folded, punctuation stripped.
function normalize(s, key) {
  let out = String(s).normalize('NFC').toLowerCase().replace(/[.,!?;:"'`()\[\]{}…—–-]/g, '').trim();
  if (key === 'surface-yo') out = out.replace(/ё/g, 'е');
  if (key === 'surface-bare') out = out.replace(/[ً-ْٰ]/g, '');
  return out;
}

function whisper(wav, lang) {
  const model = process.env.WHISPER_MODEL;
  if (!model || !existsSync(model)) {
    throw new Error('WHISPER_MODEL is not set to a model file. A verdict without a recognizer is not a verdict.');
  }
  // --no-prompt and no initial text: the recognizer must never see bank text (I3).
  const out = execFileSync('whisper-cli', ['-m', model, '-l', lang, '-nt', '-otxt', '-of', wav, wav], { encoding: 'utf8' });
  return (existsSync(`${wav}.txt`) ? readFileSync(`${wav}.txt`, 'utf8') : out).trim();
}

function verdictFor(word, key, heard) {
  if (key === 'unscorable') return 'Unscorable';
  const want = normalize(word, key);
  const hits = heard.filter((h) => normalize(h, key) === want).length;
  if (hits >= 2) return 'Pass';
  if (hits === 1) return 'Weak';
  return 'Fail';
}

async function main() {
  const lang = args.lang || 'en';
  const key = KEY[lang];
  if (!key) throw new Error(`census C8 names no comparison key for ${lang}`);
  const store = existsSync(OUT) ? JSON.parse(readFileSync(OUT, 'utf8')) : { version: 1, measured: [], clips: {} };

  if (args['dry-run']) {
    console.log(`lang=${lang} key=${key}`);
    console.log(`whisper model: ${process.env.WHISPER_MODEL || '(unset)'} `);
    console.log(`second recognizer: ${process.env.STT_ENDPOINT || '(unset — census D2 says Google STT, already wired server-side)'}`);
    console.log(`existing verdicts: ${Object.keys(store.clips).length}`);
    console.log('Nothing was written. Set WHISPER_MODEL and STT_ENDPOINT to measure.');
    return;
  }

  // Deliberately fails loudly. See the header: a half-measured language would
  // put PASS beside clips nobody checked.
  if (!process.env.WHISPER_MODEL) throw new Error('WHISPER_MODEL unset — refusing to write verdicts with one recognizer');
  if (!process.env.STT_ENDPOINT) throw new Error('STT_ENDPOINT unset — F2 needs two independent recognizers (D2)');

  const words = JSON.parse(readFileSync(args.words || `audio_clarity/words-${lang}.json`, 'utf8'));
  const tmp = mkdtempSync(join(tmpdir(), 'clarity-'));
  let done = 0;
  for (const { word, variant = 'normal', url } of words.slice(0, Number(args.limit) || words.length)) {
    const mp3 = join(tmp, `${done}.mp3`);
    const wav = join(tmp, `${done}.wav`);
    execFileSync('curl', ['-s', '-o', mp3, url]);
    execFileSync('afconvert', ['-f', 'WAVE', '-d', 'LEI16@16000', mp3, wav]);
    const heard = [whisper(wav, lang), await secondRecognizer(wav, lang)];
    store.clips[`${lang}|${word}|${variant}`] = { v: verdictFor(word, key, heard) };
    done += 1;
  }
  if (!store.measured.includes(lang)) store.measured.push(lang);
  writeFileSync(OUT, `${JSON.stringify(store, null, 2)}\n`);
  console.log(`audio-verdicts: ${done} clips measured for ${lang}`);
}

async function secondRecognizer(wav, lang) {
  const r = await fetch(process.env.STT_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    // Audio and a language. No word, no hint, no vocabulary (I3).
    body: JSON.stringify({ lang, sampleRate: 16000, audio: readFileSync(wav).toString('base64') }),
  });
  const j = await r.json().catch(() => ({}));
  return j.transcript || '';
}

main().catch((e) => {
  console.error(`audio-verdicts: ${e.message}`);
  process.exit(1);
});
