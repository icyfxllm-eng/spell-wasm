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
        assert_eq!(p.words.len() + p.removed_purity + p.removed_soundalike, tier.len(), "{lang}: pool is not all of Expert");
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
