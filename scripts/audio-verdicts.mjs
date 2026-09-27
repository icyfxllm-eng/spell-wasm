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
import { KEY, normalize } from './lib/audio-norm.mjs';

const OUT = 'config/audio-verdicts.json';
// The transcripts live BESIDE the verdicts, not in them. config/ is compiled
// into the wasm with include_str!, and `heard` is the one field the app
// explicitly never reads -- audio_verdict.rs says so in its own comment. At
// 3,165 English clips it was 250 KB of the 364 KB blob, in an offline-first
// bundle, for data only a report looks at; fifteen measured languages would
// have been megabytes. Same write, two files.
const TRANSCRIPTS = 'audio_clarity/transcripts.json';
const args = Object.fromEntries(
  process.argv.slice(2).flatMap((a, i, all) => (a.startsWith('--') ? [[a.slice(2), all[i + 1]?.startsWith('--') === false ? all[i + 1] : true]] : [])),
);




function whisper(wav, lang) {
  const model = process.env.WHISPER_MODEL;
  if (!model || !existsSync(model)) {
    throw new Error('WHISPER_MODEL is not set to a model file. A verdict without a recognizer is not a verdict.');
  }
  // --no-prompt and no initial text: the recognizer must never see bank text (I3).
  const out = execFileSync('whisper-cli', ['-m', model, '-l', lang, '-nt', '-otxt', '-of', wav, wav], { encoding: 'utf8' });
  return (existsSync(`${wav}.txt`) ? readFileSync(`${wav}.txt`, 'utf8') : out).trim();
}

/// F2 step 4: a word in the collision table is not recognizable by design, so a
/// recognizer returning a different member of its set proves nothing about the
/// clip. Without this the gate withholds "for" because both recognizers heard
/// "four", and "some" because they heard "sum" -- clear clips of common words,
/// silenced by a test that did not know they were homophones.
function collisionSets(lang) {
  const p = `assets/words/${lang}/homophones.txt`;
  if (!existsSync(p)) return [];
  return readFileSync(p, 'utf8')
    .split('\n')
    .filter((l) => l.trim() && !l.startsWith('#'))
    .map((l) => l.trim().split(/\s+/));
}

function verdictFor(word, key, heard, sets, lang) {
  if (key === 'unscorable') return 'Unscorable';
  const want = normalize(word, key, lang);
  const hits = heard.filter((h) => normalize(h, key, lang) === want).length;
  if (hits >= 2) return 'Pass';
  if (hits === 1) return 'Weak';
  // Nothing matched. Before calling it FAIL, ask whether what WAS heard is a
  // member of this word's collision set.
  const mine = sets.find((set) => set.some((w) => normalize(w, key, lang) === want));
  if (mine) {
    const heardMember = heard.some((h) => {
      const n = normalize(h, key, lang);
      return n && mine.some((w) => normalize(w, key, lang) === n);
    });
    if (heardMember) return 'ExemptHomophone';
  }
  return 'Fail';
}

async function main() {
  const lang = args.lang || 'en';
  const key = KEY[lang];
  if (!key) throw new Error(`census C8 names no comparison key for ${lang}`);
  const store = existsSync(OUT) ? JSON.parse(readFileSync(OUT, 'utf8')) : { version: 1, measured: [], clips: {} };
  const heardBy = existsSync(TRANSCRIPTS) ? JSON.parse(readFileSync(TRANSCRIPTS, 'utf8')) : {};
  // A store written before the split carries its transcripts inline; lift them
  // out once rather than leaving two shapes in circulation.
  for (const [k, e] of Object.entries(store.clips)) {
    if (e.heard) { heardBy[k] = e.heard; delete e.heard; }
  }
  const save = (st, tr) => {
    writeFileSync(OUT, `${JSON.stringify(st, null, 2)}\n`);
    writeFileSync(TRANSCRIPTS, `${JSON.stringify(tr, null, 1)}\n`);
  };

  // Re-score from the transcripts already stored, touching no network and no
  // recognizer. A change to the normaliser is a change to the VERDICT, not to
  // what was heard, and re-measuring 3,165 clips to apply one is both slow and
  // a bill. `heard` is recorded for exactly this reason.
  if (args.rescore) {
    const sets = collisionSets(lang);
    let changed = 0;
    for (const [k, e] of Object.entries(store.clips)) {
      const [l, word] = k.split('|');
      if (l !== lang) continue;
      const heard = heardBy[k];
      if (!heard?.length) continue; // measured before transcripts were recorded
      const v = verdictFor(word, key, heard, sets, lang);
      if (v !== e.v) { changed += 1; e.v = v; }
    }
    save(store, heardBy);
    console.log(`audio-verdicts: re-scored ${lang} from stored transcripts — ${changed} verdict(s) changed`);
    return;
  }

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
  const sets = collisionSets(lang);
  const tmp = mkdtempSync(join(tmpdir(), 'clarity-'));
  let done = 0;
  for (const { word, variant = 'normal', url } of words.slice(0, Number(args.limit) || words.length)) {
    const mp3 = join(tmp, `${done}.mp3`);
    const wav = join(tmp, `${done}.wav`);
    execFileSync('curl', ['-s', '-o', mp3, url]);
    execFileSync('afconvert', ['-f', 'WAVE', '-d', 'LEI16@16000', mp3, wav]);
    const heard = [whisper(wav, lang), await secondRecognizer(wav, lang)];
    // Record WHAT WAS HEARD beside the verdict. A pilot run produced a FAIL
    // for "for" that could not be explained afterwards -- "for" is in the
    // collision table, so it should have been exempt -- and the transcripts
    // were gone. A verdict nobody can account for is not evidence.
    const k = `${lang}|${word}|${variant}`;
    store.clips[k] = { v: verdictFor(word, key, heard, sets, lang) };
    heardBy[k] = heard;
    done += 1;
  }
  // Only a run that covered the whole servable set may call a language
  // measured: F7's gate is strict about measured languages, and a sample that
  // claimed the title would make the gate lie about everything it did not look
  // at. A partial run still writes its verdicts -- they are evidence either way.
  if (args.complete && !store.measured.includes(lang)) store.measured.push(lang);
  save(store, heardBy);
  console.log(
    `audio-verdicts: ${done} clips measured for ${lang}` +
      (args.complete ? ' (marked complete)' : ' (partial — the language is not marked measured)'),
  );
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
