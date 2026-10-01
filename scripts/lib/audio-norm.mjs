// CC-AUDIO-CLARITY — the ONE normaliser both audio tools compare through.
//
// It exists because there were two. F2's harness (audio-verdicts.mjs) and F4's
// bake-off (audio-bakeoff.mjs) each carried their own copy, and the copies had
// drifted: the bake-off's stripped neither whisper.cpp's "== Fox" decoration
// nor the digit forms, so `fifth` scored as a split decision in all nine
// voices of the en run and `fox` cost the winning voice a point. A ranking is
// a comparison between voices, so a normaliser that is wrong the same way for
// everyone is survivable -- but one that is wrong differently from the harness
// that later measures the winner is not, because then the bake-off and F2
// disagree about the same clip. One function, imported twice.
//
// I9: every comparison runs on NFC, case-folded, punctuation stripped.

/// Recognizers write numbers as digits: "fifth" comes back "5th", "seven"
/// comes back "7". That is a transcription convention, not a mishearing, and
/// scoring it FAIL would withhold a perfectly clear clip.
///
/// English only, and deliberately so: these values are English words, so the
/// map can only ever be right for English. Folding "7" into "seven" for es or
/// fr would be a mistranslation wearing a normalisation's clothes.
export const DIGIT_WORDS = {
  '0': 'zero', '1': 'one', '2': 'two', '3': 'three', '4': 'four', '5': 'five',
  '6': 'six', '7': 'seven', '8': 'eight', '9': 'nine', '10': 'ten',
  // The first full-bank run turned up `twelve -> "12"` and `hundred -> "100"`,
  // which are the same convention the list above already covers and were only
  // missing because the original list stopped at ten. Completed rather than
  // patched word by word: the next run should not discover `thirteen` too.
  '11': 'eleven', '12': 'twelve', '13': 'thirteen', '14': 'fourteen',
  '15': 'fifteen', '16': 'sixteen', '17': 'seventeen', '18': 'eighteen',
  '19': 'nineteen', '20': 'twenty', '30': 'thirty', '40': 'forty',
  '50': 'fifty', '60': 'sixty', '70': 'seventy', '80': 'eighty', '90': 'ninety',
  '100': 'hundred', '1000': 'thousand', '1000000': 'million',
  '1st': 'first', '2nd': 'second', '3rd': 'third', '4th': 'fourth', '5th': 'fifth',
  '6th': 'sixth', '7th': 'seventh', '8th': 'eighth', '9th': 'ninth', '10th': 'tenth',
  '11th': 'eleventh', '12th': 'twelfth', '20th': 'twentieth',
};

/// The stripped set includes `=`, `*`, `_`, `~` and `|` because whisper.cpp
/// decorates some transcripts with them -- "== Fox" for a perfectly clear clip
/// of `fox`. Same reasoning as DIGIT_WORDS: a transcription convention is not
/// a mishearing.
export function normalize(s, key, lang) {
  let out = String(s)
    .normalize('NFC')
    .toLowerCase()
    .replace(/[.,!?;:"'`()\[\]{}…—–\-=*_~|]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (key === 'surface-yo') out = out.replace(/ё/g, 'е');
  if (key === 'surface-bare') out = out.replace(/[ً-ْٰ]/g, '');
  if (lang === 'en' && Object.prototype.hasOwnProperty.call(DIGIT_WORDS, out)) {
    out = DIGIT_WORDS[out];
  }
  return out;
}

/// Census C8: what a recognizer's output is compared against, per language.
/// zh is the odd one -- the player types pinyin, but a recognizer hears and
/// returns hanzi, so hanzi is the key. ja is unresolved: a recognizer may
/// return kanji for a kana entry, which would read as a false FAIL, so it is
/// UNSCORABLE until Eric decides (the spec's own mechanism for exactly this).
export const KEY = {
  en: 'surface', es: 'surface', fr: 'surface', de: 'surface', pt: 'surface',
  pl: 'surface', sw: 'surface', fil: 'surface', vi: 'surface',
  ru: 'surface-yo',      // ё/е equivalence, per CC-PLAYER-CONTRACT D1
  hi: 'surface', ko: 'surface',
  ar: 'surface-bare',    // diacritics stripped from the recognizer side
  zh: 'hanzi',
  ja: 'unscorable',
};

// CC-AUDIO-CLARITY — whisper.cpp's language code is not always ours.
//
// whisper-cli rejects an unknown code outright ("error: unknown language
// 'fil'") and writes nothing, and both harnesses record that silence as a
// transcript the normaliser then scores as a miss. So a wrong code does not
// fail loudly, it fails as a perfect zero -- which is exactly what fil scored
// in every run before 2026-10-01, and why "whisper cannot do Tagalog" looked
// like a finding instead of a bug. Tagalog is `tl` to whisper.
//
// Checked 2026-10-01: of our fifteen, fil is the only mismatch. Every other
// code is accepted as-is, so this map stays a one-entry exception rather than
// a full table that would drift from LANG_VOICES.
export const WHISPER_LANG = { fil: 'tl' };

export function whisperLang(lang) {
  return WHISPER_LANG[lang] || lang;
}
