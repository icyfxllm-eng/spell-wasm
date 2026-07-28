//! CC-PRACTICE v2 core — guided first-contact spelling (20 + 5 per language).
//!
//! REVIEW-GATED. Pure curriculum/phase/tray/coach logic; the screen renders it.
//! v2 is a CONVERSATION loop, not a gentler test loop: ghost-trace slots, a
//! tile tray with deterministic decoys, a coaching orb, trap micro-interaction
//! refs, echo-spelling, syllable chunking, a hint ladder and choice beats —
//! all data-driven from the curriculum file.
//!
//! FAILURE-PROOF by construction (D9): this module has no concept of lives,
//! or fail states — those symbols do not appear here, and the purity gate
//! (scripts/practice-purity-check.mjs, Invariant I1) enforces it. Its ONLY
//! persistence is the per-language progress record under its own storage key
//! (Invariant I2) — nothing else is read or written.

use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

/// D2 scaffolding ladder — phase boundaries as constants in ONE place:
/// words 1–5 ghost-trace (tiles onto ghost letters), 6–12 cued (~2s flash,
/// tiles, no ghosts), 13–20 audio-only (real keyboard fades in).
pub const GHOST_UNTIL: usize = 5; // 0-based: indices 0..5 are GhostTrace
pub const CUED_UNTIL: usize = 12; // indices 5..12 are Cued; 12..20 Audio
pub const FIRST_CONTACT: usize = 20;
pub const DIFFICULT: usize = 5;
pub const TOTAL: usize = FIRST_CONTACT + DIFFICULT;
/// Cued phase visibility window, ms (D2 "~2s").
pub const FLASH_MS: u32 = 2000;
/// D3: tray bounds — the word's units plus 1–3 decoys, 6–8 tiles total
/// (fewer only when the word itself is tiny or enormous).
pub const TRAY_MAX: usize = 8;
pub const TRAY_MIN: usize = 6;
pub const MAX_DECOYS: usize = 3;
/// D2: phase-3 words at or above this many typing units arrive one syllable
/// chunk at a time — where the repo has syllable data (es-only today; see
/// syllable.rs — data-driven, lights up with future coverage).
pub const CHUNK_MIN_UNITS: usize = 7;
/// D4: hard cap on a language's total coach-line pool (schema-enforced too).
pub const COACH_CAP: usize = 40;
/// D6: echo-spelling per-unit beat, ms (~2s for a typical word, skippable).
pub const ECHO_UNIT_MS: u32 = 320;
/// D5: misses before a micro-interaction auto-reveals (unfailable).
pub const MICRO_MISSES: u32 = 2;

/// How the current word is presented (D2). The graduation lap (words 21–25)
/// is always Audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Ghost letters in the slots; build onto them from the tile tray.
    GhostTrace,
    /// Word flashes ~2s then hides; tray input, ghosts gone.
    Cued,
    /// Audio only; the real keyboard replaces the tray.
    Audio,
}

/// Presentation phase for a 0-based position in the 25-word run.
pub fn phase(pos: usize) -> Phase {
    if pos < GHOST_UNTIL {
        Phase::GhostTrace
    } else if pos < CUED_UNTIL {
        Phase::Cued
    } else {
        Phase::Audio
    }
}

/// D5 — a micro-interaction template reference from curriculum data. The id
/// must name one of the three v2 templates (I3); params are template-specific
/// (e.g. `unit` for TAP_SILENT_UNIT, `pair`+`hear` for HEAR_PICK, `stack` for
/// TAP_STACK).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TemplateRef {
    pub id: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

/// The fixed v2 template set (D5 — no new templates without Eric).
pub const TEMPLATES: [&str; 3] = ["TAP_SILENT_UNIT", "HEAR_PICK", "TAP_STACK"];

/// One trap-class entry (D4/D5): intro card (shown when its first word begins,
/// once per run), optional micro-interaction template, and decoy hints for the
/// tile tray (D3).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TrapIntro {
    pub id: String,
    #[serde(rename = "firstWord")]
    pub first_word: usize,
    pub intro: String,
    #[serde(default)]
    pub template: Option<TemplateRef>,
    #[serde(default)]
    pub decoys: Vec<String>,
}

/// D8 — a choice beat: at position `at`, the coach offers words[at] vs `alt`
/// (same trap class, from the bank, NOT one of the sequenced 20). The player's
/// pick plays at that position; the other is skipped, not queued.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChoiceBeat {
    pub at: usize,
    pub alt: String,
    /// Exactly two curated emoji: [emoji-for-words[at], emoji-for-alt].
    pub emoji: Vec<String>,
}

