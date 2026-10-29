#!/usr/bin/env node
// CC-VOICE-CLARITY lead-in: derive config/audio-leadin-en.txt, ONCE.
//
//   node scripts/leadin-words.mjs          (prints what it would write)
//   node scripts/leadin-words.mjs --write
//
// The list is every English word whose bare-clip verdict (profile vc1) is Fail
// or Weak in config/audio-verdicts.json, minus the English quarantine files and
// minus any word in a TRUE collision set (a homophone is not unclear, it is
// ambiguous by design). After the lead-in clips are scored those verdicts turn
// Pass, so re-running this would shrink the list and un-flag the very words the
// lead-in fixed. The file is therefore a frozen, reviewed list: edit it by hand
// and review the diff; do not regenerate it after the first write.
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { setsOfKind } from './lib/soundalikes.mjs';

const OUT = 'config/audio-leadin-en.txt';
const verdicts = JSON.parse(readFileSync('config/audio-verdicts.json', 'utf8')).clips;
const lines = (p) => readFileSync(p, 'utf8').split('\n').map((l) => l.trim().toLowerCase()).filter((l) => l && !l.startsWith('#'));

const quarantined = new Set();
for (const f of readdirSync('assets/words-quarantine/en')) {
  if (f.endsWith('.txt')) lines(`assets/words-quarantine/en/${f}`).forEach((w) => quarantined.add(w));
}
const retier = 'assets/words/en/retier/quarantine.txt';
if (existsSync(retier)) lines(retier).forEach((w) => quarantined.add(w));
const colliding = new Set(setsOfKind('en', ['true']).flat().map((w) => w.toLowerCase()));

const words = Object.entries(verdicts)
  .filter(([k, v]) => k.startsWith('en|') && k.endsWith('|normal') && (v.v === 'Fail' || v.v === 'Weak'))
  .map(([k]) => k.split('|')[1])
  .filter((w) => !quarantined.has(w) && !colliding.has(w))
  .sort();

const header = [
  '# English words whose clip opens with "The word is" (CC-VOICE-CLARITY lead-in, 2026-10-29).',
  '# Derived once by scripts/leadin-words.mjs from config/audio-verdicts.json: every English',
  '# word with a Fail or Weak verdict on the bare vc1 clip, minus the English quarantine files',
  '# (assets/words-quarantine/en, assets/words/en/retier/quarantine.txt) and minus words in a',
  '# true collision set. FROZEN: later verdicts turn Pass because of the lead-in, so do not',
  '# regenerate. Read by backend/app.py (server renders the lead-in, cache key |rp:vc1li) and',
  '# by src/api.rs (client adds &v=li so devices drop the old immutable clip). One word per line.',
];
const text = `${header.join('\n')}\n${words.join('\n')}\n`;
if (process.argv.includes('--write')) writeFileSync(OUT, text);
console.log(`${words.length} words${process.argv.includes('--write') ? ` written to ${OUT}` : ' (dry run)'}`);
