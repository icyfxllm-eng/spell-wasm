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
//! let grants = entitlements::regional_grants_for_country("CH"); // ["de","fr"]
//! let grants: Vec<&str> = grants.iter().map(String::as_str).collect();
//! let ent = entitlements::resolve_entitlements(purchased, &grants, audit_override);
//! if ent.lang_level("de") == entitlements::AccessLevel::Full { /* … */ }
//! ```

use std::collections::BTreeMap;

use crate::consts::{Edition, BUILTIN_LANGS, EN};

/// The App Store / Play product id for the one-time **Complete** unlock (parent
/// premium + all languages Full). Defined HERE, in the core, so the literal
/// lives in exactly ONE place; the purchase adapter imports it. Hard-coding this
/// string anywhere else trips the CI grep gate.
///
/// **Absent from education builds** (CC-EDITIONS D4). D4 asks for zero purchase
/// surfaces and F3 for "excluded at build time (not stubbed)", so the product id
/// is not merely unused in an education binary — it is not compiled into it. A
/// `cfg` rather than a runtime check is the difference between "a school's app
/// does not show a purchase" and "a school's app does not CONTAIN one", and only
/// the second survives someone running `strings` on the artifact.
///
/// Any future purchase adapter referencing this must itself be
/// `#[cfg(not(feature = "education"))]` — in education this const does not exist
/// and the reference will fail to compile. That build error is the feature
/// working, not a bug to route around.
#[cfg(not(feature = "education"))]
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
    /// Per-language access level. EVERY shipped language is present — a lookup is
    /// total, never missing.
    ///
    /// The FREE_TIER floor is `Preview` with ONE exception: a language blocked on
    /// RTL support resolves to `None` in every edition and for every input
    /// (CC-LINEUP-SWAP D2). It is not "free at a lower tier", it is unreachable,
    /// and saying `Preview` would be a lie a caller could act on.
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
    for (code, _, _, _) in BUILTIN_LANGS {
        let level = if code == EN { AccessLevel::Full } else { AccessLevel::Preview };
        languages.insert(code, level);
    }
    let mut set = EntitlementSet {
        languages,
        custom_lists_cap: Some(FREE_CUSTOM_LISTS_CAP),
        photo_ocr: false,
        multiple_profiles: false,
        progress_reports: false,
    };
    // The baseline must not claim something the resolver will refuse. `free_tier`
    // is public and documented as THE canonical baseline, so leaving ar/fa/ur at
    // Preview here would make the constant itself lie about a language nobody can
    // reach. Clamped at the source; `resolve_for_edition` clamps again at the end
    // (idempotent) because `raise_to_complete` runs in between and would otherwise
    // lift these back to Full.
    clamp_rtl_blocked(&mut set);
    set
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
    resolve_for_edition(crate::consts::EDITION, purchased, regional_grants, audit_override)
}

/// The resolver with the edition injected (CC-EDITIONS F2).
///
/// [`resolve_entitlements`] is this with the build's own [`crate::consts::EDITION`],
/// which is the only way production ever calls it — the edition is a compile-time
/// fact (D2), not a caller's choice. The parameter exists so tests can exercise
/// BOTH editions from one test binary; without it, a property test could only
/// ever check the edition it happened to be compiled as, which is no check at all.
pub fn resolve_for_edition(
    edition: Edition,
    purchased: bool,
    regional_grants: &[&str],
    audit_override: bool,
) -> EntitlementSet {
    // D6: the audit bypass does not exist in an education build. Schools get the
    // real gates, not the reviewer's skeleton key. Applied here rather than at
    // the call site so no future adapter can hand education an override.
    let audit_override = audit_override && edition == Edition::Consumer;

    let mut set = if audit_override {
        // Feature 9: audit builds see the maximum, full stop — no purchase
        // surface, no per-flag branching downstream.
        grant_everything()
    } else {
        let mut set = free_tier();

        // Regional grants raise the named languages to Full (union via max).
        for &lang in regional_grants {
            if let Some(level) = set.languages.get_mut(lang) {
                *level = (*level).max(AccessLevel::Full);
            }
            // A grant for a non-shipped language is silently ignored (defensive;
            // the map CI check guarantees this never happens for the bundled map).
        }

        // COMPLETE purchase: every language to Full + the parent-premium set.
        // Still a union — `max`/OR — so it can only add.
        if purchased {
            raise_to_complete(&mut set);
        }
        set
    };

    // CC-EDITIONS D3: education is ONE MORE GRANT SOURCE into the union, never a
    // bypass. It raises audit-PASSED languages to Full and turns the
    // parent-premium set on; it cannot reach a language the audit gate holds
    // shut, because it goes through `is_active_lang` exactly like every other
    // surface.
    if edition == Edition::Education {
        apply_education_grant(&mut set);
    }

    // CC-LINEUP-SWAP D2, enforced last so nothing above can outrank it: an RTL
    // language is not merely un-granted, it resolves to `None`. Last means
    // `grant_everything`, a purchase, a regional grant and the education grant
    // all lose to it — which is what "cannot be activated by ANY code path
    // (including AUDIT_MODE)" has to mean if it means anything.
    clamp_rtl_blocked(&mut set);

    set
}

