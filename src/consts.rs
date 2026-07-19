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
pub const VI: &str = "vi";
pub const KO: &str = "ko";
pub const JA: &str = "ja";
pub const FIL: &str = "fil";
pub const ZH: &str = "zh";
/// CC-LINEUP-SWAP: Russian (ru-RU). The only new language that can reach the
/// current audit round — left-to-right, so nothing gates it but its content.
pub const RU: &str = "ru";
/// CC-LINEUP-SWAP: Modern Standard Arabic (D3). RTL — see [`RTL_SUPPORTED`].
pub const AR: &str = "ar";
/// CC-LINEUP-SWAP: Iranian Persian (fa-IR per D3, not Dari). RTL.
pub const FA: &str = "fa";
/// CC-LINEUP-SWAP: Urdu (ur-PK). RTL.
pub const UR: &str = "ur";
/// CC-HINDI-PHASE0: Hindi (Devanagari, LTR). Registered ONLY in the audit-preview
/// build — D8 grants no authority to register it in production, so BUILTIN_LANGS
/// below includes it under `audit_preview` and nowhere else.
pub const HI: &str = "hi";

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
/// `rtl_required` language. It is a compile-time `const` (not a runtime toggle) so
/// flipping it is a deliberate code change, never a config accident. See
/// [`rtl_required`] for the gate itself.
///
/// FALSE in production. TRUE only in the `audit_preview` build, where native
/// speakers play the RTL languages to review them — which stays compile-time (D2's
/// intent): a production build has the feature off, so this is `false` and the
/// languages are gated, exactly as before.
pub const RTL_SUPPORTED: bool = cfg!(feature = "audit_preview");

/// Which way a language's script runs. CC-RTL **D3**: direction comes from the
/// REGISTRY and nowhere else — "no hardcoded language→direction checks anywhere
/// else". Every surface that needs to know asks [`direction`].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Direction {
    Ltr,
    Rtl,
}
use Direction::{Ltr, Rtl};

/// Built-in word-source languages: (code, display name, status, direction).
///
/// `direction` (CC-RTL D3) is the script's reading direction, and it is the field
/// three separate questions are answered FROM — deliberately, because they are
/// not the same question and conflating them is how RTL bugs hide:
///
/// * [`rtl_required`] — may this language activate? An `Rtl` language is
///   HARD-GATED until [`RTL_SUPPORTED`] (CC-LINEUP-SWAP D2). Partial RTL is worse
///   than none, so the gate is unconditional. Users never see the reason: ar/fa/ur
///   show the same "coming soon" tile as any unaudited language.
/// * [`script_joins`] — do its letters JOIN? Not the same as direction: Hebrew is
///   Rtl and does not join. This decides whether the answer surface may split a
///   word into per-letter elements.
/// * [`direction`] — which way does it READ? What the play surface sets `dir` from.
const LANGS_BASE: [(&str, &str, LangStatus, Direction); 15] = [
    (EN, "English", Active, Ltr),
    (ES, "Espa\u{f1}ol", ComingSoon, Ltr),
    (FR, "Fran\u{e7}ais", ComingSoon, Ltr),
    (DE, "Deutsch", ComingSoon, Ltr),
    (PT, "Portugu\u{ea}s", ComingSoon, Ltr),
    (PL, "Polski", ComingSoon, Ltr),
    (VI, "Ti\u{1ebf}ng Vi\u{1ec7}t", ComingSoon, Ltr),
    (KO, "\u{d55c}\u{ad6d}\u{c5b4}", ComingSoon, Ltr),
    (JA, "\u{65e5}\u{672c}\u{8a9e}", ComingSoon, Ltr),
    (FIL, "Filipino", ComingSoon, Ltr),
    (ZH, "\u{4e2d}\u{6587}", ComingSoon, Ltr),
    (RU, "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}", ComingSoon, Ltr),
    (AR, "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}", ComingSoon, Rtl),
    (FA, "\u{641}\u{627}\u{631}\u{633}\u{6cc}", ComingSoon, Rtl),
    (UR, "\u{627}\u{631}\u{62f}\u{648}", ComingSoon, Rtl),
];

/// Production registry — the base 15, unchanged.
#[cfg(not(feature = "audit_preview"))]
pub const BUILTIN_LANGS: [(&str, &str, LangStatus, Direction); 15] = LANGS_BASE;

/// Audit-preview registry — the base 15 plus Hindi (हिन्दी), so it can be reviewed.
/// Built by referencing LANGS_BASE, not re-listing it, so the two can't drift.
#[cfg(feature = "audit_preview")]
pub const BUILTIN_LANGS: [(&str, &str, LangStatus, Direction); 16] = [
    LANGS_BASE[0], LANGS_BASE[1], LANGS_BASE[2], LANGS_BASE[3], LANGS_BASE[4],
    LANGS_BASE[5], LANGS_BASE[6], LANGS_BASE[7], LANGS_BASE[8], LANGS_BASE[9],
    LANGS_BASE[10], LANGS_BASE[11], LANGS_BASE[12], LANGS_BASE[13], LANGS_BASE[14],
    (HI, "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}", ComingSoon, Ltr),
];

