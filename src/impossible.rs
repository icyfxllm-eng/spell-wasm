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

/// Why Impossible is locked for a language. Each maps to one of v1 F1's reasons
/// when the picker lands (Phase M-B); the registry only says which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockReason {
    /// Fewer than 150 eligible rows, or no verified sound-alike table yet
    /// (v1.1 A3 fails closed): "Not enough words yet".
    NotEnoughWords,
    /// zh and ja (v1 D4): "Coming with drawn entry".
    DrawnEntry,
    /// The language is not at Full access (v1 F11): "Unlock with SpellGame Complete".
    NeedsComplete,
}

/// The registry's answer for one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Nothing to show: a flag is off, the profile is Jr/Kid, or another gate
    /// closes the language (RTL, web, DEV_PREVIEW). v1 I8: Impossible never opens
    /// a language that is otherwise closed.
    Hidden,
    /// Visible, not selectable, with a reason (v1 I5).
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
    if !i.master || i.junior || !(i.lang_flag)(lang) || !(i.lang_open)(lang) {
        return Availability::Hidden;
    }
    // v1.2 P2: English is the re-tier Expert tier or nothing.
    if lang == "en" && !i.en_retier {
        return Availability::Hidden;
    }
    let Some(p) = (i.pools)(lang) else {
        return Availability::Locked(LockReason::NotEnoughWords);
    };
    match p.reason.as_deref() {
        Some("drawn-entry") => return Availability::Locked(LockReason::DrawnEntry),
        _ if p.status != "pool" || p.table == "missing" || p.words.len() < FLOOR => {
            return Availability::Locked(LockReason::NotEnoughWords)
        }
        _ => {}
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
    /// Not offered on this surface or for this profile, or its flags are off.
    Hidden,
    /// Visible, not selectable: "Needs two Impossible languages" (M7).
    NeedsTwoLanguages,
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
    if surface != Surface::Normal || !i.master || !i.mix_flag || i.junior {
        return MixStatus::Hidden;
    }
    let langs: Vec<&'static str> =
        registry_langs().filter(|l| matches!(lang_availability(l, i), Availability::Available(_))).collect();
    if langs.len() < MIX_MIN_LANGS {
        MixStatus::NeedsTwoLanguages
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

#[cfg(test)]
mod tests;
