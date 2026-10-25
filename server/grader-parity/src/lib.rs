//! CC-GAME-SERVER A6.1: one deterministic grading fixture, judged by the shared
//! grader (`spell_wasm::native_api::grade`) on whatever target this is built for.
//!
//! The native test (`spell-server/tests/grader_parity.rs`) and the WASM test
//! (`tests/wasm.rs`, run under `wasm-bindgen-test-runner`) both compare the
//! fixture's verdict digest against the same golden file. Two runners agreeing
//! with one golden is the proof that native and WASM verdicts are identical.
//!
//! Nothing here decides a verdict. It only builds inputs and records outputs.

use sha2::{Digest, Sha256};
use spell_wasm::native_api::{citation, grade, is_active_lang, Tier, BUILTIN_LANGS};
use unicode_normalization::UnicodeNormalization;

/// Cases per language (A6.1).
pub const CASES_PER_LANG: usize = 1000;

/// The golden file both runners compare against.
pub const GOLDEN: &str = include_str!("../golden.txt");

#[derive(Clone, Debug)]
pub struct Case {
    pub lang: &'static str,
    pub tier: Tier,
    pub entry: String,
    pub typed: String,
    pub kid: bool,
    /// Which perturbation produced `typed` (for debugging a mismatch).
    pub how: &'static str,
}

/// splitmix64: the same stream on every target, no float, no platform RNG.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn seed_for(lang: &str) -> u64 {
    // FNV-1a over the code: stable across targets.
    lang.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// The languages the online server can serve: every registry language the
/// app build treats as active (A6.1, F8).
pub fn languages() -> Vec<&'static str> {
    BUILTIN_LANGS.iter().map(|row| row.0).filter(|l| is_active_lang(l)).collect()
}

const PERTURBATIONS: [&str; 12] = [
    "exact", "upper", "nfd", "pad", "drop", "swap", "insert", "replace", "other_word", "strip_marks", "tone_shift",
    "tone_drop",
];

fn perturb(rng: &mut Rng, how: &str, word: &str, other: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    match how {
        "exact" => word.to_string(),
        "upper" => word.to_uppercase(),
        "nfd" => word.nfd().collect(),
        "pad" => format!("  {word} "),
        "drop" if n > 1 => {
            let i = rng.below(n);
            chars.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, c)| *c).collect()
        }
        "swap" if n > 1 => {
            let i = rng.below(n - 1);
            let mut c = chars.clone();
            c.swap(i, i + 1);
            c.into_iter().collect()
        }
        "insert" => {
            let i = rng.below(n + 1);
            let pick: Vec<char> = other.chars().collect();
            let ins = if pick.is_empty() { 'a' } else { pick[rng.below(pick.len())] };
            let mut c = chars.clone();
            c.insert(i, ins);
            c.into_iter().collect()
        }
        "replace" if n > 0 => {
            let i = rng.below(n);
            let pick: Vec<char> = other.chars().collect();
            let rep = if pick.is_empty() { 'z' } else { pick[rng.below(pick.len())] };
            let mut c = chars.clone();
            c[i] = rep;
            c.into_iter().collect()
        }
        "other_word" => other.to_string(),
        // Accent-lenient Kid Mode is the point of this one.
        "strip_marks" => word.nfd().filter(|c| !('\u{300}'..='\u{36f}').contains(c)).collect(),
        // Mandarin citations carry tone digits; elsewhere these are plain edits.
        "tone_shift" => word
            .chars()
            .map(|c| match c {
                '1'..='4' => char::from(b'1' + ((c as u8 - b'1' + 1) % 4)),
                other => other,
            })
            .collect(),
        "tone_drop" => word.chars().filter(|c| !c.is_ascii_digit()).collect(),
        _ => word.to_string(),
    }
}

/// The A6.1 fixture: `CASES_PER_LANG` cases for each active language, spread
/// evenly over the four tiers, half of them graded as Kid Mode.
pub fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for lang in languages() {
        let mut rng = Rng::new(seed_for(lang));
        for i in 0..CASES_PER_LANG {
            let tier = Tier::ALL[i % 4];
            let pool = spell_wasm::native_api::tier_for(lang, tier.name());
            if pool.is_empty() {
                continue;
            }
            let entry = pool[rng.below(pool.len())].to_string();
            let other_entry = pool[rng.below(pool.len())];
            let how = PERTURBATIONS[rng.below(PERTURBATIONS.len())];
            let typed = perturb(&mut rng, how, citation(&entry), citation(other_entry));
            let kid = rng.below(2) == 1;
            out.push(Case { lang, tier, entry, typed, kid, how });
        }
    }
    out
}

