//! CC-IMPOSTOR F2 — the distractor generator.
//!
//! Hear the word, see four spellings, tap the real one. The three wrong cards
//! are GENERATED (D1) — never authored — so the mode needs no new content and
//! cannot drift from the bank.
//!
//! # Why the transforms are not the trap registry
//!
//! D1 says distractors come from "the per-language trap-class rule tables (the
//! same data behind tier calibration)". Those tables cover `ru` and `ar` only,
//! two of fifteen, and they are MATCHERS — `contains_any`, `regex`,
//! `final_char_any` — which answer "does this word contain a trap?" and never
//! "how do I corrupt it plausibly?". Several classes are `manual` by design:
//! Russian's largest error class needs stress position, and the registry says
//! outright that a surface-form lexicon cannot supply it (вода is a trap, роза
//! is not, identical letters).
//!
//! Eric's ruling was option B. Every language gets the SHAPE transforms below,
//! built from its own typing units; English additionally gets real
//! [`Trap`] classes, so the language he can review has D5's intended learning
//! moment ("oh, silent H!") while the rest degrade to a shape label until their
//! tables are authored. That degradation is already how D5 handles audit: a
//! language without cleared strings shows the highlight and no quip, never
//! machine-fallback text.
//!
//! # Fairness is the non-negotiable part (D3)
//!
//! A distractor that is a real word somewhere the player might study is a
//! fairness bug no audit catches, so the generator CHECKS rather than hopes:
//! every candidate is rejected if it is valid in the target language or in any
//! lineup language sharing its script. Measured over the real banks, this
//! excludes 0.0% of words in fourteen languages and 0.2% in Japanese — far
//! under D3's 2% flag threshold.

use crate::norm::fold_strict;

/// What was done to the real word. The label the reveal-why moment reads (D5).
// Ord is derived so the candidate pool sorts to ONE canonical order before
// anything is picked from it. Without a total order the pool's order would
// depend on the sequence transforms happened to be pushed in, and I2's
// determinism would hold only by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trap {
    // ---- shape transforms, every language ----
    /// Two adjacent units swapped.
    Transpose,
    /// A unit doubled.
    Double,
    /// A unit dropped.
    Drop,
    /// A unit replaced by another from the language's own alphabet.
    Substitute,
    // ---- English trap classes, authored (D1 option B) ----
    /// Silent letter removed: knife -> nife.
    SilentLetter,
    /// The i-before-e muddle: believe -> beleive.
    IeEi,
    /// A doubled consonant single, or a single one doubled.
    DoubleConsonant,
    /// -ance / -ence, -able / -ible: existence -> existance.
    SuffixVowel,
    /// ph -> f, and other same-sound spellings: phone -> fone.
    Homophone,
}

impl Trap {
    /// The i18n key for D5's one-line label. English trap classes get their own
    /// line; shape transforms share four generic ones.
    pub fn label_key(self) -> &'static str {
        match self {
            Trap::Transpose => "imp.trap.transpose",
            Trap::Double => "imp.trap.double",
            Trap::Drop => "imp.trap.drop",
            Trap::Substitute => "imp.trap.substitute",
            Trap::SilentLetter => "imp.trap.silent",
            Trap::IeEi => "imp.trap.ieei",
            Trap::DoubleConsonant => "imp.trap.doubles",
            Trap::SuffixVowel => "imp.trap.suffix",
            Trap::Homophone => "imp.trap.homophone",
        }
    }

    /// D2's families: Hard draws all three distractors from ONE family, so the
    /// player cannot win by noticing that only one card was mangled differently.
    pub fn family(self) -> u8 {
        match self {
            Trap::Transpose | Trap::Double | Trap::Drop => 0, // length/order
            Trap::Substitute | Trap::Homophone => 1,          // letter identity
            Trap::SilentLetter | Trap::IeEi | Trap::DoubleConsonant | Trap::SuffixVowel => 2,
        }
    }
}