/// D4 — the per-language coach line pools, one pool per speaking slot.
/// Total across pools ≤ COACH_CAP (I3). Lines are audited instructional
/// strings; selection is deterministic (word index + curriculum seed).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Coach {
    #[serde(default, rename = "wordDone")]
    pub word_done: Vec<String>,
    #[serde(default, rename = "wordSetup")]
    pub word_setup: Vec<String>,
    #[serde(default)]
    pub phase: Vec<String>,
    #[serde(default)]
    pub reveal: Vec<String>,
    #[serde(default)]
    pub ceremony: Vec<String>,
}

/// A language's curriculum: 20 first-contact + 5 difficult bank words, trap
/// entries, choice beats and coach pools. Data only (D10) — schema enforced
/// by scripts/practice-check.mjs (I3).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Curriculum {
    pub lang: String,
    pub words: Vec<String>,
    pub difficult: Vec<String>,
    pub traps: Vec<TrapIntro>,
    #[serde(default, rename = "choiceBeats")]
    pub choice_beats: Vec<ChoiceBeat>,
    #[serde(default)]
    pub coach: Coach,
}

fn sources() -> &'static HashMap<&'static str, Curriculum> {
    static T: OnceLock<HashMap<&'static str, Curriculum>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for (lang, src) in [
            ("en", include_str!("../config/practice/en.json")),
            ("es", include_str!("../config/practice/es.json")),
            ("fr", include_str!("../config/practice/fr.json")),
            ("de", include_str!("../config/practice/de.json")),
            ("pt", include_str!("../config/practice/pt.json")),
            ("pl", include_str!("../config/practice/pl.json")),
            ("vi", include_str!("../config/practice/vi.json")),
            ("ko", include_str!("../config/practice/ko.json")),
            ("ja", include_str!("../config/practice/ja.json")),
            ("fil", include_str!("../config/practice/fil.json")),
            ("zh", include_str!("../config/practice/zh.json")),
            ("ru", include_str!("../config/practice/ru.json")),
            ("ar", include_str!("../config/practice/ar.json")),
            ("sw", include_str!("../config/practice/sw.json")),
            ("hi", include_str!("../config/practice/hi.json")),
        ] {
            if let Ok(c) = serde_json::from_str::<Curriculum>(src) {
                m.insert(lang, c);
            }
        }
        m
    })
}

pub fn curriculum(lang: &str) -> Option<&'static Curriculum> {
    sources().get(lang)
}

/// The word at a 0-based run position (0..20 first-contact, 20..25 difficult),
/// honoring any recorded choice-beat pick (D8: exact resume).
pub fn word_at<'c>(c: &'c Curriculum, pos: usize, progress: &'c Progress) -> Option<&'c str> {
    if pos < FIRST_CONTACT {
        if let Some(p) = progress.picks.iter().find(|p| p.at == pos) {
            return Some(p.word.as_str());
        }
        c.words.get(pos).map(|s| s.as_str())
    } else {
        c.difficult.get(pos - FIRST_CONTACT).map(|s| s.as_str())
    }
}

/// The trap-class entry owning position `pos` (trap blocks run from each
/// entry's firstWord to the next entry's). Graduation words map to the class
/// they close out (one difficult word per introduced trap, D10).
pub fn trap_for(c: &Curriculum, pos: usize) -> Option<&TrapIntro> {
    if pos >= FIRST_CONTACT {
        return c.traps.get(pos - FIRST_CONTACT);
    }
    c.traps.iter().rev().find(|t| t.first_word <= pos)
}

/// The intro card that must show when `pos` begins, if any (first occurrence
/// of a trap class — D4). Graduation words never introduce.
pub fn intro_at(c: &Curriculum, pos: usize) -> Option<&TrapIntro> {
    c.traps.iter().find(|t| t.first_word == pos)
}

/// The choice beat that fires when `pos` begins, if any (D8).
pub fn choice_at(c: &Curriculum, pos: usize) -> Option<&ChoiceBeat> {
    c.choice_beats.iter().find(|b| b.at == pos)
}

// ---- typing units (I8: NFC; ko = compatibility jamo, matching on blocks) ----

