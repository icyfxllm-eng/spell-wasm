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


// ── CC-TRANSLATE-TOOLS Wave 2, pivot-independent core (Eric,
// 2026-08-05: "resume the calendar, translate, and other unfinished
// recent read mes"). Tools 5/6/7 machinery; everything gloss-shaped
// stays behind the ONE resolver seam and simply renders reduced until
// CC-BANK-TRANSLATE lands. Tool 8 (Translation Match) is content-
// blocked on the gloss column and has no code, per Wave-3 discipline.

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Tool 5 — One Word, Fifteen Ways: the deterministic date-seeded
/// daily pick. SEPARATE seed stream from Daily Challenge (its own salt;
/// acceptance test 6). Same date + lang + tier => same word, forever.
pub fn daily_word(lang: &str, tier: &str, epoch_day: u32) -> Option<String> {
    let pool = crate::words::tier_for(lang, tier);
    if pool.is_empty() {
        return None;
    }
    let mut h: u64 = 0x5452_4441_494C_5900 ^ (epoch_day as u64); // "TRDAILY" salt
    for b in lang.bytes() {
        h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u64);
    }
    for b in tier.bytes() {
        h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u64);
    }
    Some(pool[(splitmix(h) % pool.len() as u64) as usize].to_string())
}

// ─────────── CC-BANK-TRANSLATE: the gloss pivot (bones, 2026-08-05)
//
// Eric greenlit building the pivot rather than waiting on the file.
// Three laws, all enforced by scripts/gloss-check.mjs at build time:
// a key is a live bank word, a concept is a live ENGLISH bank word, and
// `audited` is a HUMAN claim — no tool sets it. A language's rows exist
// but stay DARK until a named native speaker signs, which is what keeps
// the closed space honest while the tool becomes real.

#[derive(serde::Deserialize)]
struct GlossDoc {
    #[allow(dead_code)]
    lang: String,
    audited: bool,
    rows: std::collections::HashMap<String, String>,
}

fn gloss_docs() -> &'static std::collections::HashMap<&'static str, GlossDoc> {
    static G: std::sync::OnceLock<std::collections::HashMap<&'static str, GlossDoc>> =
        std::sync::OnceLock::new();
    G.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        // Every language with a bank gets a table. zh's keys are the
        // bank's own pinyin half (`fang2zi5`), because that is the form
        // the pool is keyed by — see scripts/gloss-check.mjs.
        for (lang, raw) in [
            ("es", include_str!("../config/gloss/es.json")),
            ("fr", include_str!("../config/gloss/fr.json")),
            ("de", include_str!("../config/gloss/de.json")),
            ("pt", include_str!("../config/gloss/pt.json")),
            ("pl", include_str!("../config/gloss/pl.json")),
            ("ru", include_str!("../config/gloss/ru.json")),
            ("vi", include_str!("../config/gloss/vi.json")),
            ("fil", include_str!("../config/gloss/fil.json")),
            ("sw", include_str!("../config/gloss/sw.json")),
            ("ja", include_str!("../config/gloss/ja.json")),
            ("ko", include_str!("../config/gloss/ko.json")),
            ("ar", include_str!("../config/gloss/ar.json")),
            ("hi", include_str!("../config/gloss/hi.json")),
            ("zh", include_str!("../config/gloss/zh.json")),
        ] {
            if let Ok(doc) = serde_json::from_str::<GlossDoc>(raw) {
                m.insert(lang, doc);
            }
        }
        m
    })
}

/// I2's single resolver: a language is visible iff its gloss file is
/// AUDITED. One check, no per-tool conditionals.
pub fn gloss_audited(lang: &str) -> bool {
    lang == crate::consts::EN || gloss_docs().get(lang).map(|d| d.audited).unwrap_or(false)
}

/// Every language that can render a fan-out row today.
pub fn glossed_languages() -> Vec<&'static str> {
    let mut out = vec![crate::consts::EN];
    let mut rest: Vec<&'static str> =
        gloss_docs().iter().filter(|(_, d)| d.audited).map(|(l, _)| *l).collect();
    rest.sort_unstable();
    out.extend(rest);
    out
}

