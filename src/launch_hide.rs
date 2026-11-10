//! CC-LAUNCH-EN build 318 (Eric, 2026-11-09): what the English launch does not show.
//!
//! Eric's device pass on build 317 circled these as not part of the launch:
//! the Spell It, photo list, word stories and online Spell-Off switches in
//! Tools (Spell It's mic is not shipping, so its siblings go with it), the two
//! debug rows, the volume boost and game-sounds-level sliders, "Play online
//! with family", and the drawer's "How this works" and Credits rows.
//!
//! Hidden in every build that is not a dev or test build, so TestFlight and the
//! App Store match. Dev and test builds keep the rows, so the e2e suite still
//! exercises the features behind them. A hidden Tools switch also stops being a
//! player flag in those builds ([`flag_hidden`], read by `flags::stored`): a
//! tester who switched Spell It on in an earlier build would otherwise keep the
//! mic with no switch left to turn it off.

/// True in TestFlight and App Store builds.
pub const HIDE: bool = !cfg!(any(feature = "dev_preview", feature = "testseam"));

/// Player flags whose Tools switch the launch hides. Their release default is
/// off (config/flags.release.json), so hiding the switch turns them off.
pub const HIDDEN_TOOL_FLAGS: &[&str] = &["photo_list", "spell_aloud", "word_stories", "online_spelloff"];

/// Settings rows the launch hides (by id).
pub const HIDDEN_ROWS: &[&str] = &[
    "toolPhotoRow",
    "toolSpellAloudRow",
    "toolStoriesRow",
    "toolSpelloffRow",
    "olFamilyRow",
    "nativeStatusRow",
    "audioSourceRow",
    "volumeRow",
    "gameSoundsLevelRow",
];

/// Drawer help rows the launch hides (by the element the row clicks).
pub const HIDDEN_DRAWER_TARGETS: &[&str] = &["spellItGuideAgain", "creditsBtn"];

/// Whether a player flag's switch is hidden in this build.
pub fn flag_hidden(name: &str) -> bool {
    HIDE && HIDDEN_TOOL_FLAGS.contains(&name)
}

/// Whether a drawer help row is hidden in this build.
pub fn drawer_hidden(target: &str) -> bool {
    HIDE && HIDDEN_DRAWER_TARGETS.contains(&target)
}

/// Hide the rows. Called whenever Settings is reflected; idempotent.
pub fn apply() {
    if !HIDE {
        return;
    }
    for id in HIDDEN_ROWS {
        if crate::dom::exists(id) {
            crate::dom::add_class(id, "btn-hide");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_tools_are_player_flags_that_ship_off() {
        let release: serde_json::Value =
            serde_json::from_str(include_str!("../config/flags.release.json")).unwrap();
        for f in HIDDEN_TOOL_FLAGS {
            assert!(crate::flags::PLAYER_FLAGS.contains(f), "{f} is not a Tools switch");
            assert_eq!(release[*f]["ship"], serde_json::json!(false), "{f} must ship off if its switch is hidden");
        }
    }

    #[test]
    fn every_hidden_row_exists_in_the_markup() {
        let html = include_str!("../index.html");
        for id in HIDDEN_ROWS.iter().chain(HIDDEN_DRAWER_TARGETS) {
            assert!(html.contains(&format!("id=\"{id}\"")), "#{id} is not in index.html");
        }
    }

    #[test]
    fn shields_switch_stays() {
        assert!(!HIDDEN_TOOL_FLAGS.contains(&"attempts_shields"));
        assert!(!HIDDEN_ROWS.contains(&"toolShieldsRow"));
    }
}