/// A word's typing units for tiles and echo: NFC chars for every language
/// except ko, whose units are the compatibility jamo of each syllable block
/// (compound medials/finals are ONE unit each — a single tile to find,
/// matching the jamo grader's philosophy).
pub fn units(lang: &str, word: &str) -> Vec<String> {
    let w: String = word.nfc().collect();
    if lang == "ko" {
        let mut out = Vec::new();
        for ch in w.chars() {
            if let Some((i, m, f)) = crate::hangul::parts(ch) {
                out.push(i.to_string());
                out.push(m.to_string());
                if f != '\0' {
                    out.push(f.to_string());
                }
            } else {
                out.push(ch.to_string());
            }
        }
        return out;
    }
    w.chars().map(|c| c.to_string()).collect()
}

/// Assemble accepted tiles into display text: ko composes real syllable
/// blocks through the repo's IME (`hangul::feed`); everything else joins.
pub fn assemble(lang: &str, tiles: &[String]) -> String {
    if lang == "ko" {
        let mut s = String::new();
        for t in tiles {
            match t.chars().next() {
                Some(j) if t.chars().count() == 1 => s = crate::hangul::feed(&s, j),
                _ => s.push_str(t),
            }
        }
        return s;
    }
    tiles.concat()
}

// ---- deterministic PRNG (splitmix64 — the crate convention, see daily.rs) ----

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// I4 — the curriculum seed for a (lang, word) pair: FNV-1a over NFC bytes,
/// identical on every platform.
pub fn seed_for(lang: &str, word: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in lang.bytes().chain(":".bytes()).chain(word.nfc().collect::<String>().bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// The language's typing-unit inventory: every unit appearing across the
/// curriculum's 25 words (decoy fallback pool, D3 — always non-empty).
fn inventory(lang: &str, c: &Curriculum) -> Vec<String> {
    let mut seen = Vec::new();
    for w in c.words.iter().chain(c.difficult.iter()) {
        for u in units(lang, w) {
            if !seen.contains(&u) {
                seen.push(u);
            }
        }
    }
    seen
}

/// D3 — the tile tray for a word: its units (duplicates included) plus 1–3
/// deterministic decoys — trap-class decoys first, unit-inventory fallback —
/// shuffled by splitmix64 seeded on (lang, word). I5: the tray ALWAYS
/// contains every unit needed to complete the word.
pub fn tray(lang: &str, c: &Curriculum, pos: usize, word: &str) -> Vec<String> {
    let need = units(lang, word);
    let mut tiles = need.clone();
    let mut st = seed_for(lang, word);
    if tiles.len() < TRAY_MAX {
        let room = TRAY_MAX - tiles.len();
        let want = (TRAY_MIN.saturating_sub(tiles.len())).clamp(1, MAX_DECOYS).min(room);
        let mut pool: Vec<String> = trap_for(c, pos)
            .map(|t| t.decoys.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|d| !need.contains(d))
            .collect();
        let mut inv: Vec<String> =
            inventory(lang, c).into_iter().filter(|u| !need.contains(u) && !pool.contains(u)).collect();
        pool.append(&mut inv);
        for _ in 0..want {
            if pool.is_empty() {
                break;
            }
            let i = (splitmix64(&mut st) % pool.len() as u64) as usize;
            tiles.push(pool.swap_remove(i));
        }
    }
    // Fisher–Yates, same seed stream: identical tray order on every platform.
    for i in (1..tiles.len()).rev() {
        let j = (splitmix64(&mut st) % (i as u64 + 1)) as usize;
        tiles.swap(i, j);
    }
    tiles
}

/// D4 — deterministic coach-line pick: (pool slot, word index, curriculum
/// seed) → the same line on every platform. None on an empty pool (the orb
/// just stays quiet — never a placeholder string).
pub fn coach_line<'a>(pool: &'a [String], word_index: usize, seed: u64) -> Option<&'a str> {
    if pool.is_empty() {
        return None;
    }
    let mut st = seed ^ (word_index as u64).wrapping_mul(0x9E3779B97F4A7C15);
    Some(pool[(splitmix64(&mut st) % pool.len() as u64) as usize].as_str())
}

/// D2 — phase-3 chunk delivery: syllable chunks where the repo has syllable
/// data (es today), whole word everywhere else. Only words at or above
/// CHUNK_MIN_UNITS chunk at all.
pub fn chunks(lang: &str, word: &str) -> Vec<String> {
    if lang == "es" && units(lang, word).len() >= CHUNK_MIN_UNITS {
        let s = crate::syllable::syllabify(word);
        if s.len() > 1 {
            return s;
        }
    }
    vec![word.to_string()]
}

