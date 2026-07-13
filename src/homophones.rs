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
