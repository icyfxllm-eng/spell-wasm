//! Pillar 3 — the "Tools & Features" hub in Settings.
//!
//! One discoverable place that LISTS, EXPLAINS, and TOGGLES every feature flag,
//! so players stop hunting for hidden contextual buttons. Each row is
//! icon + localized name + a one-line description + an on/off switch + a live
//! availability hint. The switch writes the real `localStorage['spell_flag_<name>']`
//! that `flags::<name>()` reads, then re-runs that tool's `reflect` so the change
//! applies live where possible; per-run tools (ghost / syllable / word stories)
//! note that they apply on the next round.
//!
//! Kid Mode simplifies the list: the owner/complex/dark rows are hidden and only
//! the two friendly play aids (ghost racing, syllable replay) remain.

use crate::dom;
use crate::flags;
use crate::i18n::t;
use crate::native_lang;
use crate::App;

/// How a tool's usability is gated — drives the live hint suffix.
#[derive(Clone, Copy, PartialEq)]
enum Avail {
    /// Works everywhere; a per-run tool notes "applies next round".
    Universal,
    /// Needs the native iOS bridge (Say It / Photo / Spell-aloud). The hint
    /// resolves to "ready here" on a real device vs "not on this device".
    Native,
    /// Needs the (currently offline) match server. Static hint.
    Server,
}

/// One hub row. `flag` is the storage name (`spell_flag_<flag>`); `toggle`/`hint`/
/// `row` are element ids; `avail_key` is the static availability i18n key.
struct Tool {
    flag: &'static str,
    toggle: &'static str,
    hint: &'static str,
    row: &'static str,
    avail_key: &'static str,
    avail: Avail,
    /// A per-run tool whose change lands on the next round, not live.
    next_round: bool,
    /// Row survives Kid Mode (a friendly play aid). Everything else is hidden
    /// for little spellers.
    kid_ok: bool,
}

const TOOLS: [Tool; 8] = [
    Tool { flag: "ghost_racing", toggle: "toolGhostToggle", hint: "toolGhostHint", row: "toolGhostRow", avail_key: "tools.ghost.avail", avail: Avail::Universal, next_round: true, kid_ok: true },
    Tool { flag: "syllable_replay", toggle: "toolSyllableToggle", hint: "toolSyllableHint", row: "toolSyllableRow", avail_key: "tools.syllable.avail", avail: Avail::Universal, next_round: true, kid_ok: true },
    Tool { flag: "say_it", toggle: "toolSayItToggle", hint: "toolSayItHint", row: "toolSayItRow", avail_key: "tools.sayit.avail", avail: Avail::Native, next_round: false, kid_ok: false },
    Tool { flag: "photo_list", toggle: "toolPhotoToggle", hint: "toolPhotoHint", row: "toolPhotoRow", avail_key: "tools.photo.avail", avail: Avail::Native, next_round: false, kid_ok: false },
    Tool { flag: "spell_aloud", toggle: "toolSpellAloudToggle", hint: "toolSpellAloudHint", row: "toolSpellAloudRow", avail_key: "tools.spellaloud.avail", avail: Avail::Native, next_round: false, kid_ok: false },
    Tool { flag: "word_stories", toggle: "toolStoriesToggle", hint: "toolStoriesHint", row: "toolStoriesRow", avail_key: "tools.stories.avail", avail: Avail::Universal, next_round: true, kid_ok: false },
    Tool { flag: "online_spelloff", toggle: "toolSpelloffToggle", hint: "toolSpelloffHint", row: "toolSpelloffRow", avail_key: "tools.spelloff.avail", avail: Avail::Server, next_round: false, kid_ok: false },
    Tool { flag: "attempts_shields", toggle: "toolShieldsToggle", hint: "toolShieldsHint", row: "toolShieldsRow", avail_key: "tools.shields.avail", avail: Avail::Universal, next_round: false, kid_ok: false },
];

/// The live availability hint for a row: the static descriptor plus a dynamic
/// suffix (native "ready here"/"not on this device", or "applies next round").
fn hint_text(tool: &Tool, native: bool) -> String {
    let mut s = t(tool.avail_key);
    match tool.avail {
        Avail::Native => {
            s.push(' ');
            s.push_str(&t(if native { "tools.availHere" } else { "tools.availNotHere" }));
        }
        Avail::Universal if tool.next_round => {
            s.push(' ');
            s.push_str(&t("tools.appliesNextRound"));
        }
        _ => {}
    }
    s
}

/// Sync every row to the current world: switch state from the stored flag, live
/// availability hint, and Kid-Mode visibility. Called whenever Settings is
/// (re)applied, so it's always current when the panel opens. Safe to call any
/// time (idempotent, read-only except for the DOM it owns).
pub fn reflect(app: &App) {
    let kid = app.borrow().kid;
    let native = native_lang::available();
    for tool in &TOOLS {
        let hidden = kid && !tool.kid_ok;
        dom::toggle_class(tool.row, "btn-hide", hidden);
        if hidden {
            continue;
        }
        dom::input(tool.toggle).set_checked(flags::is_on(tool.flag));
        dom::set_text(tool.hint, &hint_text(tool, native));
    }
}

/// Attach the change handler on every tool switch. Flipping a row writes the real
/// `spell_flag_<name>` and re-runs that tool's reflect so the change applies live
/// where possible.
pub fn wire(app: &App) {
    for tool in &TOOLS {
        let a = app.clone();
        let flag = tool.flag;
        let toggle = tool.toggle;
        dom::on::<web_sys::Event, _>(tool.toggle, "change", move |_| {
            let on = dom::input(toggle).checked();
            crate::storage::set_raw(&format!("spell_flag_{flag}"), if on { "on" } else { "off" });
            apply_reflect(&a, flag);
        });
    }
}

/// Re-run the affected tool's own visibility reflect after its flag flips, so the
/// change takes effect live. Per-run tools (ghost / syllable / word stories) have
/// nothing to reflect here — they read the flag when the next word/round renders.
fn apply_reflect(app: &App, flag: &str) {
    match flag {
        "say_it" => crate::say_it::reflect_gating(app),
        "photo_list" => crate::photo_list::reflect_visibility(),
        "spell_aloud" => crate::spell_aloud::reflect(app),
        "online_spelloff" => crate::online_spelloff::reflect_gate(),
        // attempts_shields reveals/hides its own settings row + enables Climb
        // shields; mirror the exact toggle settings::apply_settings performs.
        "attempts_shields" => {
            dom::toggle_class("extraAttemptsRow", "btn-hide", !flags::attempts_shields());
        }
        _ => {}
    }
}
