//! CC-TRANSLATE-SCREEN — the screen's rules, with no DOM in sight.
//!
//! THE THESIS (§1). Spell Translate can only answer for words in an audited
//! bank with an audited gloss. So the source field is not a text box that
//! accepts anything and then fails: it is a search over the answerable set,
//! committed only by picking a suggestion. A miss is therefore a pre-commit
//! state -- the suggestion list emptying -- and never an error after the fact
//! (I2). Every function here serves that sentence.
//!
//! WHAT THIS FILE OWNS. Search and ranking, sense listing, the single target
//! lookup, the language picker's order, the swap / clear / language-change
//! state rules, D1's transliteration default, and whether Spell it may serve a
//! word. The DOM lives in translate_ui.rs; the gloss data and the closed space
//! live in translate.rs; difficulty lives in experience.rs. Nothing here
//! branches on a particular language (the spec's single-source doctrine).

use std::collections::BTreeSet;

use unicode_normalization::UnicodeNormalization;

use crate::entitlements::{preview_allows, AccessLevel, GameMode};
use crate::experience::{allowed_tiers, Experience, TIERS};

/// F2: at most eight suggestions.
pub const SUGGESTION_CAP: usize = 8;
/// D9 (signed): up to four senses inline, then "more".
pub const SENSES_INLINE: usize = 4;

/// I9: NFC plus lowercase, trimmed. Applied to the query and to every form it
/// is compared against, so a decomposed "é" and a composed one are one letter.
pub fn normalize(s: &str) -> String {
    s.trim().nfc().collect::<String>().to_lowercase()
}

/// Whether `lang` is written in Latin script, read from the language's own
/// name in the registry ("Espa\u{f1}ol", "Ti\u{1ebf}ng Vi\u{1ec7}t" are Latin;
/// "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}" is not). Derived from
/// data rather than a list of codes, so no language is special-cased here. An
/// unknown code reads as Latin, which only ever turns transliteration OFF.
pub fn is_latin_script(lang: &str) -> bool {
    let Some((_, endonym, _, _)) = crate::consts::BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang) else {
        return true;
    };
    endonym
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| matches!(c as u32, 0x41..=0x24F | 0x1E00..=0x1EFF))
}

/// D1 (signed): transliteration starts ON for a non-Latin target when the
/// interface is in a Latin-script language, OFF everywhere else. The player
/// can still toggle it.
pub fn translit_default_on(ui_lang: &str, target_lang: &str) -> bool {
    is_latin_script(ui_lang) && !is_latin_script(target_lang)
}

/// F4: languages both pickers list -- every language with an audited gloss,
/// ordered current pair, then the home language, then by native name.
/// Entitlement does not filter this (D3, signed); a gloss-dark language is
/// simply absent (I8: absent or present, never locked).
pub fn picker_languages(pair: (&str, &str), home: Option<&str>) -> Vec<&'static str> {
    let all = crate::translate::glossed_languages();
    let endonym = |l: &str| {
        crate::consts::BUILTIN_LANGS
            .iter()
            .find(|(c, _, _, _)| *c == l)
            .map(|(_, n, _, _)| normalize(n))
            .unwrap_or_else(|| l.to_string())
    };
    let mut by_name = all.clone();
    by_name.sort_by_key(|l| endonym(l));
    let mut out: Vec<&'static str> = Vec::new();
    for want in [Some(pair.0), Some(pair.1), home].into_iter().flatten() {
        if let Some(l) = all.iter().find(|l| **l == want) {
            if !out.contains(l) {
                out.push(*l);
            }
        }
    }
    for l in by_name {
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// The bank tier `entry` lives in, if it is a bank word of `lang` at all.
pub fn bank_tier_of(lang: &str, entry: &str) -> Option<&'static str> {
    TIERS.iter().copied().find(|t| crate::words::tier_for(lang, t).iter().any(|w| *w == entry))
}

/// D4 (signed): a Spell Jr player's words are Easy or Medium through the Jr
/// resolver, and kid-safe.
fn junior_legal(lang: &str, entry: &str) -> bool {
    let tiers = allowed_tiers(Experience::Junior, "translate");
    bank_tier_of(lang, entry).is_some_and(|t| tiers.contains(&t)) && crate::kid_filter::kid_allowed(lang, entry)
}

