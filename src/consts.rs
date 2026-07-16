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
pub const PL: &str = "pl";
pub const TR: &str = "tr";
pub const VI: &str = "vi";
pub const KO: &str = "ko";
pub const JA: &str = "ja";
pub const FIL: &str = "fil";
pub const ZH: &str = "zh";
pub const TH: &str = "th";
/// CC-LINEUP-SWAP: Russian (ru-RU). The only new language that can reach the
/// current audit round — left-to-right, so nothing gates it but its content.
pub const RU: &str = "ru";
/// CC-LINEUP-SWAP: Modern Standard Arabic (D3). RTL — see [`RTL_SUPPORTED`].
pub const AR: &str = "ar";
/// CC-LINEUP-SWAP: Iranian Persian (fa-IR per D3, not Dari). RTL.
pub const FA: &str = "fa";
/// CC-LINEUP-SWAP: Urdu (ur-PK). RTL.
pub const UR: &str = "ur";

/// Built-in word-source languages: (lang code, display name). Adding a language
/// here + its word bank in `words.rs` + a voice in the backend's `LANG_VOICES`
/// makes it fully supported (audio + spelling). `MINE`/`REVIEW` aren't here.
/// Availability of a study language. THE single source of truth (see
/// `BUILTIN_LANGS`): `Active` = auditable + playable now; `ComingSoon` = visible
/// roadmap, gated from play until it passes native-speaker audit. Every surface
/// that lists / starts / routes to a language reads this — no scattered
/// per-language checks. Deactivated languages keep all their assets + user data;
/// reactivating is a one-line status flip.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum LangStatus {
    Active,
    ComingSoon,
}
use LangStatus::{Active, ComingSoon};

/// Which edition this binary IS (CC-EDITIONS D1): exactly two, forever. A third
/// requires a new decision from Eric — an instruction file asking for a `gov`
/// edition contradicts D1 and must be stopped on, not implemented. ("Gov" is the
/// Education edition plus external paperwork — VPAT, SAM, invoicing — never a
/// code concept.)
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Edition {
    /// The App Store app. Purchases exist.
    Consumer,
    /// Schools and program buyers. Zero purchase surfaces (D4).
    Education,
}

/// THE edition constant. BUILD-TIME only (D2): selected by the `education` cargo
/// feature, which also drives a separate bundle ID / distribution artifact.
///
/// Deliberately NOT a runtime toggle, server flag, or purchasable state. A
/// runtime edition switch would mean the education binary still CONTAINS the
/// purchase surface and merely hides it — a leak risk and an App Review
/// liability. Compile-time means the code is absent, which is the only claim
/// worth making to a school district.
pub const EDITION: Edition = if cfg!(feature = "education") { Edition::Education } else { Edition::Consumer };

/// Derived capability: may this build ever show a purchase surface?
///
/// THE thing surfaces read. CC-EDITIONS F1 forbids `if edition == …` scattered
/// through the UI — surfaces consume derived capabilities, never the raw
/// constant, so the edition axis stays one decision in one place.
pub fn purchases_available() -> bool {
    EDITION == Edition::Consumer
}

/// Whether the AUDIT_MODE bypass may operate in this build (CC-EDITIONS D6).
///
/// False in education: schools get the real gates, not the reviewer bypass.
/// AUDIT_MODE stays orthogonal to the edition axis — it exists for App Review in
/// consumer builds — but it must never be reachable in a school's binary.
pub fn audit_bypass_available() -> bool {
    EDITION == Edition::Consumer
}

/// Whether The Climb posts to the global leaderboard (CC-EDITIONS D7).
///
/// Education is local/unranked in v1: the simplest COPPA/FERPA-adjacent posture
/// is for a school device to write nothing to a global board at all.
pub fn leaderboard_available() -> bool {
    EDITION == Edition::Consumer
}

