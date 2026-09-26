//! CC-BUILD219-FIXES F1 — home-row membership, decided by one flag.
//! CC-HUB-NAV C1 — and that flag now lives on the mode registry.
//!
//! The "Ways to play" row is static markup: launchers with their own ids, i18n
//! keys and feature gates. This module does not render it. It reads the modes
//! registry and suppresses the members whose `hubTile` is false, which is what
//! lets a tile leave the row as a DATA edit -- F1's invariant is that the flag
//! is the only thing deciding membership, so there is deliberately no
//! `if id == "climb"` here or anywhere else.
//!
//! # Why this moved off its own file
//!
//! It used to read `config/hub-tiles.json`, which named DOM ELEMENT ids while
//! the mode registry names MODE ids. Two registries with no shared vocabulary
//! and no join: CC-HUB-NAV's §0 C1 halted on exactly that, because a drawer
//! cannot resolve tiles and sheet rows through "the registry" when there are
//! two of them and they do not agree on what a thing is called.
//!
//! Now there is one. The row's membership AND its launcher id are one field on
//! the mode entry, so a tile is described in the same place as everything else
//! about that mode.
//!
//! The element lives on this field rather than in `play_hub`'s `LAUNCH`,
//! because `LAUNCH` answers a different question -- where a SHEET tile goes --
//! and deliberately EXCLUDES `core` modes rather than binding them to `None`,
//! which would claim they are in-round aids. The Climb and the Daily are
//! exactly that case: no sheet tile, one home-row launcher each. Reaching for
//! `LAUNCH` meant reversing that, which its own test caught.
//!
//! THE REGISTRY IS COMPILED INTO THE SITE BUILD, so no `hubTile` may name an
//! app-only launcher: its symbols would reach the site bundle and breach the
//! picture wall (CC-PICTURE-BANK I1). That is how the first cut of F1 broke,
//! caught by the site e2e. `no_hub_tile_names_an_app_only_launcher` below is
//! that rule, carried over from the file this replaced.
//!
//! Suppression is its own class. `climb::reflect_auth` toggles `btn-hide` on
//! `climbBtn` on every auth change (Kid Mode has no leaderboard), so a registry
//! that shared that class would be undone the moment the player signed in.

/// The class that takes a tile out of the row. Distinct from `btn-hide`, which
/// belongs to the per-tile feature gates.
pub const OFF: &str = "tile-off";

/// Element ids to suppress, from `(mode id, hubTile)` pairs. Split from both
/// the registry and the DOM so the invariant is testable without either: F1
/// asks for exactly this -- an all-true registry suppresses nothing, and
/// flipping one flag suppresses precisely that tile.
fn suppressed_from<'a>(rows: impl Iterator<Item = Option<&'a crate::modes::HubTile>>) -> Vec<String> {
    rows.flatten().filter(|t| !t.member).map(|t| t.element.clone()).collect()
}

fn suppressed() -> Vec<String> {
    // `modes::all()` re-parses and hands back an owned Vec, so the ids are
    // cloned rather than borrowed out of a temporary.
    let all = crate::modes::all();
    suppressed_from(all.iter().map(|m| m.hub_tile.as_ref()))
}

/// Apply the registry to the live row. Additive only: a tile is suppressed or
/// left alone, never force-shown, because whether a MEMBER is visible right now
/// is its feature gate's business, not this registry's.
pub fn apply() {
    for id in suppressed() {
        crate::dom::add_class(&id, OFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F1's invariant, stated as the spec states it. Real mode ids, because the
    /// flag is now useless without a launcher to resolve to.
    #[test]
    fn the_flag_is_the_only_thing_that_decides_row_membership() {
        let t = |el: &str, member| crate::modes::HubTile { element: el.into(), member };
        let all_on = [t("climbBtn", true), t("dailyBtn", true)];
        assert!(suppressed_from(all_on.iter().map(Some)).is_empty(), "every member present when every flag is true");

        let one_off = [t("climbBtn", true), t("dailyBtn", false)];
        assert_eq!(suppressed_from(one_off.iter().map(Some)), vec!["dailyBtn".to_string()], "flipping one flag removes exactly that tile");
    }

    /// Not a member of the row at all is not the same as suppressed, and must
    /// stay cheap: most modes carry no field and the registry only suppresses.
    #[test]
    fn a_mode_with_no_flag_is_left_alone() {
        assert!(suppressed_from([None, None].into_iter()).is_empty());
    }

    /// The shipped registry: Climb out, nothing else.
    #[test]
    fn climb_is_the_only_tile_out_of_the_row() {
        assert_eq!(suppressed(), vec!["climbBtn".to_string()]);
    }

    /// A typo in an id would silently suppress nothing. Every resolved id must
    /// be real markup.
    #[test]
    fn every_registered_id_exists_in_the_markup() {
        let html = include_str!("../index.html");
        for m in crate::modes::all().iter() {
            if let Some(t) = &m.hub_tile {
                assert!(html.contains(&format!("id=\"{}\"", t.element)),
                        "{} names {}, which is not in index.html", m.id, t.element);
            }
        }
    }

    /// The registry ships in the SITE bundle as well, so the ids it resolves
    /// must not name an app-only launcher. The first cut of F1 did, and the
    /// site e2e caught it: web-picture-wall-scan and the picture-wall spec both
    /// fail if the gallery's symbols reach the site wasm.
    #[test]
    fn no_hub_tile_names_an_app_only_launcher() {
        for m in crate::modes::all().iter() {
            if let Some(t) = &m.hub_tile {
                for tok in ["wordPicOpen", "wpGallery", "wpReveal"] {
                    assert_ne!(t.element, tok,
                               "{} names {tok}, which would cross the picture wall (CC-PICTURE-BANK I1)", m.id);
                }
            }
        }
    }

    /// The suppression class must NOT be the one the Kid-Mode gate toggles, or
    /// reflect_auth would put the tile back on the next auth change.
    #[test]
    fn suppression_does_not_share_a_class_with_the_kid_mode_gate() {
        assert_ne!(OFF, "btn-hide");
        let climb = include_str!("climb.rs");
        assert!(climb.contains("toggle_class(\"climbBtn\", \"btn-hide\""),
                "if reflect_auth stopped using btn-hide, re-check this separation");
        assert!(!climb.contains(OFF), "the auth gate must not touch the membership class");
    }
}