/// One line per case: the inputs, the perturbation, and the verdict.
pub fn verdict_lines() -> Vec<(String, String)> {
    cases()
        .into_iter()
        .map(|c| {
            let v = grade(c.lang, c.kid, &c.typed, &c.entry, c.tier);
            let line = serde_line(&c, v);
            (c.lang.to_string(), line)
        })
        .collect()
}

fn esc(s: &str) -> String {
    s.chars().flat_map(|c| c.escape_unicode()).collect()
}

fn serde_line(c: &Case, v: bool) -> String {
    format!("{}\t{}\t{}\t{}\t{}\t{}\t{}", c.lang, c.tier.name(), esc(&c.entry), esc(&c.typed), c.kid, c.how, v)
}

/// The report both runners print and the golden file holds: one line per
/// language (`lang cases accepted sha256`) and a final `all` line.
pub fn report() -> String {
    let lines = verdict_lines();
    let mut out = String::new();
    let mut all = Sha256::new();
    let mut total = 0usize;
    let mut accepted_total = 0usize;
    for lang in languages() {
        let mut h = Sha256::new();
        let mut n = 0usize;
        let mut accepted = 0usize;
        for (_, line) in lines.iter().filter(|(l, _)| l == lang) {
            h.update(line.as_bytes());
            h.update(b"\n");
            all.update(line.as_bytes());
            all.update(b"\n");
            n += 1;
            if line.ends_with("\ttrue") {
                accepted += 1;
            }
        }
        total += n;
        accepted_total += accepted;
        out.push_str(&format!("{lang} {n} {accepted} {}\n", hex(&h.finalize())));
    }
    out.push_str(&format!("all {total} {accepted_total} {}\n", hex(&all.finalize())));
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A6.3 (informational, never gating): the solo game's verdict for the same
/// case, built from the same primitives in the order `game::submit_guess`
/// applies them (game.rs:2178-2245). English's exact-match case goes to the
/// Flask backend in solo play, which compares `answer.lower().strip()` with
/// `word.lower().strip()`; that comparison is reproduced here.
pub fn solo_verdict(c: &Case) -> bool {
    use spell_wasm::native_api as n;
    let word = citation(&c.entry);
    if c.lang == "en" {
        let other = n::fold_strict(&c.typed) != n::fold_strict(word) && n::homophone_accepts("en", word, &c.typed);
        if other || n::variant_accepts(word, &c.typed) {
            return true;
        }
        return c.typed.trim().to_lowercase() == word.trim().to_lowercase();
    }
    if c.lang == "zh" {
        let surface = n::sandhi_lookup(&c.entry).map(|(s, _)| s);
        let (v, _) = n::grade_sandhi_aware(
            &c.typed,
            word,
            surface,
            n::ToneMode::for_kid(c.kid),
            n::tier_accepts_surface(c.tier.name()),
        );
        return v.is_correct();
    }
    if c.lang == "ko" {
        return n::jamo_grade(&c.typed, word).correct;
    }
    n::answer_matches_in(c.lang, &c.typed, word, c.kid) || n::homophone_accepts(c.lang, word, &c.typed)
}

/// Per language: (cases, shared-accepts-solo-rejects, solo-accepts-shared-rejects),
/// plus up to `examples` sample disagreements.
pub fn disagreements(examples: usize) -> Vec<(&'static str, usize, usize, usize, Vec<String>)> {
    let all = cases();
    languages()
        .into_iter()
        .map(|lang| {
            let mut n = 0;
            let (mut shared_only, mut solo_only) = (0, 0);
            let mut ex = Vec::new();
            for c in all.iter().filter(|c| c.lang == lang) {
                n += 1;
                let shared = grade(c.lang, c.kid, &c.typed, &c.entry, c.tier);
                let solo = solo_verdict(c);
                if shared != solo {
                    if shared {
                        shared_only += 1;
                    } else {
                        solo_only += 1;
                    }
                    if ex.len() < examples {
                        ex.push(format!(
                            "{} {:?} typed {:?} kid={} ({}): shared={} solo={}",
                            c.tier.name(),
                            c.entry,
                            c.typed,
                            c.kid,
                            c.how,
                            shared,
                            solo
                        ));
                    }
                }
            }
            (lang, n, shared_only, solo_only, ex)
        })
        .collect()
}