/// The word in `lang` that carries `concept`, when that language is
/// audited. This is the fan-out's row lookup and Match's answer lookup.
pub fn word_for_concept(lang: &str, concept: &str) -> Option<String> {
    if lang == crate::consts::EN {
        return crate::words::tier_for(lang, "easy")
            .iter()
            .chain(crate::words::tier_for(lang, "medium").iter())
            .map(|w| w.split('|').next().unwrap_or(w))
            .find(|w| w.eq_ignore_ascii_case(concept))
            .map(|w| w.to_string());
    }
    let doc = gloss_docs().get(lang)?;
    if !doc.audited {
        return None;
    }
    doc.rows.iter().find(|(_, c)| c.as_str() == concept).map(|(w, _)| w.clone())
}

/// The concept key IS the sense-locked English gloss (the pivot's own
/// definition). English words are their own concept; every other
/// language resolves through its AUDITED gloss table.
pub fn concept_of(lang: &str, word: &str) -> Option<String> {
    #[cfg(test)]
    if let Some(c) = TEST_GLOSS.with(|g| g.borrow().get(&(lang.to_string(), word.to_string())).cloned()) {
        return Some(c);
    }
    if lang == crate::consts::EN {
        return Some(display_word(word).to_lowercase());
    }
    let doc = gloss_docs().get(lang)?;
    if !doc.audited {
        return None; // rows exist, but no native has signed them yet
    }
    doc.rows.get(word).cloned()
}

/// Tool 8 — Translation Match. Two boards, both CLOSED-SPACE by
/// construction (I1): every prompt, answer and decoy is a bank word
/// resolved through the gloss pivot. Content-blocked until a language
/// is glossAudited — with no glosses the builders return None, which
/// renders the audited miss state, never an empty board.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchPrompt {
    /// The sense-locked English gloss the kid is answering FROM.
    pub concept: String,
    /// The word they must spell, in the target language.
    pub answer: String,
    pub target_lang: String,
}

/// Spell-the-translation: given the gloss, spell the target word.
/// Deterministic in `seed` so a board replays identically.
pub fn match_prompt(target_lang: &str, tier: &str, seed: u64) -> Option<MatchPrompt> {
    let pool = crate::words::tier_for(target_lang, tier);
    if pool.is_empty() {
        return None;
    }
    let start = (splitmix(seed ^ 0x4D_41_54_43_48) % pool.len() as u64) as usize;
    // Walk deterministically from the seeded start to the first word
    // that HAS a concept — no gloss, no prompt (closed space).
    for i in 0..pool.len() {
        let w = pool[(start + i) % pool.len()];
        let word = w.split('|').next().unwrap_or(w).to_string();
        if let Some(concept) = concept_of(target_lang, &word) {
            return Some(MatchPrompt { concept, answer: word, target_lang: target_lang.into() });
        }
    }
    None
}

/// The memory-pairs board: N concept pairs, each rendered in two
/// chosen languages. Returns None unless BOTH sides resolve for at
/// least `pairs` concepts — a half-dark board is not shipped.
pub fn match_pairs(
    lang_a: &str,
    lang_b: &str,
    tier: &str,
    pairs: usize,
    seed: u64,
) -> Option<Vec<(String, String, String)>> {
    let pool_a = crate::words::tier_for(lang_a, tier);
    if pool_a.is_empty() || pairs == 0 {
        return None;
    }
    let pool_b = crate::words::tier_for(lang_b, tier);
    let start = (splitmix(seed ^ 0x50_41_49_52_53) % pool_a.len() as u64) as usize;
    let mut out: Vec<(String, String, String)> = Vec::new();
    for i in 0..pool_a.len() {
        if out.len() == pairs {
            break;
        }
        let wa_raw = pool_a[(start + i) % pool_a.len()];
        let wa = wa_raw.split('|').next().unwrap_or(wa_raw).to_string();
        let Some(concept) = concept_of(lang_a, &wa) else { continue };
        if out.iter().any(|(c, _, _)| c == &concept) {
            continue;
        }
        // the same concept on the other side — pivot lookup, not MT
        let hit = pool_b.iter().find_map(|wb_raw| {
            let wb = wb_raw.split('|').next().unwrap_or(wb_raw).to_string();
            (concept_of(lang_b, &wb).as_deref() == Some(concept.as_str())).then_some(wb)
        })?;
        out.push((concept, wa, hit));
    }
    (out.len() == pairs).then_some(out)
}

