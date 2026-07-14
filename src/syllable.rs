//! Spanish syllabification — algorithmic, because Spanish spelling maps to
//! syllables by regular rules (unlike English, which needs a pronunciation
//! dictionary the word-data pipeline does not carry — so this module is
//! deliberately es-only; see Feature F7 / Decision D4).
//!
//! Pure logic, no `web_sys`, so it runs under `cargo test --lib` on the host.
//! The rules implemented (RAE orthographic syllable division):
//!   * nucleus = a vowel, or a diphthong/triphthong of adjacent vowels;
//!   * two strong (open a/e/o) vowels, or an accented weak vowel (í/ú) next to
//!     another vowel, form a HIATUS and split into separate syllables;
//!   * a single consonant between vowels opens the next syllable (V-CV);
//!   * two consonants split (VC-CV) unless they are an inseparable onset cluster
//!     (consonant + l/r, e.g. bl, br, tr, gr, pl, cr…) which stays together
//!     (V-CCV);
//!   * three consonants split as VC-CCV when the last two are a valid cluster,
//!     else VCC-CV; four split down the middle;
//!   * the digraphs ch, ll, rr are single, inseparable consonant units.

/// Vowel classes that decide diphthong vs. hiatus.
#[derive(Clone, Copy, PartialEq)]
enum V {
    /// Open/strong: a e o (and their acute-accented forms).
    Strong,
    /// Closed/weak, unaccented: i u ü (and y acting as a vowel).
    Weak,
    /// Closed/weak but accented: í ú — always breaks a would-be diphthong.
    WeakAccented,
}

/// Classify a (already-lowercased) char as a vowel class, or `None` for a
/// consonant. `y` is handled by the caller (it is a vowel only word-finally).
fn vowel_class(c: char) -> Option<V> {
    match c {
        'a' | 'e' | 'o' | 'á' | 'é' | 'ó' | 'à' | 'è' | 'ò' => Some(V::Strong),
        'i' | 'u' | 'ü' | 'ï' => Some(V::Weak),
        'í' | 'ú' => Some(V::WeakAccented),
        _ => None,
    }
}

/// Do vowel classes `x` then `y` stay in one nucleus (diphthong/triphthong link)?
/// Hiatus (split) when either is an accented weak vowel, or both are strong.
fn is_diphthong(x: V, y: V) -> bool {
    if x == V::WeakAccented || y == V::WeakAccented {
        return false;
    }
    !(x == V::Strong && y == V::Strong)
}

/// Is `a`+`b` an inseparable onset cluster (consonant + l/r) that must open the
/// next syllable together? Both must be single consonant letters.
fn onset_cluster(a: char, b: char) -> bool {
    match b {
        'r' => matches!(a, 'p' | 'b' | 'f' | 't' | 'd' | 'c' | 'g' | 'k'),
        'l' => matches!(a, 'p' | 'b' | 'f' | 'c' | 'g' | 'k'),
        _ => false,
    }
}

/// One tokenized unit of the word: a single vowel, or a consonant (which may be
/// the two-char digraph ch/ll/rr). `start` indexes the ORIGINAL chars (the break
/// point at a unit boundary) so the output syllables preserve the input's exact
/// characters (accents, case).
struct Unit {
    start: usize,
    vowel: Option<V>,
    /// For a single-consonant unit, its lowercased letter (for cluster tests).
    /// A vowel or a digraph carries `'\0'`, which never forms a cluster.
    cons: char,
}

fn is_digraph(a: char, b: char) -> bool {
    matches!((a, b), ('c', 'h') | ('l', 'l') | ('r', 'r'))
}

