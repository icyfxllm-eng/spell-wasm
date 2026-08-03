//! CC-TRANSLATE-TOOLS core (ship 126) — the honest pivot-less start.
//!
//! THE CLOSED-SPACE LAW (I1): only bank words are ever rendered, spoken,
//! or suggested; every miss path is the single audited "not in my word
//! list yet" state. No MT, no translation API, no phrase path — the
//! gate's symbol scan enforces the vocabulary repo-wide.
//!
//! SUBORDINATION, recorded: CC-BANK-TRANSLATE (not yet in hand) is
//! authoritative on the gloss pivot. This module therefore ships every
//! pivot-INDEPENDENT tool — the tab, the single-word card (native
//! script, audio, zh transliteration toggle), and both doors ("Now
//! spell it" via the shared request_word door; "Add to plan" via the
//! planner's own band-legality) — plus `gloss_rows`, the ONE resolver
//! seam (entitlement ∧ glossAudited). Today no language is glossAudited,
//! so fan-outs simply render the word's own row: absent, not locked
//! (widget D3 precedent). When BANK-TRANSLATE lands its schema, rows
//! appear through this seam and nowhere else.
//!
//! I4: no free-text input exists on this surface — lookup is pick-only,
//! from the kid's own pools and the current tier's bank page.

pub struct GlossRow {
    pub lang: String,
    pub word: String,
}

/// THE resolver seam (I2): per-language visibility = entitlement ∧
/// glossAudited, one function, no per-tool checks. glossAudited is
/// false for every language until CC-BANK-TRANSLATE lands its audited
/// gloss data — an empty answer renders as ABSENT rows, never locked.
pub fn gloss_rows(_word: &str, _lang: &str) -> Vec<GlossRow> {
    Vec::new()
}

/// zh bank entries are "pinyin|hanzi" — both halves already audited
/// bank content, so the transliteration toggle is a re-render, not a
/// generated romanization. Languages without a shipped scheme return
/// None and the card shows no toggle (honest absence, tool 4).
pub fn transliteration(entry: &str) -> Option<(String, String)> {
    entry
        .split_once('|')
        .map(|(pinyin, hanzi)| (hanzi.to_string(), pinyin.to_string()))
}

/// Display/spoken halves of a bank entry (zh split, everyone else as-is).
pub fn display_word(entry: &str) -> String {
    match entry.split_once('|') {
        Some((_, hanzi)) => hanzi.to_string(),
        None => entry.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_space_renders_no_rows_without_audited_gloss() {
        // Acceptance #2's unit face: with no glossAudited language the
        // resolver yields nothing — absent, not locked, not generated.
        assert!(gloss_rows("water", "en").is_empty());
    }

    #[test]
    fn transliteration_only_where_a_scheme_ships() {
        let (native, roman) = transliteration("shu\u{01d0}|\u{6c34}").unwrap();
        assert_eq!(native, "\u{6c34}");
        assert_eq!(roman, "shu\u{01d0}");
        assert!(transliteration("water").is_none(), "no scheme, no toggle");
        assert!(transliteration("\u{432}\u{43e}\u{434}\u{430}").is_none(), "ru has no shipped scheme yet");
    }

    #[test]
    fn display_word_shows_the_script_side() {
        assert_eq!(display_word("shu\u{01d0}|\u{6c34}"), "\u{6c34}");
        assert_eq!(display_word("water"), "water");
    }
}
