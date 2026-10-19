//! CC-IMPOSSIBLE -- the ONE registry for the Impossible tier and Impossible Mix.
//!
//! v1 section 2: availability, eligibility and entitlement come from here and
//! nowhere else; UI code reads [`lang_availability`] / [`mix_status`] and never
//! re-derives a gate. This is Phase A of v1.2 (Mix) on top of the v1 Phase A
//! pieces Mix needs: the pool artifacts, the F3 gate and the F12 flags.
//!
//! Pools are GENERATED (`tools/impossible/build_pools.py` writes
//! `assets/impossible/impossible_pool.<lang>.json`). v1.2 section 10: a pool is
//! 100% of the language's Expert tier after PURITY, the v1.1 sound-alike filter
//! (kinds "true" and "accent") and the proper-noun assertion. English draws from
//! the re-tier Expert tier only, and only while en.retier is on (v1.2 P2).
//!
//! CC-IMPOSSIBLE-COMPLETE v1 F1: a language whose SENSE-CUE table is still `missing` can
//! open through an `exclusion_only` gate (`data/impossible/exclusion/<lang>.json`, built
//! by `tools/impossible/build_exclusion.py`). The artifact says so (`table`); this file
//! only reads the artifact. An `exclusion_only` gate opens Impossible and nothing else:
//! no cues, no sound-alike bonus (D-C1, I3).
//!
//! Nothing here holds homophone data or scans for sound-alikes (v1.1 I2a): the
//! filtering happened in the generator, through the SENSE-CUE API.
//!
//! Every gate is a pure function of [`Inputs`], so the tests cover every branch
//! without touching storage; [`live_inputs`] is the one impure read.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::entitlements::{AccessLevel, EntitlementSet};

/// v1 D3, kept by v1.2 P3.
pub const FLOOR: usize = 150;

/// v1.2 X4: Mix needs at least this many qualifying languages.
pub const MIX_MIN_LANGS: usize = 2;

/// One language's generated pool artifact.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Pool {
    pub lang: String,
    /// "expert", or "en-retier-expert" for English.
    pub source: String,
    /// wordListHash of the source tier when the artifact was built (v1 I9).
    #[serde(rename = "bankHash")]
    pub bank_hash: Option<String>,
    #[serde(rename = "expertRows")]
    pub expert_rows: usize,
    /// The sound-alike table's status at build time.
    pub table: String,
    #[serde(rename = "removedPurity")]
    pub removed_purity: usize,
    #[serde(rename = "removedSoundalike")]
    pub removed_soundalike: usize,
    /// "pool" or "locked".
    pub status: String,
    /// Why it is locked: "drawn-entry", "table-missing" or "thin".
    pub reason: Option<String>,
    pub words: Vec<String>,
}

fn source(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => include_str!("../assets/impossible/impossible_pool.en.json"),
        "es" => include_str!("../assets/impossible/impossible_pool.es.json"),
        "fr" => include_str!("../assets/impossible/impossible_pool.fr.json"),
        "de" => include_str!("../assets/impossible/impossible_pool.de.json"),
        "pt" => include_str!("../assets/impossible/impossible_pool.pt.json"),
        "pl" => include_str!("../assets/impossible/impossible_pool.pl.json"),
        "vi" => include_str!("../assets/impossible/impossible_pool.vi.json"),
        "ko" => include_str!("../assets/impossible/impossible_pool.ko.json"),
        "ja" => include_str!("../assets/impossible/impossible_pool.ja.json"),
        "fil" => include_str!("../assets/impossible/impossible_pool.fil.json"),
        "zh" => include_str!("../assets/impossible/impossible_pool.zh.json"),
        "ru" => include_str!("../assets/impossible/impossible_pool.ru.json"),
        "ar" => include_str!("../assets/impossible/impossible_pool.ar.json"),
        "sw" => include_str!("../assets/impossible/impossible_pool.sw.json"),
        "hi" => include_str!("../assets/impossible/impossible_pool.hi.json"),
        _ => return None,
    })
}