/// D2. Kid Mode caps at Medium; the resolver is [`effective_tier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// I5 — Kid Mode never receives hard or expert rounds. One place decides, so no
/// caller can forget.
pub fn effective_tier(t: Tier, kid: bool) -> Tier {
    match (t, kid) {
        (Tier::Hard, true) | (Tier::Expert, true) => Tier::Medium,
        _ => t,
    }
}

/// One playable round: four cards, one of them real.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub word: String,
    /// Card faces in display order. Exactly one equals `word`.
    pub cards: Vec<String>,
    pub correct: usize,
    /// Aligned with `cards`; the correct card's entry is `None`.
    pub traps: Vec<Option<Trap>>,
}

impl Round {
    /// The trap the player fell for, for D5's reveal and D6's analytics.
    pub fn trap_at(&self, pick: usize) -> Option<Trap> {
        self.traps.get(pick).copied().flatten()
    }
}

/// The shortest word worth building a round from.
///
/// Too short and every transform lands in the same place, so all four cards
/// read as noise — the Russian two-letter round was цк / цкк / цо / кц.
///
/// The floor is PER SCRIPT because "short" is not a universal length. A first
/// pass used four units everywhere, which is right for an alphabet and absurd
/// for a syllabary: it excluded 43.7% of Japanese and 54.3% of Korean, where a
/// three-character word is perfectly ordinary. One kana or one Hangul block
/// carries about as much information as a Latin syllable, so those scripts get
/// a floor of two.
///
/// Calibrated against D3's exclusion report rather than guessed. Four units for
/// alphabets put eleven of fifteen languages over the 2% flag — Vietnamese at
/// 13.3% and Arabic at 8.3%, both of which simply have short words. Three units
/// reads fine (дом gives дмо / доо / домм) and leaves only Korean above the
/// line, at 3.0%.
fn min_units(lang: &str) -> usize {
    match lang {
        "ja" | "ko" => 2,
        _ => 3,
    }
}

fn split(w: &str) -> Vec<String> {
    w.chars().map(|c| c.to_string()).collect()
}