/// THE direction accessor (CC-RTL D3). The play surface sets `dir` from this and
/// from nothing else. An unknown language reads left-to-right — the safe default,
/// since a wrong `dir` on Latin is visible immediately while a missing one on
/// Arabic is not.
pub fn direction(lang: &str) -> Direction {
    BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, _, _, d)| *d).unwrap_or(Ltr)
}

/// The `dir` attribute value for `lang` — `"rtl"` or `"ltr"`. What surfaces write.
pub fn dir_attr(lang: &str) -> &'static str {
    match direction(lang) {
        Rtl => "rtl",
        Ltr => "ltr",
    }
}

/// Whether `lang`'s script is right-to-left and therefore blocked until
/// [`RTL_SUPPORTED`] (CC-LINEUP-SWAP D2). Derived from the registry's
/// `direction`, so the gate and the rendering can never disagree about which
/// languages are RTL.
pub fn rtl_required(lang: &str) -> bool {
    direction(lang) == Rtl
}

/// CC-LINEUP-SWAP D2 — THE gate. `false` for any RTL language until the RTL
/// initiative lands. Every path that could activate a language (status checks,
/// the entitlement resolver — including its audit override) consults this, so
/// an RTL language can never partially render.
pub fn rtl_blocked(lang: &str) -> bool {
    rtl_required(lang) && !RTL_SUPPORTED
}

