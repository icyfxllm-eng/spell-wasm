#!/usr/bin/env node
// CC-AR-HI-PREAUDIT F2/F3 — orthographic lints for the Arabic and Hindi banks.
//
// Both banks pass every fail-mode rule here TODAY (measured 2026-09-23, before
// a line of this was written: ar 6,238 words with zero harakat, zero tatweel,
// zero tanwin, zero presentation forms, zero Persian lookalikes, zero non-NFC;
// hi 2,674 words with zero L-HI1..L-HI5 hits). So this check cleans nothing up.
//
// It is a TRIPWIRE. The two banks are the ones nobody on this project can read,
// which is exactly why a bad edit to them would survive review: a stray fatha or
// a Persian ی pasted from a web page looks identical to the correct letter at
// arm's length, and the first person to notice would be a paying auditor or a
// player. That is the failure this exists to make impossible.
//
// Scope note: L-AR6 (strip_harakat(vocalized) == canonical) is NOT here. The
// `vocalized` field does not exist yet, and per Eric's 2026-09-23 rescope F1 is
// deferred until an auditor has sourced the data. A lint over an absent field
// would be theatre.
import { readFileSync } from 'node:fs';

const TIERS = ['easy', 'medium', 'hard', 'expert'];
const cp = (c) => c.codePointAt(0);
const inRange = (c, lo, hi) => cp(c) >= lo && cp(c) <= hi;

// ---------------------------------------------------------------- Arabic ----
// The 28 letters, the six hamza carriers, ة and ى. Deliberately written out
// rather than expressed as a range: the Arabic block also holds Persian, Urdu
// and Quranic characters that a range would wave through, and L-AR4 exists
// precisely because those look like the real thing.
const AR_LETTERS = new Set([
  ...'ابتثجحخدذرزسش',
  ...'صضطظعغفقكلمنهوي',
  ...'ءآأؤإئ', // ء آ أ ؤ إ ئ
  ...'ةى',                          // ة ى
]);
const AR_TANWIN = (c) => inRange(c, 0x064b, 0x064d);
const AR_HARAKAT = (c) => inRange(c, 0x064b, 0x065f) || cp(c) === 0x0670;
const AR_TATWEEL = (c) => cp(c) === 0x0640;
const AR_PRESENTATION = (c) => inRange(c, 0xfb50, 0xfdff) || inRange(c, 0xfe70, 0xfeff);
// ی U+06CC, ک U+06A9, ہ U+06C1, ھ U+06BE — Persian/Urdu lookalikes.
const AR_FOREIGN = (c) => [0x06cc, 0x06a9, 0x06c1, 0x06be].includes(cp(c));

function lintAr(word) {
  for (const c of word) {
    if (AR_TANWIN(c)) return ['L-AR5', `tanwin U+${cp(c).toString(16).toUpperCase()} — citation forms only`];
  }
  for (const c of word) {
    if (AR_HARAKAT(c) || AR_TATWEEL(c))
      return ['L-AR2', `${AR_TATWEEL(c) ? 'tatweel' : 'harakat'} U+${cp(c).toString(16).toUpperCase()} in a canonical answer`];
    if (AR_PRESENTATION(c)) return ['L-AR3', `presentation form U+${cp(c).toString(16).toUpperCase()}`];
    if (AR_FOREIGN(c)) return ['L-AR4', `Persian/Urdu lookalike ${c} U+${cp(c).toString(16).toUpperCase()}`];
  }
  for (const c of word) {
    if (!AR_LETTERS.has(c)) return ['L-AR1', `${c} U+${cp(c).toString(16).toUpperCase()} is outside the Arabic alphabet`];
  }
  return null;
}

// ---------------------------------------------------------------- Hindi -----
const HI_VOWEL = (c) => inRange(c, 0x0905, 0x0914);
const HI_CONSONANT = (c) => inRange(c, 0x0915, 0x0939);
const HI_MATRA = (c) => inRange(c, 0x093e, 0x094c) || inRange(c, 0x0962, 0x0963);
const HI_NUKTA = (c) => cp(c) === 0x093c;
const HI_HALANT = (c) => cp(c) === 0x094d;
const HI_NASAL = (c) => cp(c) === 0x0901 || cp(c) === 0x0902;
const HI_LEGAL = (c) =>
  HI_VOWEL(c) || HI_CONSONANT(c) || HI_MATRA(c) || HI_NUKTA(c) || HI_HALANT(c) ||
  HI_NASAL(c) || cp(c) === 0x0903 /* visarga */ || cp(c) === 0x093d /* avagraha */ ||
  cp(c) === 0x200c || cp(c) === 0x200d;

function lintHi(word) {
  const cs = [...word];
  for (const c of cs) {
    if (!HI_LEGAL(c)) return ['L-HI1', `${c} U+${cp(c).toString(16).toUpperCase()} is not Hindi Devanagari`];
  }
  for (let i = 0; i < cs.length; i++) {
    const c = cs[i], prev = cs[i - 1], pprev = cs[i - 2];
    // L-HI4 goes FIRST, and the selftest is why: a second matra also has no
    // consonant to attach to, so L-HI2 would answer first and report the
    // vaguer of the two true things. The more specific rule should name it.
    if (HI_MATRA(c) && prev && HI_MATRA(prev)) return ['L-HI4', `two matras in a row at position ${i}`];
    if (HI_NASAL(c) && prev && HI_NASAL(prev)) return ['L-HI4', `anusvara and chandrabindu on one akshara at position ${i}`];
    // L-HI2: a matra hangs off a consonant, optionally through its nukta.
    if (HI_MATRA(c) && !(prev && (HI_CONSONANT(prev) || (HI_NUKTA(prev) && pprev && HI_CONSONANT(pprev)))))
      return ['L-HI2', `matra ${c} at position ${i} with no consonant to attach to`];
    // L-HI3: a nukta modifies the consonant it follows; nothing else.
    if (HI_NUKTA(c) && !(prev && HI_CONSONANT(prev)))
      return ['L-HI3', `nukta at position ${i} does not follow a consonant`];
  }
  return null;
}

