//! CC-ENTITLEMENTS — the pure entitlement core (Phase 1).
//!
//! # The single-source-of-truth doctrine
//! Entitlement is answered by ONE pure function — [`resolve_entitlements`] — and
//! *every* feature asks it. There are NO scattered `if(purchased)` /
//! `if(country == …)` checks anywhere else in the app. This module (and, in
//! later phases, the thin StoreKit / Flask **adapters**) are the ONLY places the
//! platform strings `storefront`, `CF-IPCountry`, and the product id are allowed
//! to appear; the CI grep gate `scripts/entitlement-core-purity-check.mjs`
//! enforces that.
//!
//! # Purity
//! Everything here is a pure function of its inputs — NO I/O, NO network, NO
//! storage, NO clock. The adapters (later phases) gather the raw signals
//! (a verified purchase from StoreKit, the request's `CF-IPCountry` header) and
//! hand them in; this core just computes. Deterministic.
//!
//! # The clean interface the adapters will call
//! ```ignore
//! // Purchase adapter (StoreKit) knows `purchased`. Region adapter (Flask/CF)
//! // turns a country code into grants via `regional_grants_for_country`.
//! let grants = entitlements::regional_grants_for_country("CH"); // ["de","fr","it"]
//! let grants: Vec<&str> = grants.iter().map(String::as_str).collect();
//! let ent = entitlements::resolve_entitlements(purchased, &grants, audit_override);
//! if ent.lang_level("de") == entitlements::AccessLevel::Full { /* … */ }
//! ```

use std::collections::BTreeMap;

use crate::consts::{BUILTIN_LANGS, EN};

/// The App Store / Play product id for the one-time **Complete** unlock (parent
/// premium + all languages Full). Defined HERE, in the core, so the literal
/// lives in exactly ONE place; the purchase adapter imports it. Hard-coding this
/// string anywhere else trips the CI grep gate.
pub const COMPLETE_PRODUCT_ID: &str = "net.spellgame.complete";

/// FREE_TIER custom-list cap (Feature 8): free users may keep at most this many
/// custom word lists. The Complete unlock lifts the cap (unlimited).
pub const FREE_CUSTOM_LISTS_CAP: u32 = 2;

/// Per-language access is a LEVEL, ordered `None < Preview < Full`. The derived
/// `Ord` follows declaration order, so `max` gives union semantics for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessLevel {
    /// Not accessible at all.
    None,
    /// FREE_TIER taster: standard mode, first difficulty tier only (see
    /// [`preview_allows`]). Excludes Daily, The Climb, ghost racing, Little
    /// Speller.
    Preview,
    /// Everything for that language: all tiers + all modes.
    Full,
}

/// The game modes the entitlement core distinguishes for Preview gating. The
/// frontend maps its own screen/mode identifiers onto these; the core never
/// reads raw mode strings scattered around the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// Ordinary hear-it/spell-it practice.
    Standard,
    /// Daily Challenge.
    Daily,
    /// The Climb.
    Climb,
    /// Ghost racing (race your best run).
    GhostRace,
    /// Little Speller (young-learner mode).
    LittleSpeller,
}

/// The resolved entitlements for one user, right now. Produced ONLY by
/// [`resolve_entitlements`]. Features read this; they never re-derive access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementSet {
    /// Per-language access level. EVERY shipped language is present and always at
    /// least `Preview` (FREE_TIER floor) — a lookup is total, never missing.
    pub languages: BTreeMap<&'static str, AccessLevel>,
    /// Max number of custom word lists. `Some(n)` = capped at n; `None` =
    /// unlimited (the Complete `custom_lists_unlimited` entitlement).
    pub custom_lists_cap: Option<u32>,
    /// VisionKit photo → word-list OCR (Complete parent-premium).
    pub photo_ocr: bool,
    /// Multiple child profiles (Complete parent-premium).
    pub multiple_profiles: bool,
    /// Progress reports (Complete parent-premium).
    pub progress_reports: bool,
}

impl EntitlementSet {
    /// Access level for `lang` (`None` if it is not a shipped language).
    pub fn lang_level(&self, lang: &str) -> AccessLevel {
        self.languages.get(lang).copied().unwrap_or(AccessLevel::None)
    }

    /// The `custom_lists_unlimited` entitlement (cap lifted).
    pub fn custom_lists_unlimited(&self) -> bool {
        self.custom_lists_cap.is_none()
    }
}

