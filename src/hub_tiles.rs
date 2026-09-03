//! CC-BUILD219-FIXES F1 — home-row membership, decided by one flag.
//!
//! The "Ways to play" row is static markup: six launchers, each with its own id,
//! i18n key and feature gate. This module does not render it. It reads
//! `config/hub-tiles.json` and suppresses the members whose `hubTile` is false,
//! which is what lets a tile leave the row as a DATA edit -- F1's invariant is
//! that the flag is the only thing deciding membership, so there is deliberately
//! no `if id == "climb"` here or anywhere else.
//!
//! Suppression is its own class. `climb::reflect_auth` toggles `btn-hide` on
//! `climbBtn` on every auth change (Kid Mode has no leaderboard), so a registry
//! that shared that class would be undone the moment the player signed in.

use serde::Deserialize;

#[derive(Deserialize)]
struct Reg {
    tiles: Vec<Tile>,
}

#[derive(Deserialize)]
struct Tile {
    id: String,
    #[serde(rename = "hubTile")]
    hub_tile: bool,
}

/// The class that takes a tile out of the row. Distinct from `btn-hide`, which
/// belongs to the per-tile feature gates.
pub const OFF: &str = "tile-off";

fn registry() -> &'static Reg {
    use std::sync::OnceLock;
    static R: OnceLock<Reg> = OnceLock::new();
    R.get_or_init(|| {
        serde_json::from_str(include_str!("../config/hub-tiles.json")).expect("hub-tiles.json parses")
    })
}

/// Ids to suppress. Split from the DOM call so the invariant is testable
/// without a browser: F1 asks for exactly this -- an all-true registry
/// suppresses nothing, and flipping one flag suppresses precisely that tile.
fn suppressed_in(reg: &Reg) -> Vec<&str> {
    reg.tiles.iter().filter(|t| !t.hub_tile).map(|t| t.id.as_str()).collect()
}

/// Apply the registry to the live row. Additive only: a tile is suppressed or
/// left alone, never force-shown, because whether a MEMBER is visible right now
/// is its feature gate's business, not this registry's.
pub fn apply() {
    for id in suppressed_in(registry()) {
        crate::dom::add_class(id, OFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Reg {
        serde_json::from_str(s).unwrap()
    }

    /// F1's invariant, stated as the spec states it.
    #[test]
    fn the_flag_is_the_only_thing_that_decides_row_membership() {
        let all_on = parse(r#"{"tiles":[{"id":"a","hubTile":true},{"id":"b","hubTile":true}]}"#);
        assert!(suppressed_in(&all_on).is_empty(), "every mode present when every flag is true");

        let one_off = parse(r#"{"tiles":[{"id":"a","hubTile":true},{"id":"b","hubTile":false}]}"#);
        assert_eq!(suppressed_in(&one_off), vec!["b"], "flipping one flag removes exactly that tile");
    }

    /// The shipped registry: Climb out, nothing else.
    #[test]
    fn climb_is_the_only_tile_out_of_the_row() {
        assert_eq!(suppressed_in(registry()), vec!["climbBtn"]);
    }

    /// The registry ships in the SITE bundle as well, so it must not name an
    /// app-only mode. The first cut of F1 did, and the site e2e caught it:
    /// web-picture-wall-scan and the picture-wall spec both fail if the
    /// gallery's symbols reach the site wasm. These literals are cfg(test) and
    /// never compiled into a shipped binary.
    #[test]
    fn the_registry_never_names_an_app_only_mode() {
        let raw = include_str!("../config/hub-tiles.json");
        for tok in ["word_picture", "wordPicOpen", "wordpic", "wordPicTile", "spellpic"] {
            assert!(!raw.contains(tok),
                    "hub-tiles.json names {tok}, which would cross the picture wall (CC-PICTURE-BANK I1)");
        }
    }

    /// A typo in an id would silently suppress nothing, and the tile would stay
    /// in the row looking like the feature failed. Every id must be real markup.
    #[test]
    fn every_registered_id_exists_in_the_markup() {
        let html = include_str!("../index.html");
        for t in registry().tiles.iter() {
            assert!(html.contains(&format!("id=\"{}\"", t.id)),
                    "hub-tiles.json names {}, which is not in index.html", t.id);
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
