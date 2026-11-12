//! Accept-any homophone equivalence for grading.
//!
//! Some spelling distinctions cannot be carried by the audio prompt — in
//! Spanish, `b/v`, a silent `h`, seseo (`s`/`z`/soft-`c`) and yeísmo (`y`/`ll`)
//! all sound identical, so a learner who spells a real homophone of the prompt
//! word should not be marked wrong for a difference their ear could never catch
//! (decision addendum, 2026-07).
//!
//! This is deliberately DATA-DRIVEN: the equivalence groups live in
//! the generated sound-alike tables (read only through `crate::soundalikes`), never as
//! per-pair conditionals in the scoring code. Grading stays untouched — it just
//! consults [`accepts`] as an additional acceptance path when the strict/lenient
//! fold rejects. A language whose table has no sets simply accepts nothing extra, and
//! this layer is a no-op.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::norm::fold_strict;

/// The sets this layer accepts: CC-SENSE-CUE v1.1 F7 says grading reads
/// `kind: "true"` sets only, through the one API in `crate::soundalikes`. The
/// tables are COMPUTED by tools/build_collisions.py, never hand-listed
/// (CC-SENSE-CUE F1/D6). Russian is still the STRESS-INDEPENDENT key only
/// (devoicing and assimilation); vowel reduction waits on src/ru_stress_data.rs
/// going live, which waits on a human audit.
fn sets(lang: &str) -> Vec<&'static crate::soundalikes::SoundalikeSet> {
    crate::soundalikes::sets_of_kind(lang, &["true"])
}

/// Map of `fold_strict(member) -> indices into sets(lang)` for one language, built
/// once. A word can sit in SEVERAL sets: English sets are unmerged groups of words
/// sharing one pronunciation, and a word with two pronunciations joins two groups
/// (2026-10-17; merging them used to chain air/or/hour into one "sound").
fn table(lang: &str) -> &'static HashMap<String, Vec<usize>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static HashMap<String, Vec<usize>>>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    // Keyed on the table actually in use, not the language: with en.retier on,
    // English reads a different table (crate::soundalikes::active_key).
    let key = crate::soundalikes::active_key(lang);
    if let Some(t) = cache.lock().unwrap().get(&key) {
        return t;
    }
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (gid, set) in sets(lang).iter().enumerate() {
        for m in &set.members {
            let ids = map.entry(fold_strict(&m.member_id)).or_default();
            if !ids.contains(&gid) {
                ids.push(gid);
            }
        }
    }
    // Leak the per-language map so we can hand out a 'static reference; there is
    // one small map per language for the process lifetime.
    let leaked: &'static HashMap<String, Vec<usize>> = Box::leak(Box::new(map));
    cache.lock().unwrap().insert(key, leaked);
    leaked
}

/// The other spellings that sound like `word` (CC-WORDGRID F-C3 needs them to
/// tell whether a crossing can distinguish the two): every member of every set
/// the word is in, once each, in set order. Empty when the word is in no set.
pub fn group_members(lang: &str, word: &str) -> Vec<String> {
    let key = fold_strict(word);
    let Some(gids) = table(lang).get(&key) else { return Vec::new() };
    let all = sets(lang);
    let mut out: Vec<String> = Vec::new();
    for &gid in gids {
        if let Some(set) = all.get(gid) {
            for m in &set.members {
                if fold_strict(&m.member_id) != key && !out.contains(&m.member_id) {
                    out.push(m.member_id.clone());
                }
            }
        }
    }
    out
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
        // Accepted only when the two share a set: a shared pronunciation, not a
        // chain of them. A target-only set (F6 override) is fair in one direction:
        // only when the prompt is one of its target members.
        (Some(a), Some(b)) => {
            let all = sets(lang);
            let prompt = fold_strict(word);
            a.iter().filter(|g| b.contains(g)).any(|&g| {
                all.get(g).is_some_and(|s| s.target_only.is_empty() || s.target_only.iter().any(|t| fold_strict(t) == prompt))
            })
        }
        _ => false,
    }
}

// ---- CC-DE-BANK F1: bank homophone groups with carrier phrases -------------------
//
// One line per target word in `assets/words/<lang>/homophone-groups.tsv`:
//     target <TAB> other,other <TAB> carrier        (carrier may be empty)
// Written by tools/bank/de/de_ingest.py from the auditor's Same Sound column
// (Keep rows only); carriers are written by a human, never generated (no LLM
// text in bank content). Standard and expert disambiguate with the carrier; Kid
// Mode accepts any spelling in the group.

/// One bank homophone entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankGroup {
    pub target: String,
    pub others: Vec<String>,
    pub carrier: String,
}

fn parse_groups(src: &str) -> Vec<BankGroup> {
    src.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.split('\t');
            let target = c.next()?.trim().to_string();
            let others: Vec<String> = c.next()?.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            let carrier = c.next().unwrap_or("").trim().to_string();
            (!target.is_empty() && !others.is_empty()).then_some(BankGroup { target, others, carrier })
        })
        .collect()
}

