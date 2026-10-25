//! G-D4 / F10 — display names.
//!
//! Adults type a name (14 characters, the local Head-to-Head limit) and it
//! must pass the app's profanity filter. Kids never choose: the server makes
//! `<Color> <Piece> <NN>` from fixed lists. Phase B localizes the lists by
//! `ui_lang` (O12); until then they are English.

use crate::store::random_bytes;

pub const MAX_NAME_CHARS: usize = 14;

const COLORS: [&str; 8] = ["Red", "Blue", "Green", "Gold", "Teal", "Plum", "Coral", "Silver"];
/// BoardGame's pieces (F10).
const PIECES: [&str; 4] = ["Hat", "Wand", "Bee", "Book"];

#[derive(Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    Rejected,
}

/// Validates an adult's typed name.
pub fn adult(raw: &str) -> Result<String, NameError> {
    let name: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(NameError::TooLong);
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(NameError::Rejected);
    }
    // Check the whole name and each word, so "nice <slur>" cannot hide a term.
    if spell_wasm::native_api::is_blocked(&name) || name.split(' ').any(spell_wasm::native_api::is_blocked) {
        return Err(NameError::Rejected);
    }
    Ok(name)
}

/// A generated kid name. Any `display_name` the kid's client sent is ignored.
pub fn kid() -> String {
    let b = random_bytes::<3>();
    format!("{} {} {:02}", COLORS[b[0] as usize % COLORS.len()], PIECES[b[1] as usize % PIECES.len()], b[2] % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adult_names() {
        assert_eq!(adult("  Grandma   Jo "), Ok("Grandma Jo".into()));
        assert_eq!(adult(""), Err(NameError::Empty));
        assert_eq!(adult("abcdefghijklmno"), Err(NameError::TooLong));
        assert_eq!(adult("a\u{7}b"), Err(NameError::Rejected));
        assert_eq!(adult("fuck"), Err(NameError::Rejected));
        assert_eq!(adult("hi fuck"), Err(NameError::Rejected));
    }

    #[test]
    fn kid_names_have_the_shape() {
        for _ in 0..200 {
            let n = kid();
            let parts: Vec<&str> = n.split(' ').collect();
            assert_eq!(parts.len(), 3, "{n}");
            assert!(COLORS.contains(&parts[0]) && PIECES.contains(&parts[1]));
            assert_eq!(parts[2].len(), 2);
        }
    }
}
