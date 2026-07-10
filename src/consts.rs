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

/// Built-in word-source languages: (lang code, display name). Adding a language
/// here + its word bank in `words.rs` + a voice in the backend's `LANG_VOICES`
/// makes it fully supported (audio + spelling). `MINE`/`REVIEW` aren't here.
pub const BUILTIN_LANGS: [(&str, &str); 14] = [
    (EN, "English"),
    (ES, "Espa\u{f1}ol"),
    (FR, "Fran\u{e7}ais"),
    (DE, "Deutsch"),
    (PT, "Portugu\u{ea}s"),
    (IT, "Italiano"),
    (NL, "Nederlands"),
    (PL, "Polski"),
    (SV, "Svenska"),
    (NB, "Norsk"),
    (TR, "T\u{fc}rk\u{e7}e"),
    (VI, "Ti\u{1ebf}ng Vi\u{1ec7}t"),
    (KO, "\u{d55c}\u{ad6d}\u{c5b4}"),
    (JA, "\u{65e5}\u{672c}\u{8a9e}"),
];

/// Whether `lang` is a built-in, backend-voiced language (not My Words/Misses).
pub fn is_builtin_lang(lang: &str) -> bool {
    BUILTIN_LANGS.iter().any(|(code, _)| *code == lang)
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
    Achievement { id: "timed10", ic: "\u{23F1}", nm: "Beat the Clock", desc: "Reach a 10-chain in Timed mode." },
    Achievement { id: "cleared", ic: "\u{2728}", nm: "Clean Slate", desc: "Clear all your missed words." },
    Achievement { id: "importer", ic: "\u{1F4E5}", nm: "Own Words", desc: "Import your own word list." },
];
