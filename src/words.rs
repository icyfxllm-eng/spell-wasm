//! `LANGUAGES` is used only for the "Speak in" voice picker in the Import
//! ("My Words") modal. `EN_*` is the built-in English word bank that backs
//! normal play; its audio is fetched from the backend's `/api/speak`.

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

pub const EN_EASY: &[&str] = &[
    "cat","dog","sun","run","hat","bed","cup","fish","milk","tree","book","star","frog","jump","rain","gold","hand","ship","leaf","nest",
    "bird","corn","desk","door","duck","farm","fox","gift","girl","hill","horse","kite","lamp","lion","map","moon","mouse","nose","pig","rock",
    "rose","seed","snow","sock","swim","tent","wind","wolf","worm","zoo",
];
pub const EN_MEDIUM: &[&str] = &[
    "bicycle","kitchen","monster","picture","brother","holiday","library","machine","balloon","diamond","journey","mountain","calendar","elephant","sandwich","chocolate","umbrella","treasure","hospital","festival",
    "adventure","afternoon","building","business","children","computer","dinosaur","envelope","exercise","favorite","furniture","grocery","hamburger","important","keyboard","language","mushroom","notebook","orchestra","painting",
    "pancake","pumpkin","question","rainbow","sausage","sculpture","sneakers","stadium","suitcase","telephone",
];
pub const EN_HARD: &[&str] = &[
    "rhythm","conscience","necessary","separate","definitely","embarrass","occurrence","privilege","maintenance","restaurant","lieutenant","mischievous","accommodate","millennium","questionnaire","bureaucracy","liaison","exaggerate","harass","vacuum",
    "acquaintance","acquire","apparent","arctic","awkward","believe","catastrophe","cemetery","colonel","committee","neighbor","controversy","definite","dilemma","entrepreneur","fluorescent","foreign","fulfill","gauge","guarantee",
    "hierarchy","humorous","immediately","independent","irresistible","jewelry","judgment","leisure","license","maneuver",
];
pub const EN_EXPERT: &[&str] = &[
    "onomatopoeia","sacrilegious","conscientious","idiosyncrasy","chiaroscuro","prestidigitation","surveillance","bourgeoisie","fuchsia","isthmus","mnemonic","paradigm","succinct","yacht","zephyr","archipelago","kaleidoscope","labyrinth","pneumonia","quinoa",
    "aberration","amanuensis","anachronism","apotheosis","brouhaha","cacophony","chrysanthemum","circumlocution","connoisseur","ebullient","ecclesiastical","epitome","ephemeral","esoteric","gregarious","hyperbole","inchoate","insouciant","juxtaposition","kowtow",
    "machiavellian","obfuscate","perspicacious","phenomenon","protagonist","quintessential","rendezvous","schadenfreude","silhouette","soliloquy",
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