/// D3 education semantics: every AUDIT-PASSED language to Full, the whole
/// parent-premium set on.
///
/// The audit gate is consulted through [`crate::consts::is_active_lang`], the
/// same chokepoint every other surface reads, so D3(a) ("editions NEVER bypass
/// audit gates") and D3(b) (rtlRequired stays blocked) hold by construction
/// rather than by a second list that could drift out of step.
///
/// Worth stating plainly, because it surprises: TODAY this changes no language
/// level at all. English is the only audit-passed language and FREE_TIER already
/// gives it Full, so education's language grants are byte-identical to
/// consumer's. Everything else is ComingSoon — audit-gated-off — and D3(a) says
/// those stay exactly as in consumer. What education actually buys today is the
/// parent-premium set and the absence of purchase surfaces. That is not a bug in
/// this function; it is what "editions never bypass audit gates" MEANS while
/// only one language has passed audit.
fn apply_education_grant(set: &mut EntitlementSet) {
    for (code, level) in set.languages.iter_mut() {
        if crate::consts::is_active_lang(code) {
            *level = (*level).max(AccessLevel::Full);
        }
        // Audit-gated-off: left exactly as consumer resolved it (D3(a)).
    }
    set.custom_lists_cap = None; // unlimited
    set.photo_ocr = true;
    set.multiple_profiles = true;
    set.progress_reports = true;
}

