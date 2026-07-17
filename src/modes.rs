//! CC-MODE-HUB F1 — the mode registry loader + the Play hub's visibility rule.
//!
//! `config/modes.json` is the single source of truth for what a mode IS; this
//! module is the only thing that reads it. Surfaces ask [`visible`] what to
//! render and never re-derive the answer, so "deleting a registry entry removes
//! the mode from every surface with zero code changes" (A1.1) holds because there
//! is exactly one place that could disagree.
//!
//! # Absence, not locks
//! The hub NEVER renders a lock or an upsell. A mode the player cannot use is
//! simply not there. That is the Little Speller zero-purchase-surface doctrine
//! generalized (A2.2/A2.3), and it is why [`visible`] returns a filtered list
//! rather than a list of (mode, enabled) pairs — the type makes the wrong thing
//! unrepresentable.
//!
//! # Purity
//! [`visible`] is a pure function of the registry and a [`HubCtx`]. It reads no
//! DOM, no storage, no clock — the caller gathers the context and this decides,
//! so every rule below is unit-testable without a browser.

use serde::Deserialize;

use crate::entitlements::AccessLevel;

/// Whether a mode appears, teases, or is invisible.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// A tappable tile.
    Live,
    /// A non-tappable teaser (D7: no notify-me hook in v1).
    ComingSoon,
    /// Renders nowhere, for anyone.
    Hidden,
}

/// One registry entry. Mirrors `config/modes.json`; see that file for what each
/// field means and why it holds the value it does.
#[derive(Deserialize, Debug, Clone)]
pub struct Mode {
    pub id: String,
    #[serde(rename = "nameKey")]
    pub name_key: String,
    #[serde(rename = "descKey")]
    pub desc_key: String,
    pub icon: String,
    pub status: Status,
    #[serde(rename = "kidSafe")]
    pub kid_safe: bool,
    pub platforms: Vec<String>,
    #[serde(rename = "entitlementLevel")]
    pub entitlement_level: Level,
    #[serde(rename = "requiresPremium")]
    pub requires_premium: Option<String>,
    pub languages: Option<Vec<String>>,
}

/// The registry's spelling of [`AccessLevel`]. Separate so the JSON schema and
/// the resolver's enum can evolve independently; [`Level::as_access`] is the one
/// place they meet.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    None,
    Preview,
    Full,
}

