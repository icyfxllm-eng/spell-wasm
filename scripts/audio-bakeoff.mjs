#!/usr/bin/env node
// CC-AUDIO-CLARITY v1.1 F4 — the bake-off.
//
// Runs every candidate voice for a language through the same two recognizers
// F2 uses and ranks them by how often a machine hears the right word. Writes
// audio_clarity/bakeoff/<lang>.md and RECOMMENDS ONLY: switching a language's
// default voice needs Eric's signature, per language (D6).
//
//   BAKEOFF=1 on the server, then:
//   node scripts/audio-bakeoff.mjs --lang en --voices en-US-Neural2-D,en-US-Neural2-F
//
// Why this exists: the Phase B pilot found `half` transcribed as "have" and
// `leaf` as "leave" by both recognizers, while `thief` and `fifth` passed.
// Word-final /f/ rendered close to /v/ is a property of a voice, not of the
// pipeline — F1 had already padded these clips and C3 had already shown there
// was no trimmer. Either a signed F3 row fixes each word, or a different voice
// fixes the class. This measures the second.
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const args = Object.fromEntries(
  process.argv.slice(2).flatMap((a, i, all) => (a.startsWith('--') ? [[a.slice(2), all[i + 1]?.startsWith('--') === false ? all[i + 1] : true]] : [])),
);
const lang = args.lang || 'en';
const SPEAK = process.env.SPEAK_ENDPOINT || 'http://127.0.0.1:8000/api/speak';

/// F4 step 2: a candidate whose variety does not match the bank's is excluded
/// BEFORE it is scored, not scored badly. A voice that says the wrong variety
/// is not a worse option, it is not an option (D17).
function eligible(voice) {
  const want = JSON.parse(readFileSync('config/bank-variety.json', 'utf8')).varieties[lang];
  return want ? voice.startsWith(`${want}-`) : true;
}

function heardBoth(wav) {
  const model = process.env.WHISPER_MODEL;
  const w = execFileSync('whisper-cli', ['-m', model, '-l', lang, '-nt', wav], { encoding: 'utf8' }).trim();
  const b64 = readFileSync(wav).toString('base64');
  const r = execFileSync('curl', ['-s', '-X', 'POST', process.env.STT_ENDPOINT, '-H', 'Content-Type: application/json',
    '-d', JSON.stringify({ lang, sampleRate: 16000, audio: b64 })], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  let g = '';
  try { g = JSON.parse(r).transcript || ''; } catch { g = ''; }
  return [w, g];
}

const norm = (s) => String(s).normalize('NFC').toLowerCase().replace(/[.,!?;:"'`()\[\]{}…—–-]/g, '').trim();

async function main() {
  for (const need of ['WHISPER_MODEL', 'STT_ENDPOINT']) {
    if (!process.env[need]) throw new Error(`${need} unset — a bake-off with one recognizer ranks nothing`);
  }
  const voices = String(args.voices || '').split(',').map((v) => v.trim()).filter(Boolean);
  if (!voices.length) throw new Error('pass --voices a,b,c (the candidates from census C5)');
  const words = JSON.parse(readFileSync(args.words || `audio_clarity/words-${lang}.json`, 'utf8'));
  const tmp = mkdtempSync(join(tmpdir(), 'bakeoff-'));
  const rows = [];

  for (const voice of voices) {
    if (!eligible(voice)) {
      rows.push({ voice, excluded: 'variety does not match the bank (D17)' });
      continue;
    }
    let pass = 0; let weak = 0; let fail = 0;
    const misheard = [];
    for (const { word } of words) {
      const mp3 = join(tmp, 'c.mp3');
      const wav = join(tmp, 'c.wav');
      execFileSync('curl', ['-s', '-o', mp3, `${SPEAK}?word=${encodeURIComponent(word)}&lang=${lang}&variant=normal&voice=${encodeURIComponent(voice)}`]);
      execFileSync('afconvert', ['-f', 'WAVE', '-d', 'LEI16@16000', mp3, wav]);
      const heard = heardBoth(wav);
      const hits = heard.filter((h) => norm(h) === norm(word)).length;
      if (hits >= 2) pass += 1;
      else if (hits === 1) weak += 1;
      else { fail += 1; misheard.push(`${word} -> ${heard.map((h) => JSON.stringify(h)).join(' / ')}`); }
    }
    rows.push({ voice, pass, weak, fail, misheard });
  }

  rows.sort((a, b) => (b.pass ?? -1) - (a.pass ?? -1) || (a.weak ?? 0) - (b.weak ?? 0));
  const out = [`# Bake-off — ${lang}`, '', `${words.length} words per voice, both recognizers, blind (I3).`, ''];
  out.push('| Voice | Pass | Weak | Fail |', '|---|---|---|---|');
  for (const r of rows) {
    out.push(r.excluded ? `| ${r.voice} | — | — | — | excluded: ${r.excluded}` : `| ${r.voice} | ${r.pass} | ${r.weak} | ${r.fail} |`);
  }
  out.push('', '## What each voice was misheard on', '');
  for (const r of rows.filter((x) => x.misheard?.length)) {
    out.push(`### ${r.voice}`, '', ...r.misheard.map((m) => `- ${m}`), '');
  }
  out.push('', '**Recommendation only.** Switching a language\'s default voice needs',
    'Eric\'s signature for that language (D6), and the switch itself is atomic:',
    'every clip regenerates and passes F1 and F2 before any player hears one',
    '(F4 step 5), one language at a time, never mid-session (D16).');
  mkdirSync('audio_clarity/bakeoff', { recursive: true });
  writeFileSync(`audio_clarity/bakeoff/${lang}.md`, `${out.join('\n')}\n`);
  console.log(`audio-bakeoff: ${rows.length} voices, ${words.length} words -> audio_clarity/bakeoff/${lang}.md`);
}

main().catch((e) => { console.error(`audio-bakeoff: ${e.message}`); process.exit(1); });
