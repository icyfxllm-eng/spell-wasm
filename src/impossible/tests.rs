//! CC-IMPOSSIBLE Phase A tests: v1 A1-A3, A9, A10 (as amended by v1.1 and v1.2
//! section 10) and v1.2 X1-X4, X9-X11. Gates are tested through fixture
//! `Inputs`; pool contents are tested against the real artifacts and banks.

use super::*;
use crate::entitlements::{free_tier, resolve_entitlements};

/// The languages with a verified SENSE-CUE table (v1.2 P5 after SENSE-CUE Phase B).
const POOL_LANGS: [&str; 3] = ["en", "es", "ru"];

/// CC-IMPOSSIBLE-COMPLETE F1: languages that open through an `exclusion_only` gate.
const GATED_LANGS: [&str; 7] = ["fr", "de", "pt", "pl", "ko", "fil", "sw"];

/// hi, vi and ar are held (Eric, 2026-10-19): the recogniser could not hear them.
const HELD_LANGS: [&str; 3] = ["vi", "ar", "hi"];

/// Every language with a pool today, in registry order.
fn pool_langs() -> Vec<&'static str> {
    registry_langs().filter(|l| POOL_LANGS.contains(l) || GATED_LANGS.contains(l)).collect()
}

/// The tier a language's pool is drawn from, as the app serves it.
fn source_tier(lang: &str) -> &'static [&'static str] {
    if lang == "en" {
        crate::word_data_en_retier::EN_RETIER_EXPERT
    } else {
        crate::words::tier_for(lang, "expert")
    }
}

fn fixture(lang: &str, n: usize) -> &'static Pool {
    Box::leak(Box::new(Pool {
        lang: lang.into(),
        source: "expert".into(),
        bank_hash: None,
        expert_rows: n,
        table: "verified".into(),
        removed_purity: 0,
        removed_soundalike: 0,
        removed_reviewed: 0,
        status: if n >= FLOOR { "pool" } else { "locked" }.into(),
        reason: if n >= FLOOR { None } else { Some("thin".into()) },
        words: (0..n).map(|k| format!("{lang}{k}")).collect(),
    }))
}

fn all_full() -> &'static EntitlementSet {
    static FULL: OnceLock<EntitlementSet> = OnceLock::new();
    FULL.get_or_init(|| resolve_entitlements(true, &[], false))
}

fn yes(_: &str) -> bool {
    true
}

fn real_pool(l: &str) -> Option<&'static Pool> {
    pool(l)
}

/// Every gate open, real pools, Full entitlements, en.retier on; `edit` closes
/// whichever gate a test is about.
fn run<'a, R>(edit: impl FnOnce(&mut Inputs<'a>), f: impl FnOnce(&Inputs<'a>) -> R) -> R {
    let mut i = Inputs {
        master: true,
        lang_flag: &yes,
        mix_flag: true,
        en_retier: true,
        junior: false,
        lang_open: &yes,
        entitlements: all_full(),
        pools: &real_pool,
    };
    edit(&mut i);
    f(&i)
}

// ---------- pool contents (v1 A1, A2, A9; v1.2 X9) ----------

#[test]
fn every_registry_language_has_an_artifact() {
    for lang in registry_langs() {
        let p = pool(lang).unwrap_or_else(|| panic!("{lang}: pool artifact missing or unparseable"));
        assert_eq!(p.lang, lang);
        assert!(p.status == "pool" || p.reason.is_some(), "{lang}: a locked artifact must say why");
    }
}

/// en (re-tier), es and ru have pools through verified tables; fr de pt pl ko fil sw through
/// the F1 exclusion gate; zh and ja are locked by D4; hi, vi and ar are held.
#[test]
fn which_languages_have_pools_today() {
    for lang in registry_langs() {
        let p = pool(lang).unwrap();
        let want = if POOL_LANGS.contains(&lang) || GATED_LANGS.contains(&lang) {
            None
        } else if lang == "zh" || lang == "ja" {
            Some("drawn-entry")
        } else {
            Some("table-missing")
        };
        assert_eq!(p.reason.as_deref(), want, "{lang}");
        assert_eq!(p.status == "pool", want.is_none(), "{lang}");
        let table = if GATED_LANGS.contains(&lang) { "exclusion_only" } else if POOL_LANGS.contains(&lang) { "verified" } else { "missing" };
        assert_eq!(p.table, table, "{lang}");
    }
    assert_eq!(pool("en").unwrap().source, "en-retier-expert");
}

