//! The Climb's account + leaderboard path, compiled OUT (CC-LAUNCH-EN Phase B
//! item 2a). This file stands in for `climb.rs` in every build without the
//! `accounts_net` feature, which is the App Store build: no sign-up, sign-in,
//! front door, account sheet, leaderboard view or run upload, and no request to
//! `/api/auth/*` or `/api/climb/*` exists in the binary at all.
//!
//! Local Climb play is untouched: a run is started from the level picker
//! (`Route::Level("climb")`), its best chains stay in `model::LB_KEY`, and
//! Spell Racing's ghost is local. Only the network path is gone.
//!
//! The markup in index.html stays (it is shared by every build); this stub
//! keeps every entrance to it hidden, and the drawer's account row reads
//! `ACCOUNTS` to stay absent rather than offering a dead "Sign in".
//!
//! The public surface mirrors climb.rs so callers do not change.

use crate::{dom, App};

/// False here: this build carries no account path.
pub const ACCOUNTS: bool = false;

pub fn is_logged_in() -> bool {
    false
}

/// No account, so no session token to share with the online Spell Off.
pub fn bearer() -> Option<String> {
    None
}

pub(crate) fn username() -> Option<String> {
    None
}

/// Hide every account and leaderboard entrance. Nothing is wired.
pub fn setup(_app: &App) {
    reflect_auth();
}

pub fn reflect_auth() {
    for id in ["climbBtn", "accountBtn", "setAccountRow"] {
        dom::toggle_class(id, "btn-hide", true);
    }
    // Online Spell Off needs a signed-in account, so its entry stays hidden too.
    crate::online_spelloff::reflect_gate();
}

pub fn open_account_sheet() {}

pub fn open_front_door() {}

/// A finished run stays on the device's own board (game.rs records it there
/// before calling this); nothing is posted.
pub fn submit_run(_difficulty: &str, _chain: u32, _duration_ms: f64) {}

#[cfg(test)]
mod tests {
    /// The stub must not carry a single route of the real module.
    #[test]
    fn the_stub_names_no_account_or_leaderboard_route() {
        let src = include_str!("climb_off.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        let code: String = body.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for needle in ["call(", "fetch", "spawn_local", "\"/api/"] {
            assert!(!code.contains(needle), "climb_off.rs carries {needle}");
        }
        assert!(!super::is_logged_in() && super::bearer().is_none() && super::username().is_none());
    }
}