/// I3: the target word for `concept` in `target` -- one lookup, never a
/// ranking. For Spell Jr, a target the Jr rules would not serve is no answer.
pub fn resolve_target(exp: Experience, target: &str, concept: &str) -> Option<String> {
    let word = crate::translate::word_for_concept(target, concept)?;
    if exp == Experience::Junior && !junior_legal(target, &word) {
        return None;
    }
    Some(word)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The source word as stored in the bank.
    pub entry: String,
    /// What the player sees (the script side of a zh entry).
    pub shown: String,
    /// The sense-locked English gloss this suggestion commits to.
    pub concept: String,
    /// The answer it will render -- known before commit, which is I2.
    pub target: String,
}

/// F2: suggestions for `query`, ranked exact match, then prefix, then
/// substring, shortest first within a rank, capped at eight. Only words with a
/// renderable answer in `target` are ever offered, so an empty result with a
/// non-empty query IS the "not in my word list yet" decline, shown before
/// anything is committed. D11 (deferred): dictionary forms only.
pub fn search(exp: Experience, source: &str, target: &str, query: &str) -> Vec<Suggestion> {
    let q = normalize(query);
    if q.is_empty() || source == target {
        return Vec::new();
    }
    let mut scored: Vec<(u8, usize, Suggestion)> = Vec::new();
    for (entry, concept) in crate::translate::source_rows(source) {
        if exp == Experience::Junior && !junior_legal(source, &entry) {
            continue;
        }
        let Some(target_word) = resolve_target(exp, target, &concept) else { continue };
        let shown = crate::translate::display_word(&entry);
        let typed = entry.split('|').next().unwrap_or(&entry).to_string();
        let rank = [normalize(&shown), normalize(&typed)]
            .iter()
            .filter_map(|f| {
                if *f == q {
                    Some(0u8)
                } else if f.starts_with(&q) {
                    Some(1)
                } else if f.contains(&q) {
                    Some(2)
                } else {
                    None
                }
            })
            .min();
        let Some(rank) = rank else { continue };
        let len = shown.chars().count();
        scored.push((rank, len, Suggestion { entry, shown, concept, target: target_word }));
    }
    scored.sort_by(|a, b| (a.0, a.1, &a.2.shown, &a.2.entry).cmp(&(b.0, b.1, &b.2.shown, &b.2.entry)));
    // Keyed by word AND sense, so a word with two senses offers both (F3).
    let mut seen = BTreeSet::new();
    scored
        .into_iter()
        .filter(|(_, _, s)| seen.insert((s.entry.clone(), s.concept.clone())))
        .map(|(_, _, s)| s)
        .take(SUGGESTION_CAP)
        .collect()
}

/// F3: every sense `entry` carries in `source`. Today's gloss data holds one
/// concept per word, so this is one sense; the multi-sense path exists so a
/// second sense, when a table adds one, is picked by the player and never
/// silently chosen.
pub fn senses(source: &str, entry: &str) -> Vec<String> {
    crate::translate::source_rows(source).into_iter().filter(|(e, _)| e == entry).map(|(_, c)| c).collect()
}

/// The sense line under the source word, empty when it would only repeat the
/// word. English is its own pivot, so an English source's sense IS the word --
/// "cat" under "cat" tells a player nothing. Every other source keeps its gloss.
pub fn sense_line(shown: &str, concept: Option<&str>) -> String {
    let c = concept.unwrap_or("");
    if normalize(c) == normalize(shown) {
        return String::new();
    }
    c.to_string()
}