/// v1 A1 / v1.2 X9 `mix_pool_is_all_expert`: a pool is a subset of its source
/// Expert tier, shares nothing with the other tiers, and is ALL of that tier
/// bar the PURITY and sound-alike removals (no slice, no ranking).
#[test]
fn impossible_pool_subset_and_all_expert() {
    for lang in pool_langs() {
        let p = pool(lang).unwrap();
        let tier: std::collections::HashSet<&str> = source_tier(lang).iter().copied().collect();
        for w in &p.words {
            assert!(tier.contains(w.as_str()), "{lang}: {w} is not in its Expert tier");
        }
        assert_eq!(p.expert_rows, tier.len(), "{lang}: expertRows drifted from the tier");
        assert_eq!(p.words.len() + p.removed_purity + p.removed_soundalike + p.removed_reviewed, tier.len(), "{lang}: pool is not all of Expert");
        let others: Vec<&str> = if lang == "en" {
            use crate::word_data_en_retier::*;
            [EN_RETIER_EASY, EN_RETIER_MEDIUM, EN_RETIER_HARD].concat()
        } else {
            crate::consts::TIER_ORDER.iter().filter(|t| **t != "expert").flat_map(|t| crate::words::tier_for(lang, t).iter().copied()).collect()
        };
        let others: std::collections::HashSet<&str> = others.into_iter().collect();
        assert!(p.words.iter().all(|w| !others.contains(w.as_str())), "{lang}: pool overlaps a non-Expert tier");
    }
}

/// v1 A2 as replaced by v1.1 A2: no pool word is a member of any "true" or
/// "accent" set in the language's table (English: the re-tier table).
#[test]
fn impossible_pool_no_soundalikes() {
    crate::flags::set_test_override(Some("on")); // en.retier on, so "en" reads the re-tier table
    for lang in POOL_LANGS {
        let members: std::collections::HashSet<String> = crate::soundalikes::sets_of_kind(lang, &["true", "accent"])
            .iter()
            .flat_map(|s| s.members.iter().map(|m| m.member_id.to_lowercase()))
            .collect();
        assert!(!members.is_empty(), "{lang}: a verified table with no sets would make this test vacuous");
        for w in &pool(lang).unwrap().words {
            assert!(!members.contains(&w.to_lowercase()), "{lang}: {w} has a sound-alike and is in the pool");
        }
    }
    crate::flags::set_test_override(None);
}

/// v1.1 A2.1 / A2.2: the English pairs the spec names never appear.
#[test]
fn english_named_pairs_absent() {
    let en = &pool("en").unwrap().words;
    for w in ["sink", "sync", "cot", "caught"] {
        assert!(!en.iter().any(|x| x == w), "{w} is in the English pool");
    }
}

/// v1 A9 / I9: the artifact's recorded bank hash equals the served tier's hash,
/// so a bank edit without `tools/impossible/build_pools.py` fails here.
#[test]
fn impossible_pool_artifact_fresh() {
    for lang in registry_langs() {
        let p = pool(lang).unwrap();
        if p.reason.as_deref() == Some("drawn-entry") {
            assert!(p.bank_hash.is_none(), "{lang}: D4-locked artifacts read no bank");
            continue;
        }
        let want = format!("0x{:016X}", crate::wordid::compute_list_hash(source_tier(lang)));
        assert_eq!(p.bank_hash.as_deref(), Some(want.as_str()), "{lang}: stale Impossible pool; rerun tools/impossible/build_pools.py");
    }
}

// ---------- the gate (v1 A3, A10; v1.2 X2, X10, X11) ----------

/// v1 A3 / v1.2 X11 `mix_floor`: 149 filtered rows is locked, 150 is in.
#[test]
fn impossible_gate_threshold() {
    let p149 = fixture("es", 149);
    let p150 = fixture("es", 150);
    let at149 = |_: &str| Some(p149);
    let at150 = |_: &str| Some(p150);
    assert_eq!(run(|i| i.pools = &at149, |i| lang_availability("es", i)), Availability::Locked(LockReason::PoolBelowFloor));
    assert_eq!(run(|i| i.pools = &at150, |i| lang_availability("es", i)), Availability::Available(150));
}