/// CC-LINEUP-SWAP D2 as a resolver-level clamp: a language blocked on RTL
/// support resolves to `None` in every edition, for every input.
fn clamp_rtl_blocked(set: &mut EntitlementSet) {
    for (code, level) in set.languages.iter_mut() {
        if crate::consts::rtl_blocked(code) {
            *level = AccessLevel::None;
        }
    }
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
    use crate::consts::{Edition, BUILTIN_LANGS};

    /// Consumer-edition resolve. These tests pin CONSUMER semantics, so they say
    /// so explicitly — inheriting the build's edition would make them silently
    /// assert nothing in an education build.
    fn resolve_consumer(p: bool, g: &[&str], a: bool) -> EntitlementSet {
        resolve_for_edition(Edition::Consumer, p, g, a)
    }

    fn all_langs() -> Vec<&'static str> {
        BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).collect()
    }

    /// Registry languages MINUS the RTL-blocked ones. CC-LINEUP-SWAP D2 puts
    /// ar/fa/ur at `None` for every input and every edition, so a test asserting
    /// "Full/Preview for every language" must exclude them or it is asserting the
    /// opposite of the gate. `rtl_blocked_langs` covers them explicitly instead.
    fn resolvable_langs() -> Vec<&'static str> {
        all_langs().into_iter().filter(|l| !crate::consts::rtl_blocked(l)).collect()
    }

    fn rtl_blocked_langs() -> Vec<&'static str> {
        all_langs().into_iter().filter(|l| crate::consts::rtl_blocked(l)).collect()
    }

    // ---- Acceptance 1: no purchase + no grants == exactly FREE_TIER ----
    #[test]
    fn no_purchase_no_grants_is_free_tier() {
        let got = resolve_consumer(false, &[], false);
        assert_eq!(got, free_tier());
        // English Full, every other language Preview.
        assert_eq!(got.lang_level("en"), AccessLevel::Full);
        for lang in resolvable_langs() {
            if lang != "en" {
                assert_eq!(got.lang_level(lang), AccessLevel::Preview, "{lang} should be Preview");
            }
        }
        // D2: the FREE_TIER Preview floor does not reach an RTL-blocked language.
        for lang in rtl_blocked_langs() {
            assert_eq!(got.lang_level(lang), AccessLevel::None, "{lang} is RTL-blocked, not Preview");
        }
        assert_eq!(got.custom_lists_cap, Some(FREE_CUSTOM_LISTS_CAP));
        assert!(!got.photo_ocr && !got.multiple_profiles && !got.progress_reports);
    }

    // ---- Acceptance 2: union holds for every {purchase, grants} combo ----
    #[test]
    fn union_over_all_combinations() {
        let langs = resolvable_langs();
        for &purchased in &[false, true] {
            // try each single-language grant, plus none, plus a multi grant
            let mut grant_cases: Vec<Vec<&str>> = vec![vec![], vec!["de", "fr", "ru"]];
            for l in &langs {
                grant_cases.push(vec![*l]);
            }
            for grants in &grant_cases {
                let ent = resolve_consumer(purchased, grants, false);
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
        let a = resolve_consumer(true, &["de"], false);
        let b = resolve_consumer(true, &[], false);
        assert_eq!(a, b, "a regional grant adds nothing on top of a purchase");
        assert_eq!(a.lang_level("de"), AccessLevel::Full);
    }

    // ---- audit override == ALL ----
    #[test]
    fn audit_override_grants_everything() {
        let ent = resolve_consumer(false, &[], true);
        for lang in resolvable_langs() {
            assert_eq!(ent.lang_level(lang), AccessLevel::Full, "{lang} Full under audit");
        }
        // CC-LINEUP-SWAP D2: "cannot be activated by ANY code path (including
        // AUDIT_MODE)". The audit bypass is the strongest input the resolver has,
        // so this is the assertion that gives the RTL gate teeth.
        for lang in rtl_blocked_langs() {
            assert_eq!(ent.lang_level(lang), AccessLevel::None, "{lang} must stay blocked even under AUDIT_MODE");
        }
        assert!(ent.custom_lists_unlimited());
        assert!(ent.photo_ocr && ent.multiple_profiles && ent.progress_reports);
        // audit ignores purchase surface entirely
        assert_eq!(ent, resolve_consumer(true, &["de"], true));
    }

    #[test]
    fn custom_list_cap_two_free_unlimited_with_complete() {
        assert_eq!(resolve_consumer(false, &[], false).custom_lists_cap, Some(2));
        assert_eq!(resolve_consumer(true, &[], false).custom_lists_cap, None);
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
    // Production only: the regional-grant map is a shipping-entitlements concept.
    // audit_preview adds review-only languages (Hindi) that have no home country
    // and take part in no grant, so "every non-en lang has a country" is a
    // not-audit_preview invariant.
    #[cfg(not(feature = "audit_preview"))]
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
        assert_eq!(regional_grants_for_country("CH"), vec!["de", "fr"], "it left the lineup");
        assert_eq!(regional_grants_for_country("KR"), vec!["ko"]);
        assert!(regional_grants_for_country("ZZ").is_empty());

        // CC-LINEUP-SWAP D6 anchors.
        assert_eq!(regional_grants_for_country("RU"), vec!["ru"]);
        assert_eq!(regional_grants_for_country("KZ"), vec!["ru"], "ru is official in Kazakhstan");
        assert_eq!(regional_grants_for_country("EG"), vec!["ar"]);
        assert_eq!(regional_grants_for_country("PK"), vec!["ur"]);
        // CC-HINDI-PHASE0 D1: Turkish is cut permanently and "Turkey becomes
        // unmapped, like India". Both countries now grant nothing — TR because
        // its language left the lineup, IN because Hindi has not entered it.
        assert!(regional_grants_for_country("TR").is_empty(), "D1: Turkey is unmapped");
        // Iran: granted in the map, but reachable only via the web CF-IPCountry
        // path — there is no Iranian App Store for the storefront path to see.
        assert_eq!(regional_grants_for_country("IR"), vec!["fa"]);
        // D6: India maps to NOTHING (Hindi isn't in the lineup; Urdu would be
        // wrong for most of its users). CC-HINDI-PHASE0 D2 names hi-IN as the
        // future variant, but D8 grants this file zero authority to register it.
        assert!(regional_grants_for_country("IN").is_empty(), "D6: India grants nothing");
        // No cut language has a home country left anywhere in the map.
        for country in ["IT", "NL", "NO", "SE", "SM", "TR"] {
            assert!(
                regional_grants_for_country(country).is_empty(),
                "{country} was a home country for a cut language",
            );
        }
        assert_eq!(regional_grants_for_country("BE"), vec!["fr"], "nl left BE");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::consts::{is_active_lang, rtl_blocked, Edition, BUILTIN_LANGS};
    use proptest::prelude::*;

    fn lang_pool() -> Vec<&'static str> {
        BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).collect()
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
        /// CC-EDITIONS D3(a) — education NEVER exceeds consumer for a language the
        /// audit gate holds shut, or for an rtlRequired one. This is the property
        /// that makes "editions never bypass audit gates" a fact rather than a
        /// promise: whatever the inputs, an unaudited language resolves the same
        /// in a school's binary as in the App Store's.
        #[test]
        fn edu_never_exceeds_consumer_for_gated_langs(
            purchased in any::<bool>(),
            audit in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let con = resolve_for_edition(Edition::Consumer, purchased, &grants, audit);
            let edu = resolve_for_edition(Edition::Education, purchased, &grants, audit);
            for &lang in &pool {
                if !is_active_lang(lang) || rtl_blocked(lang) {
                    prop_assert!(
                        edu.lang_level(lang) <= con.lang_level(lang),
                        "{} is gated but education ({:?}) exceeded consumer ({:?})",
                        lang, edu.lang_level(lang), con.lang_level(lang),
                    );
                }
            }
        }

        /// CC-EDITIONS D3(b) — education is FULL for every audit-passed language,
        /// whatever the purchase/grant/audit inputs.
        #[test]
        fn edu_is_full_for_every_audit_passed_lang(
            purchased in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let edu = resolve_for_edition(Edition::Education, purchased, &grants, false);
            for &lang in &pool {
                if is_active_lang(lang) {
                    prop_assert_eq!(edu.lang_level(lang), AccessLevel::Full, "{} audit-passed -> Full in education", lang);
                }
            }
            // The parent-premium set is unconditional in education.
            prop_assert!(edu.photo_ocr && edu.multiple_profiles && edu.progress_reports);
            prop_assert!(edu.custom_lists_unlimited());
        }

        /// CC-EDITIONS D3(c) — consumer union semantics are untouched by the
        /// edition axis existing. The default build must be byte-for-byte what it
        /// was before this feature landed.
        ///
        /// Also the WIRING test: it proves `resolve_entitlements` resolves through
        /// the build's own EDITION, and that the default build is Consumer. That
        /// makes it edition-specific by nature, hence the cfg — the education
        /// build has its own counterpart below.
        #[cfg(not(feature = "education"))]
        #[test]
        fn consumer_semantics_unchanged(
            purchased in any::<bool>(),
            audit in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            prop_assert_eq!(
                resolve_for_edition(Edition::Consumer, purchased, &grants, audit),
                resolve_entitlements(purchased, &grants, audit),
                "the default build IS the consumer edition",
            );
        }

        /// CC-EDITIONS done-criterion — an rtlRequired language resolves
        /// IDENTICALLY (blocked) in both editions, for every input.
        #[test]
        fn rtl_langs_resolve_identically_blocked_in_both_editions(
            purchased in any::<bool>(),
            audit in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let con = resolve_for_edition(Edition::Consumer, purchased, &grants, audit);
            let edu = resolve_for_edition(Edition::Education, purchased, &grants, audit);
            for &lang in pool.iter().filter(|l| rtl_blocked(l)) {
                prop_assert_eq!(con.lang_level(lang), AccessLevel::None, "{} blocked in consumer", lang);
                prop_assert_eq!(edu.lang_level(lang), AccessLevel::None, "{} blocked in education", lang);
            }
        }

        /// CC-EDITIONS D6 — AUDIT_MODE does not operate in an education build.
        /// Schools get the real gates, not the reviewer's skeleton key.
        #[test]
        fn audit_bypass_is_inert_in_education(
            purchased in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            prop_assert_eq!(
                resolve_for_edition(Edition::Education, purchased, &grants, true),
                resolve_for_edition(Edition::Education, purchased, &grants, false),
                "audit_override must change nothing in education",
            );
        }

        /// The education build's wiring counterpart: `resolve_entitlements` must
        /// resolve through Edition::Education when compiled with the feature. If
        /// this ever passed in a consumer build, the feature flag would be inert.
        #[cfg(feature = "education")]
        #[test]
        fn education_build_wires_the_education_edition(
            purchased in any::<bool>(),
            audit in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            prop_assert_eq!(
                resolve_for_edition(Edition::Education, purchased, &grants, audit),
                resolve_entitlements(purchased, &grants, audit),
                "an --features education build IS the education edition",
            );
        }

        fn audit_dominates(purchased in any::<bool>(), grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6)) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let audit = resolve_for_edition(Edition::Consumer, purchased, &grants, true);
            for &lang in &pool {
                let expected = if crate::consts::rtl_blocked(lang) { AccessLevel::None } else { AccessLevel::Full };
                prop_assert_eq!(audit.lang_level(lang), expected);
            }
            prop_assert!(audit.custom_lists_unlimited());
        }
    }
}