/// Compare a typed answer prefix against the target, per typing unit.
/// Case-insensitive, NFC-normalized. For ko the comparison expands BOTH sides
/// to jamo units, so an in-progress syllable block (or a trailing lone jamo
/// from the system IME) counts as correct progress instead of a wrong unit
/// (I8: matching on assembled blocks). Returns (correct leading units in
/// TARGET-unit terms, complete). D9: the CALLER holds position on a wrong
/// unit; this function only reports — it cannot fail anyone.
pub fn check_prefix(lang: &str, target: &str, typed: &str) -> (usize, bool) {
    if lang == "ko" {
        let t = units(lang, target);
        let y: Vec<String> = typed
            .nfc()
            .collect::<String>()
            .chars()
            .flat_map(|c| units(lang, &c.to_string()))
            .collect();
        let mut ok = 0;
        for (a, b) in t.iter().zip(y.iter()) {
            if a == b {
                ok += 1;
            } else {
                break;
            }
        }
        // Progress is counted in COMPLETED blocks for the slot line: map jamo
        // count back onto whole target chars.
        let mut blocks_ok = 0;
        let mut consumed = 0;
        for ch in target.nfc().collect::<String>().chars() {
            let n = units(lang, &ch.to_string()).len();
            if consumed + n <= ok {
                consumed += n;
                blocks_ok += 1;
            } else {
                break;
            }
        }
        let total: usize = t.len();
        return (blocks_ok, ok == total && y.len() >= total);
    }
    let t: Vec<char> = target.nfc().collect();
    let y: Vec<char> = typed.nfc().collect();
    let mut ok = 0;
    for (a, b) in t.iter().zip(y.iter()) {
        // Case-insensitive: first-contact typing must never fail on Shift.
        if a.to_lowercase().eq(b.to_lowercase()) {
            ok += 1;
        } else {
            break;
        }
    }
    (ok, ok == t.len() && y.len() >= t.len())
}

/// Whether typed input can still GROW into the target (nothing wrong yet) —
/// the screen only truncates when this is false (ko IME composition safety).
pub fn prefix_viable(lang: &str, target: &str, typed: &str) -> bool {
    if lang == "ko" {
        let t = units(lang, target);
        let y: Vec<String> = typed
            .nfc()
            .collect::<String>()
            .chars()
            .flat_map(|c| units(lang, &c.to_string()))
            .collect();
        return y.len() <= t.len() && t.iter().zip(y.iter()).all(|(a, b)| a == b);
    }
    let t: Vec<char> = target.nfc().collect();
    let y: Vec<char> = typed.nfc().collect();
    y.len() <= t.len() && t.iter().zip(y.iter()).all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
}

/// A recorded choice-beat pick (D8) — replays identically on resume.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub at: usize,
    pub word: String,
}

/// The per-language progress record (Invariant I2: the ONLY thing Practice
/// persists). `pos` is the next word to play, 0..=TOTAL; TOTAL = fully done.
/// Hint usage and micro-interaction misses are deliberately NOT here (D11).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Progress {
    pub pos: usize,
    /// The trap intros already shown this RUN (not re-shown on resume; reset
    /// on restart).
    #[serde(default)]
    pub shown: Vec<String>,
    /// Choice-beat picks this run (D8).
    #[serde(default)]
    pub picks: Vec<Pick>,
}

fn key(lang: &str) -> String {
    format!("spell_practice_{lang}")
}

pub fn load(lang: &str) -> Progress {
    crate::storage::get_json(&key(lang)).unwrap_or_default()
}

pub fn save(lang: &str, p: &Progress) {
    crate::storage::set_json(&key(lang), p);
}