// L-HI5 asks a question about the FILE, not about a word: NFC decomposes
// U+0958..U+095F (they are composition exclusions), so one surviving means the
// bank was written without ever being normalized. Checked on the raw text.
const HI_PRECOMPOSED = /[क़-य़]/;

// ------------------------------------------------------------ flag-only -----
// L-HI6 (ZWJ/ZWNJ) and L-HI7 (word-final halant) are REPORTED, never fatal.
// Both have legitimate forms: ZWNJ controls a half-form, and अर्थात् genuinely
// ends in a halant. Failing the build on a correct spelling would be the exact
// mistake this file exists to prevent.
const HI_JOINER = /[‌‍]/;

function bank(lang, tier) {
  return readFileSync(`assets/words/${lang}/${tier}.txt`, 'utf8')
    .split('\n').map((w) => w.trim()).filter(Boolean);
}

function run() {
  const problems = [], flags = [];
  let arN = 0, hiN = 0;
  for (const tier of TIERS) {
    for (const w of bank('ar', tier)) {
      arN++;
      if (w.normalize('NFC') !== w) problems.push(`ar/${tier} ${w}: not NFC`);
      const hit = lintAr(w);
      if (hit) problems.push(`ar/${tier} ${w}: ${hit[0]} — ${hit[1]}`);
    }
    const raw = readFileSync(`assets/words/hi/${tier}.txt`, 'utf8');
    if (HI_PRECOMPOSED.test(raw))
      problems.push(`hi/${tier}: L-HI5 — a precomposed nukta letter (U+0958..U+095F) survives, so this file was never NFC-normalized`);
    for (const w of bank('hi', tier)) {
      hiN++;
      if (w.normalize('NFC') !== w) problems.push(`hi/${tier} ${w}: not NFC`);
      const hit = lintHi(w);
      if (hit) problems.push(`hi/${tier} ${w}: ${hit[0]} — ${hit[1]}`);
      if (HI_JOINER.test(w)) flags.push(`L-HI6 hi/${tier} ${w}: contains a zero-width joiner`);
      if (HI_HALANT([...w].pop())) flags.push(`L-HI7 hi/${tier} ${w}: ends in a halant`);
    }
  }
  return { problems, flags, arN, hiN };
}

// ------------------------------------------------------------- selftest -----
// Acceptance tests 2 and 3: every rule is proved to bite by a fixture built to
// break it. A lint nobody has seen fail is a lint nobody knows works.
const FIXTURES = [
  ['ar', 'مَدرسة', 'L-AR2'],           // مَدرسة, a fatha
  ['ar', 'كتابــ', 'L-AR2'],           // tatweel
  ['ar', 'ﺍن', 'L-AR3'],                                    // presentation-form alef
  ['ar', 'کتاب', 'L-AR4'],                        // Persian ک
  ['ar', 'كتابً', 'L-AR5'],                  // كتاباً
  ['ar', 'كتاbs', 'L-AR1'],                            // Latin letters
  ['hi', 'ाक', 'L-HI2'],                                    // leading matra
  ['hi', 'अ़', 'L-HI3'],                                    // nukta on a vowel
  ['hi', 'किा', 'L-HI4'],                              // two matras
  ['hi', 'कंँ', 'L-HI4'],                              // anusvara + chandrabindu
  ['hi', 'कA', 'L-HI1'],                                    // Latin in Devanagari
];

function selftest() {
  const bad = [];
  for (const [lang, word, rule] of FIXTURES) {
    const hit = lang === 'ar' ? lintAr(word) : lintHi(word);
    if (!hit) bad.push(`${rule}: fixture ${JSON.stringify(word)} passed the lint`);
    else if (hit[0] !== rule) bad.push(`${rule}: fixture ${JSON.stringify(word)} reported ${hit[0]} instead`);
  }
  // The other half of a good lint: it does not fire on correct words.
  for (const [lang, word] of [['ar', 'مدرسة'], ['hi', 'हिन्दी'],
                              ['ar', 'أن'], ['hi', 'कृपया']]) {
    const hit = lang === 'ar' ? lintAr(word) : lintHi(word);
    if (hit) bad.push(`false positive: correct ${lang} word ${JSON.stringify(word)} reported ${hit[0]}`);
  }
  return bad;
}

const selfBad = selftest();
if (selfBad.length) {
  console.error('ar-hi-ortho-check: SELFTEST FAILED — the lints themselves are wrong');
  for (const b of selfBad) console.error(`  ${b}`);
  process.exit(1);
}
const { problems, flags, arN, hiN } = run();
if (problems.length) {
  console.error('ar-hi-ortho-check: FAILED');
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}
console.log(`ar-hi-ortho-check: OK — ar ${arN} words, hi ${hiN} words, ${FIXTURES.length} fixtures bite, ${flags.length} flag-only note(s)`);
for (const f of flags) console.log(`  note ${f}`);