/// The units a substitution may introduce.
///
/// Normally the language's own keyboard rows, so a swapped letter is one the
/// player could type. Korean is the exception and it MATTERS: its keyboard rows
/// are jamo, but its words are written in composed syllable BLOCKS, so
/// substituting from the rows produced cards like 서ㅕ농공업고등학교 — a bare
/// jamo sitting among blocks, wrong at a glance and worthless as a distractor.
/// For Korean the alphabet is the blocks the bank actually uses.
fn alphabet(lang: &str) -> Vec<String> {
    if lang == "ko" {
        return korean_blocks();
    }
    let mut out: Vec<String> = Vec::new();
    for row in crate::keyboard::unit_rows(lang) {
        for c in row.chars() {
            // zh types pinyin plus a tone-digit row. A digit substituted into a
            // letter position gives shu1sa4n; a digit transposed to the front
            // gives 4hu1san4. Neither is a spelling anyone would produce.
            if c.is_ascii_digit() {
                continue;
            }
            let s = c.to_string();
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out
}

/// The 120 most common syllable blocks in the Korean bank — enough variety for
/// a distractor, cheap to scan, and every one a real block a reader recognises.
fn korean_blocks() -> Vec<String> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    for i in 0..2000usize {
        let Some(w) = crate::word_index::word_at("ko", i) else { break };
        for c in w.chars() {
            match seen.iter_mut().find(|(s, _)| s.chars().next() == Some(c)) {
                Some((_, n)) => *n += 1,
                None => seen.push((c.to_string(), 1)),
            }
        }
    }
    seen.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    seen.into_iter().take(120).map(|(s, _)| s).collect()
}

// ---------------------------------------------------------------- english

const SILENT: [(&str, &str); 8] = [
    ("kn", "n"), ("wr", "r"), ("gh", ""), ("mb", "m"),
    ("ps", "s"), ("wh", "w"), ("bt", "t"), ("mn", "m"),
];
const HOMOPHONE: [(&str, &str); 7] = [
    ("ph", "f"), ("tion", "sion"), ("ough", "uff"), ("ce", "se"),
    ("ck", "k"), ("qu", "kw"), ("x", "ks"),
];
const SUFFIX: [(&str, &str); 4] =
    [("ence", "ance"), ("ance", "ence"), ("able", "ible"), ("ible", "able")];

/// English trap transforms, in D5's sense: each produces the misspelling a
/// learner actually makes, and carries a label that teaches the rule.
fn english_candidates(w: &str) -> Vec<(String, Trap)> {
    let mut out = Vec::new();
    for (from, to) in SILENT {
        if w.contains(from) {
            out.push((w.replacen(from, to, 1), Trap::SilentLetter));
        }
    }
    for (from, to) in HOMOPHONE {
        if w.contains(from) {
            out.push((w.replacen(from, to, 1), Trap::Homophone));
        }
    }
    for (from, to) in SUFFIX {
        if w.ends_with(from) {
            let stem = &w[..w.len() - from.len()];
            out.push((format!("{stem}{to}"), Trap::SuffixVowel));
        }
    }
    if w.contains("ie") {
        out.push((w.replacen("ie", "ei", 1), Trap::IeEi));
    }
    if w.contains("ei") {
        out.push((w.replacen("ei", "ie", 1), Trap::IeEi));
    }
    // Doubled consonant made single, and single consonants doubled.
    let cs: Vec<char> = w.chars().collect();
    for i in 0..cs.len().saturating_sub(1) {
        if cs[i] == cs[i + 1] && !"aeiou".contains(cs[i]) {
            let mut v = cs.clone();
            v.remove(i);
            out.push((v.into_iter().collect(), Trap::DoubleConsonant));
        }
    }
    for i in 0..cs.len() {
        if !"aeiou".contains(cs[i]) && (i + 1 >= cs.len() || cs[i + 1] != cs[i]) {
            let mut v = cs.clone();
            v.insert(i, cs[i]);
            out.push((v.into_iter().collect(), Trap::DoubleConsonant));
        }
    }
    out
}

/// Shape transforms, available in every language.
fn shape_candidates(lang: &str, w: &str, alpha: &[String]) -> Vec<(String, Trap)> {
    let u = split(w);
    // A pinyin tone digit is structure, not a letter: moving it produces
    // 4hu1san4 and doubling it produces e44. Positions holding one are frozen.
    let movable = |i: usize| -> bool {
        !u[i].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
    };
    let mut out = Vec::new();
    for i in 0..u.len().saturating_sub(1) {
        if u[i] != u[i + 1] && movable(i) && movable(i + 1) {
            let mut v = u.clone();
            v.swap(i, i + 1);
            out.push((v.concat(), Trap::Transpose));
        }
    }
    for i in 0..u.len() {
        if !movable(i) {
            continue;
        }
        let mut v = u.clone();
        v.insert(i, u[i].clone());
        out.push((v.concat(), Trap::Double));
        if u.len() > 2 {
            let mut v = u.clone();
            v.remove(i);
            out.push((v.concat(), Trap::Drop));
        }
        for a in alpha {
            if *a != u[i] {
                let mut v = u.clone();
                v[i] = a.clone();
                out.push((v.concat(), Trap::Substitute));
            }
        }
    }
    let _ = lang;
    out
}

// ---------------------------------------------------------------- fairness

/// Languages sharing a script, for D3's cross-language validity screen.
///
/// zh sits in the Latin group because its typed form is pinyin. Its entries
/// carry tone digits so a collision with an English word is unlikely, but "the
/// generator must check, not hope" is the whole point of D3.
fn same_script(lang: &str) -> &'static [&'static str] {
    const LATIN: &[&str] = &["en", "es", "fr", "de", "pt", "pl", "vi", "fil", "sw", "zh"];
    match lang {
        "ru" => &["ru"],
        "ar" => &["ar"],
        "hi" => &["hi"],
        "ja" => &["ja"],
        "ko" => &["ko"],
        _ => LATIN,
    }
}