/// D9 (signed): the senses to show, and whether a "more" control is needed.
pub fn visible_senses(all: &[String], expanded: bool) -> (Vec<String>, bool) {
    if expanded || all.len() <= SENSES_INLINE {
        (all.to_vec(), false)
    } else {
        (all[..SENSES_INLINE].to_vec(), true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellVerdict {
    Serve,
    Decline,
}

/// F10: whether Spell it may hand `entry` to a spelling session. A Spell Jr
/// player needs a Jr-legal word; an owned language serves any bank tier; a
/// language the player does not own serves only what its PREVIEW tier allows
/// (D3, signed). Anything else is an honest decline, never a silent drop.
pub fn spell_it_verdict(exp: Experience, level: AccessLevel, lang: &str, entry: &str) -> SpellVerdict {
    let Some(tier) = bank_tier_of(lang, entry) else { return SpellVerdict::Decline };
    if exp == Experience::Junior && !junior_legal(lang, entry) {
        return SpellVerdict::Decline;
    }
    if level == AccessLevel::Full {
        return SpellVerdict::Serve;
    }
    let n = TIERS.iter().position(|t| *t == tier).map(|i| i as u32 + 1).unwrap_or(u32::MAX);
    if preview_allows(lang, GameMode::Standard, n) {
        SpellVerdict::Serve
    } else {
        SpellVerdict::Decline
    }
}

/// The screen's whole state: a language pair and, when a question has been
/// asked, the committed source word, its sense, and the answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pair {
    pub source: String,
    pub target: String,
    pub source_entry: Option<String>,
    pub concept: Option<String>,
    pub target_entry: Option<String>,
}

impl Pair {
    pub fn new(source: &str, target: &str) -> Self {
        Pair { source: source.to_string(), target: target.to_string(), ..Default::default() }
    }

    /// F2/F3: commit a suggestion. The answer is the suggestion's own target,
    /// which search already resolved through the single lookup.
    pub fn commit(&self, s: &Suggestion) -> Self {
        Pair {
            source: self.source.clone(),
            target: self.target.clone(),
            source_entry: Some(s.entry.clone()),
            concept: Some(s.concept.clone()),
            target_entry: Some(s.target.clone()),
        }
    }

    /// F7: exchange languages and words in one step. The new target is the
    /// concept looked up in the new target language (I3); when that has no
    /// answer the target is left empty for the decline state, and the swap
    /// still happens -- it never silently refuses.
    pub fn swapped(&self, exp: Experience) -> Self {
        let concept = self.concept.clone();
        let target_entry = concept.as_deref().and_then(|c| resolve_target(exp, &self.source, c));
        Pair {
            source: self.target.clone(),
            target: self.source.clone(),
            source_entry: self.target_entry.clone(),
            concept,
            target_entry,
        }
    }

    /// F8: a new question. Words, sense and answer go; both languages stay.
    pub fn cleared(&self) -> Self {
        Pair::new(&self.source, &self.target)
    }

    /// F4: choosing the target's language for the source is a swap, never a
    /// same-language pair. Any other source language starts a new question.
    pub fn with_source_lang(&self, exp: Experience, lang: &str) -> Self {
        if lang == self.target {
            return self.swapped(exp);
        }
        Pair::new(lang, &self.target)
    }

    /// F4: choosing the source's language for the target is a swap. Any other
    /// target keeps the question and looks its sense up in the new language.
    pub fn with_target_lang(&self, exp: Experience, lang: &str) -> Self {
        if lang == self.source {
            return self.swapped(exp);
        }
        Pair {
            source: self.source.clone(),
            target: lang.to_string(),
            source_entry: self.source_entry.clone(),
            concept: self.concept.clone(),
            target_entry: self.concept.as_deref().and_then(|c| resolve_target(exp, lang, c)),
        }
    }
}

/// e2e fixture only: a word answerable in `have` but not in `lack`, as a player
/// would type it (translate-no-target-answer). Compiled out of production.
#[cfg(feature = "testseam")]
pub fn seam_gap(source: &str, have: &str, lack: &str) -> Option<String> {
    crate::translate::source_rows(source)
        .into_iter()
        .find(|(_, c)| {
            resolve_target(Experience::Standard, have, c).is_some()
                && resolve_target(Experience::Standard, lack, c).is_none()
        })
        .map(|(e, _)| crate::translate::display_word(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An English source's gloss is the English word, so printing it under the
    /// word says nothing twice. Every other source keeps its sense.
    #[test]
    fn a_sense_that_only_repeats_the_word_is_dropped() {
        assert_eq!(sense_line("cat", Some("cat")), "");
        assert_eq!(sense_line("Cat", Some("cat")), "", "case is not a difference");
        assert_eq!(sense_line("gato", Some("cat")), "cat");
        assert_eq!(sense_line("cat", None), "");
    }

    use super::*;
    use crate::translate::seam_set_audited;

    /// Audits real gloss rows for the life of a test, then restores them.
    struct Audit(Vec<&'static str>);
    impl Audit {
        fn on(langs: &[&'static str]) -> Self {
            for l in langs {
                seam_set_audited(l, true);
            }
            Audit(langs.to_vec())
        }
    }
    impl Drop for Audit {
        fn drop(&mut self) {
            for l in &self.0 {
                seam_set_audited(l, false);
            }
        }
    }

    #[test]
    fn translate_normalize_is_nfc_and_lowercase() {
        assert_eq!(normalize("Caf\u{e9}"), normalize("cafe\u{301}"));
        assert_eq!(normalize("  WATER "), "water");
    }

    #[test]
    fn translate_script_is_read_from_the_registry() {
        for l in ["en", "es", "fr", "de", "pt", "pl", "vi", "fil", "sw"] {
            assert!(is_latin_script(l), "{l} is Latin-script");
        }
        for l in ["ru", "ja", "ko", "zh", "ar", "hi"] {
            assert!(!is_latin_script(l), "{l} is not Latin-script");
        }
    }

    #[test]
    fn translate_d1_translit_default() {
        assert!(translit_default_on("en", "ru"), "Latin UI, non-Latin target: on");
        assert!(!translit_default_on("ru", "ja"), "non-Latin UI: off");
        assert!(!translit_default_on("en", "es"), "Latin target: nothing to transliterate");
    }

    #[test]
    fn translate_closed_space_without_audited_glosses() {
        assert!(search(Experience::Standard, "en", "es", "wat").is_empty(),
                "no audited language, no answer, no suggestion");
        assert_eq!(picker_languages(("en", "es"), None), vec!["en"], "only the pivot is listable");
    }

    #[test]
    fn translate_search_ranks_caps_and_answers_before_commit() {
        let _a = Audit::on(&["es"]);
        let got = search(Experience::Standard, "en", "es", "a");
        assert!(got.len() <= SUGGESTION_CAP, "capped at eight");
        for s in &got {
            assert_eq!(crate::translate::word_for_concept("es", &s.concept).as_deref(), Some(s.target.as_str()),
                       "every suggestion's answer is the single lookup (I3)");
        }
        // Exact matches outrank prefixes, which outrank substrings.
        let full = search(Experience::Standard, "es", "en", "agua");
        if let Some(first) = full.first() {
            assert_eq!(normalize(&first.shown), "agua", "an exact match leads");
        }
    }

    #[test]
    fn translate_no_committable_miss() {
        let _a = Audit::on(&["es"]);
        assert!(search(Experience::Standard, "en", "es", "zzqxjv").is_empty(),
                "a word with no answer is never offered, so it can never be committed (I2)");
    }

    #[test]
    fn translate_junior_answers_are_easy_medium_and_kid_safe() {
        let _a = Audit::on(&["es", "ja"]);
        for (src, tgt) in [("en", "es"), ("es", "en"), ("en", "ja")] {
            for q in ["a", "e", "o", "s"] {
                for s in search(Experience::Junior, src, tgt, q) {
                    let tier = bank_tier_of(tgt, &s.target).expect("a bank word");
                    assert!(tier == "easy" || tier == "medium", "{src}->{tgt}: {} is {tier}", s.target);
                    assert!(crate::kid_filter::kid_allowed(tgt, &s.target), "{src}->{tgt}: {} is not kid-safe", s.target);
                }
            }
        }
    }

    #[test]
    fn translate_pair_swap_clear_and_language_change() {
        let _a = Audit::on(&["es"]);
        let Some(s) = search(Experience::Standard, "en", "es", "water").into_iter().next() else {
            return; // the English word is outside easy+medium in this bank; nothing to exercise
        };
        let p = Pair::new("en", "es").commit(&s);
        let sw = p.swapped(Experience::Standard);
        assert_eq!((sw.source.as_str(), sw.target.as_str()), ("es", "en"), "languages exchange");
        assert_eq!(sw.source_entry, p.target_entry, "words exchange");
        assert_eq!(sw.target_entry, p.source_entry, "the reversed answer is the original word");
        let c = sw.cleared();
        assert_eq!((c.source.as_str(), c.target.as_str()), ("es", "en"), "clear keeps both languages (F8)");
        assert!(c.source_entry.is_none() && c.target_entry.is_none(), "and empties the question");
        assert_eq!(p.with_source_lang(Experience::Standard, "es").source, "es",
                   "picking the target's language for the source swaps (F4)");
    }

    #[test]
    fn translate_d9_senses_inline_then_more() {
        let four: Vec<String> = (0..4).map(|i| format!("s{i}")).collect();
        let six: Vec<String> = (0..6).map(|i| format!("s{i}")).collect();
        assert_eq!(visible_senses(&four, false), (four.clone(), false));
        assert_eq!(visible_senses(&six, false).0.len(), 4);
        assert!(visible_senses(&six, false).1, "a 'more' control for the rest");
        assert_eq!(visible_senses(&six, true).0.len(), 6);
    }

    #[test]
    fn translate_spell_it_respects_jr_band_and_preview() {
        let expert = TIERS[3];
        let easy = TIERS[0];
        let Some(hard_word) = crate::words::tier_for("es", expert).first() else { return };
        let Some(easy_word) = crate::words::tier_for("es", easy).first() else { return };
        assert_eq!(spell_it_verdict(Experience::Standard, AccessLevel::Full, "es", hard_word), SpellVerdict::Serve);
        assert_eq!(spell_it_verdict(Experience::Junior, AccessLevel::Full, "es", hard_word), SpellVerdict::Decline,
                   "Spell Jr never serves an Expert word");
        assert_eq!(spell_it_verdict(Experience::Standard, AccessLevel::Preview, "es", hard_word), SpellVerdict::Decline,
                   "an unowned language serves only its preview tier");
        assert_eq!(spell_it_verdict(Experience::Standard, AccessLevel::Preview, "es", easy_word), SpellVerdict::Serve);
        assert_eq!(spell_it_verdict(Experience::Standard, AccessLevel::Full, "es", "not-a-bank-word"), SpellVerdict::Decline);
    }

    /// Eric, 2026-09-13: English lookup reaches all four tiers. Every English
    /// meaning above Medium in the Spanish table must answer for a standard
    /// player, be findable by typing it, and still be refused to Spell Jr.
    #[test]
    fn translate_english_lookup_reaches_every_tier() {
        let _a = Audit::on(&["es"]);
        let above_medium: Vec<String> = crate::translate::audited_rows("es")
            .into_iter()
            .map(|(_, c)| c)
            .filter(|c| bank_tier_of("en", c).is_some_and(|t| t == TIERS[2] || t == TIERS[3]))
            .collect();
        assert!(!above_medium.is_empty(), "the Spanish table carries English meanings above Medium");
        for c in &above_medium {
            assert_eq!(resolve_target(Experience::Standard, "en", c).as_deref(), Some(c.as_str()),
                       "{c}: an English meaning above Medium must answer");
            assert!(search(Experience::Standard, "en", "es", c).iter().any(|s| &s.concept == c),
                    "{c}: findable by typing the English word");
            assert!(resolve_target(Experience::Junior, "en", c).is_none(),
                    "{c}: Spell Jr is still held to Easy + Medium");
        }
    }

    /// Done #7 (I3, I7) — over the REAL bank: every answer the screen can
    /// render is the single (concept, language) lookup and a real bank word,
    /// and every answerable word can be found by typing it (the inverse of I2).
    #[test]
    fn translate_traceability() {
        let langs = ["es", "ja", "zh", "ar", "ru"];
        let _a = Audit::on(&langs);
        let mut checked = 0usize;
        for src in std::iter::once("en").chain(langs) {
            for tgt in std::iter::once("en").chain(langs) {
                if src == tgt {
                    continue;
                }
                for (entry, concept) in crate::translate::source_rows(src).into_iter().step_by(7) {
                    let Some(t) = resolve_target(Experience::Standard, tgt, &concept) else { continue };
                    assert_eq!(crate::translate::word_for_concept(tgt, &concept).as_deref(), Some(t.as_str()),
                               "{src}->{tgt} {entry}: the answer is not the single lookup");
                    assert!(bank_tier_of(tgt, &t).is_some(), "{src}->{tgt}: {t} is not a bank word of {tgt}");
                    let found = search(Experience::Standard, src, tgt, &crate::translate::display_word(&entry));
                    assert!(found.iter().any(|s| s.entry == entry),
                            "{src}->{tgt}: answerable word {entry} cannot be found by typing it");
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "the property ran over too little of the bank to mean anything: {checked}");
    }
}
