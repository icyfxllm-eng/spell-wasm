pub const TIER_ORDER: [&str; 4] = ["easy", "medium", "hard", "expert"];

pub fn tier_time(tier: &str) -> u32 {
    match tier {
        "easy" => 12,
        "medium" => 16,
        "hard" => 22,
        "expert" => 32,
        _ => 16,
    }
}

pub const LEVEL_OPTS: [(&str, &str); 5] = [
    ("climb", "Climb \u{2192}"),
    ("easy", "Easy"),
    ("medium", "Medium"),
    ("hard", "Hard"),
    ("expert", "Expert"),
];

pub const MINE: &str = "__mine";
pub const REVIEW: &str = "__review";
/// The built-in English word source. Its audio comes from the backend's
/// `/api/speak`; the word itself is still picked and known client-side.
pub const EN: &str = "en";
/// Built-in Spanish word source (backend TTS voice `es-ES`).
pub const ES: &str = "es";
pub const FR: &str = "fr";
pub const DE: &str = "de";
pub const PT: &str = "pt";
pub const IT: &str = "it";
pub const NL: &str = "nl";
pub const PL: &str = "pl";
pub const SV: &str = "sv";
pub const NB: &str = "nb";
pub const TR: &str = "tr";
pub const VI: &str = "vi";
pub const KO: &str = "ko";
pub const JA: &str = "ja";
pub const FIL: &str = "fil";
pub const ZH: &str = "zh";
pub const TH: &str = "th";

/// Built-in word-source languages: (lang code, display name). Adding a language
/// here + its word bank in `words.rs` + a voice in the backend's `LANG_VOICES`
/// makes it fully supported (audio + spelling). `MINE`/`REVIEW` aren't here.
/// Availability of a study language. THE single source of truth (see
/// `BUILTIN_LANGS`): `Active` = auditable + playable now; `ComingSoon` = visible
/// roadmap, gated from play until it passes native-speaker audit. Every surface
/// that lists / starts / routes to a language reads this — no scattered
/// per-language checks. Deactivated languages keep all their assets + user data;
/// reactivating is a one-line status flip.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LangStatus {
    Active,
    ComingSoon,
}
use LangStatus::{Active, ComingSoon};

pub const BUILTIN_LANGS: [(&str, &str, LangStatus); 17] = [
    (EN, "English", Active),
    // Spanish stays gated until a native-speaker audit passes.
    (ES, "Espa\u{f1}ol", ComingSoon),
    (FR, "Fran\u{e7}ais", ComingSoon),
    (DE, "Deutsch", ComingSoon),
    (PT, "Portugu\u{ea}s", ComingSoon),
    (IT, "Italiano", ComingSoon),
    (NL, "Nederlands", ComingSoon),
    (PL, "Polski", ComingSoon),
    (SV, "Svenska", ComingSoon),
    (NB, "Norsk", ComingSoon),
    (TR, "T\u{fc}rk\u{e7}e", ComingSoon),
    (VI, "Ti\u{1ebf}ng Vi\u{1ec7}t", ComingSoon),
    (KO, "\u{d55c}\u{ad6d}\u{c5b4}", ComingSoon),
    (JA, "\u{65e5}\u{672c}\u{8a9e}", ComingSoon),
    (FIL, "Filipino", ComingSoon),
    (ZH, "\u{4e2d}\u{6587}", ComingSoon),
    (TH, "\u{e44}\u{e17}\u{e22}", ComingSoon),
];

/// A language's availability status (ComingSoon for anything not in the registry).
pub fn lang_status(lang: &str) -> LangStatus {
    BUILTIN_LANGS.iter().find(|(c, _, _)| *c == lang).map(|(_, _, s)| *s).unwrap_or(ComingSoon)
}

/// True only for languages playable right now (passed audit). Gating for study
/// languages; `uiLang` / interface localization is unaffected.
pub fn is_active_lang(lang: &str) -> bool {
    lang_status(lang) == Active
}

/// Whether `lang` is a built-in, backend-voiced language (not My Words/Misses).
pub fn is_builtin_lang(lang: &str) -> bool {
    BUILTIN_LANGS.iter().any(|(code, _, _)| *code == lang)
}


pub const CORRECT_DELAY_MS: i32 = 2200;

/// Attempts allowed per word, across every mode (English, My Words, Misses).
pub const MAX_TRIES: u32 = 3;

pub const SR_MAXBOX: u32 = 5;
// ms intervals per box, index = box number (box 0 unused)
pub const SR_INT: [i64; 6] = [0, 0, 10 * 60 * 1000, 24 * 3600 * 1000, 3 * 24 * 3600 * 1000, 7 * 24 * 3600 * 1000];

pub const PRAISE: [&str; 8] = [
    "Clean.", "Locked in.", "On a roll.", "Nice ear.", "Spot on.", "Sharp.", "Chain grows.", "Perfect.",
];


/// Maps a base language code to the dictionaryapi.dev language code it supports.
pub fn def_lang(base: &str) -> Option<&'static str> {
    match base {
        "en" => Some("en"),
        "es" => Some("es"),
        "fr" => Some("fr"),
        "de" => Some("de"),
        "it" => Some("it"),
        "pt" => Some("pt-BR"),
        "ru" => Some("ru"),
        "ja" => Some("ja"),
        "ko" => Some("ko"),
        "ar" => Some("ar"),
        "tr" => Some("tr"),
        "hi" => Some("hi"),
        _ => None,
    }
}

pub struct Achievement {
    pub id: &'static str,
    pub ic: &'static str,
    pub nm: &'static str,
    pub desc: &'static str,
}

pub const ACHIEVEMENTS: [Achievement; 7] = [
    Achievement { id: "first", ic: "\u{1F3AF}", nm: "First Word", desc: "Spell your first word correctly." },
    Achievement { id: "chain5", ic: "\u{1F525}", nm: "On a Roll", desc: "Reach a 5-word chain." },
    Achievement { id: "chain10", ic: "\u{26A1}", nm: "Locked In", desc: "Reach a 10-word chain." },
    Achievement { id: "chain25", ic: "\u{1F31F}", nm: "Unstoppable", desc: "Reach a 25-word chain." },
    Achievement { id: "timed10", ic: "\u{23F1}", nm: "Beat the Clock", desc: "Reach a 10-chain in Quick Bee." },
    Achievement { id: "cleared", ic: "\u{2728}", nm: "Clean Slate", desc: "Clear all your missed words." },
    Achievement { id: "importer", ic: "\u{1F4E5}", nm: "Own Words", desc: "Import your own word list." },
];

#[cfg(test)]
mod tests {
    use super::*;


}

#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]
    fn only_en_active() {
        assert!(is_active_lang("en"));
        for (code, _, _) in BUILTIN_LANGS {
            if code != "en" {
                assert!(!is_active_lang(code), "{code} should be coming_soon");
            }
        }
        assert!(!is_active_lang("es") && !is_active_lang("ko") && !is_active_lang("zh"));
    }
}
