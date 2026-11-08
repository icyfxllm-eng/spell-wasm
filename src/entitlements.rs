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

/// CC-LAUNCH-EN D10: the English unlock. Raises English to `Full` and nothing
/// else -- it NEVER sets Complete (I4.1). Same education rule as Complete.
#[cfg(not(feature = "education"))]
pub const ENGLISH_PRODUCT_ID: &str = "net.spellgame.english";

/// CC-LAUNCH-EN D5/D10: the English-owner's path to Complete. Owning it IS
/// owning Complete; it exists only so the upgrade costs the difference and no
/// path to Complete costs more than Complete itself.
#[cfg(not(feature = "education"))]
pub const COMPLETE_UPGRADE_PRODUCT_ID: &str = "net.spellgame.complete_upgrade";

/// Every product the purchase adapter asks StoreKit for, in sheet order. The
/// adapter loads exactly these and nothing else, so a product added in App Store
/// Connect but not here can never grant anything.
#[cfg(not(feature = "education"))]
pub const PRODUCT_IDS: [&str; 3] = [ENGLISH_PRODUCT_ID, COMPLETE_PRODUCT_ID, COMPLETE_UPGRADE_PRODUCT_ID];

/// What the player owns, as verified purchases (CC-LAUNCH-EN F4). The purchase
/// adapter builds this from `Transaction.currentEntitlements` via
/// [`owned_from_product_ids`] and from nowhere else (I4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Owned {
    /// `net.spellgame.english`.
    pub english: bool,
    /// `net.spellgame.complete` or `net.spellgame.complete_upgrade`.
    pub complete: bool,
}

impl Owned {
    /// Owns nothing: the free tier.
    pub const NONE: Owned = Owned { english: false, complete: false };
    /// Owns Complete (either route to it).
    pub const COMPLETE: Owned = Owned { english: false, complete: true };
    /// Owns English only.
    pub const ENGLISH: Owned = Owned { english: true, complete: false };
}

/// Map verified product ids onto [`Owned`]. Unknown ids are ignored, so a stray
/// or future product can never raise access by accident.
pub fn owned_from_product_ids(ids: &[&str]) -> Owned {
    let mut o = Owned::NONE;
    #[cfg(not(feature = "education"))]
    for &id in ids {
        match id {
            ENGLISH_PRODUCT_ID => o.english = true,
            COMPLETE_PRODUCT_ID | COMPLETE_UPGRADE_PRODUCT_ID => o.complete = true,
            _ => {}
        }
    }
    #[cfg(feature = "education")]
    let _ = ids;
    o
}

/// CC-LAUNCH-EN D4 / O4, Eric 2026-11-08: Complete is not on sale in this
/// release, because it unlocks nothing beyond English yet. While this is
/// false, no purchase sheet offers net.spellgame.complete or
/// net.spellgame.complete_upgrade, and the "every language we add" line goes
/// with them. App Store Connect status alone is not enough: the sandbox still
/// returns both products. Ownership is unaffected: a restored or
/// family-shared Complete still unlocks everything.
pub const COMPLETE_ON_SALE: bool = false;

/// Which Complete product a purchase sheet may offer (I4.3, D5): the upgrade iff
/// English is owned and Complete is not; full Complete iff neither is owned;
/// nothing once Complete is owned. Never both, so nobody pays twice for English.
#[cfg(not(feature = "education"))]
pub fn complete_offer(o: Owned) -> Option<&'static str> {
    match (o.english, o.complete) {
        (_, true) => None,
        (true, false) => Some(COMPLETE_UPGRADE_PRODUCT_ID),
        (false, false) => Some(COMPLETE_PRODUCT_ID),
    }
}

// CC-MYWORDS-LISTS D6 (Eric, 2026-09-17: "no free limit"): the custom-list cap
// is gone. It counted LISTS -- the very thing dated lists multiply, roughly one
// a week -- it was declared here and enforced nowhere, and organizing saved
// words was never the thing to charge for (the photo feature is gated on its
// own). Any future free limit counts words, and would be its own named thing.

/// Per-language access is a LEVEL, ordered `None < Preview < Full`. The derived
/// `Ord` follows declaration order, so `max` gives union semantics for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessLevel {
    /// Not accessible at all.
    None,
    /// FREE_TIER taster: standard mode, first difficulty tier only (see
    /// [`preview_allows`]). Includes the Daily (D19); excludes The Climb and
    /// ghost racing. Little Speller is outside this map and free.
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
    /// Spell Racing (race your best run).
    GhostRace,
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

}

