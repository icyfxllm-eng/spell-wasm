//! CC-PRACTICE core — guided first-contact spelling (20 + 5 per language).
//!
//! REVIEW-GATED. Pure curriculum/phase/progress logic; the screen renders it.
//! FAILURE-PROOF by construction (D3): this module has no concept of lives,
//! timers, scores, or fail states — those symbols do not appear here, and the
//! purity gate (scripts/practice-purity-check.mjs, Invariant I1) enforces it.
//! Its ONLY persistence is the per-language progress record under its own
//! storage key (Invariant I2) — nothing else is read or written.

use std::collections::HashMap;
use std::sync::OnceLock;

/// D2 scaffolding ladder — the phase boundaries, as constants in ONE place:
/// words 1–5 copy (visible), 6–12 flash (~2s then hidden), 13–20 audio-only.
pub const COPY_UNTIL: usize = 5; // 0-based: indices 0..5 are Copy
pub const FLASH_UNTIL: usize = 12; // indices 5..12 are Flash; 12..20 Audio
pub const FIRST_CONTACT: usize = 20;
pub const DIFFICULT: usize = 5;
pub const TOTAL: usize = FIRST_CONTACT + DIFFICULT;
/// Flash phase visibility window, ms (D2 "~2s").
pub const FLASH_MS: u32 = 2000;

/// How the current word is presented (D2). The graduation lap (words 21–25)
/// is always Audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Word stays visible while typing (copy).
    Copy,
    /// Word flashes ~2s, then hides (cued recall).
    Flash,
    /// Audio only (full recall).
    Audio,
}

/// Presentation phase for a 0-based position in the 25-word run.
pub fn phase(pos: usize) -> Phase {
    if pos < COPY_UNTIL {
        Phase::Copy
    } else if pos < FLASH_UNTIL {
        Phase::Flash
    } else {
        Phase::Audio
    }
}

/// One trap-class intro card (D4): shown when its first word begins, once per
/// run, dismissible, re-shown only on full restart.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TrapIntro {
    pub id: String,
    #[serde(rename = "firstWord")]
    pub first_word: usize,
    pub intro: String,
}

/// A language's curriculum: 20 first-contact + 5 difficult bank words + trap
/// intros. Data only (D4) — schema enforced by scripts/practice-check.mjs.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Curriculum {
    pub lang: String,
    pub words: Vec<String>,
    pub difficult: Vec<String>,
    pub traps: Vec<TrapIntro>,
}

fn sources() -> &'static HashMap<&'static str, Curriculum> {
    static T: OnceLock<HashMap<&'static str, Curriculum>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for (lang, src) in [
            ("en", include_str!("../config/practice/en.json")),
            ("es", include_str!("../config/practice/es.json")),
            ("fr", include_str!("../config/practice/fr.json")),
            ("de", include_str!("../config/practice/de.json")),
            ("pt", include_str!("../config/practice/pt.json")),
            ("pl", include_str!("../config/practice/pl.json")),
            ("vi", include_str!("../config/practice/vi.json")),
            ("ko", include_str!("../config/practice/ko.json")),
            ("ja", include_str!("../config/practice/ja.json")),
            ("fil", include_str!("../config/practice/fil.json")),
            ("zh", include_str!("../config/practice/zh.json")),
            ("ru", include_str!("../config/practice/ru.json")),
            ("ar", include_str!("../config/practice/ar.json")),
            ("sw", include_str!("../config/practice/sw.json")),
            ("hi", include_str!("../config/practice/hi.json")),
        ] {
            if let Ok(c) = serde_json::from_str::<Curriculum>(src) {
                m.insert(lang, c);
            }
        }
        m
    })
}

pub fn curriculum(lang: &str) -> Option<&'static Curriculum> {
    sources().get(lang)
}

/// The word at a 0-based run position (0..20 first-contact, 20..25 difficult).
pub fn word_at(c: &Curriculum, pos: usize) -> Option<&str> {
    if pos < FIRST_CONTACT {
        c.words.get(pos).map(|s| s.as_str())
    } else {
        c.difficult.get(pos - FIRST_CONTACT).map(|s| s.as_str())
    }
}

/// The intro card that must show when `pos` begins, if any (first occurrence
/// of a trap class — D4/Feature 2). Graduation words never introduce.
pub fn intro_at(c: &Curriculum, pos: usize) -> Option<&TrapIntro> {
    c.traps.iter().find(|t| t.first_word == pos)
}

