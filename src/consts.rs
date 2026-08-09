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
/// The sole RTL language after Persian and Urdu were cut (CC-MASTER-PARITY Phase A).
pub const AR: &str = "ar";
/// CC-MASTER-PARITY Track S: Swahili (Kiswahili, sw-TZ). Latin script, LTR —
/// renders with no special machinery. Real Leipzig CC BY bank.
pub const SW: &str = "sw";
/// Hindi (Devanagari, LTR). Was audit-preview-only under CC-HINDI-PHASE0 D8;
/// PROMOTED to the production registry as the 15th language by Eric's ruling
/// (2026-07-25) with a full Leipzig-Wikipedia bank (build-hi-bank.py).
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

/// Whether the app can render, input, and mirror right-to-left scripts.
///
/// CC-LINEUP-SWAP D2: this is the ONLY switch that can ever un-gate an
/// `rtl_required` language. It is a compile-time `const` (not a runtime toggle) so
/// flipping it is a deliberate code change, never a config accident. See
/// [`rtl_required`] for the gate itself.
///
/// TRUE — Arabic UNGATED by Eric's ruling (2026-07-25) after on-device review
/// of the RTL rendering ("the arabic looks fine"). The rendering stack (F1–F6)
/// was built and exercised under audit_preview; the production flip is this
/// line. Native-speaker content audit (G4 / Gig B) remains an open follow-up —
/// tracked in docs/CC-MASTER-PARITY.md Track A, not a render gate anymore.
pub const RTL_SUPPORTED: bool = true;

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
    (ES, "Espa\u{f1}ol", Active, Ltr),
    (FR, "Fran\u{e7}ais", Active, Ltr),
    (DE, "Deutsch", Active, Ltr),
    (PT, "Portugu\u{ea}s", Active, Ltr),
    (PL, "Polski", Active, Ltr),
    (VI, "Ti\u{1ebf}ng Vi\u{1ec7}t", Active, Ltr),
    (KO, "\u{d55c}\u{ad6d}\u{c5b4}", Active, Ltr),
    (JA, "\u{65e5}\u{672c}\u{8a9e}", Active, Ltr),
    (FIL, "Filipino", Active, Ltr),
    (ZH, "\u{4e2d}\u{6587}", Active, Ltr),
    (RU, "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}", Active, Ltr),
    (AR, "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}", Active, Rtl),
    (SW, "Kiswahili", Active, Ltr),
    (HI, "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}", Active, Ltr),
];

/// THE registry. All fifteen are Active: the APP ships every language.
///
/// The web is a different question and it is answered in exactly one place,
/// [`is_active_lang`], via the `web` cargo feature -- not by a second
/// registry and not by scattered `if (isWeb)` checks. Eric, 2026-07-31:
/// "For testflight all languages unlocked the site english only."
///
/// Worth knowing when re-reading this table: commit 3381daf (2026-07-19)
/// flipped ten languages to Active with "[AWAITING ERIC]" in its title and
/// "STAGED FOR REVIEW, NOT SHIPPED ... activate the subset Eric confirms by
/// reverting the ones he isn't ready for" in its body. Nobody reverted
/// anything, so the unaudited fifteen shipped by default rather than by
/// decision. They are Active now BY decision -- TestFlight is internal
/// testing and Eric wants testers on every language -- but the native-audit
/// sign-offs those languages still lack are tracked in CC-LEARNING-ENGINE D2
/// and are not satisfied by this line.
pub const BUILTIN_LANGS: [(&str, &str, LangStatus, Direction); 15] = LANGS_BASE;

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

/// Whether `lang`'s script REQUIRES SHAPING — its glyphs combine, so the word
/// must reach the text shaper as ONE run.
///
/// This is what the answer surface needs to know, and it is NOT the same
/// question as "is it RTL". Splitting a word into per-element letters is
/// harmless for Latin, Hangul and kana (nothing combines) and destroys Arabic
/// (everything joins) and Devanagari (matras and conjuncts attach to their base
/// consonant — split `कि` into two spans and the matra detaches).
///
/// It used to derive from `rtl_required`, on the reasoning that every RTL
/// language in the lineup was Arabic-script. That framing asked the wrong
/// question. Hindi is left-to-right and still needs the shaper, so it silently
/// took the per-letter path while Spell Picture's own `complex_script` — a
/// second, hand-maintained list — correctly protected it. Two lists, one of
/// them wrong, and nothing comparing them.
///
/// So this is now its own predicate, and `wordpic_screen` derives from it
/// rather than keeping a parallel copy. Adding a language whose script shapes
/// means adding it HERE, once.
pub fn script_joins(lang: &str) -> bool {
    matches!(lang, AR | HI)
}