/// Split a Spanish `word` into its syllables, in order. Non-alphabetic input, or
/// a word with fewer than two syllables, comes back as a single-element vec
/// holding the whole word (callers gate the "hear it slowly" affordance on
/// `len() >= 2`). The concatenation of the result always equals `word`.
pub fn syllabify(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let n = chars.len();

    // 1) Tokenize into vowel / consonant units (digraphs = one consonant unit).
    let mut units: Vec<Unit> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let c = lower[i];
        // `y` is a vowel only at the very end after a vowel (rey, hoy, muy);
        // elsewhere it is a consonant (reyes, mayo).
        let y_is_vowel = c == 'y' && i + 1 == n && i > 0 && vowel_class(lower[i - 1]).is_some();
        if let Some(v) = vowel_class(c) {
            units.push(Unit { start: i, vowel: Some(v), cons: '\0' });
            i += 1;
        } else if y_is_vowel {
            units.push(Unit { start: i, vowel: Some(V::Weak), cons: '\0' });
            i += 1;
        } else if i + 1 < n && is_digraph(c, lower[i + 1]) {
            units.push(Unit { start: i, vowel: None, cons: '\0' });
            i += 2;
        } else {
            units.push(Unit { start: i, vowel: None, cons: c });
            i += 1;
        }
    }

    // 2) Group vowel units into nuclei, splitting a vowel run at each hiatus.
    //    `nuclei[k] = (first_unit, last_unit)` inclusive.
    let mut nuclei: Vec<(usize, usize)> = Vec::new();
    let mut u = 0;
    while u < units.len() {
        if let Some(mut prev) = units[u].vowel {
            let start = u;
            let mut last = u;
            let mut v = u + 1;
            while v < units.len() {
                match units[v].vowel {
                    Some(cur) if is_diphthong(prev, cur) => {
                        last = v;
                        prev = cur;
                        v += 1;
                    }
                    _ => break,
                }
            }
            nuclei.push((start, last));
            u = last + 1;
        } else {
            u += 1;
        }
    }

    // No nucleus (no vowels) or a single nucleus → one syllable (whole word).
    if nuclei.len() < 2 {
        return vec![word.to_string()];
    }

    // 3) For each gap between consecutive nuclei, decide how many of the
    //    intervening consonants stay as the LEFT syllable's coda; the rest open
    //    the right syllable. The right syllable's first char is a break point.
    let mut break_chars: Vec<usize> = Vec::new();
    for w in 0..nuclei.len() - 1 {
        let cons_start = nuclei[w].1 + 1;
        let cons_end = nuclei[w + 1].0; // exclusive
        let k = cons_end - cons_start;
        let coda = match k {
            0 | 1 => 0,
            2 => {
                if onset_cluster(units[cons_start].cons, units[cons_start + 1].cons) {
                    0
                } else {
                    1
                }
            }
            3 => {
                if onset_cluster(units[cons_start + 1].cons, units[cons_start + 2].cons) {
                    1
                } else {
                    2
                }
            }
            _ => k - 2,
        };
        let break_unit = cons_start + coda;
        break_chars.push(units[break_unit].start);
    }

    // 4) Slice the ORIGINAL chars at the break points.
    let mut out: Vec<String> = Vec::with_capacity(break_chars.len() + 1);
    let mut prev = 0;
    for &b in &break_chars {
        out.push(chars[prev..b].iter().collect());
        prev = b;
    }
    out.push(chars[prev..n].iter().collect());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer table (RAE orthographic division), > 50 words, spanning
    /// diphthongs, hiatus, consonant clusters, digraphs, ñ/accents.
    const CASES: &[(&str, &str)] = &[
        // --- simple CV / codas ---
        ("casa", "ca-sa"),
        ("gato", "ga-to"),
        ("mesa", "me-sa"),
        ("perro", "pe-rro"),
        ("algo", "al-go"),
        ("papel", "pa-pel"),
        ("árbol", "ár-bol"),
        ("español", "es-pa-ñol"),
        ("niño", "ni-ño"),
        ("examen", "e-xa-men"),
        ("computadora", "com-pu-ta-do-ra"),
        ("nosotros", "no-so-tros"),
        // --- monosyllables (single-syllable → whole word) ---
        ("sol", "sol"),
        ("flor", "flor"),
        ("pan", "pan"),
        // --- diphthongs ---
        ("aire", "ai-re"),
        ("peine", "pei-ne"),
        ("causa", "cau-sa"),
        ("cielo", "cie-lo"),
        ("bueno", "bue-no"),
        ("reina", "rei-na"),
        ("auto", "au-to"),
        ("ciudad", "ciu-dad"),
        ("veinte", "vein-te"),
        ("familia", "fa-mi-lia"),
        ("estudiante", "es-tu-dian-te"),
        // --- hiatus ---
        ("día", "dí-a"),
        ("río", "rí-o"),
        ("leer", "le-er"),
        ("caer", "ca-er"),
        ("tía", "tí-a"),
        ("país", "pa-ís"),
        ("maíz", "ma-íz"),
        ("baúl", "ba-úl"),
        ("poeta", "po-e-ta"),
        ("teatro", "te-a-tro"),
        // --- inseparable onset clusters (bl br cl cr dr fl fr gl gr pl pr tr) ---
        ("libro", "li-bro"),
        ("blanco", "blan-co"),
        ("plato", "pla-to"),
        ("grande", "gran-de"),
        ("padre", "pa-dre"),
        ("madre", "ma-dre"),
        ("problema", "pro-ble-ma"),
        ("abril", "a-bril"),
        ("transporte", "trans-por-te"),
        // --- three-consonant boundaries ---
        ("instante", "ins-tan-te"),
        ("siempre", "siem-pre"),
        ("nombre", "nom-bre"),
        ("hombre", "hom-bre"),
        ("escuela", "es-cue-la"),
        // --- silent h as a plain consonant ---
        ("ahora", "a-ho-ra"),
        ("prohibir", "pro-hi-bir"),
        ("ahí", "a-hí"),
        // --- digraphs ch / ll / rr stay whole ---
        ("chico", "chi-co"),
        ("muchacho", "mu-cha-cho"),
        ("calle", "ca-lle"),
        ("guerra", "gue-rra"),
        ("queso", "que-so"),
    ];

    #[test]
    fn known_answers() {
        let mut failures = Vec::new();
        for (word, expected) in CASES {
            let got = syllabify(word).join("-");
            if got != *expected {
                failures.push(format!("{word}: expected {expected}, got {got}"));
            }
        }
        assert!(failures.is_empty(), "syllabification mismatches:\n{}", failures.join("\n"));
    }

    #[test]
    fn case_count_exceeds_fifty() {
        assert!(CASES.len() >= 50, "known-answer list must have ≥50 words, has {}", CASES.len());
    }

    #[test]
    fn concatenation_is_lossless() {
        // The joined syllables must reproduce the input exactly (no dropped or
        // added characters), for every case and a few extras.
        for (word, _) in CASES {
            assert_eq!(&syllabify(word).concat(), word, "lossy split for {word}");
        }
        for word in ["", "z", "aeiou", "strengths"] {
            assert_eq!(syllabify(word).concat(), *word, "lossy split for {word:?}");
        }
    }

    #[test]
    fn empty_and_vowelless_are_single_units() {
        assert_eq!(syllabify(""), vec![String::new()]);
        assert_eq!(syllabify("str"), vec!["str".to_string()]);
    }
}