/// D3: a distractor may not be a real word anywhere in its script group.
fn is_fair(lang: &str, cand: &str, real: &str) -> bool {
    if cand.is_empty() || cand == real {
        return false;
    }
    !same_script(lang).iter().any(|l| crate::word_index::is_valid(l, cand))
}

// ---------------------------------------------------------------- rounds

fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// D1: seeded by (word, roundIndex) so a round is reproducible in a test or a
/// bug report; D2: the tier decides how close the three wrong cards sit.
///
/// Returns `None` when three fair distractors cannot be found — D3's excluded
/// word. Measured, that is 0.0% of words in fourteen languages and 0.2% in ja.
pub fn round(lang: &str, word: &str, index: u64, tier: Tier) -> Option<Round> {
    let real = fold_strict(word.split('|').next().unwrap_or(word));
    if real.chars().count() < min_units(lang) {
        return None;
    }
    let alpha = alphabet(lang);
    let mut pool: Vec<(String, Trap)> = Vec::new();
    if lang == "en" {
        pool.extend(english_candidates(&real));
    }
    pool.extend(shape_candidates(lang, &real, &alpha));

    // Fairness screen first: everything below chooses from LEGAL cards only.
    pool.retain(|(c, _)| is_fair(lang, c, &real));
    pool.sort();
    pool.dedup();
    if pool.len() < 3 {
        return None;
    }

    // D2 — distance. Easy keeps one near-miss and two visibly-wrong cards;
    // Expert allows only single-unit differences; Hard draws one family so no
    // card stands out by its KIND of wrongness.
    let near = |c: &str| dist(&real, c) <= 1;
    let chosen: Vec<(String, Trap)> = match tier {
        Tier::Easy => {
            let mut v: Vec<(String, Trap)> = Vec::new();
            if let Some(n) = pick(&pool, index, |c| near(&c.0)) {
                v.push(n);
            }
            v.extend(take(&pool, index + 1, 3 - v.len(), |c| !near(&c.0) || v.is_empty()));
            v
        }
        Tier::Medium => {
            // Different families, so the three wrongs do not rhyme.
            let mut v: Vec<(String, Trap)> = Vec::new();
            for fam in [0u8, 1, 2, 0, 1, 2] {
                if v.len() == 3 {
                    break;
                }
                if let Some(c) =
                    pick(&pool, index + fam as u64, |c| c.1.family() == fam && !v.contains(c))
                {
                    v.push(c);
                }
            }
            v.extend(take(&pool, index, 3usize.saturating_sub(v.len()), |c| !v.contains(c)));
            v
        }
        Tier::Hard => {
            let fam = (mix(index) % 3) as u8;
            let mut v = take(&pool, index, 3, |c| c.1.family() == fam);
            if v.len() < 3 {
                v = take(&pool, index, 3, |_| true);
            }
            v
        }
        Tier::Expert => {
            let mut v = take(&pool, index, 3, |c| dist(&real, &c.0) <= 1);
            if v.len() < 3 {
                v = take(&pool, index, 3, |_| true);
            }
            v
        }
    };
    if chosen.len() < 3 {
        return None;
    }

    // Place the real card deterministically among the four.
    let slot = (mix(index ^ 0xA5A5) % 4) as usize;
    let mut cards = Vec::with_capacity(4);
    let mut traps = Vec::with_capacity(4);
    let mut it = chosen.into_iter();
    for i in 0..4 {
        if i == slot {
            cards.push(real.clone());
            traps.push(None);
        } else if let Some((c, t)) = it.next() {
            cards.push(c);
            traps.push(Some(t));
        }
    }
    Some(Round { word: real, cards, correct: slot, traps })
}