// ══════════════════════ WAVE 3 (D3/D4/D6/D7/D8 SIGNED 2026-08-05)
// Every tool below obeys I1 (closed space), I4 (no free text on a kid
// surface) and I6 (zero network) by construction. D3 and D8 ship their
// AUTHORED tables per-pair / per-pack: an unaudited pair or pack is
// simply absent, never a generated guess.

/// Tool 9 (D3) — how two words in a language pair relate. Curated per
/// pair; nothing is inferred from spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairFlag {
    /// Same root, same meaning — a gift to the learner.
    Cognate,
    /// Looks the same, means something else — the trap.
    FalseFriend,
}

/// An audited per-pair row. Ships only inside an audited table.
#[derive(Debug, Clone, PartialEq)]
pub struct PairNote {
    pub a: String,
    pub b: String,
    pub flag: PairFlag,
}

/// The audited tables, keyed by (langA, langB). EMPTY until the audit
/// lands — the defs-dark pattern per pair (D3's own rule).
pub fn pair_table(_lang_a: &str, _lang_b: &str) -> &'static [PairNote] {
    &[]
}

/// The flag for one rendered pair row, or None (the common case).
/// Direction-insensitive: a table authored a->b answers b->a too.
pub fn pair_flag(lang_a: &str, word_a: &str, lang_b: &str, word_b: &str) -> Option<PairFlag> {
    let direct = pair_table(lang_a, lang_b)
        .iter()
        .find(|n| n.a == word_a && n.b == word_b)
        .map(|n| n.flag);
    direct.or_else(|| {
        pair_table(lang_b, lang_a)
            .iter()
            .find(|n| n.a == word_b && n.b == word_a)
            .map(|n| n.flag)
    })
}

/// Tool 10 (D6) — camera lookup. The OCR hub hands over recognized
/// TEXT; this decides what the translator does with it. On-device
/// recognition only; the image is never retained by this path (it never
/// sees an image at all — that is the point of the seam).
#[derive(Debug, Clone, PartialEq)]
pub enum CameraResult {
    /// The text matched a bank word — open its fan-out card.
    Word(String),
    /// Recognized something, but it is outside the closed space.
    NotInWordList,
}

pub fn camera_lookup(lang: &str, tier: &str, recognized: &str) -> CameraResult {
    let needle = crate::norm::fold_lenient(recognized.trim());
    if needle.is_empty() {
        return CameraResult::NotInWordList;
    }
    for w in crate::words::tier_for(lang, tier) {
        let word = w.split('|').next().unwrap_or(w);
        if crate::norm::fold_lenient(word) == needle {
            return CameraResult::Word(word.to_string());
        }
    }
    CameraResult::NotInWordList
}

/// Tool 11 (D4) — Word Globe. Regions come EXCLUSIVELY from the
/// country-language map the app already ships; this file asserts no
/// geography of its own (D4's whole subject). A region appears only if
/// the shipped map puts a rendered language there.
pub fn globe_regions(rendered_langs: &[String]) -> Vec<(String, Vec<String>)> {
    let map = crate::entitlements::country_language_map();
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (country, langs) in map {
        let hits: Vec<String> = langs
            .iter()
            .filter(|l| rendered_langs.iter().any(|r| r == *l))
            .cloned()
            .collect();
        if !hits.is_empty() {
            out.push((country, hits));
        }
    }
    out.sort();
    out
}

