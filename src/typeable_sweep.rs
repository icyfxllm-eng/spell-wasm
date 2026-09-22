//! Every word a mode can ASK A PLAYER TO SPELL must be enterable on that
//! language's own keyboard.
//!
//! `keyboard::every_word_char_is_typeable` already proves character
//! reachability over the whole bank. This sweep is one level stricter and one
//! level closer to the player: it runs the words each MODE actually serves
//! through `wordmode::typeable`, which models what the keyboard can COMPOSE --
//! Mandarin pinyin plus a tone digit, Vietnamese tones off the tone row, Korean
//! syllables decomposed into the jamo the Dubeolsik layout carries. A character
//! being reachable somewhere is not the same as a word being enterable.
//!
//! It also pins the fold: Spell Search and Spell Cross serve `lexicon::fold`,
//! while the bank sweep checks `norm::fold_strict`. If those two ever diverge,
//! a served word could contain something no test has looked at.

#[cfg(test)]
mod tests {
    use crate::consts::BUILTIN_LANGS;
    use crate::spelldoku::wordmode::typeable;
    use crate::wordsearch::gen::Tier;
    use crate::wordsearch::lexicon::{daily_pool, fold, pool};

    /// Spell Search and Spell Cross draw from these, and Spell Cross is typed.
    #[test]
    fn every_word_a_mode_serves_can_be_typed_on_its_keyboard() {
        let mut checked = 0usize;
        let mut bad: Vec<String> = Vec::new();
        for (code, _, _, _) in BUILTIN_LANGS {
            for tier in [Tier::Jr, Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
                for w in pool(code, tier) {
                    checked += 1;
                    if !typeable(code, &w) {
                        bad.push(format!("{code} {}: {w:?}", tier.id()));
                    }
                }
            }
            // The Daily draws from the whole bank filtered by length, which is a
            // wider set than any one tier's pool.
            for tier in [Tier::Jr, Tier::Easy] {
                for w in daily_pool(code, tier) {
                    checked += 1;
                    if !typeable(code, &w) {
                        bad.push(format!("{code} daily: {w:?}"));
                    }
                }
            }
        }
        assert!(checked > 10_000, "the sweep really ran ({checked} words)");
        bad.sort();
        bad.dedup();
        assert!(bad.is_empty(), "{} words cannot be typed on their own keyboard:\n  {}", bad.len(), bad.join("\n  "));
    }

    /// The two folds must agree about what a player has to reproduce. The bank
    /// sweep checks `fold_strict`; the modes serve `fold`.
    #[test]
    fn the_served_fold_never_adds_a_character_the_bank_sweep_did_not_see() {
        for (code, _, _, _) in BUILTIN_LANGS {
            for tier in [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
                for w in pool(code, tier) {
                    let served: std::collections::HashSet<char> = fold(&w).chars().collect();
                    let swept: std::collections::HashSet<char> = crate::norm::fold_strict(&w).chars().collect();
                    let extra: Vec<char> = served.difference(&swept).copied().collect();
                    assert!(
                        extra.is_empty(),
                        "{code} {}: {w:?} is served with {extra:?}, which fold_strict removes -- the bank sweep never checked those",
                        tier.id()
                    );
                }
            }
        }
    }
}