/// Compare a typed answer prefix against the target, per typing unit
/// (grapheme-approximate: NFC chars — matches the keyboard's own unit for
/// every lineup language). Returns how many leading units are correct and
/// whether the whole word is complete. D3: the CALLER holds position on a
/// wrong unit; this function only reports — it cannot fail anyone.
pub fn check_prefix(target: &str, typed: &str) -> (usize, bool) {
    use unicode_normalization::UnicodeNormalization;
    let t: Vec<char> = target.nfc().collect();
    let y: Vec<char> = typed.nfc().collect();
    let mut ok = 0;
    for (a, b) in t.iter().zip(y.iter()) {
        // Case-insensitive: first-contact typing must never fail on Shift.
        if a.to_lowercase().eq(b.to_lowercase()) {
            ok += 1;
        } else {
            break;
        }
    }
    (ok, ok == t.len() && y.len() >= t.len())
}

/// The per-language progress record (Invariant I2: the ONLY thing Practice
/// persists). `pos` is the next word to play, 0..=TOTAL; TOTAL = fully done.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Progress {
    pub pos: usize,
    /// The trap intros already shown this RUN (not re-shown on resume; reset
    /// on restart).
    #[serde(default)]
    pub shown: Vec<String>,
}

fn key(lang: &str) -> String {
    format!("spell_practice_{lang}")
}

pub fn load(lang: &str) -> Progress {
    crate::storage::get_json(&key(lang)).unwrap_or_default()
}

pub fn save(lang: &str, p: &Progress) {
    crate::storage::set_json(&key(lang), p);
}

/// Full restart for ONE language (Feature 4): position and shown-intros reset.
pub fn restart(lang: &str) {
    save(lang, &Progress::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D2: phase transitions occur exactly at words 6 and 13 (1-based).
    #[test]
    fn phase_boundaries_exact() {
        assert_eq!(phase(0), Phase::Copy);
        assert_eq!(phase(4), Phase::Copy, "word 5 is still Copy");
        assert_eq!(phase(5), Phase::Flash, "word 6 begins Flash");
        assert_eq!(phase(11), Phase::Flash, "word 12 is still Flash");
        assert_eq!(phase(12), Phase::Audio, "word 13 begins Audio");
        assert_eq!(phase(19), Phase::Audio);
        assert_eq!(phase(24), Phase::Audio, "graduation is audio-only");
    }

    /// Every lineup language has a parsed curriculum of exactly 20+5 with all
    /// five trap intros resolvable to a position in the 20.
    #[test]
    fn every_language_curriculum_parses() {
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            let c = curriculum(code).unwrap_or_else(|| panic!("{code}: curriculum missing/unparsed"));
            assert_eq!(c.words.len(), FIRST_CONTACT, "{code}");
            assert_eq!(c.difficult.len(), DIFFICULT, "{code}");
            assert_eq!(c.traps.len(), 5, "{code}");
            for t in &c.traps {
                assert!(t.first_word < FIRST_CONTACT, "{code}/{}", t.id);
                assert!(!t.intro.is_empty() && t.intro.chars().count() <= 90, "{code}/{}", t.id);
                assert_eq!(intro_at(c, t.first_word).map(|x| x.id.as_str()), Some(t.id.as_str()), "{code}/{}", t.id);
            }
            for pos in 0..TOTAL {
                assert!(word_at(c, pos).is_some(), "{code} pos {pos}");
            }
        }
    }

    /// D3 mechanics: the checker reports progress, never failure; wrong units
    /// hold position; case never blocks.
    #[test]
    fn prefix_checker_is_failure_proof() {
        assert_eq!(check_prefix("casa", ""), (0, false));
        assert_eq!(check_prefix("casa", "c"), (1, false));
        assert_eq!(check_prefix("casa", "cx"), (1, false), "wrong unit: position holds at 1");
        assert_eq!(check_prefix("casa", "CASA"), (4, true), "case-insensitive");
        assert_eq!(check_prefix("niño", "niño"), (4, true), "NFC-equal diacritics");
        assert_eq!(check_prefix("niño", "nin\u{0303}o"), (4, true), "NFD input normalizes");
    }

    /// Progress round-trips through its own record only (host storage is a
    /// no-op, so this exercises the shape, not persistence).
    #[test]
    fn progress_shape_round_trips() {
        let p = Progress { pos: 7, shown: vec!["b-v".into()] };
        let json = serde_json::to_string(&p).unwrap();
        let back: Progress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pos, 7);
        assert_eq!(back.shown, vec!["b-v".to_string()]);
    }
}