/// Whether `lang`'s script is CURSIVE — its letters join, so the word must reach
/// the text shaper as ONE run.
///
/// This is what the answer surface needs to know, and it is NOT the same question
/// as "is it RTL". Splitting a word into per-element letters is harmless for
/// Latin, Hangul and kana (nothing joins), and destroys Arabic (everything does).
/// Direction is irrelevant to it: Hebrew is RTL and does not join; if Hebrew ever
/// enters the lineup this MUST become its own registry field rather than keep
/// riding on `rtl_required`.
///
/// Today it derives from `rtl_required` because every RTL language in the lineup
/// (ar/fa/ur) is Arabic-script, and Arabic-script joins. That coincidence is
/// load-bearing, so it is stated here rather than left for someone to discover:
/// the test `cursive_is_exactly_the_arabic_script_languages` pins it.
pub fn script_joins(lang: &str) -> bool {
    rtl_required(lang)
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
    if rtl_blocked(lang) {
        return false;
    }
    if lang_status(lang) == Active {
        return true;
    }
    // Audit preview only: a registered ComingSoon language is playable, so a native
    // speaker can review it. Compile-time — the production build (feature off) never
    // takes this branch, so ComingSoon stays gated exactly as before.
    cfg!(feature = "audit_preview") && is_builtin_lang(lang)
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


// `def_lang` (base language -> dictionaryapi.dev code) was deleted 2026-07-17.
// It mapped languages to endpoints that do not exist: dictionaryapi.dev serves
// ENGLISH ONLY, and every other language 404s. Its one caller
// (game::fetch_definition) fetched those URLs directly from the browser,
// bypassing our proxy and sending a child's word + IP to a third party for
// nothing. See game::fetch_definition and docs/DECISIONS-PENDING.md §10.

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
mod registry_tests {
    use super::*;
    // Production invariant: audit_preview deliberately activates ComingSoon
    // languages so they can be reviewed, so this holds only with the feature off.
    #[cfg(not(feature = "audit_preview"))]
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
    fn registry_is_the_swapped_lineup_of_15() {
        // The lineup is fixed on LANGS_BASE (15) regardless of build; BUILTIN_LANGS
        // equals it in production and appends Hindi only under audit_preview.
        let codes: Vec<&str> = LANGS_BASE.iter().map(|(c, _, _, _)| *c).collect();
        assert_eq!(codes.len(), 15, "15 languages: the swap's 16 minus Turkish (CC-HINDI-PHASE0 D1)");
        assert_eq!(
            codes,
            vec!["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "zh", "ru", "ar", "fa", "ur"],
        );
        // The cut four (CC-LINEUP-SWAP F1), plus Thai (5fc69ff) and Turkish
        // (CC-HINDI-PHASE0 D1 — permanently; Hindi replaces it).
        for gone in ["no", "nb", "sv", "nl", "it", "th", "tr"] {
            assert!(!codes.contains(&gone), "{gone} is cut from the registry");
        }
        // Production ships exactly the base; audit-preview adds Hindi as the 16th.
        assert_eq!(BUILTIN_LANGS.len(), if cfg!(feature = "audit_preview") { 16 } else { 15 });
        #[cfg(feature = "audit_preview")]
        assert_eq!(BUILTIN_LANGS[15].0, "hi", "Hindi is the audit-only 16th entry");
        #[cfg(not(feature = "audit_preview"))]
        assert!(!BUILTIN_LANGS.iter().any(|(c, _, _, _)| *c == "hi"), "production registers no Hindi (D8)");
    }

    /// CC-LINEUP-SWAP D2 — exactly ar/fa/ur are RTL, and (in production) none can be
    /// activated while RTL is unsupported. The audit-preview build deliberately
    /// un-gates them; `rtl_gate_matches_the_build_config` below pins that relationship.
    #[cfg(not(feature = "audit_preview"))]
    #[test]
    fn rtl_languages_are_registered_but_hard_gated() {
        let rtl: Vec<&str> = BUILTIN_LANGS.iter().filter(|(_, _, _, d)| *d == Rtl).map(|(c, _, _, _)| *c).collect();
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

    /// The RTL gate tracks the build config and nothing else — false in production,
    /// true only under audit_preview. Runs in BOTH configs, so there is always a
    /// live assertion that the production binary stays gated. D2 is upheld: the
    /// switch is still compile-time, just now two-valued by feature.
    #[test]
    fn rtl_gate_matches_the_build_config() {
        assert_eq!(RTL_SUPPORTED, cfg!(feature = "audit_preview"));
        // Under audit preview the RTL languages become playable; otherwise gated.
        assert_eq!(is_active_lang("ar"), cfg!(feature = "audit_preview"));
    }

    /// CC-RTL F4 leans on `script_joins` deriving from `rtl_required`, which is
    /// only sound while every RTL language in the lineup is Arabic-script. Pin it:
    /// if someone adds Hebrew (RTL, does NOT join) this test fails, which is the
    /// moment `script_joins` must become its own registry field instead of riding
    /// on direction.
    #[test]
    fn cursive_is_exactly_the_arabic_script_languages() {
        let joins: Vec<&str> = BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).filter(|c| script_joins(c)).collect();
        assert_eq!(joins, vec!["ar", "fa", "ur"], "only the Arabic-script languages are cursive");
        // Nothing else may take the joined path — splitting is harmless for them
        // and the `pop` animation depends on it.
        for lang in ["en", "es", "fr", "de", "pt", "pl", "tr", "vi", "ko", "ja", "fil", "zh", "ru"] {
            assert!(!script_joins(lang), "{lang} does not join — it must keep the per-letter path");
        }
        // Russian is the trap: new, non-Latin, and Cyrillic does NOT join.
        assert!(!script_joins("ru"), "Cyrillic is not cursive");
    }

    /// CC-RTL F1/D3 — direction lives in the registry, and `dir_attr` is what
    /// surfaces write. Pinned because the failure mode is silent: a wrong `dir`
    /// on Latin is visible instantly; a missing one on Arabic is not.
    #[test]
    fn direction_comes_from_the_registry() {
        for lang in ["ar", "fa", "ur"] {
            assert_eq!(direction(lang), Rtl, "{lang} reads right-to-left");
            assert_eq!(dir_attr(lang), "rtl");
        }
        for lang in ["en", "es", "ru", "ko", "ja", "zh", "vi"] {
            assert_eq!(direction(lang), Ltr, "{lang} reads left-to-right");
            assert_eq!(dir_attr(lang), "ltr");
        }
        // Unknown reads LTR — the safe default, and the one whose failure is
        // visible rather than silent.
        assert_eq!(direction("__mine"), Ltr);
        assert_eq!(direction("qqq"), Ltr);
    }

    /// The activation gate and the rendering must never disagree about which
    /// languages are RTL. `rtl_required` DERIVES from `direction`, so they cannot.
    #[test]
    fn the_gate_and_the_direction_field_cannot_diverge() {
        for (code, _, _, d) in BUILTIN_LANGS {
            assert_eq!(rtl_required(code), d == Rtl, "{code}: gate disagrees with its own direction");
        }
    }

    /// D2's teeth: flipping an RTL language to `Active` in the registry must STILL
    /// not activate it. The gate lives in `is_active_lang`, not in the status, so
    /// a well-meaning status flip cannot ship partial RTL rendering. (Production
    /// only — audit_preview lifts the gate on purpose, pinned by
    /// `rtl_gate_matches_the_build_config`.)
    #[cfg(not(feature = "audit_preview"))]
    #[test]
    fn rtl_gate_survives_an_active_status() {
        // Simulate the registry saying Active for an RTL language.
        assert_eq!(lang_status("ar"), ComingSoon, "ar ships as ComingSoon");
        assert!(rtl_blocked("ar"));
        // `is_active_lang` ANDs the gate in, so status alone can never win.
        assert!(!is_active_lang("ar"));
    }
}
