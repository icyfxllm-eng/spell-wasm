//! Runtime feature flags — the one place a shipped-dark feature is switched on.
//!
//! Each flag is a `pub fn -> bool` that is **OFF by default**, so the feature
//! compiles into the binary but is invisible until the default is flipped (or a
//! `localStorage` override turns it on for testing/QA). A flag being OFF must be
//! a true no-op: the feature registers nothing, wires nothing, and renders
//! nothing — zero observable diff from before the feature existed.
//!
//! Sibling feature branches add their own `pub fn` here too; keep them
//! independent (no cross-flag coupling) so they merge without conflict.

/// localStorage override helper: returns `Some(true/false)` when the key is set
/// to a recognized truthy/falsy value, else `None` (use the compiled default).
fn override_bool(key: &str) -> Option<bool> {
    match crate::storage::get_raw(key).as_deref() {
        Some("1") | Some("on") | Some("true") | Some("yes") => Some(true),
        Some("0") | Some("off") | Some("false") | Some("no") => Some(false),
        _ => None,
    }
}

/// Feature F2 "Say It" — on-device pronunciation practice (see the word, say it).
/// **Default OFF**: the mode is not registered, the launcher stays hidden, and no
/// speech code path is reachable. Enable for QA on a device by setting
/// `localStorage['spell_flag_say_it'] = '1'`. iOS-only at runtime regardless of
/// this flag (it needs the native on-device SFSpeechRecognizer bridge).
pub fn say_it() -> bool {
    override_bool("spell_flag_say_it").unwrap_or(false)
}

/// Feature "Spell It Out Loud" — voice spelling INPUT (a mic beside the answer
/// field; the player speaks letter names "C… A… T" and the parser produces the
/// same string a keyboard would). **Default OFF**: the mic never renders and no
/// capture path is reachable — a true no-op, zero diff (Invariant I6). Enable for
/// a device QA pass with `localStorage['spell_flag_spell_aloud'] = '1'`. iOS-only
/// at runtime regardless (it needs the on-device speech bridge).
pub fn spell_aloud() -> bool {
    override_bool("spell_flag_spell_aloud").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    // NOTE: `say_it()` reads localStorage via `crate::storage`, which is a
    // web-sys call that is inert off-wasm. The default-OFF contract is asserted
    // at the source level (the `unwrap_or(false)` above); the flag-off "mode is
    // absent" behavior is proven end-to-end by the Rust gate tests in
    // `say_it.rs` and the web E2E `sayit` spec. No host-side unit test here would
    // exercise real localStorage.
}
