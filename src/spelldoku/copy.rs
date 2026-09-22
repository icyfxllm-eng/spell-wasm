//! CC-SPELLDOKU-RULES v1 F4 — the ONE source of composer prompts and rejection
//! copy. Every string the player reads while placing a symbol comes from here,
//! keyed by what the board actually is, so a Letters board can never say
//! "number" and a Numbers board can never say "letter" (I-R6).
//!
//! This returns i18n KEYS rather than text: the keys are the contract, the
//! translations live in the locale files, and a host test can pin the mapping
//! without a browser.

/// What the board's symbols are. Tier Mode is a Numbers board whose digits
/// index a difficulty band (v1.3), so it is a third reading, not a fourth kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Numbers,
    Letters,
    Tier,
}

impl Kind {
    /// What the board is, from what it is serving.
    pub fn of(words: bool, tier_mode: bool) -> Kind {
        match (tier_mode, words) {
            (true, _) => Kind::Tier,
            (_, true) => Kind::Letters,
            _ => Kind::Numbers,
        }
    }
}

/// Every string the placing flow can show, as i18n keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Copy {
    /// No cell chosen yet.
    pub pick_cell: &'static str,
    /// A cell is chosen and the player must choose a symbol (C2).
    pub pick_symbol: &'static str,
    /// A symbol is chosen and the player must spell its word.
    pub spell: &'static str,
    /// What they typed is not that symbol's word.
    pub misspelled: &'static str,
    /// The hint has nowhere legal left to point.
    pub hint_blocked: &'static str,
}

