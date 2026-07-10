//! `LANGUAGES` is used only for the "Speak in" voice picker in the Import
//! ("My Words") modal. The built-in word banks that back normal play live in
//! `word_data.rs` — a @generated file produced by `scripts/build-wordlists.py`
//! from the curated sources in `assets/words/{code}/{tier}.txt` (edit those and
//! re-run the pipeline; never edit `word_data.rs` by hand). Their audio is
//! fetched from the backend's `/api/speak`.

use crate::word_data::*;

pub struct LangInfo {
    pub name: &'static str,
    pub code: &'static str,
}

pub const LANGUAGES: [(&str, LangInfo); 14] = [
    ("en", LangInfo { name: "English", code: "en-US" }),
    ("es", LangInfo { name: "Espa\u{f1}ol", code: "es-ES" }),
    ("fr", LangInfo { name: "Fran\u{e7}ais", code: "fr-FR" }),
    ("de", LangInfo { name: "Deutsch", code: "de-DE" }),
    ("it", LangInfo { name: "Italiano", code: "it-IT" }),
    ("pt", LangInfo { name: "Portugu\u{ea}s", code: "pt-BR" }),
    ("nl", LangInfo { name: "Nederlands", code: "nl-NL" }),
    ("sv", LangInfo { name: "Svenska", code: "sv-SE" }),
    ("pl", LangInfo { name: "Polski", code: "pl-PL" }),
    ("tr", LangInfo { name: "T\u{fc}rk\u{e7}e", code: "tr-TR" }),
    ("ro", LangInfo { name: "Rom\u{e2}n\u{103}", code: "ro-RO" }),
    ("id", LangInfo { name: "Indonesia", code: "id-ID" }),
    ("nb", LangInfo { name: "Norsk", code: "nb-NO" }),
    ("ca", LangInfo { name: "Catal\u{e0}", code: "ca-ES" }),
];

pub fn en_tier(tier: &str) -> &'static [&'static str] {
    match tier {
        "easy" => EN_EASY,
        "medium" => EN_MEDIUM,
        "hard" => EN_HARD,
        "expert" => EN_EXPERT,
        _ => EN_MEDIUM,
    }
}

pub fn es_tier(tier: &str) -> &'static [&'static str] {
    match tier {
        "easy" => ES_EASY,
        "medium" => ES_MEDIUM,
        "hard" => ES_HARD,
        "expert" => ES_EXPERT,
        _ => ES_MEDIUM,
    }
}

fn simple_tier(easy: &'static [&'static str], medium: &'static [&'static str], hard: &'static [&'static str], expert: &'static [&'static str], tier: &str) -> &'static [&'static str] {
    match tier {
        "easy" => easy,
        "medium" => medium,
        "hard" => hard,
        "expert" => expert,
        _ => medium,
    }
}

/// Word bank for a built-in language + tier (English by default).
pub fn tier_for(lang: &str, tier: &str) -> &'static [&'static str] {
    use crate::consts::{DE, ES, FIL, FR, IT, JA, KO, NB, NL, PL, PT, SV, TR, VI};
    match lang {
        ES => es_tier(tier),
        FR => simple_tier(FR_EASY, FR_MEDIUM, FR_HARD, FR_EXPERT, tier),
        DE => simple_tier(DE_EASY, DE_MEDIUM, DE_HARD, DE_EXPERT, tier),
        PT => simple_tier(PT_EASY, PT_MEDIUM, PT_HARD, PT_EXPERT, tier),
        IT => simple_tier(IT_EASY, IT_MEDIUM, IT_HARD, IT_EXPERT, tier),
        NL => simple_tier(NL_EASY, NL_MEDIUM, NL_HARD, NL_EXPERT, tier),
        PL => simple_tier(PL_EASY, PL_MEDIUM, PL_HARD, PL_EXPERT, tier),
        SV => simple_tier(SV_EASY, SV_MEDIUM, SV_HARD, SV_EXPERT, tier),
        NB => simple_tier(NB_EASY, NB_MEDIUM, NB_HARD, NB_EXPERT, tier),
        TR => simple_tier(TR_EASY, TR_MEDIUM, TR_HARD, TR_EXPERT, tier),
        VI => simple_tier(VI_EASY, VI_MEDIUM, VI_HARD, VI_EXPERT, tier),
        KO => simple_tier(KO_EASY, KO_MEDIUM, KO_HARD, KO_EXPERT, tier),
        JA => simple_tier(JA_EASY, JA_MEDIUM, JA_HARD, JA_EXPERT, tier),
        FIL => simple_tier(FIL_EASY, FIL_MEDIUM, FIL_HARD, FIL_EXPERT, tier),
        _ => en_tier(tier),
    }
}