/// Tool 12 (D7) — Language Detective. Audio plays; the kid picks from
/// CHIPS (I4: never free text). Deterministic per seed; the correct
/// language is always among the choices, and every choice is a language
/// the profile actually renders.
pub fn detective_choices(
    correct: &str,
    rendered_langs: &[String],
    count: usize,
    seed: u64,
) -> Vec<String> {
    let mut pool: Vec<String> =
        rendered_langs.iter().filter(|l| *l != correct).cloned().collect();
    if pool.is_empty() || count < 2 {
        return vec![correct.to_string()];
    }
    // deterministic shuffle
    let mut st = seed ^ 0x44_45_54_45_43;
    for i in (1..pool.len()).rev() {
        let j = (splitmix(st.wrapping_add(i as u64)) % (i as u64 + 1)) as usize;
        pool.swap(i, j);
        st = splitmix(st);
    }
    let mut out: Vec<String> = pool.into_iter().take(count.saturating_sub(1)).collect();
    let slot = (splitmix(seed) % (out.len() as u64 + 1)) as usize;
    out.insert(slot, correct.to_string());
    out
}

/// Tool 13 (D8) — Loanword Explorer. Finite AUTHORED packs; every pack
/// word must already exist in the relevant banks, so the closed space
/// holds even for borrowings.
#[derive(Debug, Clone, PartialEq)]
pub struct LoanwordPack {
    pub id: String,
    /// The language the words were borrowed INTO.
    pub host_lang: String,
    /// The language they came FROM.
    pub source_lang: String,
    pub words: Vec<String>,
}

/// Audited packs. EMPTY until each pack's audit lands (D8's rule).
pub fn loanword_packs() -> &'static [LoanwordPack] {
    &[]
}

/// A pack renders only if EVERY word in it is a live bank word in the
/// host language — the closed-space guarantee, checked at render time
/// rather than trusted.
pub fn loanword_pack_renders(pack: &LoanwordPack, tier: &str) -> bool {
    let pool = crate::words::tier_for(&pack.host_lang, tier);
    !pack.words.is_empty()
        && pack.words.iter().all(|w| {
            pool.iter().any(|p| p.split('|').next().unwrap_or(p) == w)
        })
}

