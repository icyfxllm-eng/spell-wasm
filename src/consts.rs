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
pub const BUILTIN_LANGS: [(&str, &str); 17] = [
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
    (FIL, "Filipino"),
    (ZH, "\u{4e2d}\u{6587}"),
    (TH, "\u{e44}\u{e17}\u{e22}"),
];

/// Whether `lang` is a built-in, backend-voiced language (not My Words/Misses).
pub fn is_builtin_lang(lang: &str) -> bool {
    BUILTIN_LANGS.iter().any(|(code, _)| *code == lang)
}

/// Feature flag: Korean + Thai handwriting are OFF by default, pending
/// native-auditor review of their cultural practice grids (spec step 10).
pub const DRAW_KO_TH_ENABLED: bool = false;
/// Master flag: draw stays fully off until the ML Kit recognizer ships (steps 3-10).
pub const DRAW_MLKIT_READY: bool = false;

/// Per-locale input-mode gating for the handwriting (draw) mode. Handwriting is
/// offered ONLY where it's a genuine literacy skill AND a stroke recognizer
/// exists — Chinese and Japanese (Korean/Thai behind DRAW_KO_TH_ENABLED).
/// Every other locale — English, all European, Vietnamese (Latin script;
/// tone marks are a keyboard skill), Filipino, and Latin My Words — is
/// type + speak only, and the draw button must not render at all.
pub fn draw_available(lang: &str) -> bool {
    // Master gate: the ML Kit Digital Ink stroke recognizer + cultural practice
    // grids aren't built yet (spec steps 3-10), so ALL handwriting stays hidden
    // for now — no half-working draw button ships. Flip DRAW_MLKIT_READY to true
    // when the DigitalInkPlugin lands; the per-locale config below then applies.
    if !DRAW_MLKIT_READY {
        return false;
    }
    match lang {
        ZH | JA => true,
        KO | TH => DRAW_KO_TH_ENABLED,
        _ => false,
    }
}

/// Languages whose Expert tier tests CHARACTER/kanji recall — a difficulty that
/// the app's phonetic input (pinyin for 中文, kana for 日本語) cannot test, per
/// the CJK input-mode report. For these, Expert answers must be DRAWN so the
/// stroke recognizer checks the actual character.
const CHAR_EXPERT_LANGS: [&str; 2] = [ZH, JA];

/// Pure policy: does this (lang, tier) test character recall that only drawing
/// can capture? True for 中文/日本語 Expert regardless of recognizer readiness —
/// this is the *intent*. Gate it with [`expert_requires_drawing`] for the
/// runtime decision.
pub fn is_char_expert(lang: &str, tier: &str) -> bool {
    tier == "expert" && CHAR_EXPERT_LANGS.contains(&lang)
}

/// Runtime decision: should this word's answer be collected as a DRAWN character
/// instead of typed input? True only when it's a character-Expert word AND the
/// drawing recognizer is actually available. Until `DRAW_MLKIT_READY` flips,
/// this is always false and Expert falls back to the current typed input — the
/// honest interim from the input-mode report (pinyin/kana can't test the
/// character, but a half-working draw pad would be worse). When the ML Kit
/// DigitalInkPlugin lands, 中文/日本語 Expert routes through drawing automatically
/// and the drawn stroke is scored by `draw_judge`.
pub fn expert_requires_drawing(lang: &str, tier: &str) -> bool {
    is_char_expert(lang, tier) && draw_available(lang)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_expert_is_zh_ja_expert_only() {
        assert!(is_char_expert(ZH, "expert"));
        assert!(is_char_expert(JA, "expert"));
        // not lower tiers (common words; character difficulty matters less there)
        assert!(!is_char_expert(ZH, "hard"));
        assert!(!is_char_expert(JA, "medium"));
        // not native-script languages that already test real spelling
        assert!(!is_char_expert(KO, "expert"));
        assert!(!is_char_expert(TH, "expert"));
        assert!(!is_char_expert(EN, "expert"));
    }

    #[test]
    fn expert_drawing_is_gated_off_until_mlkit_ready() {
        // The intent (is_char_expert) is set, but the runtime decision stays
        // false while the recognizer is unbuilt — Expert falls back to typing.
        assert!(!DRAW_MLKIT_READY, "test assumes recognizer not shipped yet");
        assert!(!expert_requires_drawing(ZH, "expert"));
        assert!(!expert_requires_drawing(JA, "expert"));
        // and never for anything else regardless of the flag
        assert!(!expert_requires_drawing(EN, "expert"));
        assert!(!expert_requires_drawing(ZH, "easy"));
    }
}