/// The pool artifact for `lang`, parsed once. `None` for a language outside the
/// registry or an artifact that fails to parse (the gate rebuilds and checks
/// these on every push, so that is a build bug -- and it fails closed: no pool,
/// no Impossible).
pub fn pool(lang: &str) -> Option<&'static Pool> {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Option<&'static Pool>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(p) = cache.lock().unwrap().get(lang) {
        return *p;
    }
    let parsed = source(lang)
        .and_then(|json| serde_json::from_str::<Pool>(json).ok())
        .map(|p| &*Box::leak(Box::new(p)));
    cache.lock().unwrap().insert(lang.to_string(), parsed);
    parsed
}

/// Why Impossible or Mix is not available (CC-IMPOSSIBLE-COMPLETE F3). Every surface
/// renders the copy for [`LockReason::code`]; none computes availability itself (I5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockReason {
    /// Fewer than 150 eligible rows after exclusion (v1 D3): `language_locked:pool_below_floor`.
    PoolBelowFloor,
    /// No verified sound-alike table and no exclusion gate (v1.1 A3 fails closed):
    /// `language_locked:no_exclusion_gate`.
    NoExclusionGate,
    /// zh and ja until drawn entry ships (v1 D4): `language_locked:no_drawn_entry`.
    NoDrawnEntry,
    /// The player does not own the language (v1 F11), or, for Mix, owns no language
    /// beyond free English: "Comes with SpellGame Complete".
    NeedsComplete,
    /// Mix only: the player owns two or more languages but fewer than two pass their gates.
    NeedsTwoLanguages,
    /// Spell Jr / Kid profile (v1 I3, I7).
    ProfileBlocked,
    /// The master, language or Mix flag is off.
    FlagOff,
}

impl LockReason {
    /// The stable reason code every surface keys its copy on.
    pub fn code(self) -> &'static str {
        match self {
            LockReason::PoolBelowFloor => "language_locked:pool_below_floor",
            LockReason::NoExclusionGate => "language_locked:no_exclusion_gate",
            LockReason::NoDrawnEntry => "language_locked:no_drawn_entry",
            LockReason::NeedsComplete => "needs_complete",
            LockReason::NeedsTwoLanguages => "needs_two_languages",
            LockReason::ProfileBlocked => "profile_blocked",
            LockReason::FlagOff => "flag_off",
        }
    }
}

/// The registry's answer for one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Nothing to show: the language is closed to everyone by another gate (RTL, web,
    /// DEV_PREVIEW), or English without the re-tier. v1 I8: Impossible never opens a
    /// language that is otherwise closed.
    Hidden,
    /// Not playable, with the one true reason (F3).
    Locked(LockReason),
    /// Playable, with this many pool words.
    Available(usize),
}

/// Everything the gates read, gathered once so they stay pure.
pub struct Inputs<'a> {
    /// `flags::impossible()`.
    pub master: bool,
    /// `flags::impossible_lang(lang)`.
    pub lang_flag: &'a dyn Fn(&str) -> bool,
    /// `flags::impossible_mix()`.
    pub mix_flag: bool,
    /// `flags::en_retier()`.
    pub en_retier: bool,
    /// Spell Jr / Kid Mode profile (v1 I3).
    pub junior: bool,
    /// The language's release gates (`consts::is_active_lang` and not RTL-blocked).
    pub lang_open: &'a dyn Fn(&str) -> bool,
    pub entitlements: &'a EntitlementSet,
    /// The pool lookup; [`pool`] in play, a fixture in tests.
    pub pools: &'a dyn Fn(&str) -> Option<&'static Pool>,
}

/// v1 F3 + v1.1 A3 + v1.2 P2-P4 for one language.
pub fn lang_availability(lang: &str, i: &Inputs) -> Availability {
    if i.junior {
        return Availability::Locked(LockReason::ProfileBlocked);
    }
    if !i.master || !(i.lang_flag)(lang) {
        return Availability::Locked(LockReason::FlagOff);
    }
    if !(i.lang_open)(lang) {
        return Availability::Hidden;
    }
    // v1.2 P2: English is the re-tier Expert tier or nothing.
    if lang == "en" && !i.en_retier {
        return Availability::Hidden;
    }
    let Some(p) = (i.pools)(lang) else {
        return Availability::Locked(LockReason::NoExclusionGate);
    };
    if p.reason.as_deref() == Some("drawn-entry") {
        return Availability::Locked(LockReason::NoDrawnEntry);
    }
    // `exclusion_only` (F1) passes this gate; only a `missing` table with no gate fails it.
    if p.table == "missing" {
        return Availability::Locked(LockReason::NoExclusionGate);
    }
    if p.status != "pool" || p.words.len() < FLOOR {
        return Availability::Locked(LockReason::PoolBelowFloor);
    }
    if i.entitlements.lang_level(lang) != AccessLevel::Full {
        return Availability::Locked(LockReason::NeedsComplete);
    }
    Availability::Available(p.words.len())
}