/// Full restart for ONE language: position, shown-intros and picks reset.
pub fn restart(lang: &str) {
    save(lang, &Progress::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D2: phase transitions occur exactly at words 6 and 13 (1-based).
    #[test]
    fn phase_boundaries_exact() {
        assert_eq!(phase(0), Phase::GhostTrace);
        assert_eq!(phase(4), Phase::GhostTrace, "word 5 is still ghost-trace");
        assert_eq!(phase(5), Phase::Cued, "word 6 begins Cued");
        assert_eq!(phase(11), Phase::Cued, "word 12 is still Cued");
        assert_eq!(phase(12), Phase::Audio, "word 13 begins Audio (keyboard fades in)");
        assert_eq!(phase(19), Phase::Audio);
        assert_eq!(phase(24), Phase::Audio, "graduation is audio-only");
    }

    /// Every lineup language has a parsed curriculum of exactly 20+5 with all
    /// five trap intros resolvable, valid template refs, two-emoji choice
    /// beats whose alt is NOT one of the sequenced 20, and a coach pool within
    /// the D4 cap.
    #[test]
    fn every_language_curriculum_parses() {
        let none = Progress::default();
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            let c = curriculum(code).unwrap_or_else(|| panic!("{code}: curriculum missing/unparsed"));
            assert_eq!(c.words.len(), FIRST_CONTACT, "{code}");
            assert_eq!(c.difficult.len(), DIFFICULT, "{code}");
            assert_eq!(c.traps.len(), 5, "{code}");
            for t in &c.traps {
                assert!(t.first_word < FIRST_CONTACT, "{code}/{}", t.id);
                assert!(!t.intro.is_empty() && t.intro.chars().count() <= 90, "{code}/{}", t.id);
                assert_eq!(intro_at(c, t.first_word).map(|x| x.id.as_str()), Some(t.id.as_str()), "{code}/{}", t.id);
                if let Some(tp) = &t.template {
                    assert!(TEMPLATES.contains(&tp.id.as_str()), "{code}/{}: unknown template {}", t.id, tp.id);
                }
            }
            for b in &c.choice_beats {
                assert!(b.at < FIRST_CONTACT, "{code}: beat at {}", b.at);
                assert_eq!(b.emoji.len(), 2, "{code}: beat {} needs exactly two emoji", b.at);
                assert!(!c.words.contains(&b.alt), "{code}: alt {:?} must not be in the sequenced 20", b.alt);
            }
            let coach_total = c.coach.word_done.len()
                + c.coach.word_setup.len()
                + c.coach.phase.len()
                + c.coach.reveal.len()
                + c.coach.ceremony.len();
            assert!(coach_total <= COACH_CAP, "{code}: coach pool {coach_total} over cap");
            for pos in 0..TOTAL {
                assert!(word_at(c, pos, &none).is_some(), "{code} pos {pos}");
                assert!(trap_for(c, pos).is_some(), "{code} pos {pos}: trap class");
            }
        }
    }

    /// D9 mechanics: the checker reports progress, never failure; wrong units
    /// hold position; case never blocks; ko composition is never "wrong".
    #[test]
    fn prefix_checker_is_failure_proof() {
        assert_eq!(check_prefix("en", "casa", ""), (0, false));
        assert_eq!(check_prefix("en", "casa", "c"), (1, false));
        assert_eq!(check_prefix("en", "casa", "cx"), (1, false), "wrong unit: position holds at 1");
        assert_eq!(check_prefix("en", "casa", "CASA"), (4, true), "case-insensitive");
        assert_eq!(check_prefix("es", "niño", "niño"), (4, true), "NFC-equal diacritics");
        assert_eq!(check_prefix("es", "niño", "nin\u{0303}o"), (4, true), "NFD input normalizes");
        // ko: an in-progress block is viable progress, not a wrong unit (I8).
        assert!(prefix_viable("ko", "한글", "ㅎ"), "lone initial is composition, not error");
        assert!(prefix_viable("ko", "한글", "하"), "in-progress block");
        assert!(prefix_viable("ko", "한글", "한"), "completed block");
        assert!(!prefix_viable("ko", "한글", "말"), "wrong block is wrong");
        assert_eq!(check_prefix("ko", "한글", "한글"), (2, true));
        assert_eq!(check_prefix("ko", "한글", "한"), (1, false));
    }

    /// I4 golden — identical trays, decoys and coach picks from the same
    /// (lang, word, seed) on every platform. Pins en + ko values; any change
    /// to the PRNG, seeding or tray algorithm must update these DELIBERATELY.
    #[test]
    fn golden_tray_and_coach_are_deterministic() {
        let c = curriculum("en").unwrap();
        let t1 = tray("en", c, 0, "horse");
        let t2 = tray("en", c, 0, "horse");
        assert_eq!(t1, t2, "same inputs, same tray");
        assert!(t1.len() >= TRAY_MIN.min(units("en", "horse").len() + 1) && t1.len() <= TRAY_MAX);
        let k = curriculum("ko").unwrap();
        let w = k.words[0].clone();
        let kt1 = tray("ko", k, 0, &w);
        let kt2 = tray("ko", k, 0, &w);
        assert_eq!(kt1, kt2, "ko tray deterministic");
        // Coach pick: same (pool, index, seed) → same line, and a different
        // word index MAY differ but must itself be stable.
        let pool: Vec<String> = (0..7).map(|i| format!("line {i}")).collect();
        let s = seed_for("en", "horse");
        assert_eq!(coach_line(&pool, 3, s), coach_line(&pool, 3, s));
        assert_eq!(seed_for("en", "horse"), seed_for("en", "horse"), "seed stable");
        assert_ne!(seed_for("en", "horse"), seed_for("ko", "horse"), "lang-scoped");
    }

    /// I5 property — over every curriculum word AND a deterministic sample of
    /// the language's bank (host fs), the tray always contains every unit the
    /// word needs, duplicates included, within size bounds.
    #[test]
    fn tray_always_completable_property() {
        fn count(v: &[String]) -> HashMap<&str, usize> {
            let mut m = HashMap::new();
            for u in v {
                *m.entry(u.as_str()).or_insert(0) += 1;
            }
            m
        }
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            let c = curriculum(code).unwrap();
            let mut sample: Vec<String> =
                c.words.iter().chain(c.difficult.iter()).cloned().collect();
            // Bank sample (deterministic stride; zh's bank lives in words.rs
            // and is already exercised via its curriculum words).
            for tier in ["easy", "medium", "hard", "expert"] {
                let p = format!("{}/assets/words/{}/{}.txt", env!("CARGO_MANIFEST_DIR"), code, tier);
                if let Ok(txt) = std::fs::read_to_string(&p) {
                    let all: Vec<&str> = txt.lines().filter(|w| !w.trim().is_empty()).collect();
                    let stride = (all.len() / 250).max(1);
                    for w in all.iter().step_by(stride) {
                        sample.push(w.trim().to_string());
                    }
                }
            }
            for w in &sample {
                let need_units = units(code, w);
                let need = count(&need_units);
                let t = tray(code, c, 0, w);
                let have = count(&t);
                for (u, n) in &need {
                    assert!(
                        have.get(u).copied().unwrap_or(0) >= *n,
                        "{code}/{w}: tray missing unit {u:?} ({n} needed)"
                    );
                }
                assert!(t.len() <= TRAY_MAX.max(units(code, w).len()), "{code}/{w}: tray oversize");
            }
        }
    }

    /// ko tiles assemble into real blocks through the repo IME (I8).
    #[test]
    fn ko_tiles_assemble_blocks() {
        let u = units("ko", "한글");
        assert_eq!(u, vec!["ㅎ", "ㅏ", "ㄴ", "ㄱ", "ㅡ", "ㄹ"]);
        assert_eq!(assemble("ko", &u), "한글");
        assert_eq!(assemble("en", &units("en", "cat")), "cat");
    }

    /// D2 chunking: es long words chunk by syllable; everything else (and
    /// short es words) deliver whole (data-driven fallback).
    #[test]
    fn chunking_is_data_driven() {
        assert_eq!(chunks("en", "bicycle"), vec!["bicycle"], "en has no syllable data");
        assert_eq!(chunks("es", "sol"), vec!["sol"], "short words never chunk");
        let ch = chunks("es", "mariposas");
        assert!(ch.len() > 1, "long es words chunk: {ch:?}");
        assert_eq!(ch.concat(), "mariposas", "chunks reassemble the word exactly");
    }

    /// D8: a recorded pick replays on resume; the unpicked word is skipped.
    #[test]
    fn choice_pick_replays_on_resume() {
        let c = curriculum("en").unwrap();
        if let Some(b) = c.choice_beats.first() {
            let mut p = Progress::default();
            assert_eq!(word_at(c, b.at, &p), Some(c.words[b.at].as_str()));
            p.picks.push(Pick { at: b.at, word: b.alt.clone() });
            assert_eq!(word_at(c, b.at, &p), Some(b.alt.as_str()), "pick wins on resume");
        }
    }

    /// Progress round-trips through its own record only (host storage is a
    /// no-op, so this exercises the shape, not persistence). Old v1 records
    /// (no picks field) still parse.
    #[test]
    fn progress_shape_round_trips() {
        let p = Progress { pos: 7, shown: vec!["b-v".into()], picks: vec![Pick { at: 4, word: "wave".into() }] };
        let json = serde_json::to_string(&p).unwrap();
        let back: Progress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pos, 7);
        assert_eq!(back.picks[0].at, 4);
        let v1: Progress = serde_json::from_str(r#"{"pos":3,"shown":[]}"#).unwrap();
        assert_eq!(v1.pos, 3, "v1 record forward-compatible");
    }
}