/// **FREE_TIER** — the canonical baseline entitlement, defined HERE in the core
/// (not assembled in frontend code). This is *the* constant the doctrine refers
/// to; it is a builder fn only because a `BTreeMap` cannot be a `const`.
///
/// It grants:
/// - **`Preview` for every shipped language, English included** — one rule
///   (CC-LAUNCH-EN item 2): Easy tier in standard mode plus the Daily Challenge
///   (D19), see [`preview_allows`]. Little Speller rides outside this map and is
///   free (D1). This reverses the July 2026 "English Full free forever" ruling.
/// - parent-premium entitlements all OFF.
pub fn free_tier() -> EntitlementSet {
    let mut languages = BTreeMap::new();
    for (code, _, _, _) in BUILTIN_LANGS {
        languages.insert(code, AccessLevel::Preview);
    }
    let mut set = EntitlementSet {
        languages,
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

/// **THE resolver.** Union of `FREE_TIER ∪ regional_grants ∪ (ENGLISH if owned)
/// ∪ (COMPLETE if owned)`, resolved to an [`EntitlementSet`].
///
/// Semantics:
/// - **Union only, nothing subtracts.** Purchase never overrides a grant, nor
///   vice-versa; adding any input can only *raise* access (monotonic).
/// - **Per-language: `max` level.** `regional_grants` raise the named languages
///   to `Full` -- never English (D2); owning English raises English alone;
///   owning Complete raises *every* language to `Full`. `max` means a
///   language granted regionally AND owned via purchase is simply `Full` (no
///   double count).
/// - **`audit_override`**: when `true`, resolve EVERYTHING to maximum (all
///   languages `Full`, all parent-premium on, custom lists unlimited) with no
///   purchase surface — Feature 9. It is an explicit input, not a scattered flag.
///
/// `owned` — *verified* purchases (the adapter does verification).
/// `regional_grants` — SpellGame language codes granted Full for the user's
/// region (from [`regional_grants_for_country`]); unknown codes are ignored.
pub fn resolve_entitlements(
    owned: Owned,
    regional_grants: &[&str],
    audit_override: bool,
) -> EntitlementSet {
    resolve_for_edition(crate::consts::EDITION, owned, regional_grants, audit_override)
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
    owned: Owned,
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
            // D2: English is excluded from the regional grant. It is sold on
            // its own; a grant that handed it out would make the English SKU a
            // lie in its home countries.
            if lang == EN {
                continue;
            }
            if let Some(level) = set.languages.get_mut(lang) {
                *level = (*level).max(AccessLevel::Full);
            }
            // A grant for a non-shipped language is silently ignored (defensive;
            // the map CI check guarantees this never happens for the bundled map).
        }

        // COMPLETE purchase: every language to Full + the parent-premium set.
        // Still a union — `max`/OR — so it can only add.
        if owned.english {
            if let Some(level) = set.languages.get_mut(EN) {
                *level = (*level).max(AccessLevel::Full);
            }
        }
        if owned.complete {
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
/// Worth stating plainly: since CC-LAUNCH-EN dropped English to Preview in
/// FREE_TIER, this raises English to Full in education; schools never see a
/// purchase. Everything else is ComingSoon — audit-gated-off — and D3(a) says
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
/// Preview = **standard mode**, difficulty tiers `1..=preview_max_tier(lang)`,
/// plus the **Daily Challenge** (CC-LAUNCH-EN D19: the daily hook stays free).
/// It EXCLUDES The Climb and ghost racing -- those require `Full`.
///
/// `tier` is the 1-based difficulty index (1 = first/easiest tier).
pub fn preview_allows(lang: &str, mode: GameMode, tier: u32) -> bool {
    match mode {
        GameMode::Standard => tier >= 1 && tier <= preview_max_tier(lang),
        // D19: the Daily is free at every level.
        GameMode::Daily => true,
        // Every premium mode is Full-only; Preview never reaches them.
        GameMode::Climb | GameMode::GhostRace => false,
    }
}

/// The four difficulty tiers, easiest first. Index + 1 is the 1-based tier
/// number [`preview_allows`] takes.
pub const TIERS: [&str; 4] = ["easy", "medium", "hard", "expert"];

/// 1-based tier number for a tier name, `None` for anything that is not one of
/// the four difficulty tiers (climb, impossible, a custom source).
pub fn tier_number(tier: &str) -> Option<u32> {
    TIERS.iter().position(|t| *t == tier).map(|i| i as u32 + 1)
}

/// CC-LAUNCH-EN D21 / D24: may this player play `tier` of `lang`? THE question
/// every word-serving surface asks, so no screen keeps its own tier lock.
///
/// - A Little Speller (kid) profile is free at every difficulty (D1, D24).
/// - `Full` opens every tier.
/// - `Preview` opens the tiers [`preview_allows`] gives standard mode, which is Easy.
/// - `None` opens nothing.
///
/// A name that is not a difficulty tier is open at Full only, so an unknown
/// source can never widen Preview.
pub fn tier_open(level: AccessLevel, kid: bool, lang: &str, tier: &str) -> bool {
    if kid {
        return true;
    }
    match level {
        AccessLevel::Full => true,
        AccessLevel::Preview => tier_number(tier).is_some_and(|n| preview_allows(lang, GameMode::Standard, n)),
        AccessLevel::None => false,
    }
}

/// The difficulty tiers [`tier_open`] allows, easiest first. Never empty for a
/// player who can play the language at all, because Preview always keeps Easy.
pub fn open_tiers(level: AccessLevel, kid: bool, lang: &str) -> Vec<&'static str> {
    TIERS.iter().copied().filter(|t| tier_open(level, kid, lang, t)).collect()
}

/// CC-LAUNCH-EN D19 / D24: may this player enter `mode` in `lang` at all? The
/// Daily is open at Preview, while the Climb and ghost racing need Full. A kid
/// profile is never locked out.
pub fn mode_open(level: AccessLevel, kid: bool, lang: &str, mode: GameMode) -> bool {
    if kid {
        return true;
    }
    match level {
        AccessLevel::Full => true,
        AccessLevel::Preview => preview_allows(lang, mode, 1),
        AccessLevel::None => false,
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
        let o = if p { Owned::COMPLETE } else { Owned::NONE };
        resolve_for_edition(Edition::Consumer, o, g, a)
    }

    fn all_langs() -> Vec<&'static str> {
        BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).collect()
    }

    /// Registry languages MINUS the RTL-blocked ones. CC-LINEUP-SWAP D2 puts
    /// ar at `None` for every input and every edition (RTL_SUPPORTED is false in
    /// production until Arabic rendering is verified), so a test asserting
    /// "Full/Preview for every language" must exclude it or it is asserting the
    /// opposite of the gate. `rtl_blocked_langs` covers it explicitly instead.
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
        // CC-LAUNCH-EN item 2: one rule, English included -- every language Preview.
        for lang in resolvable_langs() {
            assert_eq!(got.lang_level(lang), AccessLevel::Preview, "{lang} should be Preview");
        }
        // D2: the FREE_TIER Preview floor does not reach an RTL-blocked language.
        for lang in rtl_blocked_langs() {
            assert_eq!(got.lang_level(lang), AccessLevel::None, "{lang} is RTL-blocked, not Preview");
        }
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
                    // D2: a grant never reaches English.
                    let expected = if purchased || (grants.contains(&lang) && lang != "en") {
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
        assert!(ent.photo_ocr && ent.multiple_profiles && ent.progress_reports);
        // audit ignores purchase surface entirely
        assert_eq!(ent, resolve_consumer(true, &["de"], true));
    }


    // ---- Preview constants / helpers (Feature 11) ----
    #[test]
    fn preview_is_standard_first_tier_plus_daily() {
        for lang in all_langs() {
            assert!(preview_allows(lang, GameMode::Standard, 1), "{lang} tier1 standard ok");
            assert!(!preview_allows(lang, GameMode::Standard, 2), "{lang} tier2 gated");
            // D19: the Daily is free.
            assert!(preview_allows(lang, GameMode::Daily, 1), "{lang} Daily is free");
            // Preview NEVER leaks into premium modes.
            for m in [GameMode::Climb, GameMode::GhostRace] {
                assert!(!preview_allows(lang, m, 1), "{lang} {m:?} must be Full-only");
            }
        }
    }

    // ---- CC-LAUNCH-EN D21 / D24: the shared tier gate ----
    #[test]
    fn tier_gate_matrix() {
        use AccessLevel::*;
        for t in TIERS {
            assert!(tier_open(Full, false, "en", t), "Full opens {t}");
            assert!(tier_open(Preview, true, "en", t), "a kid is free at {t}");
            assert!(tier_open(None, true, "en", t), "a kid is never locked");
            assert!(!tier_open(None, false, "en", t));
        }
        assert!(tier_open(Preview, false, "en", "easy"));
        for t in ["medium", "hard", "expert"] {
            assert!(!tier_open(Preview, false, "en", t), "Preview locks {t}");
        }
        // A non-tier name never widens Preview.
        for t in ["climb", "impossible", "", "mine"] {
            assert!(!tier_open(Preview, false, "en", t), "{t:?}");
            assert!(tier_open(Full, false, "en", t));
        }
        assert_eq!(open_tiers(Preview, false, "en"), vec!["easy"]);
        assert_eq!(open_tiers(Full, false, "en"), TIERS.to_vec());
        assert_eq!(open_tiers(Preview, true, "en"), TIERS.to_vec());
    }

    #[test]
    fn mode_gate_matrix() {
        use AccessLevel::*;
        assert!(mode_open(Preview, false, "en", GameMode::Daily), "D19");
        assert!(mode_open(Preview, false, "en", GameMode::Standard));
        assert!(!mode_open(Preview, false, "en", GameMode::Climb));
        assert!(!mode_open(Preview, false, "en", GameMode::GhostRace));
        assert!(mode_open(Preview, true, "en", GameMode::Climb), "D24");
        assert!(mode_open(Full, false, "en", GameMode::Climb));
    }

    // ---- CC-LAUNCH-EN F4: the three products ----
    fn resolve_owned(o: Owned) -> EntitlementSet {
        resolve_for_edition(Edition::Consumer, o, &[], false)
    }

    /// A4.1 / I4.1: English raises English alone, never Complete.
    #[test]
    fn english_purchase_is_english_only() {
        let o = owned_from_product_ids(&[ENGLISH_PRODUCT_ID]);
        assert_eq!(o, Owned::ENGLISH);
        let ent = resolve_owned(o);
        assert_eq!(ent.lang_level("en"), AccessLevel::Full);
        for lang in resolvable_langs().into_iter().filter(|l| *l != "en") {
            assert_eq!(ent.lang_level(lang), AccessLevel::Preview, "{lang} stays Preview");
        }
        assert!(!ent.photo_ocr && !ent.multiple_profiles && !ent.progress_reports);
    }

    /// A4.2 / A4.3 / I4.2: either route to Complete opens everything.
    #[test]
    fn complete_and_upgrade_are_complete() {
        let full = resolve_owned(Owned::COMPLETE);
        for ids in [
            &[COMPLETE_PRODUCT_ID][..],
            &[ENGLISH_PRODUCT_ID, COMPLETE_UPGRADE_PRODUCT_ID][..],
            &[COMPLETE_UPGRADE_PRODUCT_ID][..],
            &[ENGLISH_PRODUCT_ID, COMPLETE_PRODUCT_ID][..],
        ] {
            let o = owned_from_product_ids(ids);
            assert!(o.complete, "{ids:?}");
            let ent = resolve_owned(o);
            assert_eq!(ent, full, "{ids:?} resolves to Complete");
            assert_eq!(ent.lang_level("en"), AccessLevel::Full);
        }
    }

    /// I4.4: unknown ids add nothing.
    #[test]
    fn unknown_products_grant_nothing() {
        let o = owned_from_product_ids(&["net.spellgame.pro", "", "net.spellgame.complete.x"]);
        assert_eq!(o, Owned::NONE);
        assert_eq!(resolve_owned(o), free_tier());
    }

    /// I4.3 / D5: upgrade iff english and not complete; complete iff neither.
    #[test]
    fn offer_matrix() {
        assert_eq!(complete_offer(Owned::NONE), Some(COMPLETE_PRODUCT_ID));
        assert_eq!(complete_offer(Owned::ENGLISH), Some(COMPLETE_UPGRADE_PRODUCT_ID));
        assert_eq!(complete_offer(Owned::COMPLETE), None);
        assert_eq!(complete_offer(Owned { english: true, complete: true }), None);
    }

    /// D2: English is never in the regional grant, whatever the map says.
    #[test]
    fn grant_never_reaches_english() {
        let ent = resolve_for_edition(Edition::Consumer, Owned::NONE, &["en", "de"], false);
        assert_eq!(ent.lang_level("en"), AccessLevel::Preview);
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
        // CC-HINDI-PHASE0 D1: Turkish is cut permanently and "Turkey becomes
        // unmapped, like India". Both countries now grant nothing — TR because
        // its language left the lineup, IN because Hindi has not entered it.
        assert!(regional_grants_for_country("TR").is_empty(), "D1: Turkey is unmapped");
        // CC-MASTER-PARITY Phase A cut Persian and Urdu; Persian is back (Eric,
        // 2026-10-24), so Iran grants fa again. Pakistan stays unmapped (ur cut).
        // Iran is reachable only via the web CF-IPCountry path -- there is no
        // Iranian App Store for the storefront path to see.
        assert_eq!(regional_grants_for_country("IR"), vec!["fa"], "Iran grants Persian");
        assert!(regional_grants_for_country("PK").is_empty(), "Phase A: Pakistan is unmapped (ur cut)");
        // Hindi entered the lineup 2026-07-25 (Eric's ruling; supersedes the
        // old D6 "India grants nothing"): India and Fiji now grant hi.
        assert_eq!(regional_grants_for_country("IN"), vec!["hi"], "India grants Hindi");
        assert_eq!(regional_grants_for_country("FJ"), vec!["hi"], "Fiji grants Hindi");
        // No cut language has a home country left anywhere in the map.
        for country in ["IT", "NL", "NO", "SE", "SM", "TR", "PK"] {
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

    fn own(complete: bool) -> Owned {
        Owned { english: false, complete }
    }

    // The core invariant: adding ANY input never removes an entitlement.
    proptest! {
        #[test]
        fn union_is_monotonic(
            purchased in any::<bool>(),
            add_purchase in any::<bool>(),
            english in any::<bool>(),
            add_english in any::<bool>(),
            grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
            extra_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6),
        ) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();

            let base = resolve_entitlements(Owned { english, complete: purchased }, &grants, false);

            // A superset of inputs: keep all grants, add more, and possibly turn
            // purchase on. Never remove anything.
            let mut more = grants.clone();
            for &i in &extra_idxs { more.push(pool[i]); }
            let bigger = resolve_entitlements(
                Owned { english: english || add_english, complete: purchased || add_purchase },
                &more,
                false,
            );

            // Every per-language level can only rise.
            for &lang in &pool {
                prop_assert!(bigger.lang_level(lang) >= base.lang_level(lang),
                    "lang {} dropped {:?} -> {:?}", lang, base.lang_level(lang), bigger.lang_level(lang));
            }
            // Every boolean premium can only turn on; the cap can only widen.
            prop_assert!(bigger.photo_ocr >= base.photo_ocr);
            prop_assert!(bigger.multiple_profiles >= base.multiple_profiles);
            prop_assert!(bigger.progress_reports >= base.progress_reports);
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
            let con = resolve_for_edition(Edition::Consumer, own(purchased), &grants, audit);
            let edu = resolve_for_edition(Edition::Education, own(purchased), &grants, audit);
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
            let edu = resolve_for_edition(Edition::Education, own(purchased), &grants, false);
            for &lang in &pool {
                if is_active_lang(lang) {
                    prop_assert_eq!(edu.lang_level(lang), AccessLevel::Full, "{} audit-passed -> Full in education", lang);
                }
            }
            // The parent-premium set is unconditional in education.
            prop_assert!(edu.photo_ocr && edu.multiple_profiles && edu.progress_reports);
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
                resolve_for_edition(Edition::Consumer, own(purchased), &grants, audit),
                resolve_entitlements(own(purchased), &grants, audit),
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
            let con = resolve_for_edition(Edition::Consumer, own(purchased), &grants, audit);
            let edu = resolve_for_edition(Edition::Education, own(purchased), &grants, audit);
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
                resolve_for_edition(Edition::Education, own(purchased), &grants, true),
                resolve_for_edition(Edition::Education, own(purchased), &grants, false),
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
                resolve_for_edition(Edition::Education, own(purchased), &grants, audit),
                resolve_entitlements(own(purchased), &grants, audit),
                "an --features education build IS the education edition",
            );
        }

        fn audit_dominates(purchased in any::<bool>(), grant_idxs in prop::collection::vec(0usize..BUILTIN_LANGS.len(), 0..6)) {
            let pool = lang_pool();
            let grants: Vec<&str> = grant_idxs.iter().map(|&i| pool[i]).collect();
            let audit = resolve_for_edition(Edition::Consumer, own(purchased), &grants, true);
            for &lang in &pool {
                let expected = if crate::consts::rtl_blocked(lang) { AccessLevel::None } else { AccessLevel::Full };
                prop_assert_eq!(audit.lang_level(lang), expected);
            }
        }
    }
}