fn group_source(lang: &str) -> &'static str {
    match lang {
        c if c == crate::consts::DE => include_str!("../assets/words/de/homophone-groups.tsv"),
        _ => "",
    }
}

fn bank_groups(lang: &str) -> &'static Vec<BankGroup> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static Vec<BankGroup>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(g) = cache.lock().unwrap().get(lang) {
        return g;
    }
    let leaked: &'static Vec<BankGroup> = Box::leak(Box::new(parse_groups(group_source(lang))));
    cache.lock().unwrap().insert(lang.to_string(), leaked);
    leaked
}

/// The bank entry whose TARGET is `word` (the form the key holds).
pub fn bank_entry(lang: &str, word: &str) -> Option<&'static BankGroup> {
    let k = fold_strict(word);
    bank_groups(lang).iter().find(|g| fold_strict(&g.target) == k)
}

/// Kid Mode: `typed` is another member of `word`'s bank homophone group.
pub fn bank_group_accepts(lang: &str, word: &str, typed: &str) -> bool {
    let t = fold_strict(typed);
    bank_entry(lang, word).is_some_and(|g| g.others.iter().any(|o| fold_strict(o) == t))
}

/// The carrier phrase to play after `word`, when it has homophones and one is written.
pub fn carrier(lang: &str, word: &str) -> Option<&'static str> {
    bank_entry(lang, word).map(|g| g.carrier.as_str()).filter(|c| !c.is_empty())
}

/// Standard and expert serving: a word with homophones but no carrier is never
/// served, because the player could not tell which spelling is meant.
pub fn serves_standard(lang: &str, word: &str) -> bool {
    bank_entry(lang, word).map_or(true, |g| !g.carrier.is_empty())
}

/// The build gate (CC-DE-BANK F1): every entry with homophones needs a carrier
/// WHILE it can be served in standard mode. `serves` says whether it can. Returns
/// the offending targets; empty means the bank may build.
pub fn gate_violations(groups: &[BankGroup], serves: impl Fn(&str) -> bool) -> Vec<String> {
    groups.iter().filter(|g| !g.others.is_empty() && g.carrier.is_empty() && serves(&g.target)).map(|g| g.target.clone()).collect()
}

