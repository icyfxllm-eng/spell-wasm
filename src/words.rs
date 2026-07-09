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

// Spanish built-in word bank. Words are stored with correct accents/ñ (for the
// reveal + backend TTS); the A–Z keyboard + `norm()` accept them unaccented
// (ñ→n, á→a), so no special keys are needed.
pub const ES_EASY: &[&str] = &[
    "sol","luna","casa","gato","perro","pan","luz","mar","flor","azul","rojo","mano","pie","ojo","dos","tres","agua","mesa","silla","libro",
    "papel","dedo","boca","cielo","verde","negro","nube","lluvia","viento","fuego","hoja","rana","pato","oso","pez","ave","nido","reloj","leche","queso",
    "huevo","miel","sal","uva","pera","rosa","nariz","ni\u{f1}o","\u{e1}rbol","le\u{f3}n",
];
pub const ES_MEDIUM: &[&str] = &[
    "ventana","cocina","jard\u{ed}n","monta\u{f1}a","camino","familia","escuela","trabajo","ciudad","pueblo","n\u{fa}mero","m\u{fa}sica","pintura","botella","naranja","pl\u{e1}tano","tomate","cebolla","caballo","conejo",
    "rat\u{f3}n","jirafa","ballena","hormiga","tortuga","sombrero","zapato","camisa","vestido","guitarra","pelota","juguete","castillo","princesa","bosque","puente","iglesia","mercado","cuchara","tenedor",
    "cuchillo","manzana","fresa","lim\u{f3}n","sand\u{ed}a","abeja","ara\u{f1}a","paloma","espa\u{f1}ol","semana",
];
pub const ES_HARD: &[&str] = &[
    "murci\u{e9}lago","esqueleto","bicicleta","calendario","aventura","dinosaurio","hamburguesa","computadora","tel\u{e9}fono","televisi\u{f3}n","ascensor","escalera","chimenea","ventilador","paraguas","mochila","cuaderno","bol\u{ed}grafo","tijeras","pegamento",
    "cintur\u{f3}n","chaqueta","bufanda","calcet\u{ed}n","almohada","cortina","espejo","l\u{e1}mpara","alfombra","armario","desayuno","almuerzo","zanahoria","br\u{f3}coli","espinaca","aguacate","calabaza","mariposa","cocodrilo","elefante",
    "serpiente","biblioteca","farmacia","panader\u{ed}a","restaurante","aeropuerto","hospital","universidad","dormitorio","carretera",
];
pub const ES_EXPERT: &[&str] = &[
    "ambig\u{fc}edad","verg\u{fc}enza","cig\u{fc}e\u{f1}a","biling\u{fc}e","ling\u{fc}\u{ed}stica","exhausto","exhibici\u{f3}n","exuberante","exageraci\u{f3}n","excepci\u{f3}n","adyacente","yuxtaposici\u{f3}n","hierbabuena","hemorragia","hipop\u{f3}tamo","jerogl\u{ed}fico","kil\u{f3}metro","may\u{fa}scula","min\u{fa}scula","obst\u{e1}culo",
    "ortogr\u{e1}fico","psic\u{f3}logo","quir\u{f3}fano","reivindicar","satisfacci\u{f3}n","tergiversar","vicisitud","xil\u{f3}fono","yacimiento","zagu\u{e1}n","ah\u{ed}nco","ata\u{fa}d","veh\u{ed}culo","exc\u{e9}ntrico","subrayar","desarrollar","paralelep\u{ed}pedo","idiosincrasia","transatl\u{e1}ntico","inconstitucional",
    "otorrinolaring\u{f3}logo","deshidrataci\u{f3}n","imprescindible","extraordinario","circunferencia","perpendicular","electrodom\u{e9}stico","desafortunadamente","incre\u{ed}blemente","arquitect\u{f3}nico",
];

pub fn es_tier(tier: &str) -> &'static [&'static str] {
    match tier {
        "easy" => ES_EASY,
        "medium" => ES_MEDIUM,
        "hard" => ES_HARD,
        "expert" => ES_EXPERT,
        _ => ES_MEDIUM,
    }
}

/// Word bank for a built-in language + tier (English by default).
pub fn tier_for(lang: &str, tier: &str) -> &'static [&'static str] {
    match lang {
        crate::consts::ES => es_tier(tier),
        _ => en_tier(tier),
    }
}
