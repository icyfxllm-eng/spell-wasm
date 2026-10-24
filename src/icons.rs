//! CC-VISUAL-ECLIPSE F8 -- the icon set, by name.
//!
//! `html(name)` is the one way a UI icon reaches the page: an icon slot holding
//! the drawn icon (a `<use>` of the sprite symbol) and the emoji it replaced.
//! The stylesheet shows one of the two, by `visual.icons`, so with the flag off
//! the screen is exactly as it was. Every mode's `icon` in config/modes.json is
//! a name here (AT-20; modes::tests checks it).

use crate::icons_data::ICONS;

/// The emoji an icon replaced, or "" for an unknown name.
pub fn emoji(name: &str) -> &'static str {
    ICONS.iter().find(|(n, _)| *n == name).map(|(_, e)| *e).unwrap_or("")
}

pub fn exists(name: &str) -> bool {
    ICONS.iter().any(|(n, _)| *n == name)
}

/// The slot. Decorative (`aria-hidden`): the label beside it carries the meaning.
pub fn html(name: &str) -> String {
    if !exists(name) {
        return String::new();
    }
    format!(
        "<span class=\"icon-slot\" aria-hidden=\"true\"><svg class=\"icon\"><use href=\"#i-{name}\"/></svg><span class=\"ic-emoji\">{}</span></span>",
        emoji(name)
    )
}

/// The slot followed by escaped text, the shape most labels take.
pub fn label(name: &str, text: &str) -> String {
    format!("{} {}", html(name), crate::dom::escape_html(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_its_emoji_and_html_carries_both() {
        assert!(ICONS.len() >= 45);
        for (n, e) in ICONS {
            assert!(!e.is_empty(), "{n} has no fallback emoji");
            let h = html(n);
            assert!(h.contains(&format!("#i-{n}")) && h.contains(e), "{h}");
        }
    }

    #[test]
    fn an_unknown_name_renders_nothing() {
        assert_eq!(html("no-such-icon"), "");
        assert_eq!(emoji("no-such-icon"), "");
    }

    #[test]
    fn label_escapes_its_text() {
        assert!(label("play", "<b>").contains("&lt;b&gt;"));
    }
}