/// **FREE_TIER** — the canonical baseline entitlement, defined HERE in the core
/// (not assembled in frontend code). This is *the* constant the doctrine refers
/// to; it is a builder fn only because a `BTreeMap` cannot be a `const`.
///
/// It grants:
/// - **English at `Full`** — the base free language: standard mode, all
///   difficulties, Daily, The Climb (Little Speller / earn-only shields /
///   achievements / widgets / etymology cards ride along outside this map).
/// - **`Preview` for every OTHER shipped language** — the taster (see
///   [`preview_allows`]).
/// - **custom lists capped at [`FREE_CUSTOM_LISTS_CAP`]**; parent-premium
///   entitlements all OFF.
pub fn free_tier() -> EntitlementSet {
    let mut languages = BTreeMap::new();
    for (code, _, _) in BUILTIN_LANGS {
        let level = if code == EN { AccessLevel::Full } else { AccessLevel::Preview };
        languages.insert(code, level);
    }
    EntitlementSet {
        languages,
        custom_lists_cap: Some(FREE_CUSTOM_LISTS_CAP),
        photo_ocr: false,
        multiple_profiles: false,
        progress_reports: false,
    }
}

/// **THE resolver.** Union of `FREE_TIER ∪ regional_grants ∪ (COMPLETE if
/// purchased)`, resolved to an [`EntitlementSet`].
///
/// Semantics:
/// - **Union only, nothing subtracts.** Purchase never overrides a grant, nor
///   vice-versa; adding any input can only *raise* access (monotonic).
/// - **Per-language: `max` level.** `regional_grants` raise the named languages
///   to `Full`; `purchased` raises *every* language to `Full`. `max` means a
///   language granted regionally AND owned via purchase is simply `Full` (no
///   double count).
/// - **`audit_override`**: when `true`, resolve EVERYTHING to maximum (all
///   languages `Full`, all parent-premium on, custom lists unlimited) with no
///   purchase surface — Feature 9. It is an explicit input, not a scattered flag.
///
/// `purchased` — a *verified* Complete purchase (the adapter does verification).
/// `regional_grants` — SpellGame language codes granted Full for the user's
/// region (from [`regional_grants_for_country`]); unknown codes are ignored.
pub fn resolve_entitlements(
    purchased: bool,
    regional_grants: &[&str],
    audit_override: bool,
) -> EntitlementSet {
    // Feature 9: audit builds see the maximum, full stop — no purchase surface,
    // no per-flag branching downstream.
    if audit_override {
        return grant_everything();
    }

    let mut set = free_tier();

    // Regional grants raise the named languages to Full (union via max).
    for &lang in regional_grants {
        if let Some(level) = set.languages.get_mut(lang) {
            *level = (*level).max(AccessLevel::Full);
        }
        // A grant for a non-shipped language is silently ignored (defensive; the
        // map CI check guarantees this never happens for the bundled map).
    }

    // COMPLETE purchase: every language to Full + the parent-premium set. Still
    // a union — `max`/OR — so it can only add.
    if purchased {
        raise_to_complete(&mut set);
    }

    set
}

/// Raise a set to the full COMPLETE entitlement (all langs Full + parent
/// premium). Shared by `purchased` and (via `grant_everything`) audit.
fn raise_to_complete(set: &mut EntitlementSet) {
    for level in set.languages.values_mut() {
        *level = (*level).max(AccessLevel::Full);
    }
    set.custom_lists_cap = None; // unlimited
    set.photo_ocr = true;
    set.multiple_profiles = true;
    set.progress_reports = true;
}

/// The audit maximum: FREE_TIER raised to COMPLETE across the board.
fn grant_everything() -> EntitlementSet {
    let mut set = free_tier();
    raise_to_complete(&mut set);
    set
}

// ---------------------------------------------------------------------------
// Feature 11 — Preview constants + helpers (in the core, alongside FREE_TIER)
// ---------------------------------------------------------------------------

/// Default Preview depth: the first difficulty tier only (1-based).
pub const DEFAULT_PREVIEW_TIER: u32 = 1;

/// Per-language Preview-depth overrides (1-based deepest tier the FREE_TIER
/// Preview unlocks). DEFAULT is [`DEFAULT_PREVIEW_TIER`] for every language;
/// raising a language (East-Asian tier structures differ) is a ONE-LINE data
/// change here — e.g. `("zh", 2)`. Empty today (everything defaults to tier 1).
const PREVIEW_TIER_OVERRIDES: &[(&str, u32)] = &[
    // ("zh", 1), ("ja", 1), ("ko", 1),  // ← one-line override when settled.
];

/// Deepest difficulty tier (1-based) FREE_TIER Preview unlocks for `lang`.
pub fn preview_max_tier(lang: &str) -> u32 {
    PREVIEW_TIER_OVERRIDES
        .iter()
        .find(|(c, _)| *c == lang)
        .map(|(_, t)| *t)
        .unwrap_or(DEFAULT_PREVIEW_TIER)
}