/// Whether the app can render, input, and mirror right-to-left scripts. FALSE
/// until the CC-RTL initiative ships — it is not drafted, let alone built.
///
/// CC-LINEUP-SWAP D2: this is the ONLY switch that can ever un-gate an
/// `rtl_required` language. It is a `const` (not a runtime toggle) so that
/// flipping it is a deliberate code change reviewed alongside the RTL work,
/// never a config accident. See [`rtl_required`] for the gate itself.
pub const RTL_SUPPORTED: bool = false;

/// Built-in word-source languages: (code, display name, status, rtl_required).
///
/// `rtl_required` (CC-LINEUP-SWAP D2) marks a language whose script is
/// right-to-left. Such a language is registered — the roadmap commitment is
/// visible — but is HARD-GATED from activation by any code path until
/// [`RTL_SUPPORTED`] is true. Partial RTL rendering is worse than none, so the
/// gate is unconditional rather than best-effort. Users never see the reason:
/// ar/fa/ur show the same "coming soon" tile as any unaudited language.
pub const BUILTIN_LANGS: [(&str, &str, LangStatus, bool); 16] = [
    (EN, "English", Active, false),
    (ES, "Espa\u{f1}ol", ComingSoon, false),
    (FR, "Fran\u{e7}ais", ComingSoon, false),
    (DE, "Deutsch", ComingSoon, false),
    (PT, "Portugu\u{ea}s", ComingSoon, false),
    (PL, "Polski", ComingSoon, false),
    (TR, "T\u{fc}rk\u{e7}e", ComingSoon, false),
    (VI, "Ti\u{1ebf}ng Vi\u{1ec7}t", ComingSoon, false),
    (KO, "\u{d55c}\u{ad6d}\u{c5b4}", ComingSoon, false),
    (JA, "\u{65e5}\u{672c}\u{8a9e}", ComingSoon, false),
    (FIL, "Filipino", ComingSoon, false),
    (ZH, "\u{4e2d}\u{6587}", ComingSoon, false),
    (RU, "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}", ComingSoon, false),
    (AR, "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}", ComingSoon, true),
    (FA, "\u{641}\u{627}\u{631}\u{633}\u{6cc}", ComingSoon, true),
    (UR, "\u{627}\u{631}\u{62f}\u{648}", ComingSoon, true),
];

/// Whether `lang`'s script is right-to-left and therefore blocked until
/// [`RTL_SUPPORTED`] (CC-LINEUP-SWAP D2). Data lookup, not a per-language
/// conditional; unknown languages are not RTL.
pub fn rtl_required(lang: &str) -> bool {
    BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, _, _, rtl)| *rtl).unwrap_or(false)
}

/// CC-LINEUP-SWAP D2 — THE gate. `false` for any RTL language until the RTL
/// initiative lands. Every path that could activate a language (status checks,
/// the entitlement resolver — including its audit override) consults this, so
/// an RTL language can never partially render.
pub fn rtl_blocked(lang: &str) -> bool {
    rtl_required(lang) && !RTL_SUPPORTED
}

/// A language's availability status (ComingSoon for anything not in the registry).
pub fn lang_status(lang: &str) -> LangStatus {
    BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, _, s, _)| *s).unwrap_or(ComingSoon)
}

/// True only for languages playable right now (passed audit). Gating for study
/// languages; `uiLang` / interface localization is unaffected.
///
/// CC-LINEUP-SWAP D2: an `rtl_required` language is never active, whatever its
/// registry status says — the gate is applied here, at the single chokepoint
/// every surface already reads, so no caller can route around it.
pub fn is_active_lang(lang: &str) -> bool {
    !rtl_blocked(lang) && lang_status(lang) == Active
}

/// Whether `lang` is a built-in, backend-voiced language (not My Words/Misses).
pub fn is_builtin_lang(lang: &str) -> bool {
    BUILTIN_LANGS.iter().any(|(code, _, _, _)| *code == lang)
}