/// Entries excluded from standard serving for want of a carrier (the report list).
pub fn awaiting_carrier(lang: &str) -> Vec<&'static str> {
    bank_groups(lang).iter().filter(|g| g.carrier.is_empty()).map(|g| g.target.as_str()).collect()
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
    /// F6 overrides from Eric's same-sound sheet (2026-10-17): target-only sets
    /// accept in one direction, dropped members are gone, and an auditor "no"
    /// stops acceptance -- except for curated variety pairs (license/licence).
    #[test]
    fn auditor_overrides_shape_acceptance() {
        assert!(accepts(EN, "red", "read"), "read (past tense) typed for red is fair");
        assert!(!accepts(EN, "read", "red"), "the voice says a lone 'read' as 'reed'");
        assert!(accepts(EN, "bough", "bow") && !accepts(EN, "bow", "bough"));
        assert!(accepts(EN, "fir", "fur") && !accepts(EN, "fir", "for"), "for was dropped from fir/fur");
        assert!(!accepts(EN, "asked", "ast"), "ast is not a word");
        assert!(!accepts(EN, "device", "devise"), "s vs z");
        assert!(accepts(EN, "license", "licence"), "variety pair stays accepted");
    }

    /// Un-chaining (2026-10-17): a word with two pronunciations sits in two sets
    /// and matches both, but two words that share no pronunciation do not match
    /// through it. err is EH R (air) and ER (or); air and or are not alike.
    #[test]
    fn english_sets_do_not_chain() {
        // err/or was later separated by Eric's sheet ("no" on are/er/err/or/ur), so
        // only err/air is still a shared-pronunciation pair here.
        assert!(accepts(EN, "err", "air"), "err still matches air");
        assert!(!accepts(EN, "air", "or"), "air and or share no pronunciation");
        assert!(!accepts(EN, "air", "hour"), "air and hour share no pronunciation");
        assert!(accepts(EN, "hour", "our"), "a real pair stays");
    }

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
            for set in sets(lang) {
                let ms: Vec<&str> = set.members.iter().map(|m| m.member_id.as_str()).collect();
                for a in &ms {
                    // A target-only set (F6) is complete from its targets only.
                    if !set.target_only.is_empty() && !set.target_only.iter().any(|t| t == a) {
                        continue;
                    }
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
    fn german_seed_sets_are_accepted_while_the_table_stays_missing() {
        // CC-SENSE-CUE v1.2: de has no generator, but its curated seed pairs grade
        // accept-both, and the table stays "missing" (German Impossible runs on its own exclusion data).
        for (a, b) in [("wahren", "waren"), ("wider", "wieder"), ("stadt", "statt"), ("endstand", "entstand"), ("booten", "boten"),
            ("reist", "reißt"), ("leeren", "lehren"), ("fielen", "vielen"), ("Ligen", "liegen"), ("Rad", "Rat"), ("Tod", "tot"),
            ("Lid", "Lied"), ("seid", "seit"), ("Wal", "Wahl"), ("Stiel", "Stil"), ("Miene", "Mine"), ("mehr", "Meer"),
            ("malen", "mahlen"), ("Saite", "Seite"), ("Leib", "Laib"), ("Waise", "Weise")] {
            assert!(accepts("de", a, b) && accepts("de", b, a), "{a}/{b}");
        }
        assert_eq!(crate::soundalikes::soundalikes("de").status, "missing");
        // Rulings: ae pairs and weisen/weissen stay out (accent at most), and Mohr is never seeded.
        assert!(!accepts("de", "weisen", "weißen") && !accepts("de", "Moor", "Mohr"));
        assert!(!accepts("de", "Dänen", "denen"));
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
        // Languages whose table has no sets never accept a substitution.
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

#[cfg(test)]
mod de_bank_tests {
    use super::*;
    use crate::consts::DE;
    use crate::norm::{answer_matches_in, case_only_miss};

    #[test]
    fn gate_fails_without_a_carrier_and_passes_once_excluded_or_written() {
        let bare = vec![BankGroup { target: "Mann".into(), others: vec!["man".into()], carrier: String::new() }];
        assert_eq!(gate_violations(&bare, |_| true), vec!["Mann".to_string()], "served ambiguous: the build fails");
        assert!(gate_violations(&bare, |_| false).is_empty(), "excluded from standard serving: passes");
        let written = vec![BankGroup { carrier: "der Mann, nicht man".into(), ..bare[0].clone() }];
        assert!(gate_violations(&written, |_| true).is_empty(), "carrier written: passes");
    }

    /// The shipped bank passes its own gate: serving agrees with the carrier column.
    #[test]
    fn shipped_german_bank_passes_the_gate() {
        assert!(gate_violations(bank_groups(DE), |w| serves_standard(DE, w)).is_empty());
        for g in bank_groups(DE) {
            assert_eq!(serves_standard(DE, &g.target), !g.carrier.is_empty(), "{}", g.target);
        }
    }

    #[test]
    fn seeded_pairs_are_present() {
        for (t, o) in [("Mann", "man"), ("weist", "weißt"), ("heute", "Häute"), ("sechs", "Sex")] {
            assert!(bank_group_accepts(DE, t, o), "{t}/{o}");
        }
    }

    /// Acceptance 6: the answer key stays the one target spelling in standard play.
    #[test]
    fn homophone_grading_standard_is_strict_kid_accepts_the_group() {
        assert!(!answer_matches_in(DE, "man", "Mann", false), "standard: man for Mann is wrong");
        assert!(answer_matches_in(DE, "Mann", "Mann", false));
        // Kid: the strict fold rejects it, and the group layer (game.rs) accepts it, showing Mann.
        assert!(bank_group_accepts(DE, "Mann", "man"));
        assert!(!bank_group_accepts(DE, "Mann", "Hund"), "not a free pass");
    }

    /// Acceptance 7. `answer_matches_in` folds case for every shared surface;
    /// `case_ok` is the spell game's standard/expert layer on top of it.
    #[test]
    fn case_grading_standard_is_exact_kid_forgives_with_a_hint() {
        let std = |t: &str, w: &str| answer_matches_in(DE, t, w, false) && crate::norm::case_ok(DE, t, w);
        assert!(!std("licht", "Licht"), "standard: licht for Licht is wrong");
        assert!(std("Licht", "Licht"));
        assert!(!std("LICHT", "Licht"));
        assert!(answer_matches_in(DE, "licht", "Licht", true), "Kid: case-insensitive");
        assert!(case_only_miss(DE, "licht", "Licht"), "and the hint is owed");
        assert!(!case_only_miss(DE, "Licht", "Licht"));
        assert!(!case_only_miss(DE, "lickt", "Licht"), "a real misspelling is not a case miss");
        // A language where case carries no meaning is untouched.
        assert!(crate::norm::case_ok(crate::consts::EN, "Cat", "cat"));
        assert!(!case_only_miss(crate::consts::EN, "Cat", "cat"));
    }

    #[test]
    fn german_alternates_do_not_slip_past_case() {
        assert!(!crate::norm::case_ok(DE, "xylofon", "Xylophon"), "wrong-case alternate");
        assert!(crate::norm::case_ok(DE, "Xylofon", "Xylophon"));
        assert!(crate::norm::case_ok(DE, "Maedchen", "Mädchen"), "a stand-in keeps its capital");
    }
}