/// Whether FREE_TIER **Preview** allows `(lang, mode, tier)` — the ONE helper the
/// frontend calls so there is no per-screen access logic.
///
/// Preview = **standard mode**, difficulty tiers `1..=preview_max_tier(lang)`.
/// It EXCLUDES Daily Challenge, The Climb, ghost racing, and Little Speller —
/// those require `Full` for that language, so Preview always denies them.
///
/// `tier` is the 1-based difficulty index (1 = first/easiest tier).
pub fn preview_allows(lang: &str, mode: GameMode, tier: u32) -> bool {
    match mode {
        GameMode::Standard => tier >= 1 && tier <= preview_max_tier(lang),
        // Every premium mode is Full-only; Preview never reaches them.
        GameMode::Daily
        | GameMode::Climb
        | GameMode::GhostRace
        | GameMode::LittleSpeller => false,
    }
}

// ---------------------------------------------------------------------------
// Feature 2 — country → language map (single source of truth, bundled)
// ---------------------------------------------------------------------------

/// The bundled country→language grant map. THE single source of truth — the
/// Flask backend reads the very same file (never a second copy). Sorted by key.
const COUNTRY_LANGUAGE_MAP_JSON: &str = include_str!("../config/country-language-map.json");

/// Parse the bundled map: ISO 3166-1 alpha-2 country code → granted lang codes.
/// Panics only if the bundled JSON is malformed, which the CI map check prevents.
pub fn country_language_map() -> BTreeMap<String, Vec<String>> {
    serde_json::from_str(COUNTRY_LANGUAGE_MAP_JSON)
        .expect("bundled config/country-language-map.json must be valid JSON")
}

