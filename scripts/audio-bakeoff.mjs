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
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { KEY, normalize, whisperLang } from './lib/audio-norm.mjs';

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
  // Case-INSENSITIVE, because Google's own naming is not consistent: Filipino
  // Neural2 voices ship as `fil-ph-Neural2-D` with a lowercase region while
  // every Wavenet one is `fil-PH-...`. A case-sensitive prefix test threw out
  // a real, distinct, correctly-varietied voice as if it spoke the wrong
  // variety -- which is the one thing D17 exists to say, and it was saying it
  // about a spelling difference.
  return want ? voice.toLowerCase().startsWith(`${want.toLowerCase()}-`) : true;
}

function heardBoth(wav) {
  const model = process.env.WHISPER_MODEL;
  const w = execFileSync('whisper-cli', ['-m', model, '-l', whisperLang(lang), '-nt', wav], { encoding: 'utf8' }).trim();
  const b64 = readFileSync(wav).toString('base64');
  const r = execFileSync('curl', ['-s', '-X', 'POST', process.env.STT_ENDPOINT, '-H', 'Content-Type: application/json',
    '-d', JSON.stringify({ lang, sampleRate: 16000, audio: b64 })], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  let g = '';
  try { g = JSON.parse(r).transcript || ''; } catch { g = ''; }
  return [w, g];
}

// The bake-off compares through the SAME normaliser F2 does; see
// scripts/lib/audio-norm.mjs for what the two local copies disagreed about.
const norm = (s) => normalize(s, KEY[lang] || 'surface', lang);

async function main() {
  for (const need of ['WHISPER_MODEL', 'STT_ENDPOINT']) {
    if (!process.env[need]) throw new Error(`${need} unset — a bake-off with one recognizer ranks nothing`);
  }
  const voices = String(args.voices || '').split(',').map((v) => v.trim()).filter(Boolean);
  if (!voices.length) throw new Error('pass --voices a,b,c (the candidates from census C5)');
  const words = JSON.parse(readFileSync(args.words || `audio_clarity/words-${lang}.json`, 'utf8'));
  const tmp = mkdtempSync(join(tmpdir(), 'bakeoff-'));
  const rows = [];

  // CC-AUDIO-CLARITY F4 — the alias guard.
  //
  // Google answers a voice name it does not have by serving a DIFFERENT voice,
  // with a 200 and real audio. The original candidate lists were built by
  // keeping the names that answered 200, so several languages were scoring one
  // voice under as many as six labels: de showed eleven rows for five voices,
  // fr twelve rows for four. The duplicates score identically to the digit,
  // which is how it was noticed on 2026-10-01 -- six German rows at 38/15/7.
  //
  // WHAT THIS CAN AND CANNOT PROVE. Google's synthesis is NOT deterministic:
  // the same name, word and SSML returned three different byte patterns over
  // six calls on 2026-10-02, all of exactly the same length -- one voice,
  // non-deterministic encoding. So:
  //
  //   byte-identical audio from two names  =>  the same voice. Reliable:
  //       two non-deterministic streams do not coincide by chance.
  //   no match found                       =>  PROVES NOTHING. Two samples of
  //       one voice can easily land on different renderings.
  //
  // An earlier version of this guard compared ONE clip and reported the
  // absence of a match as distinctness. That is a false negative waiting to
  // happen, and it did: fr-FR-Neural2-A and fr-FR-Wavenet-F looked identical
  // in one run and different in the next.
  //
  // So the guard samples the first PROBE_CLIPS words rather than one, and the
  // report says plainly that unflagged rows are unconfirmed, not confirmed
  // distinct. Those clips are scored anyway, so the check costs nothing.
  //
  // Repeating a single word would not work: the bake-off reaches Google
  // through the server, which caches per (voice, word), so the first
  // rendering is frozen and every repeat returns it. Different words are what
  // give independent samples.
  //
  // Deliberately NOT a catalogue lookup. Enumerating /v1/voices would put a
  // Google API key inside this harness, which today reaches Google only
  // through the server and holds no credential of its own. Hashes also catch
  // strictly more: a name can be perfectly listed and still be an alias, and
  // the catalogue cannot tell you that.
  const PROBE_CLIPS = 3;
  const seenClip = new Map();

  for (const voice of voices) {
    if (!eligible(voice)) {
      rows.push({ voice, excluded: 'variety does not match the bank (D17)' });
      continue;
    }
    let pass = 0; let weak = 0; let fail = 0;
    const misheard = [];
    // Weak is where the interesting cases live: one recognizer right and one
    // wrong is exactly the "half" shape -- whisper hears "Have", Google hears
    // "half". Reporting only fails hid the word this whole feature exists for.
    const split = [];
    let alias = null;
    for (const [i, { word }] of words.entries()) {
      const mp3 = join(tmp, 'c.mp3');
      const wav = join(tmp, 'c.wav');
      execFileSync('curl', ['-s', '-o', mp3, `${SPEAK}?word=${encodeURIComponent(word)}&lang=${lang}&variant=normal&voice=${encodeURIComponent(voice)}`]);
      if (i < PROBE_CLIPS) {
        const h = createHash('sha256').update(readFileSync(mp3)).digest('hex');
        const owner = seenClip.get(h);
        if (owner && owner !== voice) { alias = owner; break; }
        seenClip.set(h, voice);
      }
      execFileSync('afconvert', ['-f', 'WAVE', '-d', 'LEI16@16000', mp3, wav]);
      const heard = heardBoth(wav);
      const hits = heard.filter((h) => norm(h) === norm(word)).length;
      if (hits >= 2) pass += 1;
      else if (hits === 1) { weak += 1; split.push(`${word} -> ${heard.map((h) => JSON.stringify(h)).join(' / ')}`); }
      else { fail += 1; misheard.push(`${word} -> ${heard.map((h) => JSON.stringify(h)).join(' / ')}`); }
    }
    if (alias) {
      rows.push({ voice, excluded: `the same audio as ${alias} -- Google serves one voice for both names` });
      continue;
    }
    rows.push({ voice, pass, weak, fail, misheard, split });
  }

  rows.sort((a, b) => (b.pass ?? -1) - (a.pass ?? -1) || (a.weak ?? 0) - (b.weak ?? 0));
  const scored = rows.filter((r) => !r.excluded).length;
  const aliased = rows.filter((r) => r.excluded?.startsWith('the same audio')).length;
  const out = [
    `# Bake-off — ${lang}`,
    '',
    `${words.length} words per voice, both recognizers, blind (I3).`,
    '',
    `${scored} voice(s) scored from ${voices.length} candidate name(s)` +
      (aliased ? `; ${aliased} proved to be another name for a voice already scored.` : '.'),
    '',
    'Rows that were NOT flagged as aliases are **unconfirmed, not confirmed',
    'distinct**. Google synthesis is non-deterministic, so two samples of one',
    'voice can differ; a match proves sameness, a non-match proves nothing.',
    '',
  ];
  // Mark the incumbent. Without it a reader has to go and look up
  // LANG_VOICES to answer the only question the table is really asked --
  // is the voice we ship already the best one -- and a ranking whose
  // baseline is off-page invites the wrong conclusion either way.
  const current = args.current || '';
  out.push('| Voice | Pass | Weak | Fail |', '|---|---|---|---|');
  for (const r of rows) {
    const name = r.voice === current ? `${r.voice} **(current)**` : r.voice;
    out.push(r.excluded ? `| ${name} | — | — | — | excluded: ${r.excluded}` : `| ${name} | ${r.pass} | ${r.weak} | ${r.fail} |`);
  }
  if (current && !rows.some((r) => r.voice === current)) {
    out.push('', `The shipped voice, \`${current}\`, was not among the candidates measured here.`);
  }
  out.push('', '## Missed by both recognizers', '');
  for (const r of rows.filter((x) => x.misheard?.length)) {
    out.push(`### ${r.voice}`, '', ...r.misheard.map((m) => `- ${m}`), '');
  }
  out.push('', '## Split decisions — one recognizer heard it, one did not', '',
    'The `half` case lives here, not above: a word only one machine gets is the',
    'one a listener is most likely to find ambiguous, and a report that showed',
    'only total failures left it out entirely.', '');
  for (const r of rows.filter((x) => x.split?.length)) {
    out.push(`### ${r.voice}`, '', ...r.split.map((m) => `- ${m}`), '');
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