/// v1 A10 / I8, D4: a closed language stays unreachable with its flag on, and
/// zh / ja are locked whatever their flags say.
#[test]
fn impossible_closed_language() {
    let closed = |l: &str| l != "ar";
    assert_eq!(run(|i| i.lang_open = &closed, |i| lang_availability("ar", i)), Availability::Hidden);
    for lang in ["zh", "ja"] {
        assert_eq!(run(|_| {}, |i| lang_availability(lang, i)), Availability::Locked(LockReason::NoDrawnEntry), "{lang}");
    }
}

/// v1.1 A3: a `missing` table with no exclusion gate keeps the language locked (fail closed).
#[test]
fn missing_table_locks() {
    for lang in HELD_LANGS {
        assert_eq!(run(|_| {}, |i| lang_availability(lang, i)), Availability::Locked(LockReason::NoExclusionGate), "{lang}");
    }
}

/// CC-IMPOSSIBLE-COMPLETE A-1 / I2: an `exclusion_only` language is available only when its
/// pool after exclusion clears 150; below that it is `pool_below_floor`, never open.
#[test]
fn exclusion_only_opens_and_respects_floor() {
    for lang in GATED_LANGS {
        let n = pool(lang).unwrap().words.len();
        assert!(n >= FLOOR, "{lang}: gated language below the floor");
        assert_eq!(run(|_| {}, |i| lang_availability(lang, i)), Availability::Available(n), "{lang}");
    }
    let thin = fixture("fr", 149);
    let thin = Box::leak(Box::new(Pool { table: "exclusion_only".into(), ..thin.clone() }));
    let at = |_: &str| Some(&*thin);
    assert_eq!(run(|i| i.pools = &at, |i| lang_availability("fr", i)), Availability::Locked(LockReason::PoolBelowFloor));
}

/// A-3 / I3 / D-C1: an `exclusion_only` gate opens Impossible and nothing else. The SENSE-CUE
/// table behind the cues and the sound-alike bonus stays `missing` for every gated language.
#[test]
fn exclusion_only_never_enables_cues() {
    for lang in GATED_LANGS.iter().chain(HELD_LANGS.iter()) {
        assert_eq!(crate::soundalikes::soundalikes(lang).status, "missing", "{lang}: cues must stay off");
        assert!(crate::soundalikes::sets_of_kind(lang, &["true", "accent"]).is_empty(), "{lang}");
    }
}

/// A-1: the reviewed pairs are excluded, and the exclusion data names its voice (D-C16).
#[test]
fn exclusion_gate_catches_reviewed_pairs() {
    let gone = |lang: &str, w: &str| !pool(lang).unwrap().words.iter().any(|x| x == w);
    for (lang, w) in [("ko", "계통"), ("ko", "미처"), ("fr", "abandonnent"), ("sw", "unaoonekana"), ("de", "bayrischen")] {
        assert!(gone(lang, w), "{lang}: {w} is a reviewed sound-alike and is in the pool");
    }
    // REJECT rows stay: recogniser noise is not a sound-alike.
    assert!(!gone("ko", "단면"), "ko: 단면 was REJECT and must stay in the pool");
    for lang in GATED_LANGS {
        let json = std::fs::read_to_string(format!("{}/data/impossible/exclusion/{lang}.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(json.contains("\"voice\"") && json.contains("clipCacheVersion"), "{lang}");
    }
}

