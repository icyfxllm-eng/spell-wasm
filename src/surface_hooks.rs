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
