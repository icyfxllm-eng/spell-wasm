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

pub const LANGUAGES: [(&str, LangInfo); 20] = [
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
    // New languages with their own keyboard + backend voice (My Words matches).
    ("vi", LangInfo { name: "Ti\u{1ebf}ng Vi\u{1ec7}t", code: "vi-VN" }),
    ("ko", LangInfo { name: "\u{d55c}\u{ad6d}\u{c5b4}", code: "ko-KR" }),
    ("ja", LangInfo { name: "\u{65e5}\u{672c}\u{8a9e}", code: "ja-JP" }),
    ("th", LangInfo { name: "\u{e44}\u{e17}\u{e22}", code: "th-TH" }),
    ("fil", LangInfo { name: "Filipino", code: "fil-PH" }),
    // Mandarin: native audio for imported hanzi (the pinyin keyboard can't type
    // arbitrary hanzi, so My Words Chinese keeps the English layout).
    ("zh", LangInfo { name: "\u{4e2d}\u{6587}", code: "cmn-CN" }),
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

// Mandarin: entries are "pinyin|hanzi" — the pinyin (before '|') is the typed
// answer, the hanzi (after '|') is what TTS speaks + what's revealed. Hand-
// curated small set (2-syllable to avoid homophone ambiguity); native review
// flagged. Not run through the pipeline (its charset gate is for single-string
// words); the keyboard charset test validates the pinyin side separately.
pub const ZH_EASY: &[&str] = &[
    "yi1qi3|一起",
    "bu4dui4|不对",
    "ni3hao3|你好",
    "hen3hao3|很好",
    "xie4xie5|谢谢",
    "ju2zi5|橘子",
    "yi1fu2|衣服",
    "yi3zi5|椅子",
    "fei1ji1|飞机",
    "ji1dan4|鸡蛋",
    "lou2ti1|楼梯",
    "mao4zi5|帽子",
    "mi3fan4|米饭",
    "suo3yi3|所以",
    "xie2zi5|鞋子",
];
pub const ZH_MEDIUM: &[&str] = &[
    "you2ju2|邮局",
    "dian4ti1|电梯",
    "niu2nai3|牛奶",
    "dian4hua4|电话",
    "dian4nao3|电脑",
    "mian4bao1|面包",
    "xue2xiao4|学校",
    "xi3huan1|喜欢",
    "yi1yuan4|医院",
    "gong1si1|公司",
    "can1ting1|餐厅",
    "peng2you3|朋友",
    "ping2guo3|苹果",
    "yan3jing4|眼镜",
    "yang2tai2|阳台",
];
pub const ZH_HARD: &[&str] = &[
    "yin2hang2|银行",
    "zou3lang2|走廊",
    "qi4che1|汽车",
    "huo3che1|火车",
    "lao3shi1|老师",
    "shou3ji1|手机",
    "zhuo1zi5|桌子",
    "dian4shi4|电视",
    "jiao4shi4|教室",
    "shui3guo3|水果",
    "shou3biao3|手表",
    "gong1yuan2|公园",
    "xiang1jiao1|香蕉",
    "gong1gong4|公共",
    "chu2fang2|厨房",
];
pub const ZH_EXPERT: &[&str] = &[
    "ji1chang3|机场",
    "cao1chang3|操场",
    "xue2sheng1|学生",
    "shang1dian4|商店",
    "dong4wu4yuan2|动物园",
    "chao1shi4|超市",
    "che1zhan4|车站",
    "tu2shu1guan3|图书馆",
    "bing1xiang1|冰箱",
    "yang2guang1|阳光",
    "zi4xing2che1|自行车",
    "lü4se4|绿色",
    "nü3er2|女儿",
    "kao3lü4|考虑",
    "lü3you2|旅游",
];

pub fn zh_tier(tier: &str) -> &'static [&'static str] {
    match tier {
        "easy" => ZH_EASY,
        "medium" => ZH_MEDIUM,
        "hard" => ZH_HARD,
        "expert" => ZH_EXPERT,
        _ => ZH_MEDIUM,
    }
}

/// Word bank for a built-in language + tier (English by default).
pub fn tier_for(lang: &str, tier: &str) -> &'static [&'static str] {
    use crate::consts::{DE, ES, FIL, FR, IT, JA, KO, NB, NL, PL, PT, SV, TH, TR, VI, ZH};
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
        ZH => zh_tier(tier),
        TH => simple_tier(TH_EASY, TH_MEDIUM, TH_HARD, TH_EXPERT, tier),
        _ => en_tier(tier),
    }
}