/// Unit-level edit distance, capped — only "is this within one edit" is asked.
fn dist(a: &str, b: &str) -> usize {
    let (x, y) = (split(a), split(b));
    if x.len().abs_diff(y.len()) > 1 {
        return 2;
    }
    if x.len() == y.len() {
        let d = x.iter().zip(&y).filter(|(p, q)| p != q).count();
        // A transposition is two differing positions but one edit to the eye.
        return if d == 2 { 1 } else { d };
    }
    let (long, short) = if x.len() > y.len() { (&x, &y) } else { (&y, &x) };
    for i in 0..long.len() {
        let mut v = (*long).clone();
        v.remove(i);
        if v == **short {
            return 1;
        }
    }
    2
}

fn pick<F: Fn(&(String, Trap)) -> bool>(
    pool: &[(String, Trap)],
    seed: u64,
    f: F,
) -> Option<(String, Trap)> {
    let ok: Vec<&(String, Trap)> = pool.iter().filter(|c| f(c)).collect();
    if ok.is_empty() {
        return None;
    }
    Some(ok[(mix(seed) % ok.len() as u64) as usize].clone())
}

fn take<F: Fn(&(String, Trap)) -> bool>(
    pool: &[(String, Trap)],
    seed: u64,
    n: usize,
    f: F,
) -> Vec<(String, Trap)> {
    let ok: Vec<&(String, Trap)> = pool.iter().filter(|c| f(c)).collect();
    let mut out: Vec<(String, Trap)> = Vec::new();
    if ok.is_empty() {
        return out;
    }
    let mut s = seed;
    let mut guard = 0;
    while out.len() < n && guard < 200 {
        s = mix(s);
        let c = ok[(s % ok.len() as u64) as usize].clone();
        if !out.contains(&c) {
            out.push(c);
        }
        guard += 1;
    }
    out
}