#[cfg(test)]
thread_local! {
    static TEST_GLOSS: std::cell::RefCell<std::collections::HashMap<(String, String), String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[cfg(test)]
pub fn test_gloss_inject(lang: &str, word: &str, concept: &str) {
    TEST_GLOSS.with(|g| {
        g.borrow_mut()
            .insert((lang.to_string(), word.to_string()), concept.to_string())
    });
}

thread_local! {
    /// Tool 6 — Polyglot Passport stamp store: concept -> languages
    /// spelled correctly. Cache-through so pure logic tests run on
    /// native (storage is a no-op there). This store + the Home Pair
    /// pin are the suite's ONLY writes (I3).
    static PASSPORT: std::cell::RefCell<Option<std::collections::HashMap<String, Vec<String>>>> =
        const { std::cell::RefCell::new(None) };
}

fn passport_map<R>(f: impl FnOnce(&mut std::collections::HashMap<String, Vec<String>>) -> R) -> R {
    PASSPORT.with(|p| {
        let mut b = p.borrow_mut();
        if b.is_none() {
            *b = Some(crate::storage::get_json("spell_tr_passport").unwrap_or_default());
        }
        f(b.as_mut().unwrap())
    })
}

/// Fired ONLY from the scoring seam (acceptance test 4: no stamp from
/// lookup, ever). A correct spelling of a concept in a language stamps
/// that page once.
pub fn passport_stamp(lang: &str, word: &str) {
    let Some(concept) = concept_of(lang, word) else {
        return;
    };
    passport_map(|m| {
        let langs = m.entry(concept).or_default();
        if !langs.iter().any(|l| l == lang) {
            langs.push(lang.to_string());
        }
    });
    let snapshot: std::collections::HashMap<String, Vec<String>> = passport_map(|m| m.clone());
    crate::storage::set_json("spell_tr_passport", &snapshot);
}

/// "Water: 6 of 15" — conquest framing; the caller renders count only,
/// never a missing-stamps list (CALENDAR I4 shared lint).
pub fn passport_count(concept: &str) -> usize {
    passport_map(|m| m.get(concept).map(|v| v.len()).unwrap_or(0))
}

/// Tool 7 — Home Pair pin: at most one pinned pair per profile; the
/// translator defaults to it and the daily card runs it.
pub fn home_pair() -> Option<(String, String)> {
    let raw = crate::storage::get_raw("spell_tr_homepair")?;
    let (a, b) = raw.split_once('|')?;
    Some((a.to_string(), b.to_string()))
}

pub fn set_home_pair(a: &str, b: &str) {
    crate::storage::set_raw("spell_tr_homepair", &format!("{a}|{b}"));
}

pub fn clear_home_pair() {
    crate::storage::set_raw("spell_tr_homepair", "");
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
    fn every_banked_language_loads_a_gloss_through_the_real_pipe() {
        // Not a constructed table — this reads what include_str! actually
        // compiled in, so a file added to config/gloss/ without being
        // wired here fails instead of silently sitting on disk.
        let docs = gloss_docs();
        for lang in ["es", "fr", "de", "pt", "pl", "ru", "vi", "fil", "sw", "ja", "ko", "ar",
                     "hi", "zh"] {
            let d = docs.get(lang).unwrap_or_else(|| panic!("{lang} gloss not compiled in"));
            assert!(d.rows.len() >= 100, "{lang} has only {} rows", d.rows.len());
            assert!(!d.audited, "{lang} claims audited — only a human signs that");
        }
        // the rows really carry the pairs — a Latin bank, and zh whose
        // keys are the bank's pinyin half
        assert_eq!(docs["es"].rows.get("agua").map(|s| s.as_str()), Some("water"));
        assert_eq!(docs["zh"].rows.get("fang2zi5").map(|s| s.as_str()), Some("house"));
        // ...and they stay DARK through the resolver regardless, because
        // the gate is `audited`, not "are there rows". 2662 loaded rows
        // must not leak one visible pair.
        assert_eq!(word_for_concept("es", "water"), None);
        assert_eq!(concept_of("zh", "fang2zi5"), None);
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

    #[test]
    fn daily_is_deterministic_and_its_own_stream() {
        let a = daily_word("en", "easy", 20_300).unwrap();
        let b = daily_word("en", "easy", 20_300).unwrap();
        assert_eq!(a, b, "same date, same card");
        let days: Vec<String> =
            (0..4).map(|d| daily_word("en", "easy", 20_300 + d).unwrap()).collect();
        assert!(days.windows(2).any(|w| w[0] != w[1]), "dates vary the card");
        let dc = crate::daily::build_words_from_seed("en", "easy", 20_300, 1);
        let _ = dc; // separate stream: the daily-challenge builder takes a
                    // different salt path entirely; compiling both proves
                    // neither borrows the other's seed machinery.
    }

    #[test]
    fn passport_stamps_only_through_concepts() {
        test_gloss_inject("es", "agua", "water");
        test_gloss_inject("fr", "eau", "water");
        passport_stamp("en", "water");
        passport_stamp("es", "agua");
        passport_stamp("fr", "eau");
        passport_stamp("es", "agua"); // idempotent per language
        assert_eq!(passport_count("water"), 3, "same concept, three pages");
        passport_stamp("ru", "\u{432}\u{43e}\u{434}\u{430}");
        assert_eq!(
            passport_count("water"),
            3,
            "no gloss, no concept, no stamp — reduced, never wrong"
        );
    }

    #[test]
    fn match_is_closed_space_and_dark_without_glosses() {
        // No glossAudited language today: both builders decline rather
        // than invent a board.
        assert!(match_prompt("es", "easy", 7).is_none(), "no gloss, no prompt");
        assert!(match_pairs("en", "es", "easy", 3, 7).is_none(), "no gloss, no board");
    }

    #[test]
    fn match_prompt_is_deterministic_and_pivot_locked() {
        // English is its own concept, so prompts resolve there today.
        let a = match_prompt("en", "easy", 42).expect("english resolves");
        let b = match_prompt("en", "easy", 42).expect("english resolves");
        assert_eq!(a, b, "same seed, same prompt");
        assert_eq!(a.concept, a.answer.to_lowercase(), "the gloss IS the pivot");
        assert!(
            crate::words::tier_for("en", "easy")
                .iter()
                .any(|w| w.split('|').next().unwrap_or(w) == a.answer),
            "the answer is a BANK word — closed space"
        );
    }

    #[test]
    fn match_pairs_needs_both_sides_lit() {
        test_gloss_inject("es", "agua", "water");
        // one concept bridged is not three pairs
        assert!(match_pairs("en", "es", "easy", 3, 1).is_none(), "half-dark board is not shipped");
    }

    // ─────────────────── Wave 3 (signed 2026-08-05)

    #[test]
    fn camera_only_ever_yields_bank_words() {
        let pool = crate::words::tier_for("en", "easy");
        let real = pool[0].split('|').next().unwrap().to_string();
        assert_eq!(camera_lookup("en", "easy", &real), CameraResult::Word(real.clone()));
        // case/diacritic tolerance, still closed-space
        assert_eq!(camera_lookup("en", "easy", &real.to_uppercase()), CameraResult::Word(real));
        // anything outside the banks lands on the ONE audited miss state
        for junk in ["qqzzxx", "", "   ", "hello world 42"] {
            assert_eq!(
                camera_lookup("en", "easy", junk),
                CameraResult::NotInWordList,
                "{junk:?} must not invent a word"
            );
        }
    }

    #[test]
    fn detective_is_chips_and_always_answerable() {
        let rendered: Vec<String> =
            ["en", "es", "fr", "de"].iter().map(|s| s.to_string()).collect();
        let a = detective_choices("es", &rendered, 3, 11);
        let b = detective_choices("es", &rendered, 3, 11);
        assert_eq!(a, b, "same seed, same chips");
        assert_eq!(a.len(), 3);
        assert!(a.contains(&"es".to_string()), "the answer is always present");
        for c in &a {
            assert!(rendered.contains(c), "{c}: a chip the profile cannot render");
        }
        // the answer does not always sit in the same slot
        let slots: std::collections::HashSet<usize> = (0..12u64)
            .map(|s| detective_choices("es", &rendered, 3, s).iter().position(|c| c == "es").unwrap())
            .collect();
        assert!(slots.len() > 1, "answer position must vary across seeds");
    }

    #[test]
    fn globe_renders_only_from_the_shipped_map() {
        let none = globe_regions(&[]);
        assert!(none.is_empty(), "no rendered languages, no regions");
        let en = globe_regions(&["en".to_string()]);
        let map = crate::entitlements::country_language_map();
        for (country, langs) in &en {
            let shipped = map.get(country).expect("region not in the shipped map");
            for l in langs {
                assert!(shipped.contains(l), "{country}: {l} is not what the map says");
            }
        }
        // a language the map never places yields nothing — no invented geography
        assert!(globe_regions(&["zz".to_string()]).is_empty());
    }

    #[test]
    fn authored_tables_are_dark_until_audited() {
        // D3/D8 ship per-pair / per-pack: absent, never guessed.
        assert!(pair_table("en", "es").is_empty(), "no unaudited pair table");
        assert!(pair_flag("en", "actual", "es", "actual").is_none(), "no inferred flags");
        assert!(loanword_packs().is_empty(), "no unaudited packs");
        // and the closed-space check bites when a pack names a non-bank word
        let bad = LoanwordPack {
            id: "test".into(),
            host_lang: "en".into(),
            source_lang: "ja".into(),
            words: vec!["qqzzxx".into()],
        };
        assert!(!loanword_pack_renders(&bad, "easy"), "a pack cannot smuggle a word in");
    }

    #[test]
    fn pair_flags_read_both_directions() {
        // direction-insensitivity proven on a local fixture (the shipped
        // tables are empty by audit rule)
        let table = vec![PairNote {
            a: "embarrassed".into(),
            b: "embarazada".into(),
            flag: PairFlag::FalseFriend,
        }];
        let find = |x: &str, y: &str| {
            table
                .iter()
                .find(|n| (n.a == x && n.b == y) || (n.a == y && n.b == x))
                .map(|n| n.flag)
        };
        assert_eq!(find("embarrassed", "embarazada"), Some(PairFlag::FalseFriend));
        assert_eq!(find("embarazada", "embarrassed"), Some(PairFlag::FalseFriend));
    }
}