/// A-14 / F11: the availability report has one row per registry language and its pool counts
/// match the artifacts (the generator also checks it on every gate run).
#[test]
fn availability_report_matches_artifacts() {
    let md = std::fs::read_to_string(format!("{}/reports/impossible-availability.md", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for lang in registry_langs() {
        let row = md.lines().find(|l| l.starts_with(&format!("| {lang} |"))).unwrap_or_else(|| panic!("{lang}: no row"));
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_eq!(cells[5], pool(lang).unwrap().words.len().to_string(), "{lang}: report count differs from the artifact");
    }
    assert_eq!(md.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| Lang")).count(), registry_langs().count());
}

/// F3 / A-6: the true reason for every lock, as a stable code.
#[test]
fn reason_codes() {
    let free = free_tier();
    // A free English-only profile: Mix comes with Complete, not "needs two languages".
    assert_eq!(run(|i| i.entitlements = &free, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::NeedsComplete));
    // A regional grant is an owned language; one passing language is still not two.
    let grant = resolve_entitlements(false, &["es"], false);
    let only_es = |l: &str| l == "es";
    assert_eq!(
        run(|i| { i.entitlements = &grant; i.lang_flag = &only_es; }, |i| mix_status(Surface::Normal, i)),
        MixStatus::Locked(LockReason::NeedsTwoLanguages)
    );
    // A Spell Jr profile is profile_blocked with every flag on.
    assert_eq!(run(|i| i.junior = true, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::ProfileBlocked));
    assert_eq!(run(|i| i.junior = true, |i| lang_availability("fr", i)), Availability::Locked(LockReason::ProfileBlocked));
    // Flags off.
    assert_eq!(run(|i| i.master = false, |i| lang_availability("fr", i)), Availability::Locked(LockReason::FlagOff));
    assert_eq!(run(|i| i.mix_flag = false, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::FlagOff));
    // Stable codes.
    for (r, code) in [
        (LockReason::NeedsComplete, "needs_complete"),
        (LockReason::NeedsTwoLanguages, "needs_two_languages"),
        (LockReason::ProfileBlocked, "profile_blocked"),
        (LockReason::FlagOff, "flag_off"),
        (LockReason::PoolBelowFloor, "language_locked:pool_below_floor"),
        (LockReason::NoExclusionGate, "language_locked:no_exclusion_gate"),
        (LockReason::NoDrawnEntry, "language_locked:no_drawn_entry"),
    ] {
        assert_eq!(r.code(), code);
    }
}

/// Flags default OFF: with the real defaults nothing is offered anywhere.
#[test]
fn defaults_offer_nothing() {
    crate::flags::set_test_override(None);
    assert!(!crate::flags::impossible() && !crate::flags::impossible_mix() && !crate::flags::impossible_lang("es"));
    assert_eq!(run(|i| i.master = false, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::FlagOff));
    assert_eq!(run(|i| i.master = false, |i| lang_availability("es", i)), Availability::Locked(LockReason::FlagOff));
    assert_eq!(run(|i| i.mix_flag = false, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::FlagOff));
}

/// v1 F11: a language below Full access is locked "Unlock with SpellGame
/// Complete"; English is Full on the free tier.
#[test]
fn entitlement_gate() {
    let free = free_tier();
    let ent: &EntitlementSet = &free;
    let got = |lang: &str| run(|i| i.entitlements = ent, |i| lang_availability(lang, i));
    assert_eq!(got("es"), Availability::Locked(LockReason::NeedsComplete));
    assert_eq!(got("ru"), Availability::Locked(LockReason::NeedsComplete));
    assert!(matches!(got("en"), Availability::Available(n) if n >= FLOOR));
}

/// v1.2 X10 `mix_english_needs_retier`: en.retier off, English contributes
/// nothing; on, English's words are exactly the re-tier pool.
#[test]
fn mix_english_needs_retier() {
    let off = run(|i| i.en_retier = false, |i| mix_pool(i));
    assert!(off.iter().all(|(l, _)| *l != "en"));
    assert_eq!(run(|i| i.en_retier = false, |i| lang_availability("en", i)), Availability::Hidden);
    let on = run(|_| {}, |i| mix_pool(i));
    let en: Vec<&str> = on.iter().filter(|(l, _)| *l == "en").map(|(_, w)| *w).collect();
    let retier: std::collections::HashSet<&str> = crate::word_data_en_retier::EN_RETIER_EXPERT.iter().copied().collect();
    assert_eq!(en.len(), pool("en").unwrap().words.len());
    assert!(en.iter().all(|w| retier.contains(w)));
}

/// v1.2 X2 `mix_unavailable_languages_excluded`: flag off, unentitled, closed,
/// or under 150 -- never in the mix.
#[test]
fn mix_unavailable_languages_excluded() {
    let no_ru = |l: &str| l != "ru";
    let langs = |i: &Inputs| match mix_status(Surface::Normal, i) {
        MixStatus::Available(v) => v,
        _ => Vec::new(),
    };
    let without_ru = || pool_langs().into_iter().filter(|l| *l != "ru").collect::<Vec<_>>();
    assert_eq!(run(|i| i.lang_flag = &no_ru, langs), without_ru());
    assert_eq!(run(|i| i.lang_open = &no_ru, langs), without_ru());
    let thin = fixture("ru", 149);
    let thin_ru = |l: &str| if l == "ru" { Some(thin) } else { pool(l) };
    assert_eq!(run(|i| i.pools = &thin_ru, langs), without_ru());
    let grant = resolve_entitlements(false, &["es"], false);
    assert_eq!(run(|i| i.entitlements = &grant, langs), vec!["en", "es"]);
    assert!(run(|i| i.lang_flag = &no_ru, |i| mix_pool(i)).iter().all(|(l, _)| *l != "ru"));
}

// ---------- Mix (v1.2 X1, X3, X4) ----------

/// v1.2 X1 `mix_pool_is_union` / M-I1: the Mix pool is exactly the union of
/// the qualifying languages' pools, every word tagged with its language.
#[test]
fn mix_pool_is_union() {
    let mix = run(|_| {}, |i| mix_pool(i));
    let want: usize = pool_langs().iter().map(|l| pool(l).unwrap().words.len()).sum();
    assert_eq!(mix.len(), want);
    for (lang, w) in &mix {
        assert!(pool(lang).unwrap().words.iter().any(|x| x == w), "{lang}: {w} is not in its own pool");
    }
}

/// v1.2 X3 `mix_no_chooser_notice`: the status lists exactly the qualifying
/// languages (no input narrows it), fewer than two is "Needs two Impossible
/// languages", and the notice is due on first run and whenever the set changes.
#[test]
fn mix_no_chooser_notice() {
    assert_eq!(run(|_| {}, |i| mix_status(Surface::Normal, i)), MixStatus::Available(pool_langs()));
    let free = free_tier();
    assert_eq!(run(|i| i.entitlements = &free, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::NeedsComplete));
    let only_es = |l: &str| l == "es";
    assert_eq!(run(|i| i.lang_flag = &only_es, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::NeedsTwoLanguages));
    // M-I7: the gate's inputs are flags, entitlements, release gates and pools;
    // none is a player setting, so there is nothing a player can untick.
    let langs = ["en", "es", "ru"];
    assert!(notice_due(&langs, None));
    assert!(!notice_due(&langs, Some("en,es,ru")));
    assert!(notice_due(&langs, Some("en,es")), "a newly eligible language re-shows the notice");
}

/// v1.2 X4 `mix_kid_surfaces` / M-I3, v1 I3: never for a Jr/Kid profile, and
/// only on the Normal surface.
#[test]
fn mix_kid_surfaces() {
    assert_eq!(run(|i| i.junior = true, |i| mix_status(Surface::Normal, i)), MixStatus::Locked(LockReason::ProfileBlocked));
    assert_eq!(run(|i| i.junior = true, |i| lang_availability("es", i)), Availability::Locked(LockReason::ProfileBlocked));
    assert!(run(|i| i.junior = true, |i| mix_pool(i)).is_empty());
    for s in [Surface::KidMode, Surface::LittleSpeller, Surface::SpellJr, Surface::Randospell, Surface::ImpossibleDaily, Surface::BoardGame, Surface::Climb] {
        assert_eq!(run(|_| {}, |i| mix_status(s, i)), MixStatus::Hidden, "{s:?}");
    }
}

// ---------- invariants ----------

/// v1.1 I2a / A2.6: the registry holds no homophone data and does no scan; the
/// generator filtered through the SENSE-CUE API. A homophone list, a table read
/// or a key function added to src/impossible.rs fails here.
#[test]
fn registry_has_no_homophone_code() {
    let src = include_str!("../impossible.rs");
    for needle in ["crate::soundalikes", "crate::homophones", concat!("soundalikes", ".json"), concat!("homophones", ".txt"), "fn key_", "HOMOPHONE"] {
        assert!(!src.contains(needle), "src/impossible.rs contains {needle:?}");
    }
}

/// v1 I10 / A14: the pool builder never edits the bank. The artifacts' sources
/// are the served tiers byte for byte, which `impossible_pool_artifact_fresh`
/// pins; here, that no pool word is missing from its tier's pinned hash input.
#[test]
fn expert_tiers_unchanged_by_pools() {
    for lang in ["es", "ru"] {
        assert_eq!(crate::wordid::list_hash(lang, "expert"), crate::wordid::compute_list_hash(crate::words::tier_for(lang, "expert")));
    }
}

// ---------- CC-IMPOSSIBLE-COMPLETE F5 / F6: the shared engine (A-4, A-8, A-9) ----------

use super::engine;

fn xorshift(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// A-4 / D-C4: German case counts at Impossible only.
#[test]
fn german_case_is_strict_only_at_impossible() {
    assert!(engine::grade("de", "Haus", "Haus"));
    assert!(!engine::grade("de", "haus", "Haus"), "haus for Haus is a miss at Impossible");
    assert!(crate::norm::answer_matches("haus", "Haus", false), "and unchanged at Expert and below");
    assert!(engine::grade("es", "CASA", "casa"), "case is not strict outside German");
    assert!(!engine::grade("es", "cafe", "café"), "accents stay strict");
    assert!(engine::grade("fr", " é crit ", "écrit"), "whitespace is ignored");
}

/// A-4 golden: outside German, the Impossible grader and the Expert grader agree on every
/// typed variant of every pool word, so strictness never differs from today's Expert fold.
#[test]
fn impossible_grading_matches_expert_outside_german() {
    for lang in pool_langs().into_iter().filter(|l| *l != "de") {
        for w in pool(lang).unwrap().words.iter().take(400) {
            for typed in [w.to_uppercase(), w.to_lowercase(), format!(" {w} "), w.chars().rev().collect::<String>()] {
                assert_eq!(engine::grade(lang, &typed, w), crate::norm::answer_matches(&typed, w, false), "{lang}: {typed:?} vs {w:?}");
            }
        }
    }
}

/// A-8 / I10 / D-C7: the limits live in one module; game.rs and lib.rs only ask it.
#[test]
fn one_engine() {
    assert!(!engine::replay_allowed(engine::MAX_REPLAYS) && engine::replay_allowed(0));
    assert!(!engine::attempt_allowed(engine::MAX_ATTEMPTS) && engine::attempt_allowed(0));
    let game = include_str!("../game.rs");
    let lib = include_str!("../lib.rs");
    for (name, src) in [("game.rs", game), ("lib.rs", lib)] {
        assert!(!src.contains("MAX_REPLAYS") && !src.contains("MAX_ATTEMPTS"), "{name} holds its own copy of an Impossible limit");
    }
    assert!(game.contains("engine::replay_allowed") && game.contains("engine::grade"), "game.rs must ask the engine");
    // The attempt gate: the extra-attempt toggle, shields and retries are all closed at Impossible.
    assert!(game.contains("&& !s.impossible_word"), "extra_attempt_ctx must exclude Impossible words");
}

/// A-9: language frequencies over 10,000 draws are within +-3% of uniform; D-C12.
#[test]
fn mix_draw_is_uniform_over_languages() {
    let langs = ["en", "es", "ru", "fr", "ko"];
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut n = [0usize; 5];
    for _ in 0..10_000 {
        let l = engine::draw_language(&langs, xorshift(&mut seed)).unwrap();
        n[langs.iter().position(|x| *x == l).unwrap()] += 1;
    }
    for (l, c) in langs.iter().zip(n) {
        let p = c as f64 / 10_000.0;
        assert!((p - 0.2).abs() <= 0.03, "{l}: {p}");
    }
    assert_eq!(engine::draw_language(&[], 0.5), None);
}

/// A-9: no word repeats within a language's pass, over 1,000 seeded runs of the real draw.
#[test]
fn mix_never_repeats_within_a_pass() {
    use std::collections::HashSet;
    let langs = ["en", "es", "ru"];
    for run in 0..1000u64 {
        let mut seed = 0xABCD_1234u64 ^ (run.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut decks: Vec<crate::deck::Deck> = langs.iter().map(|_| Default::default()).collect();
        let pools: Vec<Vec<String>> = langs.iter().map(|l| (0..150).map(|k| format!("{l}{k}")).collect()).collect();
        let mut seen: Vec<HashSet<String>> = vec![HashSet::new(); 3];
        for _ in 0..120 {
            let l = engine::draw_language(&langs, xorshift(&mut seed)).unwrap();
            let i = langs.iter().position(|x| *x == l).unwrap();
            let w = decks[i].next_with(&pools[i], &mut |n| (xorshift(&mut seed) * n as f64) as usize % n);
            assert!(seen[i].insert(w.clone()), "run {run}: {l} repeated {w} inside one pass");
        }
    }
}

/// F3 / F5 / F6: what the level picker shows. Nothing by default; a real lock shows with its
/// reason; Spell Jr sees nothing.
#[test]
fn picker_entries_follow_the_gates() {
    let none = |e: Vec<engine::PickerEntry>| e.is_empty();
    assert!(none(run(|i| i.master = false, |i| engine::picker_entries("es", i))));
    assert!(none(run(|i| i.junior = true, |i| engine::picker_entries("es", i))));
    assert_eq!(run(|_| {}, |i| engine::picker_entries("es", i)), vec![(engine::LEVEL, None), (engine::MIX_LEVEL, None)]);
    let free = free_tier();
    assert_eq!(
        run(|i| i.entitlements = &free, |i| engine::picker_entries("en", i)),
        vec![(engine::LEVEL, None), (engine::MIX_LEVEL, Some(LockReason::NeedsComplete))]
    );
    assert_eq!(
        run(|i| i.entitlements = &free, |i| engine::picker_entries("es", i)),
        vec![(engine::LEVEL, Some(LockReason::NeedsComplete)), (engine::MIX_LEVEL, Some(LockReason::NeedsComplete))]
    );
    assert_eq!(run(|_| {}, |i| engine::picker_entries("hi", i))[0], (engine::LEVEL, Some(LockReason::NoExclusionGate)));
    assert_eq!(run(|_| {}, |i| engine::picker_entries("zh", i))[0], (engine::LEVEL, Some(LockReason::NoDrawnEntry)));
    for r in [LockReason::NeedsComplete, LockReason::NeedsTwoLanguages, LockReason::PoolBelowFloor, LockReason::NoExclusionGate, LockReason::NoDrawnEntry] {
        assert!(engine::lock_copy_key(r).starts_with("impossible.lock."));
    }
}

/// F5: a serve draws only from languages that pass the gate; a closed gate returns None so the
/// caller falls back to plain Expert instead of serving a word nobody is entitled to.
#[test]
fn serve_langs_obey_the_gate() {
    assert_eq!(run(|_| {}, |i| engine::serve_langs(engine::LEVEL, "fr", i)), Some(vec!["fr"]));
    assert_eq!(run(|_| {}, |i| engine::serve_langs(engine::MIX_LEVEL, "fr", i)), Some(pool_langs()));
    assert_eq!(run(|i| i.master = false, |i| engine::serve_langs(engine::LEVEL, "fr", i)), None);
    assert_eq!(run(|_| {}, |i| engine::serve_langs(engine::LEVEL, "hi", i)), None);
    assert_eq!(run(|_| {}, |i| engine::serve_langs("expert", "fr", i)), None);
    // Misses tag: re-served at Impossible rules (M9) via the tier string.
    assert_eq!(engine::MISS_TAG, "impossible");
}

// ---------- CC-IMPOSSIBLE-COMPLETE F7: Impossible Daily (A-10) ----------

/// A-10: every device on the same language and date gets the same words in the same order, whatever
/// order its pool arrived in, and the words come from the pool.
#[test]
fn impossible_daily_is_the_same_for_everyone() {
    for lang in pool_langs() {
        let words = &pool(lang).unwrap().words;
        let a = crate::daily::impossible_slice(lang, words, "2026-10-19");
        let mut rev = words.clone();
        rev.reverse();
        assert_eq!(a, crate::daily::impossible_slice(lang, &rev, "2026-10-19"), "{lang}: pool order must not matter");
        assert_eq!(a.len(), crate::daily::IMP_DAILY_WORDS, "{lang}");
        assert!(a.iter().all(|w| words.contains(w)), "{lang}");
        assert_eq!(a.iter().collect::<std::collections::HashSet<_>>().len(), a.len(), "{lang}: a word twice in one day");
        assert_ne!(a, crate::daily::impossible_slice(lang, words, "2026-10-20"), "{lang}: the next day is a new set");
    }
    let es = crate::daily::impossible_slice("es", &pool("es").unwrap().words, "2026-10-19");
    let ru = crate::daily::impossible_slice("ru", &pool("ru").unwrap().words, "2026-10-19");
    assert_ne!(es, ru);
}

/// No word returns before the whole pool has been dealt (the Daily's cycle walk).
#[test]
fn impossible_daily_does_not_repeat_within_a_cycle() {
    let words: Vec<String> = (0..150).map(|i| format!("w{i:03}")).collect();
    let mut seen = std::collections::HashSet::new();
    // 2026-01-01 is day 0, so these fifteen days are exactly one 150/10-day cycle.
    for d in 1..=15 {
        for w in crate::daily::impossible_slice("xx", &words, &format!("2026-01-{d:02}")) {
            assert!(seen.insert(w.clone()), "{w} repeated inside one cycle (day {d})");
        }
    }
    assert_eq!(seen.len(), 150);
}

/// I9: Mix never offers Daily, and the Impossible Daily is its own surface.
#[test]
fn there_is_no_mix_daily() {
    assert_eq!(run(|_| {}, |i| mix_status(Surface::ImpossibleDaily, i)), MixStatus::Hidden);
    let g = include_str!("../game.rs");
    assert!(g.contains("level == crate::impossible::engine::LEVEL"), "the Daily button is Impossible only on the single-language level");
}

/// Eric 2026-10-19: comptroller stays in the pool and controller is accepted for it, and only for it.
#[test]
fn accepted_alternates_are_per_word() {
    assert!(engine::grade("en", "controller", "comptroller"));
    assert!(engine::grade("en", "Controller", "comptroller"));
    assert!(engine::grade("en", "comptroller", "comptroller"));
    assert!(!engine::grade("en", "controller", "controlled"), "an alternate counts only for its own word");
    assert!(!engine::grade("es", "controller", "comptroller"), "and only in its own language");
    assert!(engine::grade("en", "marquis", "marquess"));
    assert!(engine::grade("en", "bosun", "boatswain"));
    assert!(engine::grade("en", "straight jacket", "straitjacket") && engine::grade("en", "straightjacket", "straitjacket"));
    assert!(!engine::grade("en", "marquess", "marquis"), "the relation is one way: it is keyed by the pool word");
    assert!(engine::accepted_alternates("en", "demesne").is_empty());
}

/// Eric 2026-10-22: the outside-twin sweep. Genuine second spellings are accepted for the pool word; names and
/// non-words that merely share a pronunciation are not.
#[test]
fn outside_twins_of_pool_words_are_accepted() {
    for (word, twin) in [("anesthesiology", "anaesthesiology"), ("archaeological", "archeological"), ("rescission", "recision"), ("sibyl", "sybil")] {
        assert!(engine::grade("en", twin, word), "{twin} for {word}");
        assert!(engine::grade("en", word, word));
    }
    assert!(!engine::grade("en", "shali", "challis"), "a non-word twin is not a spelling");
    assert!(!engine::grade("en", "mer", "myrrh"));
    assert!(!engine::grade("en", "dumaine", "demesne"));
}

/// Every accepted-alternates key is a word in that language's pool (a stale entry would be dead data), and no
/// alternate is the pool word itself.
#[test]
fn accepted_alternates_name_pool_words() {
    let json: serde_json::Value = serde_json::from_str(include_str!("../../data/impossible/accepted/en.json")).unwrap();
    for (word, alts) in json["accepted"].as_object().unwrap() {
        assert!(pool("en").unwrap().words.iter().any(|w| w == word), "{word} is not in the English pool");
        for a in alts.as_array().unwrap() {
            assert_ne!(a.as_str().unwrap(), word);
        }
    }
}

/// Outside-twin sweep 2026-10-22: Russian ё / е are one spelling to the ear, so either is right.
#[test]
fn russian_yo_and_ye_are_both_accepted() {
    assert!(engine::grade("ru", "актёр", "актер"));
    assert!(engine::grade("ru", "актер", "актёр"));
    assert!(engine::grade("ru", "Дойдёт", "дойдет"));
    assert!(!engine::grade("ru", "актор", "актер"), "only ё/е are folded");
    assert!(!engine::grade("de", "aktor", "actor"));
}