/// CC-DEF-MATCH per-language activation (Invariant 7/8: THE registry flag, no
/// other per-language logic anywhere). True only when the language's definition
/// pool meets the D3 floor (40 prompt-grade rows) in at least one tier under
/// Eric's 2026-07-27 interim-content ruling (mechanical prescreen stands in for
/// the still-open Gig A audit; the formal native review remains a follow-up).
/// Values are pinned from `scripts/build-def-pools.py` summaries — re-run it
/// and update here (snapshot test pins the current truth). FULL ACTIVATION
/// 2026-07-27: every registered language clears the D3 floor in at least
/// easy+medium (12 langs all four tiers; ar easy-hard; ja easy-medium — the
/// screen steps DOWN to the deepest clearing tier at open). Per-tier floors
/// stay enforced at pool-fetch time; this flag is per-language reachability.
pub fn def_match(lang: &str) -> bool {
    is_builtin_lang(lang)
}

/// CC-PRACTICE D7 — per-language availability (registry-derived, no other
/// path). OPEN TO ALL 15 by Eric's ruling ("open practice to all languages",
/// 2026-07-27) under the standing interim-content posture — drafted curricula
/// + intro strings ship now, native review remains the follow-up (same as the
/// banks, UI locales, and definition pools). Snapshot-tested.
/// Mic-everywhere server rung: languages the backend can recognize via
/// Google STT when the DEVICE has no on-device model (today that means
/// sw/fil in practice; the gate covers the lineup so a device without a
/// download path still gets the disclosed option). NEVER offered in Kid
/// Mode or education builds, and never without the internet-consent card.
pub fn server_stt(lang: &str) -> bool {
    is_builtin_lang(lang)
}

pub fn practice(lang: &str) -> bool {
    is_builtin_lang(lang)
}

/// A language's availability status (ComingSoon for anything not in the registry).
pub fn lang_status(lang: &str) -> LangStatus {
    BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, _, s, _)| *s).unwrap_or(ComingSoon)
}

/// CC-PHOTO-IMPORT Phase 0 — can Vision's on-device text recognizer read this
/// language's script?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrSupport {
    /// Vision recognizes the language directly (its own recognition model).
    Native,
    /// Latin-script language Vision has no model for: run the ENGLISH recognizer
    /// with language correction OFF and let the word banks validate (G-C).
    EnglishFallback,
    /// No sound recognition path — the photo feature is HIDDEN for this language.
    Unsupported,
}

