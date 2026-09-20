//! Accept-any homophone equivalence for grading.
//!
//! Some spelling distinctions cannot be carried by the audio prompt — in
//! Spanish, `b/v`, a silent `h`, seseo (`s`/`z`/soft-`c`) and yeísmo (`y`/`ll`)
//! all sound identical, so a learner who spells a real homophone of the prompt
//! word should not be marked wrong for a difference their ear could never catch
//! (decision addendum, 2026-07).
//!
//! This is deliberately DATA-DRIVEN: the equivalence groups live in
//! `assets/words/<lang>/homophones.txt` (one group per line), never as
//! per-pair conditionals in the scoring code. Grading stays untouched — it just
//! consults [`accepts`] as an additional acceptance path when the strict/lenient
//! fold rejects. A language with no homophone file simply has an empty table and
//! this layer is a no-op.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::norm::fold_strict;

/// Per-language homophone data. Add a language by dropping in its file and a
/// matching arm here (mirrors the word-list `include_str!` pattern).
fn source(lang: &str) -> &'static str {
    match lang {
        c if c == crate::consts::ES => include_str!("../assets/words/es/homophones.txt"),
        // CC-SENSE-CUE F1/D6: these tables are COMPUTED by
        // tools/build_collisions.py, never hand-listed. The Spanish file was 16
        // hand-written groups, three of which named words that are not in the
        // bank at all -- accepting spellings for prompts that never occur.
        c if c == crate::consts::EN => include_str!("../assets/words/en/homophones.txt"),
        // Russian is the STRESS-INDEPENDENT key only (final devoicing and
        // assimilation). Vowel reduction is conditioned on stress, and applying
        // it blind merges grammatical inflections -- академии/академия -- which
        // is not the survivable direction of error. The full ru table waits on
        // src/ru_stress_data.rs going live, which waits on a human audit.
        c if c == crate::consts::RU => include_str!("../assets/words/ru/homophones.txt"),
        _ => "",
    }
}

/// Map of `fold_strict(member) -> group id` for one language, built once.
fn table(lang: &str) -> &'static HashMap<String, usize> {
    // A tiny cache keyed by language. Only a handful of languages ever carry a
    // homophone file, so a linear Vec is plenty and keeps this lock-free-simple.
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static HashMap<String, usize>>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(t) = cache.lock().unwrap().get(lang) {
        return t;
    }
    let mut map: HashMap<String, usize> = HashMap::new();
    for (gid, line) in source(lang).lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for member in line.split_whitespace() {
            map.insert(fold_strict(member), gid);
        }
    }
    // Leak the per-language map so we can hand out a 'static reference; there is
    // one small map per language for the process lifetime.
    let leaked: &'static HashMap<String, usize> = Box::leak(Box::new(map));
    cache.lock().unwrap().insert(lang.to_string(), leaked);
    leaked
}

/// The other spellings that sound like `word` (CC-WORDGRID F-C3 needs them to
/// tell whether a crossing can distinguish the two). Empty when the word is in
/// no collision group.
pub fn group_members(lang: &str, word: &str) -> Vec<String> {
    let key = fold_strict(word);
    let Some(&gid) = table(lang).get(&key) else { return Vec::new() };
    source(lang)
        .lines()
        .nth(gid)
        .map(|line| line.split_whitespace().filter(|m| fold_strict(m) != key).map(str::to_string).collect())
        .unwrap_or_default()
}

/// True if `typed` is an accepted homophone of the prompt `word` in `lang`
/// (both fall in the same equivalence group). Case/accent-normalized via
/// `fold_strict`, so it composes with the normal comparison.
pub fn accepts(lang: &str, word: &str, typed: &str) -> bool {
    let t = table(lang);
    if t.is_empty() {
        return false;
    }
    match (t.get(&fold_strict(word)), t.get(&fold_strict(typed))) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod phase_b_tests {
    use super::*;
    use crate::consts::{EN, RU};

    /// CC-SENSE-CUE Invariant 1: a learner is never told they are wrong for a
    /// real spelling the audio supports. English is the language this most
    /// affects and the one that had NO fairness layer -- submit_guess returned
    /// into the server round trip before ever reaching accepts().
    #[test]
    fn english_accepts_a_real_homophone_of_the_prompt() {
        for (prompt, typed) in [("pair", "pear"), ("pear", "pair"), ("pair", "pare"),
                                ("their", "there"), ("for", "four"), ("to", "too")] {
            assert!(accepts(EN, prompt, typed),
                    "{typed} is a real word the audio cannot distinguish from {prompt}");
        }
    }

    /// Not a free pass. A misspelling that merely resembles a homophone is
    /// still a miss -- acceptance is membership in a computed set, not fuzzy
    /// matching. This is CC-SENSE-CUE acceptance test 2.
    #[test]
    fn english_still_rejects_misspellings() {
        for (prompt, typed) in [("pair", "pare1"), ("their", "thier"),
                                ("for", "fore4"), ("pear", "peer")] {
            assert!(!accepts(EN, prompt, typed), "{typed} must remain a miss for {prompt}");
        }
    }

    /// The Russian table is the STRESS-INDEPENDENT key only. привезти/привести
    /// collide by devoicing alone; inflection pairs like академии/академия must
    /// NOT, because merging them needs stress data that is still dark.
    #[test]
    fn russian_is_devoicing_only_until_stress_data_is_audited() {
        assert!(accepts(RU, "привезти", "привести"), "з devoices to с before т");
        assert!(!accepts(RU, "академии", "академия"),
                "grammatical inflections are not homophones -- that merge needs stress");
    }

    /// Every group is internally consistent: each member accepts every other,
    /// in both directions. F2 forbids a half-covered set.
    #[test]
    fn every_generated_set_is_symmetric_and_complete() {
        for lang in [EN, RU, crate::consts::ES] {
            for line in source(lang).lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                let ms: Vec<&str> = line.split_whitespace().collect();
                for a in &ms {
                    for b in &ms {
                        assert!(accepts(lang, a, b), "{lang}: {a} must accept {b}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::ES;

    #[test]
    fn accepts_confirmed_pairs_either_direction() {
        // Eric-confirmed accept-any pairs: typing the twin scores correct.
        assert!(accepts(ES, "casa", "caza"));
        assert!(accepts(ES, "caza", "casa"));
        assert!(accepts(ES, "botar", "votar"));
        assert!(accepts(ES, "cocer", "coser"));
    }

    #[test]
    fn accepts_exact_prompt_too() {
        // The prompt spelled correctly is still in its own group.
        assert!(accepts(ES, "casa", "casa"));
    }

    #[test]
    fn rejects_non_homophones() {
        assert!(!accepts(ES, "casa", "gato"));
        assert!(!accepts(ES, "casa", "casas"));
    }

    #[test]
    fn no_table_is_noop() {
        // Languages without a homophone file never accept a substitution.
        assert!(!accepts("en", "casa", "caza"));
        assert!(!accepts("fr", "botar", "votar"));
    }

    #[test]
    fn combines_with_strict_fold() {
        // The whole point: the strict fold rejects casa/caza, homophones accept.
        assert!(!crate::norm::answer_matches("caza", "casa", false));
        assert!(accepts(ES, "casa", "caza"));
    }
}
