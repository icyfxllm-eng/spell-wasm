//! CC-ORTHO-SWEEP-ALL F0: the per-language orthography profile, as data.
//!
//! `data/<lang>/ortho-profile.json` says how a language normalizes an answer, which stand-ins native writers
//! use, where marks are letters, and what the voice is. This module reads it; it has no language branches. The
//! same chain runs on the stored form and on the typed form, in the listed order, and only the listed steps run.
//!
//! Outcomes (F0.2):
//! * Correct: typed and stored normalize equal, or typed is a listed variant (`accepted`), or a stand-in the tier
//!   accepts.
//! * Close: typed is one named step from the stored form (a stand-in the tier does not accept, or a mark left out)
//!   AND typed is not itself a bank row. Holds the combo. Not correct.
//! * Miss: everything else, including a near-sound.
//! `correct_other_sense` (a true homophone) is decided elsewhere, by the sound-alike table (`homophones`); this
//! module never reads it, so normal-play verdicts do not move with table status (I4).
//!
//! The grader consults a profile only when `flags::ortho_profile(lang)` is on. Off, nothing here runs (I11).

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Correct,
    /// `fix` is the letter diff the feedback shows, e.g. `e → é`.
    Close { fix: String },
    Miss,
}

impl Outcome {
    pub fn is_correct(&self) -> bool {
        matches!(self, Outcome::Correct)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StandIn {
    pub pattern: String,
    pub replacement: String,
    pub accepted_tiers: Vec<String>,
    pub above: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Voice {
    pub provider: String,
    pub voice_id: String,
    pub region_tag: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoundAlike {
    pub generator: String,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub lang: String,
    pub normalize: Vec<String>,
    pub stand_ins: Vec<StandIn>,
    /// Letters written with a mark that is part of the letter, not an accent: omitting the mark is another letter
    /// (ñ→n, ç→c), so at standard tiers it is a miss. Spell Jr treats them as marks (D14).
    pub letters_not_marks: Vec<String>,
    pub variants_file: String,
    pub citation_form: serde_json::Value,
    pub script_allow: serde_json::Value,
    pub voice: Voice,
    pub sound_alike: SoundAlike,
    pub near_sound_rule_ids: Vec<String>,
    pub keyboard_notes: String,
}

/// The fixed vocabulary of normalize steps this module implements. A profile naming any other step fails to load.
pub const STEPS: &[&str] = &["nfc", "casefold", "trim", "collapse_spaces", "fold_apostrophe", "strip_zwj_zwnj"];

const ES: &str = include_str!("../data/es/ortho-profile.json");
const FR: &str = include_str!("../data/fr/ortho-profile.json");
const PT: &str = include_str!("../data/pt/ortho-profile.json");

/// The profile for `lang`, if one exists. Parsed once per call; profiles are tiny and the call is per answer.
pub fn profile(lang: &str) -> Option<Profile> {
    let text = match lang {
        "es" => ES,
        "fr" => FR,
        "pt" => PT,
        _ => return None,
    };
    let p: Profile = serde_json::from_str(text).ok()?;
    p.normalize.iter().all(|s| STEPS.contains(&s.as_str())).then_some(p)
}

/// Run the profile's chain, in order.
pub fn normalize(p: &Profile, s: &str) -> String {
    let mut out = s.to_string();
    for step in &p.normalize {
        out = match step.as_str() {
            "nfc" => out.nfc().collect(),
            "casefold" => out.to_lowercase(),
            "trim" => out.trim().to_string(),
            "collapse_spaces" => out.split_whitespace().collect::<Vec<_>>().join(" "),
            "fold_apostrophe" => crate::norm::fold_apostrophes(&out),
            "strip_zwj_zwnj" => out.chars().filter(|c| *c != '\u{200c}' && *c != '\u{200d}').collect(),
            _ => out,
        };
    }
    out
}

/// `c` with its mark removed (NFD, drop combining marks), unless `c` is one of `keep` (a letter, not an accent).
fn base_of(c: char, keep: &[char]) -> char {
    if keep.contains(&c) {
        return c;
    }
    c.to_string().nfd().find(|d| !('\u{0300}'..='\u{036f}').contains(d)).unwrap_or(c)
}

fn keep_letters(p: &Profile, jr: bool) -> Vec<char> {
    if jr {
        return Vec::new();
    }
    p.letters_not_marks.iter().filter_map(|s| s.chars().next()).collect()
}

/// Is `typed` the stored form with only marks left out? Same length, position by position: each typed char is the
/// stored char or its base letter, and at least one differs.
fn mark_omission(stored: &str, typed: &str, keep: &[char]) -> Option<String> {
    let (s, t): (Vec<char>, Vec<char>) = (stored.chars().collect(), typed.chars().collect());
    if s.len() != t.len() {
        return None;
    }
    let mut fix: Vec<String> = Vec::new();
    for (a, b) in s.iter().zip(t.iter()) {
        if a == b {
            continue;
        }
        if *b != base_of(*a, keep) {
            return None;
        }
        let d = format!("{b} → {a}");
        if !fix.contains(&d) {
            fix.push(d);
        }
    }
    (!fix.is_empty()).then(|| fix.join(", "))
}

/// Is `w` itself a bank row of `lang`, compared as gameplay compares an answer? Scans the tiers (the app build has no
/// `word_index`); it runs only when a typed answer is one mark-omission from the stored form, so it is rare.
pub fn is_bank_row(lang: &str, w: &str) -> bool {
    let needle = crate::norm::fold_strict(w);
    !needle.is_empty()
        && ["easy", "medium", "hard", "expert"].iter().any(|t| {
            // zh-ok(grading): row-exists test for the es/fr/pt profiles (no zh profile); same left-hand split word_index::comparable makes
            crate::words::tier_for(lang, t).iter().any(|e| crate::norm::fold_strict(e.split('|').next().unwrap_or(e)) == needle)
        })
}

/// The verdict for one answer. `tier` is `easy|medium|hard|expert`; `jr` is Spell Jr. `is_row` says whether a string
/// is itself a bank row of the language (the "typed is not itself a row" clause: los is not łoś, año is not ano).
pub fn verdict(p: &Profile, stored: &str, typed: &str, tier: &str, jr: bool, is_row: &dyn Fn(&str) -> bool) -> Outcome {
    let (s, t) = (normalize(p, stored), normalize(p, typed));
    if t == s || crate::accepted::accepts(&p.lang, stored, typed) {
        return Outcome::Correct;
    }
    for si in &p.stand_ins {
        if t.replace(&si.pattern, &si.replacement) == s && t.contains(&si.pattern) {
            let ok = jr || si.accepted_tiers.iter().any(|x| x == tier);
            return if ok { Outcome::Correct } else { Outcome::Close { fix: format!("{} → {}", si.pattern, si.replacement) } };
        }
    }
    if let Some(fix) = mark_omission(&s, &t, &keep_letters(p, jr)) {
        if is_row(&t) {
            return Outcome::Miss;
        }
        return if jr { Outcome::Correct } else { Outcome::Close { fix } };
    }
    Outcome::Miss
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(lang: &'static str) -> impl Fn(&str) -> bool {
        move |w| is_bank_row(lang, w)
    }

    #[test]
    fn every_profile_loads_and_names_only_implemented_steps() {
        for l in ["es", "fr", "pt"] {
            let p = profile(l).unwrap_or_else(|| panic!("{l}: profile missing or names an unknown step"));
            assert_eq!(p.lang, l);
            assert!(!p.voice.voice_id.is_empty());
        }
        assert!(profile("de").is_none(), "a language without a profile has none");
    }

    #[test]
    fn mark_omission_is_close_but_a_letter_is_not_a_mark() {
        let (es, pt, fr) = (profile("es").unwrap(), profile("pt").unwrap(), profile("fr").unwrap());
        assert_eq!(verdict(&fr, "été", "ete", "easy", false, &|_| false), Outcome::Close { fix: "e → é".into() });
        assert!(verdict(&fr, "été", "ete", "easy", true, &|_| false).is_correct(), "Spell Jr accepts the omission");
        assert_eq!(verdict(&es, "año", "ano", "easy", false, &|_| false), Outcome::Miss, "ñ is a letter");
        assert_eq!(verdict(&pt, "caça", "caca", "easy", false, &|_| false), Outcome::Miss, "ç is a letter");
        assert!(verdict(&pt, "coração", "coracao", "easy", true, &|_| false).is_correct());
        assert_eq!(verdict(&pt, "coração", "coracao", "easy", false, &|_| false), Outcome::Miss, "ç→c is another letter");
        assert!(matches!(verdict(&pt, "coração", "coraçao", "easy", false, &|_| false), Outcome::Close { .. }));
        // the typed form is itself a row: a different word, not a slip
        assert_eq!(verdict(&es, "está", "esta", "medium", false, &|w| w == "esta"), Outcome::Miss);
        assert_eq!(verdict(&es, "está", "esta", "medium", true, &|w| w == "esta"), Outcome::Miss, "Jr too");
    }

    #[test]
    fn fr_oe_stand_in_is_accepted_on_easy_and_close_above() {
        let fr = profile("fr").unwrap();
        assert!(verdict(&fr, "cœur", "coeur", "easy", false, &|_| false).is_correct());
        assert!(verdict(&fr, "cœur", "coeur", "easy", true, &|_| false).is_correct());
        assert!(matches!(verdict(&fr, "cœur", "coeur", "hard", false, &|_| false), Outcome::Close { .. }));
    }

    #[test]
    fn case_space_and_nfd_do_not_change_a_verdict() {
        let es = profile("es").unwrap();
        for typed in ["Árbol", " árbol ", "a\u{301}rbol", "ÁRBOL"] {
            assert!(verdict(&es, "árbol", typed, "easy", false, &|_| false).is_correct(), "{typed:?}");
        }
        assert_eq!(verdict(&es, "árbol", "arbol", "easy", false, &|_| false), Outcome::Close { fix: "a → á".into() });
    }

    const GOLDEN: [(&str, &str); 3] = [
        ("es", include_str!("../tests/ortho/golden/es.tsv")),
        ("fr", include_str!("../tests/ortho/golden/fr.tsv")),
        ("pt", include_str!("../tests/ortho/golden/pt.tsv")),
    ];

    /// F0.3: every golden row grades as its file says. `correct_other_sense` is the sound-alike table's call (the
    /// profile must call it a non-correct and the table must accept it); `pending_table` rows must not grade correct
    /// through the profile, and the table is deliberately not asserted on them (the spec answer waits on SENSE-CUE).
    #[test]
    fn golden_files_grade_as_written() {
        for (lang, text) in GOLDEN {
            let p = profile(lang).unwrap();
            let (mut n, mut lanes) = (0, std::collections::BTreeMap::<String, usize>::new());
            for line in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
                let c: Vec<&str> = line.split('\t').collect();
                assert_eq!(c.len(), 7, "{lang}: bad row {line:?}");
                let (lane, target, typed, tier, voice, exp) = (c[0], c[1], c[2], c[3], c[4], c[5]);
                assert_eq!(voice, p.voice.voice_id, "{lang}: golden row voice differs from the profile");
                let jr = tier == "jr";
                let t = if jr { "easy" } else { tier };
                let got = verdict(&p, target, typed, t, jr, &|w| is_bank_row(lang, w));
                let label = match &got {
                    Outcome::Correct => "correct",
                    Outcome::Close { .. } => "close",
                    Outcome::Miss => "miss",
                };
                match exp {
                    "correct" | "close" | "miss" => assert_eq!(label, exp, "{lang} {line}"),
                    "correct_other_sense" => {
                        assert_eq!(label, "miss", "{lang} {line}");
                        assert!(crate::homophones::accepts(lang, target, typed), "{lang}: table should accept {line}");
                    }
                    "pending_table" => assert_ne!(label, "correct", "{lang} {line}"),
                    other => panic!("{lang}: unknown expected {other:?}"),
                }
                n += 1;
                *lanes.entry(lane.to_string()).or_default() += 1;
            }
            assert!(n >= 90, "{lang}: {n} rows");
            for lane in ["homophone", "variant", "friction"] {
                assert!(lanes.get(lane).copied().unwrap_or(0) >= 30, "{lang}: lane {lane} has {:?}", lanes.get(lane));
            }
        }
    }

    /// I11: the flags ship OFF, so the profile path never runs and the existing grader decides every answer.
    #[test]
    fn profile_flags_ship_off() {
        for l in ["es", "fr", "pt"] {
            assert!(!crate::flags::ortho_profile(l), "{l}: ortho.profile flag must default off");
        }
    }
}