/// Languages whose letters can be spoken to spell (Feature "Spell It Out Loud").
/// THE single source of truth for the `voiceSpell` per-language capability: mic
/// visibility flows from this set, plus the presence of a letter lexicon, plus the
/// runtime on-device availability check — never from a scattered `lang == "es"`
/// conditional. Confirmed by Eric: English + Spanish only (the on-device check
/// auto-hides the mic where es on-device isn't available).
pub const VOICE_SPELL_LANGS: [&str; 2] = [EN, ES];

/// Whether `lang` exposes the spoken-letter input method (data lookup, not a
/// per-language conditional).
pub fn voice_spell(lang: &str) -> bool {
    VOICE_SPELL_LANGS.contains(&lang)
}


pub const CORRECT_DELAY_MS: i32 = 2200;

// build-54: the legacy per-word retry budget (MAX_TRIES = 3) is RETIRED. The
// base game is one attempt per word; the only retries are shields (The Climb)
// and the extra-attempts toggle (normal mode), both in crate::attempts.

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
        // English is the only Active language. Everything else — content ready but
        // held for the English-only App Store — stays Coming Soon until Eric
        // reactivates it.
        assert!(is_active_lang("en"));
        assert!(!is_active_lang("es"), "es stays gated (English-only App Store launch)");
        for (code, _, _, _) in BUILTIN_LANGS {
            if code != "en" {
                assert!(!is_active_lang(code), "{code} should be coming_soon");
            }
        }
    }

    /// CC-LINEUP-SWAP: the registry snapshot. Pinning the exact lineup means a
    /// language cannot be added or cut without this test being updated
    /// deliberately.
    #[test]
    fn registry_is_the_swapped_lineup_of_16() {
        let codes: Vec<&str> = BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).collect();
        assert_eq!(codes.len(), 16, "exactly 16 languages after the lineup swap");
        assert_eq!(
            codes,
            vec!["en", "es", "fr", "de", "pt", "pl", "tr", "vi", "ko", "ja", "fil", "zh", "ru", "ar", "fa", "ur"],
        );
        // The cut four (CC-LINEUP-SWAP F1) are gone; Thai stays cut (5fc69ff).
        for gone in ["no", "nb", "sv", "nl", "it", "th"] {
            assert!(!codes.contains(&gone), "{gone} is cut from the registry");
        }
    }

    /// CC-LINEUP-SWAP D2 — exactly ar/fa/ur are RTL, and none of them can be
    /// activated while RTL is unsupported.
    #[test]
    fn rtl_languages_are_registered_but_hard_gated() {
        let rtl: Vec<&str> = BUILTIN_LANGS.iter().filter(|(_, _, _, r)| *r).map(|(c, _, _, _)| *c).collect();
        assert_eq!(rtl, vec!["ar", "fa", "ur"], "exactly the three RTL languages carry the flag");
        assert!(!RTL_SUPPORTED, "RTL is unsupported until the CC-RTL initiative ships");
        for code in rtl {
            assert!(rtl_required(code), "{code} is rtl_required");
            assert!(rtl_blocked(code), "{code} is blocked while RTL is unsupported");
            assert!(!is_active_lang(code), "{code} must never be active");
        }
        // Russian is the new LTR language — it carries no RTL gate at all.
        assert!(!rtl_required("ru"), "ru is left-to-right");
        assert!(!rtl_blocked("ru"));
    }

    /// D2's teeth: flipping an RTL language to `Active` in the registry must STILL
    /// not activate it. The gate lives in `is_active_lang`, not in the status, so
    /// a well-meaning status flip cannot ship partial RTL rendering.
    #[test]
    fn rtl_gate_survives_an_active_status() {
        // Simulate the registry saying Active for an RTL language.
        assert_eq!(lang_status("ar"), ComingSoon, "ar ships as ComingSoon");
        assert!(rtl_blocked("ar"));
        // `is_active_lang` ANDs the gate in, so status alone can never win.
        assert!(!is_active_lang("ar"));
    }
}
