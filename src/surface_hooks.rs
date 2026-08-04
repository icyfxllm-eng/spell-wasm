//! Input hooks the active answer surface installs into shared code.
//!
//! CC-PICTURE-PLATFORM I3: "Shared code never imports the Spell Picture
//! subtree. Direction of dependency: Spell Picture may import shared code;
//! shared code may never import Spell Picture."
//!
//! Before this module the dependency ran the wrong way. The F7 work let
//! Spell Picture borrow the base game's keyboard, so `game.rs` had to ask
//! `wordpic_screen` where a keystroke should go, and `spell_aloud.rs` had to
//! ask it for the picture's voice state. Six call sites, all pointing from
//! shared code INTO the picture -- which meant the `picture` cargo feature
//! could not compile the picture out without breaking the base game.
//!
//! So it is inverted here. The picture registers its handlers at wire() time;
//! shared code calls whatever is registered and knows nothing about who
//! registered it. With the feature off nothing registers, every hook stays
//! `None`, and the base game takes the path it always took.
//!
//! This is deliberately dumb -- plain function pointers, no trait objects, no
//! dyn dispatch. A surface is a handful of stateless entry points, and the
//! codebase already reaches for thread-locals for exactly this kind of state.

use std::cell::Cell;

/// What a borrowed answer surface must be able to do. Every field is
/// optional: a build with no alternate surface simply leaves them unset.
#[derive(Clone, Copy, Default)]
pub struct Hooks {
    pub type_char: Option<fn(char)>,
    pub type_jamo: Option<fn(char)>,
    pub backspace: Option<fn()>,
    /// (target word, current buffer, input live)
    pub voice_state: Option<fn() -> (String, String, bool)>,
    pub voice_set: Option<fn(&str)>,
}

thread_local! {
    static HOOKS: Cell<Hooks> = const { Cell::new(Hooks {
        type_char: None, type_jamo: None, backspace: None,
        voice_state: None, voice_set: None,
    }) };
}

/// Install the active surface's handlers. Called by the surface itself.
pub fn install(h: Hooks) {
    HOOKS.with(|c| c.set(h));
}

/// CC-YEARBOOK I3 bridge — same inversion, different lifecycle: the
/// picture registers ONCE at boot; nothing ever uninstalls it. Shared
/// code (the yearbook compiler) reads whatever is here and knows nothing
/// about the picture. With the picture feature off, both stay None and
/// the yearbook ships an honest picture-free book.
#[derive(Clone, Copy, Default)]
pub struct PictureBridge {
    /// Completed runs as (pic, lang, word count, replayed, touched).
    pub gallery: Option<fn() -> Vec<(String, String, usize, bool, u64)>>,
    /// Clean re-render of a completed piece (the FINALE export path).
    pub render_piece: Option<fn(&str, &str) -> Option<String>>,
}

thread_local! {
    static PICTURE_BRIDGE: Cell<PictureBridge> =
        const { Cell::new(PictureBridge { gallery: None, render_piece: None }) };
}

pub fn install_picture_bridge(b: PictureBridge) {
    PICTURE_BRIDGE.with(|c| c.set(b));
}

pub fn picture_bridge() -> PictureBridge {
    PICTURE_BRIDGE.with(Cell::get)
}

// CC-TONAL-POLISH D2 — the run's flawless-streak words. SHARED-owned by
// I3 law: the game (shared) writes here, and the picture subtree READS
// from shared — never the inverse. Session-only, capped, no telemetry.
thread_local! {
    static STREAK_WORDS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

pub fn streak_push(word: &str) {
    STREAK_WORDS.with(|s| {
        let mut v = s.borrow_mut();
        if !v.iter().any(|w| w == word) {
            v.push(word.to_string());
            if v.len() > 60 {
                v.remove(0);
            }
        }
    });
}

pub fn streak_clear() {
    STREAK_WORDS.with(|s| s.borrow_mut().clear());
}

pub fn streak_snapshot() -> Vec<String> {
    STREAK_WORDS.with(|s| s.borrow().clone())
}

#[cfg(test)]
pub fn streak_set(words: &[&str]) {
    STREAK_WORDS.with(|s| {
        *s.borrow_mut() = words.iter().map(|w| w.to_string()).collect()
    });
}

pub fn get() -> Hooks {
    HOOKS.with(Cell::get)
}

/// Whether an alternate surface is present in this build at all. Shared code
/// uses this instead of naming any particular mode.
pub fn present() -> bool {
    let h = get();
    h.type_char.is_some() || h.voice_state.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_registered_by_default() {
        // The base game's path must be the default, not a fallback that only
        // works because some other module happened not to load.
        assert!(!present(), "a fresh build has no alternate surface");
        assert!(get().type_char.is_none());
    }

    #[test]
    fn a_surface_can_register_and_is_then_seen() {
        fn t(_c: char) {}
        install(Hooks { type_char: Some(t), ..Default::default() });
        assert!(present());
        assert!(get().type_char.is_some());
        // Leave the slot clean for other tests in this thread.
        install(Hooks::default());
        assert!(!present());
    }
}