impl Level {
    pub fn as_access(self) -> AccessLevel {
        match self {
            Level::None => AccessLevel::None,
            Level::Preview => AccessLevel::Preview,
            Level::Full => AccessLevel::Full,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Registry {
    modes: Vec<Mode>,
}

const MODES_JSON: &str = include_str!("../config/modes.json");

/// The bundled registry, in FILE ORDER — which is hub tile order (D6,
/// RECONSTRUCTED, pending Eric). Panics only if the bundled JSON is malformed,
/// which `scripts/modes-check.mjs` prevents in CI.
pub fn all() -> Vec<Mode> {
    let reg: Registry =
        serde_json::from_str(MODES_JSON).expect("bundled config/modes.json must be valid JSON");
    reg.modes
}

/// Everything the visibility rule needs to know about right now. Gathered by the
/// caller (DOM/storage/entitlements), so [`visible`] stays pure.
pub struct HubCtx {
    /// Little Speller / Kid Mode.
    pub kid: bool,
    /// The native iOS bridge is present (mirrors how tools_hub decides `Avail::Native`).
    pub native: bool,
    /// The study language in play.
    pub lang: String,
    /// The player's resolved access level FOR `lang` — from the entitlement
    /// resolver, never re-derived here.
    pub level: AccessLevel,
    /// Parent-premium entitlements the player holds, by `EntitlementSet` field
    /// name (e.g. "photo_ocr").
    pub premium: Vec<String>,
    /// Mode ids whose runtime flag is ON, collected by the caller from
    /// `flags::is_on`.
    ///
    /// `status` and the flag are DIFFERENT questions and both must pass: the
    /// registry says what a mode IS (live / coming_soon / hidden), the flag says
    /// whether it currently RUNS. Conflating them would force word_stories
    /// (hidden, flag off) and online_spelloff (coming_soon, flag off) to lie
    /// about one or the other. Passed in rather than read here so [`visible`]
    /// stays pure and both branches are testable without storage.
    pub enabled: Vec<String>,
}

/// Whether `m` may be seen at all in `ctx` — the whole rule, in one place.
fn permitted(m: &Mode, ctx: &HubCtx) -> bool {
    // Hidden means hidden. Checked first so nothing below can resurrect it.
    if m.status == Status::Hidden {
        return false;
    }
    // The runtime flag is off -> the feature does not run, so it must not tile.
    // A tile for an inert mode is worse than no tile: it promises a thing that
    // will not happen.
    if !ctx.enabled.iter().any(|e| *e == m.id) {
        return false;
    }
    // Little Speller shows only kid-safe modes — ABSENCE, not locks.
    if ctx.kid && !m.kid_safe {
        return false;
    }
    // A mode that cannot run on this platform does not tile.
    let platform = if ctx.native { "ios" } else { "web" };
    if !m.platforms.iter().any(|p| p == platform) {
        return false;
    }
    // Language constraint (syllable_replay is es-only, spell_aloud en/es).
    if let Some(langs) = &m.languages {
        if !langs.iter().any(|l| *l == ctx.lang) {
            return false;
        }
    }
    // A2.3: a mode needing Full on a previewed language is ABSENT, not locked.
    if ctx.level < m.entitlement_level.as_access() {
        return false;
    }
    // Parent-premium requirement (photo_list needs photo_ocr).
    if let Some(req) = &m.requires_premium {
        if !ctx.premium.iter().any(|p| p == req) {
            return false;
        }
    }
    true
}

/// The modes to render, in registry file order (D6). `Live` entries are
/// tappable; `ComingSoon` entries are teasers the caller must NOT wire a tap to.
/// Nothing here is ever "locked" — an unusable mode is simply absent.
pub fn visible(modes: &[Mode], ctx: &HubCtx) -> Vec<Mode> {
    modes.iter().filter(|m| permitted(m, ctx)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> HubCtx {
        HubCtx {
            kid: false,
            native: true,
            lang: "en".to_string(),
            level: AccessLevel::Full,
            premium: vec!["photo_ocr".to_string()],
            enabled: all().iter().map(|m| m.id.clone()).collect(),
        }
    }

    fn ids(v: &[Mode]) -> Vec<String> {
        v.iter().map(|m| m.id.clone()).collect()
    }

    #[test]
    fn registry_parses_and_holds_the_shipped_modes() {
        let all = all();
        assert_eq!(all.len(), 7, "7 modes registered");
        // File order IS tile order (D6).
        assert_eq!(
            ids(&all),
            vec!["ghost_racing", "syllable_replay", "say_it", "photo_list", "spell_aloud", "word_stories", "online_spelloff"],
        );
    }

    #[test]
    fn hidden_is_hidden_for_everyone() {
        // word_stories is `hidden` (F8/D5 — a hard gate until Eric approves the
        // spec proposal in docs/word-stories-review.md). No context resurrects it.
        let all = all();
        for kid in [false, true] {
            for native in [false, true] {
                let c = HubCtx { kid, native, lang: "en".into(), level: AccessLevel::Full, premium: vec!["photo_ocr".into()], enabled: ids(&all) };
                assert!(!ids(&visible(&all, &c)).contains(&"word_stories".to_string()), "word_stories must never tile");
            }
        }
    }

    #[test]
    fn little_speller_sees_only_kidsafe_modes() {
        let all = all();
        let c = HubCtx { kid: true, ..ctx() };
        let got = ids(&visible(&all, &c));
        // Exactly the two friendly play aids; say_it is a COPPA hard-disable.
        assert_eq!(got, vec!["ghost_racing"], "en: only ghost_racing (syllable_replay is es-only)");
        assert!(!got.contains(&"say_it".to_string()), "say_it is never kid-visible (COPPA)");
        assert!(!got.contains(&"photo_list".to_string()));
    }

    #[test]
    fn little_speller_in_spanish_also_sees_syllable_replay() {
        let all = all();
        let c = HubCtx { kid: true, lang: "es".into(), ..ctx() };
        assert_eq!(ids(&visible(&all, &c)), vec!["ghost_racing", "syllable_replay"]);
    }

    #[test]
    fn web_hides_ios_only_modes() {
        let all = all();
        let c = HubCtx { native: false, ..ctx() };
        let got = ids(&visible(&all, &c));
        assert!(got.contains(&"ghost_racing".to_string()), "ghost racing is all-platforms");
        for ios_only in ["say_it", "photo_list", "spell_aloud"] {
            assert!(!got.contains(&ios_only.to_string()), "{ios_only} is iOS-only");
        }
    }

    #[test]
    fn a23_full_only_mode_is_absent_on_a_previewed_language() {
        let all = all();
        // ghost_racing needs Full (preview_allows denies GhostRace to Preview).
        let c = HubCtx { level: AccessLevel::Preview, ..ctx() };
        let got = ids(&visible(&all, &c));
        assert!(!got.contains(&"ghost_racing".to_string()), "absent on a previewed language — NOT locked");
        // ...and a preview-level mode survives.
        let c2 = HubCtx { level: AccessLevel::Preview, lang: "es".into(), ..ctx() };
        assert!(ids(&visible(&all, &c2)).contains(&"syllable_replay".to_string()));
    }

    #[test]
    fn premium_gated_mode_is_absent_without_the_entitlement() {
        let all = all();
        let c = HubCtx { premium: vec![], ..ctx() };
        assert!(!ids(&visible(&all, &c)).contains(&"photo_list".to_string()), "photo_list needs photo_ocr");
        assert!(ids(&visible(&all, &ctx())).contains(&"photo_list".to_string()), "...and appears with it");
    }

    #[test]
    fn coming_soon_is_visible_but_the_caller_must_not_tap_it() {
        let all = all();
        let got = visible(&all, &ctx());
        let spelloff = got.iter().find(|m| m.id == "online_spelloff").expect("coming_soon still tiles");
        assert_eq!(spelloff.status, Status::ComingSoon, "it is a teaser, not a live tile");
    }

    /// A mode whose runtime flag is OFF does not tile, whatever its status says.
    /// `status` and the flag answer different questions and BOTH must pass.
    #[test]
    fn a_flag_off_mode_does_not_tile() {
        let all = all();
        let c = HubCtx { enabled: vec![], ..ctx() };
        assert!(visible(&all, &c).is_empty(), "no flags on -> no tiles");
        let c = HubCtx { enabled: vec!["ghost_racing".into()], ..ctx() };
        assert_eq!(ids(&visible(&all, &c)), vec!["ghost_racing"]);
    }

    /// ...and `hidden` still beats an ON flag. word_stories' flag can be flipped
    /// on for a tester; the hub must STILL not show it (F8's hard gate).
    #[test]
    fn hidden_beats_an_enabled_flag() {
        let all = all();
        let c = HubCtx { enabled: vec!["word_stories".into()], ..ctx() };
        assert!(visible(&all, &c).is_empty(), "flag on but hidden -> still no tile");
    }

    /// A1.1 — deleting an entry removes the mode from every surface with zero code
    /// changes. Simulated by filtering the parsed registry: nothing else in this
    /// module knows any mode by name.
    #[test]
    fn a11_removing_an_entry_removes_the_mode_everywhere() {
        let all: Vec<Mode> = all().into_iter().filter(|m| m.id != "ghost_racing").collect();
        assert!(!ids(&visible(&all, &ctx())).contains(&"ghost_racing".to_string()));
    }

    /// Every copy key must resolve. A tile rendering a raw key like
    /// "tools.ghost.name" is the i18n bug class that shipped in the forge HUD.
    #[test]
    fn every_mode_copy_key_resolves() {
        for m in all() {
            for key in [&m.name_key, &m.desc_key] {
                assert_ne!(crate::i18n::t(key), *key, "{} renders the raw key {key}", m.id);
            }
        }
    }
}
