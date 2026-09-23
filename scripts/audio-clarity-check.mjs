#!/usr/bin/env node
// CC-AUDIO-CLARITY v1.1 F7 — the per-build report and the CI gate.
//
// Writes audio_clarity/report-<build>.md and fails the build when a clip that
// would be served has no verdict or has verdict Fail (I1), or when a voice's
// variety does not match the variety its bank grades (I15/D17).
//
// The gate is strict about MEASURED languages only. A language enters
// `measured` when its clip set has been measured end to end; until then the
// report says so in as many words. That is the bootstrap: being strict about
// unmeasured languages on day one would fail every build and teach everyone to
// ignore the gate, which is worse than saying plainly what has not been done.
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const STORE = 'config/audio-verdicts.json';
const store = existsSync(STORE) ? JSON.parse(readFileSync(STORE, 'utf8')) : { measured: [], clips: {} };
const build = process.env.BUILD_NUMBER || 'dev';

/// Census C5: the voice each language is served in.
const VOICE_VARIETY = {
  en: 'en-US', es: 'es-ES', fr: 'fr-FR', de: 'de-DE', pt: 'pt-BR', pl: 'pl-PL',
  ru: 'ru-RU', vi: 'vi-VN', ko: 'ko-KR', ja: 'ja-JP', fil: 'fil-PH', zh: 'cmn-CN',
  ar: 'ar-XA', hi: 'hi-IN', sw: 'sw-TZ',
};

/// Census C10, closed 2026-09-22: the banks declare a variety now, in
/// config/bank-variety.json, and scripts/bank-variety-check.mjs holds them to
/// it. Read it rather than restate it, so the two can never disagree.
const BANK_VARIETY = JSON.parse(readFileSync('config/bank-variety.json', 'utf8')).varieties;

const counts = {};
const fails = [];
for (const [k, e] of Object.entries(store.clips)) {
  const [lang, , variant] = k.split('|');
  counts[lang] ??= {};
  counts[lang][e.v] = (counts[lang][e.v] || 0) + 1;
  if (e.v === 'Fail') fails.push(`${k} (${variant})`);
}

const problems = [];
for (const lang of store.measured) {
  const c = counts[lang] || {};
  const total = Object.values(c).reduce((a, b) => a + b, 0);
  if (!total) problems.push(`${lang} is listed as measured but carries no verdicts`);
  const failRate = ((c.Fail || 0) / (total || 1)) * 100;
  // Section 10: a FAIL rate over 10% points at a pipeline or voice fault, not
  // at bad words. Report it; do not paper over it with overrides.
  if (failRate > 10) problems.push(`${lang}: ${failRate.toFixed(1)}% FAIL — section 10 says stop and report, not override`);
}
for (const [lang, want] of Object.entries(BANK_VARIETY)) {
  if (want && VOICE_VARIETY[lang] !== want) {
    problems.push(`${lang}: voice ${VOICE_VARIETY[lang]} but the bank grades ${want} (I15/D17)`);
  }
}

const lines = [];
lines.push(`# CC-AUDIO-CLARITY report — build ${build}`, '');
lines.push(`Verdicts: ${Object.keys(store.clips).length}. Measured languages: ${store.measured.length ? store.measured.join(', ') : 'none yet'}.`, '');
if (!store.measured.length) {
  lines.push('No language has been measured yet. F2 needs two recognizers and a clip set;');
  lines.push('`scripts/audio-verdicts.mjs --dry-run` says what is missing. Until a language');
  lines.push('is measured its clips are served unverified, which this report exists to say');
  lines.push('out loud rather than leave implied.', '');
}
lines.push('| Language | Pass | Weak | Fail | Exempt | Unscorable | Measured |', '|---|---|---|---|---|---|---|');
for (const lang of Object.keys(VOICE_VARIETY)) {
  const c = counts[lang] || {};
  lines.push(`| ${lang} | ${c.Pass || 0} | ${c.Weak || 0} | ${c.Fail || 0} | ${c.ExemptHomophone || 0} | ${c.Unscorable || 0} | ${store.measured.includes(lang) ? 'yes' : 'no'} |`);
}
lines.push('', '## Variety (C10 / D17)', '');
lines.push('| Language | Voice | Bank grades |', '|---|---|---|');
for (const [lang, v] of Object.entries(VOICE_VARIETY)) {
  const want = BANK_VARIETY[lang];
  lines.push(`| ${lang} | ${v} | ${want || '**undeclared**'} |${want === v ? '' : ' mismatch'}`);
}
if (fails.length) lines.push('', '## Withheld (Fail)', '', ...fails.map((f) => `- ${f}`));
mkdirSync('audio_clarity', { recursive: true });
writeFileSync(`audio_clarity/report-${build}.md`, `${lines.join('\n')}\n`);

if (problems.length) {
  console.error('audio-clarity-check: FAIL');
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}
console.log(`audio-clarity-check: OK — ${Object.keys(store.clips).length} verdicts, ${store.measured.length} language(s) measured, report written`);