/// A deterministic ten-round set (D4). Words come from the bank by index, so
/// the same seed reproduces the same set on every platform (I2).
pub fn set_of_ten(lang: &str, seed: u64, tier: Tier) -> Vec<Round> {
    let n = crate::word_index::vocabulary_size(lang);
    if n == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 0u64;
    while out.len() < 10 && i < 400 {
        let at = (mix(seed ^ i) % n as u64) as usize;
        if let Some(w) = crate::word_index::word_at(lang, at) {
            if let Some(r) = round(lang, &w, seed ^ i, tier) {
                out.push(r);
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANGS: [&str; 15] = [
        "en", "es", "fr", "de", "pt", "pl", "ru", "vi", "fil", "sw", "ja", "ko", "zh", "hi", "ar",
    ];

    /// I1 — the fairness invariant. Not one rendered distractor may be a real
    /// word in its script group. This is the assertion the mode's credibility
    /// rests on, so it walks real rounds rather than sampling transforms.
    #[test]
    fn no_distractor_is_ever_a_real_word() {
        for lang in LANGS {
            let mut checked = 0;
            for seed in 0..12u64 {
                for r in set_of_ten(lang, seed, Tier::Medium) {
                    assert_eq!(r.cards.len(), 4, "{lang}: a round must show four cards");
                    assert_eq!(r.cards[r.correct], r.word);
                    for (i, c) in r.cards.iter().enumerate() {
                        if i == r.correct {
                            continue;
                        }
                        for l in same_script(lang) {
                            assert!(
                                !crate::word_index::is_valid(l, c),
                                "{lang}: distractor {c:?} is a real {l} word"
                            );
                        }
                        checked += 1;
                    }
                }
            }
            assert!(checked > 50, "{lang}: only {checked} distractors examined");
        }
    }

    /// I2 — same (word, index, tier) gives the same cards, every time.
    #[test]
    fn rounds_are_deterministic() {
        for lang in ["en", "ja", "ru"] {
            for seed in [0u64, 5, 77] {
                let a = set_of_ten(lang, seed, Tier::Hard);
                let b = set_of_ten(lang, seed, Tier::Hard);
                assert_eq!(a, b, "{lang}: set {seed} is not reproducible");
            }
        }
    }

    /// The correct card moves around — a fixed slot would be learnable in two
    /// rounds and the mode would test nothing.
    #[test]
    fn the_real_card_is_not_always_in_the_same_place() {
        let seen: std::collections::BTreeSet<usize> = (0..40u64)
            .flat_map(|s| set_of_ten("en", s, Tier::Medium).into_iter().map(|r| r.correct))
            .collect();
        assert_eq!(seen.len(), 4, "the answer only ever appeared in {seen:?}");
    }

    /// I5 — Kid Mode never sees hard or expert.
    #[test]
    fn kid_mode_is_capped_at_medium() {
        assert_eq!(effective_tier(Tier::Hard, true), Tier::Medium);
        assert_eq!(effective_tier(Tier::Expert, true), Tier::Medium);
        assert_eq!(effective_tier(Tier::Easy, true), Tier::Easy);
        assert_eq!(effective_tier(Tier::Expert, false), Tier::Expert);
    }

    /// D2 — Expert shows only single-unit differences, so the tier means
    /// something. Easy must NOT be all near-misses, or it is not easy.
    #[test]
    fn tiers_differ_in_distance() {
        let expert: Vec<usize> = set_of_ten("en", 3, Tier::Expert)
            .iter()
            .flat_map(|r| {
                r.cards
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != r.correct)
                    .map(|(_, c)| dist(&r.word, c))
                    .collect::<Vec<_>>()
            })
            .collect();
        let close = expert.iter().filter(|d| **d <= 1).count();
        assert!(
            close * 10 >= expert.len() * 8,
            "expert should be nearly all one-edit cards, got {close}/{}",
            expert.len()
        );
    }

    /// D1 option B — English carries real trap classes, not just shapes, so the
    /// reveal-why moment can teach a rule.
    #[test]
    fn english_uses_authored_trap_classes() {
        let got: std::collections::BTreeSet<Trap> = (0..30u64)
            .flat_map(|s| set_of_ten("en", s, Tier::Medium))
            .flat_map(|r| r.traps.into_iter().flatten())
            .collect();
        assert!(
            got.iter().any(|t| t.family() == 2),
            "no authored English trap class appeared, only shapes: {got:?}"
        );
    }

    /// Every trap has a label key — D5 reads one for the miss that just
    /// happened, and a missing key would surface as a raw identifier.
    #[test]
    fn every_trap_has_a_label_key() {
        for t in [
            Trap::Transpose, Trap::Double, Trap::Drop, Trap::Substitute,
            Trap::SilentLetter, Trap::IeEi, Trap::DoubleConsonant,
            Trap::SuffixVowel, Trap::Homophone,
        ] {
            assert!(t.label_key().starts_with("imp.trap."));
        }
    }

    /// D3's exclusion report (Done #3): the per-language count of words the
    /// generator cannot serve fairly. Diagnostic, not a gate.
    #[test]
    #[ignore]
    fn exclusion_report() {
        for lang in LANGS {
            let n = crate::word_index::vocabulary_size(lang);
            let mut tried = 0;
            let mut excluded = 0;
            for i in 0..300u64 {
                let at = (mix(i) % n.max(1) as u64) as usize;
                let Some(w) = crate::word_index::word_at(lang, at) else { continue };
                tried += 1;
                if round(lang, &w, i, Tier::Medium).is_none() {
                    excluded += 1;
                }
            }
            println!(
                "{lang:4} excluded {excluded:>3}/{tried:<4} ({:.1}%)",
                100.0 * excluded as f64 / tried.max(1) as f64
            );
        }
    }
}

#[cfg(test)]
mod eyeball {
    use super::*;
    /// Not an assertion — a look at what the player would actually see.
    #[test]
    #[ignore]
    fn sample_cards() {
        for lang in ["en", "ko", "ja", "ru", "ar", "zh"] {
            println!("--- {lang} ---");
            for r in set_of_ten(lang, 11, Tier::Medium).into_iter().take(3) {
                let faces: Vec<String> = r
                    .cards
                    .iter()
                    .enumerate()
                    .map(|(i, c)| if i == r.correct { format!("[{c}]") } else { c.clone() })
                    .collect();
                println!("   {}", faces.join("   "));
            }
        }
    }
}