pub fn copy(kind: Kind) -> Copy {
    match kind {
        Kind::Numbers => Copy {
            pick_cell: "sd.pick",
            pick_symbol: "sd.pickSymbol",
            spell: "sd.spell",
            misspelled: "sd.misspelled",
            hint_blocked: "sd.hintBlockedNum",
        },
        Kind::Letters => Copy {
            pick_cell: "sd.pick",
            pick_symbol: "sd.pickSymbolWord",
            spell: "sd.spellWord",
            misspelled: "sd.misspelledWord",
            hint_blocked: "sd.hintBlockedWord",
        },
        // Tier Mode's digits are numbers, but what is spelled is a band word,
        // so it borrows the Letters phrasing for the act of spelling and keeps
        // its own rejection: a miss there draws ANOTHER word of the same tier
        // (v1.3 F4), which is why this does not say "hear it again".
        Kind::Tier => Copy {
            pick_cell: "sd.pick",
            pick_symbol: "sd.tierPick",
            spell: "sd.tierSpell",
            misspelled: "sd.tierMissed",
            hint_blocked: "sd.hintBlockedNum",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The locale sources, read straight off disk: `i18n::t` reaches into
    /// wasm-bindgen and cannot run on the host.
    const LOCALES: [(&str, &str); 15] = [
        ("en", include_str!("../i18n/locales/en.json")),
        ("es", include_str!("../i18n/locales/es.json")),
        ("fr", include_str!("../i18n/locales/fr.json")),
        ("de", include_str!("../i18n/locales/de.json")),
        ("pt", include_str!("../i18n/locales/pt.json")),
        ("pl", include_str!("../i18n/locales/pl.json")),
        ("ru", include_str!("../i18n/locales/ru.json")),
        ("ar", include_str!("../i18n/locales/ar.json")),
        ("hi", include_str!("../i18n/locales/hi.json")),
        ("zh", include_str!("../i18n/locales/zh.json")),
        ("ja", include_str!("../i18n/locales/ja.json")),
        ("ko", include_str!("../i18n/locales/ko.json")),
        ("sw", include_str!("../i18n/locales/sw.json")),
        ("vi", include_str!("../i18n/locales/vi.json")),
        ("fil", include_str!("../i18n/locales/fil.json")),
    ];

    fn dict(raw: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(raw).unwrap().as_object().unwrap().clone()
    }

    fn text(d: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
        d.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
    }

    const KINDS: [Kind; 3] = [Kind::Numbers, Kind::Letters, Kind::Tier];

    fn all_keys(c: Copy) -> [&'static str; 5] {
        [c.pick_cell, c.pick_symbol, c.spell, c.misspelled, c.hint_blocked]
    }

    /// Done #12: the prompt for each row of the F4 table, pinned in English.
    #[test]
    fn the_f4_table_is_what_a_player_reads() {
        let d = dict(LOCALES[0].1);
        for (kind, pick, spell) in [
            (Kind::Numbers, "Tap a number, then spell it", "Spell the number, then \u{2713}"),
            (Kind::Letters, "Tap a letter to hear its word", "Tap \u{25B6} to hear the word, spell it, then \u{2713}"),
            (Kind::Tier, "Pick a digit \u{2014} its tier is on the badge", "Listen, then spell the word"),
        ] {
            let c = copy(kind);
            assert_eq!(text(&d, c.pick_symbol), pick, "{kind:?} pick prompt");
            assert_eq!(text(&d, c.spell), spell, "{kind:?} spell prompt");
        }
    }

    /// Done #10 / I-R6: no string a Letters board shows contains that language's
    /// own word for "numbers", and no string a Numbers board shows contains its
    /// word for "letters". The locale supplies both words itself -- they are the
    /// mode chip's two labels -- so this needs no hand-written glossary and
    /// cannot drift from the translations.
    #[test]
    fn no_string_names_the_other_kind_of_symbol_in_any_language() {
        for (lang, raw) in LOCALES {
            let d = dict(raw);
            let numbers_word = text(&d, "sd.tier.off").to_lowercase();
            let letters_word = text(&d, "sd.tier.letters").to_lowercase();
            assert!(!numbers_word.is_empty() && !letters_word.is_empty(), "{lang}: the chip words exist");
            for (kind, banned, name) in [
                (Kind::Letters, numbers_word.as_str(), "numbers"),
                (Kind::Numbers, letters_word.as_str(), "letters"),
            ] {
                for key in all_keys(copy(kind)) {
                    let s = text(&d, key).to_lowercase();
                    assert!(!s.contains(banned), "{lang}: {kind:?} string {key} says the word for {name} -- {s}");
                }
            }
        }
    }

    /// Every key this module can hand back is defined in every locale, so a
    /// player never meets a raw key.
    #[test]
    fn every_key_resolves_in_every_locale() {
        for (lang, raw) in LOCALES {
            let d = dict(raw);
            for kind in KINDS {
                for key in all_keys(copy(kind)) {
                    assert!(!text(&d, key).is_empty(), "{lang}: {key} is missing");
                }
            }
        }
    }

    /// Done #11's companion in the core: the three readings never share a
    /// prompt, so a board cannot borrow another's wording by accident.
    #[test]
    fn each_reading_has_its_own_prompt() {
        let prompts: Vec<&str> = KINDS.iter().map(|k| copy(*k).pick_symbol).collect();
        let mut seen = prompts.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), prompts.len(), "two readings share a pick prompt: {prompts:?}");
    }

    /// Done #11: the screen never names a composer key itself. Every prompt and
    /// rejection it shows comes back from `copy()`, so there is one place to
    /// look when a string is wrong and one place a new reading has to be added.
    #[test]
    fn no_composer_string_is_chosen_outside_this_module() {
        let ui = include_str!("../spelldoku_ui.rs");
        let mut managed: Vec<&str> = KINDS.iter().flat_map(|k| all_keys(copy(*k))).collect();
        managed.sort_unstable();
        managed.dedup();
        for key in managed {
            assert!(
                !ui.contains(&format!("\"{key}\"")),
                "spelldoku_ui.rs names {key} itself -- it should ask copy::copy() for it"
            );
        }
    }

    /// Tier Mode is a numbers board, so its strings must not reach for the
    /// word "letters" either, even though what gets spelled is a band word.
    #[test]
    fn tier_mode_reads_as_a_numbers_board() {
        for (lang, raw) in LOCALES {
            let d = dict(raw);
            let letters_word = text(&d, "sd.tier.letters").to_lowercase();
            for key in all_keys(copy(Kind::Tier)) {
                let s = text(&d, key).to_lowercase();
                assert!(!s.contains(&letters_word), "{lang}: Tier string {key} says the word for letters -- {s}");
            }
        }
    }
}