/// The surfaces v1 F6 and v1.2 M6 name. Mix is Normal-only (X5).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Normal,
    ImpossibleDaily,
    BoardGame,
    Climb,
    KidMode,
    LittleSpeller,
    SpellJr,
    Randospell,
}

/// The registry's answer for Impossible Mix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixStatus {
    /// Not offered on this surface (Mix is Normal-only).
    Hidden,
    /// Visible, not selectable, with the one true reason (F3).
    Locked(LockReason),
    /// Playable. The qualifying languages, in registry order; there is no control
    /// that narrows this list (M3, M-I7).
    Available(Vec<&'static str>),
}

/// The fifteen registry languages, in registry order.
pub fn registry_langs() -> impl Iterator<Item = &'static str> {
    crate::consts::BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c)
}

/// v1.2 M1 + M3 + M7 + M8.
pub fn mix_status(surface: Surface, i: &Inputs) -> MixStatus {
    if surface != Surface::Normal {
        return MixStatus::Hidden;
    }
    if i.junior {
        return MixStatus::Locked(LockReason::ProfileBlocked);
    }
    if !i.master || !i.mix_flag {
        return MixStatus::Locked(LockReason::FlagOff);
    }
    // F3 / D-C5: Mix is a Complete feature. English is Full on the free tier, so "owns
    // Complete" means a Full language beyond English (regional grants count).
    if !registry_langs().any(|l| l != "en" && i.entitlements.lang_level(l) == AccessLevel::Full) {
        return MixStatus::Locked(LockReason::NeedsComplete);
    }
    let langs: Vec<&'static str> =
        registry_langs().filter(|l| matches!(lang_availability(l, i), Availability::Available(_))).collect();
    if langs.len() < MIX_MIN_LANGS {
        MixStatus::Locked(LockReason::NeedsTwoLanguages)
    } else {
        MixStatus::Available(langs)
    }
}

/// v1.2 M1: the Mix pool is the union of the qualifying languages' pools, each
/// word tagged with its language. Computed at run time from the artifacts; it
/// adds no rows.
pub fn mix_pool(i: &Inputs) -> Vec<(&'static str, &'static str)> {
    let MixStatus::Available(langs) = mix_status(Surface::Normal, i) else {
        return Vec::new();
    };
    langs
        .into_iter()
        .filter_map(|l| (i.pools)(l).map(|p| (l, p)))
        .flat_map(|(l, p)| p.words.iter().map(move |w| (l, w.as_str())))
        .collect()
}

/// v1.2 M3 / O1 (build default: first run and whenever the set changes): the
/// pre-run notice is due when the qualifying languages differ from the set the
/// player last acknowledged. It informs; it never lets the player choose.
pub fn notice_due(qualifying: &[&str], acknowledged: Option<&str>) -> bool {
    acknowledged != Some(qualifying.join(",").as_str())
}

/// Storage key holding the language set the player last acknowledged.
#[allow(dead_code)]
pub const NOTICE_ACK_KEY: &str = "spell_impossible_mix_ack";

/// The live inputs. The ONE impure read; everything above is pure.
#[allow(dead_code)]
pub fn with_live_inputs<R>(f: impl FnOnce(&Inputs) -> R) -> R {
    let ent = crate::play_hub::live_entitlements();
    let lang_flag = |l: &str| crate::flags::impossible_lang(l);
    let lang_open = |l: &str| crate::consts::is_active_lang(l) && !crate::consts::rtl_blocked(l);
    let pools = |l: &str| pool(l);
    let inputs = Inputs {
        master: crate::flags::impossible(),
        lang_flag: &lang_flag,
        mix_flag: crate::flags::impossible_mix(),
        en_retier: crate::flags::en_retier(),
        junior: crate::words::junior_profile(),
        lang_open: &lang_open,
        entitlements: &ent,
        pools: &pools,
    };
    f(&inputs)
}

pub mod engine;

#[cfg(test)]
mod tests;