/// THE Vision support matrix (invariant: one registry accessor, never a
/// scattered `lang == …` check — the `ocr_support` mirror of `direction`).
///
/// Values are MEASURED, never guessed (gate G-B): `VNRecognizeTextRequest
/// .supportedRecognitionLanguages()` on iOS 26.5 via
/// `VisionLanguageMatrixTests.testDumpVisionLanguageMatrix` (2026-07-25).
/// Measured list: en fr it de es pt zh(-Hans/Hant) yue ko ja ru uk th vi ar ars
/// tr id cs da nl no nn nb ms pl ro sv → 12 of our 14 are Native; fil and sw
/// (Latin script, no model) fall back to the English recognizer; nothing is
/// Unsupported. Re-run the measurement test when the iOS floor moves and update
/// here — the snapshot test below pins today's values.
pub fn ocr_support(lang: &str) -> OcrSupport {
    match lang {
        EN | ES | FR | DE | PT | PL | VI | KO | JA | ZH | RU | AR => OcrSupport::Native,
        FIL | SW => OcrSupport::EnglishFallback,
        // hi: Devanagari — absent from Vision's measured list, and the English
        // recognizer can't read the script, so the photo feature hides.
        HI => OcrSupport::Unsupported,
        _ => OcrSupport::Unsupported, // unknown/unregistered: fail closed, hide.
    }
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
    // THE web/app language split, and the only place it is expressed. The
    // site ships English; the app ships the registry. Compile-time, so the
    // web bundle cannot be talked into another language by a flag, and there
    // is no `if (isWeb)` anywhere else to drift out of sync with this one.
    if cfg!(feature = "web") && lang != EN {
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
/// Mic-everywhere (Eric's ruling, 2026-07-27): every registered language has a
/// letter lexicon (machine-drafted for the new 13, pending native review), so
/// the config gate opens for all. The mic still only RENDERS where the device
/// has on-device speech recognition for the language (the capabilities gate in
/// spell_aloud::reflect is unchanged).
pub const VOICE_SPELL_LANGS: [&str; 15] =
    [EN, ES, FR, DE, PT, PL, VI, KO, JA, FIL, ZH, RU, AR, SW, HI];

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
    Achievement { id: "timed10", ic: "\u{23F1}", nm: "Beat the Clock", desc: "Reach a 10-chain in Quick Spell." },
    Achievement { id: "cleared", ic: "\u{2728}", nm: "Clean Slate", desc: "Clear all your missed words." },
    Achievement { id: "importer", ic: "\u{1F4E5}", nm: "Own Words", desc: "Import your own word list." },
];

#[cfg(test)]
mod registry_tests {
    use super::*;
    // Production invariant: audit_preview deliberately activates ComingSoon
    // languages so they can be reviewed, so this holds only with the feature off.
    #[cfg(not(feature = "audit_preview"))]
    #[cfg(not(feature = "web"))]
    #[test]
    fn every_registered_language_is_active_in_the_app() {
        // The app ships every language (Eric, 2026-07-31). The web does not,
        // and that is asserted separately below under the `web` feature --
        // deliberately two tests, because one test that changes its
        // expectation based on cfg proves whichever config you happened to run.
        let active: Vec<&str> = BUILTIN_LANGS
            .iter()
            .filter(|(c, _, _, _)| is_active_lang(c))
            .map(|(c, _, _, _)| *c)
            .collect();
        assert_eq!(active.len(), BUILTIN_LANGS.len(), "the app gates no language");
        assert!(active.contains(&"en"));
    }

    /// The site is English-only. Runs under `cargo test --features web`.
    #[cfg(feature = "web")]
    #[test]
    fn the_web_build_is_english_only() {
        let active: Vec<&str> = BUILTIN_LANGS
            .iter()
            .filter(|(c, _, _, _)| is_active_lang(c))
            .map(|(c, _, _, _)| *c)
            .collect();
        assert_eq!(active, vec!["en"], "spellgame.net ships English and nothing else");
        // ...and the OTHER fourteen are still registered, so they render as
        // coming-soon tiles rather than vanishing. The site should say the
        // languages exist; it just does not play them.
        assert_eq!(BUILTIN_LANGS.len(), 15, "the registry itself is unchanged on web");
    }

    /// CC-LINEUP-SWAP: the registry snapshot. Pinning the exact lineup means a
    /// language cannot be added or cut without this test being updated
    /// deliberately.
    #[test]
    fn registry_is_the_swapped_lineup_of_14() {
        // 15 since Hindi's promotion (2026-07-25): the swapped 14 plus hi. The
        // registry is now IDENTICAL in every build config.
        let codes: Vec<&str> = LANGS_BASE.iter().map(|(c, _, _, _)| *c).collect();
        assert_eq!(codes.len(), 15, "the swapped 14 plus Hindi (promoted 2026-07-25)");
        assert_eq!(
            codes,
            vec!["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "zh", "ru", "ar", "sw", "hi"],
        );
        // The cut four (CC-LINEUP-SWAP F1), plus Thai (5fc69ff), Turkish
        // (CC-HINDI-PHASE0 D1 — permanently; Hindi replaces it), and Persian/Urdu
        // (CC-MASTER-PARITY Phase A).
        for gone in ["no", "nb", "sv", "nl", "it", "th", "tr", "fa", "ur"] {
            assert!(!codes.contains(&gone), "{gone} is cut from the registry");
        }
        // One registry, every build config (the audit-only Hindi split is gone).
        assert_eq!(BUILTIN_LANGS.len(), 15);
        assert_eq!(BUILTIN_LANGS[14].0, "hi", "Hindi is the 15th entry everywhere");
    }

    /// CC-PHOTO-IMPORT Phase 0 — the OCR-support snapshot, pinned to the G-B
    /// measurement (iOS 26.5, VisionLanguageMatrixTests, 2026-07-25). A registry
    /// change or a re-measurement must update this test DELIBERATELY.
    #[test]
    fn ocr_support_matches_the_measured_vision_matrix() {
        use OcrSupport::*;
        let expected = [
            ("en", Native), ("es", Native), ("fr", Native), ("de", Native),
            ("pt", Native), ("pl", Native), ("vi", Native), ("ko", Native),
            ("ja", Native), ("fil", EnglishFallback), ("zh", Native),
            ("ru", Native), ("ar", Native), ("sw", EnglishFallback),
            ("hi", Unsupported), // Devanagari: no Vision model (G-B measurement)
        ];
        for (code, want) in expected {
            assert_eq!(ocr_support(code), want, "{code} OCR class drifted from the G-B measurement");
        }
        // Every registered language has an explicit class; unknowns fail closed.
        for (code, _, _, _) in LANGS_BASE.iter() {
            assert!(expected.iter().any(|(c, _)| c == code), "{code} missing from the OCR matrix");
        }
        assert_eq!(ocr_support("xx"), Unsupported, "unregistered languages hide the feature");
    }

    /// CC-PRACTICE D7: open to every registered language (Eric's ruling,
    /// 2026-07-27); unregistered codes never render it.
    /// Mic-everywhere: the server STT rung covers every registered language
    /// (it only ever ACTIVATES when the device has no on-device model, after
    /// consent, outside Kid Mode / education — enforced in spell_aloud).
    #[test]
    fn server_stt_covers_the_lineup() {
        for (code, _, _, _) in BUILTIN_LANGS.iter() {
            assert!(server_stt(code), "{code}");
        }
        assert!(!server_stt("xx"));
    }

    #[test]
    fn practice_open_to_all_registered_languages() {
        for (code, _, _, _) in LANGS_BASE.iter() {
            assert!(practice(code), "{code} practice must be open");
        }
        assert!(!practice("xx"));
    }

    /// CC-DEF-MATCH: the per-language activation snapshot. zh activates first
    /// (its CC-CEDICT pool clears the D3 floor in all four tiers); everything
    /// else stays false until its wiktionary pool build lands and this pin is
    /// updated DELIBERATELY alongside it.
    #[test]
    fn def_match_activation_snapshot() {
        // FULL ACTIVATION (2026-07-27): every registered language's pool
        // clears the floor in >=2 tiers; per-tier floors enforced at fetch.
        for (code, _, _, _) in LANGS_BASE.iter() {
            assert!(def_match(code), "{code} pool cleared the floor at full activation");
        }
        assert!(!def_match("xx"), "unregistered languages never activate");
    }

    /// CC-LINEUP-SWAP D2 — exactly ar is RTL, and (in production) it cannot be
    /// activated while RTL is unsupported. The audit-preview build deliberately
    /// un-gates it; `rtl_gate_matches_the_build_config` below pins that relationship.
    #[cfg(not(feature = "audit_preview"))]
    #[test]
    fn rtl_languages_are_registered_but_hard_gated() {
        let rtl: Vec<&str> = BUILTIN_LANGS.iter().filter(|(_, _, _, d)| *d == Rtl).map(|(c, _, _, _)| *c).collect();
        assert_eq!(rtl, vec!["ar"], "exactly the one RTL language carries the flag");
        assert!(RTL_SUPPORTED, "RTL support ON (Arabic ungated 2026-07-25)");
        for code in rtl {
            assert!(rtl_required(code), "{code} is rtl_required (drives dir/joins)");
            assert!(!rtl_blocked(code), "{code} is no longer blocked — RTL is supported");
            // Two separate gates, and this test owns the render one. Arabic
            // is active in the app (Eric verified the rendering on device
            // 2026-07-25 and ships every language to TestFlight) and inactive
            // on web like everything but English -- so assert the gate this
            // test is about, and let the platform tests own the other.
            assert_eq!(is_active_lang(code), !cfg!(feature = "web"),
                       "{code}: render gate open, platform decides the rest");
        }
        // Russian is an LTR language — it carries no RTL gate at all.
        assert!(!rtl_required("ru"), "ru is left-to-right");
        assert!(!rtl_blocked("ru"));
    }

    /// The RTL gate is permanently ON since the 2026-07-25 ungate — in BOTH build
    /// configs. This test is the deliberate-change tripwire for ever re-gating.
    #[test]
    fn rtl_gate_matches_the_build_config() {
        assert!(RTL_SUPPORTED, "Arabic RENDERING ungated in every build config");
        // Deliberately not asserting is_active_lang here: render support and
        // content audit are different questions and this test owns the first.
        assert!(!rtl_blocked("ar"), "the render gate is open");
    }

    /// Every language must be on ONE side of the shaping question, explicitly.
    ///
    /// The previous version of this test asserted the joined set was exactly
    /// `["ar"]` and then listed fourteen languages that must split — and Hindi
    /// was in NEITHER list. It fell through the gap while Devanagari silently
    /// took the per-letter path, which detaches its matras. So the test now
    /// enumerates the whole lineup and fails if a language is unaccounted for:
    /// adding a language forces a decision here rather than allowing a default.
    #[test]
    fn every_language_declares_whether_its_script_shapes() {
        let joins: Vec<&str> =
            BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).filter(|c| script_joins(c)).collect();
        assert_eq!(joins, vec!["ar", "hi"], "Arabic joins; Devanagari conjoins");

        // Nothing else may take the joined path — splitting is harmless for
        // them and the `pop` animation depends on it.
        let splits = ["en", "es", "fr", "de", "pt", "pl", "tr", "vi", "ko", "ja", "fil", "zh", "ru", "sw"];
        for lang in splits {
            assert!(!script_joins(lang), "{lang} does not shape — it must keep the per-letter path");
        }
        // Russian is the trap: new, non-Latin, and Cyrillic does NOT join.
        assert!(!script_joins("ru"), "Cyrillic is not cursive");

        // The gap-closer: no registered language may be absent from both lists.
        for (code, ..) in BUILTIN_LANGS {
            assert!(
                joins.contains(&code) || splits.contains(&code),
                "{code} is in neither list — decide whether its script shapes"
            );
        }
    }

    // The Spell-Picture-agrees-with-us test deliberately lives in
    // wordpic_screen, not here: I3 forbids shared code from importing the
    // picture subtree, and picture-import-check enforces it. The dependency
    // runs picture -> shared, so the test does too.

    /// CC-RTL F1/D3 — direction lives in the registry, and `dir_attr` is what
    /// surfaces write. Pinned because the failure mode is silent: a wrong `dir`
    /// on Latin is visible instantly; a missing one on Arabic is not.
    #[test]
    fn direction_comes_from_the_registry() {
        for lang in ["ar"] {
            assert_eq!(direction(lang), Rtl, "{lang} reads right-to-left");
            assert_eq!(dir_attr(lang), "rtl");
        }
        for lang in ["en", "es", "ru", "ko", "ja", "zh", "vi", "sw"] {
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

    /// D2's teeth: even if an RTL language is flipped to `Active` in the registry,
    /// it must STILL not activate while RTL is unsupported. The gate lives in
    /// `is_active_lang`, not in the status, so a well-meaning status flip cannot
    /// ship unverified RTL rendering. (Production only — audit_preview lifts the
    /// gate on purpose, pinned by `rtl_gate_matches_the_build_config`.)
    #[cfg(not(feature = "audit_preview"))]
    #[test]
    fn rtl_gate_survives_an_active_status() {
        // The teeth are the IMPLICATION, not any one language's status: a
        // blocked language must never be active, whatever the registry says.
        // Asserting it across the whole lineup keeps the tripwire alive in
        // every future configuration -- including the one where someone
        // promotes an RTL language back to Active while RTL_SUPPORTED is off.
        for (code, _, _, _) in BUILTIN_LANGS {
            assert!(
                !(rtl_blocked(code) && is_active_lang(code)),
                "{code} is rtl_blocked yet active — the status flip beat the gate"
            );
        }
        // And the block derives from the const AND the direction, never from a
        // hand-maintained list that could drift from either.
        for (code, _, _, d) in BUILTIN_LANGS {
            assert_eq!(rtl_blocked(code), d == Rtl && !RTL_SUPPORTED, "{code}: block is not derived");
        }
    }
}