/// The FREE Full-language grants for a country code (empty if the country is not
/// in the map). This is the region adapter's entry point: it turns the country
/// (which the adapter derived from `CF-IPCountry`) into the `regional_grants`
/// slice for [`resolve_entitlements`].
pub fn regional_grants_for_country(country: &str) -> Vec<String> {
    country_language_map().get(country).cloned().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests — acceptance 1, 2, 7a (core parts)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::BUILTIN_LANGS;

    fn all_langs() -> Vec<&'static str> {
        BUILTIN_LANGS.iter().map(|(c, _, _)| *c).collect()
    }

    // ---- Acceptance 1: no purchase + no grants == exactly FREE_TIER ----
    #[test]
    fn no_purchase_no_grants_is_free_tier() {
        let got = resolve_entitlements(false, &[], false);
        assert_eq!(got, free_tier());
        // English Full, every other language Preview.
        assert_eq!(got.lang_level("en"), AccessLevel::Full);
        for lang in all_langs() {
            if lang != "en" {
                assert_eq!(got.lang_level(lang), AccessLevel::Preview, "{lang} should be Preview");
            }
        }
        assert_eq!(got.custom_lists_cap, Some(FREE_CUSTOM_LISTS_CAP));
        assert!(!got.photo_ocr && !got.multiple_profiles && !got.progress_reports);
    }

    // ---- Acceptance 2: union holds for every {purchase, grants} combo ----
    #[test]
    fn union_over_all_combinations() {
        let langs = all_langs();
        for &purchased in &[false, true] {
            // try each single-language grant, plus none, plus a multi grant
            let mut grant_cases: Vec<Vec<&str>> = vec![vec![], vec!["de", "fr", "it"]];
            for l in &langs {
                grant_cases.push(vec![*l]);
            }
            for grants in &grant_cases {
                let ent = resolve_entitlements(purchased, grants, false);
                for &lang in &langs {
                    let expected = if purchased || grants.contains(&lang) || lang == "en" {
                        AccessLevel::Full
                    } else {
                        AccessLevel::Preview
                    };
                    assert_eq!(
                        ent.lang_level(lang), expected,
                        "lang={lang} purchased={purchased} grants={grants:?}"
                    );
                }
                // parent-premium follows purchase only
                assert_eq!(ent.photo_ocr, purchased);
                assert_eq!(ent.multiple_profiles, purchased);
                assert_eq!(ent.progress_reports, purchased);
                assert_eq!(ent.custom_lists_unlimited(), purchased);
            }
        }
    }

    // granted lang + purchase == still Full, no double count
    #[test]
    fn grant_plus_purchase_is_just_full() {
        let a = resolve_entitlements(true, &["de"], false);
        let b = resolve_entitlements(true, &[], false);
        assert_eq!(a, b, "a regional grant adds nothing on top of a purchase");
        assert_eq!(a.lang_level("de"), AccessLevel::Full);
    }

    // ---- audit override == ALL ----
    #[test]
    fn audit_override_grants_everything() {
        let ent = resolve_entitlements(false, &[], true);
        for lang in all_langs() {
            assert_eq!(ent.lang_level(lang), AccessLevel::Full, "{lang} Full under audit");
        }
        assert!(ent.custom_lists_unlimited());
        assert!(ent.photo_ocr && ent.multiple_profiles && ent.progress_reports);
        // audit ignores purchase surface entirely
        assert_eq!(ent, resolve_entitlements(true, &["de"], true));
    }

    #[test]
    fn custom_list_cap_two_free_unlimited_with_complete() {
        assert_eq!(resolve_entitlements(false, &[], false).custom_lists_cap, Some(2));
        assert_eq!(resolve_entitlements(true, &[], false).custom_lists_cap, None);
    }

    // ---- Preview constants / helpers (Feature 11) ----
    #[test]
    fn preview_is_standard_first_tier_only() {
        for lang in all_langs() {
            assert!(preview_allows(lang, GameMode::Standard, 1), "{lang} tier1 standard ok");
            assert!(!preview_allows(lang, GameMode::Standard, 2), "{lang} tier2 gated");
            // Preview NEVER leaks into premium modes.
            for m in [GameMode::Daily, GameMode::Climb, GameMode::GhostRace, GameMode::LittleSpeller] {
                assert!(!preview_allows(lang, m, 1), "{lang} {m:?} must be Full-only");
            }
        }
    }

    #[test]
    fn preview_max_level_property() {
        use AccessLevel::*;
        // max(Preview, Full) == Full per the Ord.
        assert_eq!(Preview.max(Full), Full);
        assert_eq!(None.max(Preview), Preview);
        assert!(None < Preview && Preview < Full);
    }

    // ---- country map sanity (Rust side; the CI script is the real gate) ----
    #[test]
    fn country_map_grants_only_shipped_langs_and_covers_non_english() {
        let map = country_language_map();
        let shipped: std::collections::HashSet<&str> = all_langs().into_iter().collect();
        let mut seen = std::collections::HashSet::new();
        for (country, langs) in &map {
            assert_eq!(country.len(), 2, "country code {country} must be alpha-2");
            for l in langs {
                assert!(shipped.contains(l.as_str()), "{country} grants unshipped {l}");
                seen.insert(l.clone());
            }
        }
        for lang in all_langs() {
            if lang != "en" {
                assert!(seen.contains(lang), "non-English {lang} has no home country (bug)");
            }
        }
        // known anchors
        assert_eq!(regional_grants_for_country("CH"), vec!["de", "fr", "it"]);
        assert_eq!(regional_grants_for_country("KR"), vec!["ko"]);
        assert!(regional_grants_for_country("ZZ").is_empty());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::consts::BUILTIN_LANGS;
    use proptest::prelude::*;

    fn lang_pool() -> Vec<&'static str> {
        BUILTIN_LANGS.iter().map(|(c, _, _)| *c).collect()
    }

    // The core invariant: adding ANY input never removes an entitlement.
    proptest! {
        #[test]
        fn union_is_monotonic(
            purchased in any::<bool>(),
            add_purchase in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
            extra_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();

            let base = resolve_entitlements(purchased, &grants, false);

            // A superset of inputs: keep all grants, add more, and possibly turn
            // purchase on. Never remove anything.
            let mut more = grants.clone();
            for &i in &extra_idxs { more.push(pool[i]); }
            let bigger = resolve_entitlements(purchased || add_purchase, &more, false);

            // Every per-language level can only rise.
            for &lang in &pool {
                prop_assert!(bigger.lang_level(lang) >= base.lang_level(lang),
                    "lang {} dropped {:?} -> {:?}", lang, base.lang_level(lang), bigger.lang_level(lang));
            }
            // Every boolean premium can only turn on; the cap can only widen.
            prop_assert!(bigger.photo_ocr >= base.photo_ocr);
            prop_assert!(bigger.multiple_profiles >= base.multiple_profiles);
            prop_assert!(bigger.progress_reports >= base.progress_reports);
            // unlimited (None) is "more" than any Some(n); model as bool.
            prop_assert!(bigger.custom_lists_unlimited() >= base.custom_lists_unlimited());
        }
    }

    // Audit is always the top element — nothing exceeds it, everything is Full.
    proptest! {
        #[test]
        fn audit_dominates(purchased in any::<bool>(), grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6)) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let audit = resolve_entitlements(purchased, &grants, true);
            for &lang in &pool {
                prop_assert_eq!(audit.lang_level(lang), AccessLevel::Full);
            }
            prop_assert!(audit.custom_lists_unlimited());
        }
    }
}
