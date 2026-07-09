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

// French.
pub const FR_EASY: &[&str] = &[
    "eau","pain","chat","chien","roi","nuit","jour","main","pied","nez","mer","ciel","lune","feu","vent","ami","chaud","froid","noir","blanc",
    "bleu","vert","rouge","jaune","fleur","arbre","livre","table","porte","chaise","lait","pomme","poire","clé","café","thé","loup","ours","dent","robe",
];
pub const FR_MEDIUM: &[&str] = &[
    "maison","jardin","cuisine","fenêtre","école","voiture","bateau","avion","cheval","lapin","souris","tortue","oiseau","poisson","fromage","gâteau","orange","banane","fraise","carotte",
    "tomate","chapeau","chemise","manteau","guitare","musique","peinture","montagne","rivière","village","marché","couteau","famille","enfant","garçon","frère","tante","oncle","cousin","château",
];
pub const FR_HARD: &[&str] = &[
    "bibliothèque","ordinateur","téléphone","télévision","réfrigérateur","ascenseur","escalier","parapluie","aventure","dinosaure","squelette","bicyclette","calendrier","restaurant","boulangerie","pharmacie","hôpital","aéroport","université","anniversaire",
    "papillon","crocodile","éléphant","serpent","pingouin","girafe","écureuil","grenouille","hirondelle","coquillage","tournesol","champignon","framboise","pâtisserie","confiture","casserole","fourchette","couverture","ceinture","chaussette",
];
pub const FR_EXPERT: &[&str] = &[
    "accueil","cueillir","orgueil","chrysanthème","parallèle","dictionnaire","gouvernement","révolutionnaire","extraordinaire","psychologie","rythme","exhaustif","dénouement","éblouissant","hétérogène","kaléidoscope","labyrinthe","onomatopée","paradoxe","quincaillerie",
    "serrurier","vraisemblable","aquarelle","chuchotement","embarras","immédiatement","nécessaire","occurrence","quotidien","réciproque","susceptible","tranquille","vertigineux","xénophobie","zoologique","hippopotame","kilomètre","majuscule","minuscule","obstacle",
];

// German (nouns capitalized; ß handled by norm()).
pub const DE_EASY: &[&str] = &[
    "Hund","Katze","Baum","Haus","Sonne","Mond","Buch","Tisch","Stuhl","Tür","Hand","Fuß","Auge","Nase","Mund","Ohr","Milch","Brot","Käse","Ei",
    "Wasser","Feuer","Wind","Blume","Vogel","Fisch","Bär","Wolf","Maus","Ente","rot","blau","grün","gelb","schwarz","weiß","groß","klein","Tag","Nacht",
];
pub const DE_MEDIUM: &[&str] = &[
    "Fenster","Küche","Garten","Schule","Auto","Flugzeug","Pferd","Kaninchen","Schildkröte","Kartoffel","Tomate","Zwiebel","Karotte","Erdbeere","Banane","Orange","Schokolade","Kuchen","Brille","Hemd",
    "Jacke","Schuhe","Gitarre","Musik","Gemälde","Berg","Fluss","Dorf","Markt","Löffel","Gabel","Messer","Familie","Kind","Junge","Mädchen","Bruder","Schwester","Tante","Onkel",
];
pub const DE_HARD: &[&str] = &[
    "Bibliothek","Computer","Telefon","Fernseher","Kühlschrank","Aufzug","Treppe","Regenschirm","Abenteuer","Dinosaurier","Skelett","Fahrrad","Kalender","Restaurant","Bäckerei","Apotheke","Krankenhaus","Flughafen","Universität","Geburtstag",
    "Schmetterling","Krokodil","Elefant","Schlange","Pinguin","Giraffe","Eichhörnchen","Frosch","Muschel","Sonnenblume","Pilz","Himbeere","Marmelade","Serviette","Handschuh","Wörterbuch","Regierung","Fußball","Straße","Frühstück",
];
pub const DE_EXPERT: &[&str] = &[
    "Rhythmus","Gewissen","Bürgersteig","Sehenswürdigkeit","Wahrscheinlichkeit","Verantwortung","Persönlichkeit","Gemütlichkeit","Freundschaft","Wissenschaft","Gesellschaft","Entschuldigung","Überraschung","Fußgängerzone","Kaugummi","Xylophon","Höflichkeit","Jahreszeit","Löwenzahn","Möglichkeit",
    "Obstgarten","Pünktlichkeit","Qualität","Rücksicht","Tätigkeit","Verständnis","Weihnachten","Zärtlichkeit","Nußknacker","Anführungszeichen","Geschwindigkeit","Nachbarschaft","Aufmerksamkeit","Bewusstsein","Erfahrung","Genehmigung","Krankenversicherung","Selbstständigkeit","Unabhängigkeit","Zusammenarbeit",
];

// Portuguese (pt-BR).
pub const PT_EASY: &[&str] = &[
    "sol","lua","casa","gato","cão","pão","luz","mar","flor","azul","verde","mão","pé","olho","dois","três","água","mesa","livro","papel",
    "dedo","boca","céu","nuvem","chuva","vento","fogo","folha","sapo","pato","urso","peixe","ninho","leite","queijo","ovo","mel","sal","uva","rosa",
];
pub const PT_MEDIUM: &[&str] = &[
    "janela","cozinha","jardim","montanha","caminho","família","escola","trabalho","cidade","número","música","garrafa","laranja","banana","tomate","cebola","cavalo","coelho","girafa","baleia",
    "tartaruga","chapéu","sapato","camisa","vestido","guitarra","bola","brinquedo","castelo","floresta","ponte","igreja","mercado","colher","garfo","faca","maçã","morango","limão","abelha",
];
pub const PT_HARD: &[&str] = &[
    "morcego","esqueleto","bicicleta","calendário","aventura","dinossauro","hambúrguer","computador","telefone","televisão","elevador","escada","chaminé","ventilador","farmácia","mochila","caderno","tesoura","cinto","jaqueta",
    "cachecol","meia","almofada","cortina","espelho","lâmpada","tapete","armário","cenoura","brócolis","espinafre","abacate","abóbora","borboleta","crocodilo","elefante","serpente","pinguim","biblioteca","padaria",
];
pub const PT_EXPERT: &[&str] = &[
    "ambiguidade","vergonha","cegonha","bilíngue","linguística","exausto","exibição","exuberante","exagero","exceção","adjacente","justaposição","hemorragia","hipopótamo","hieróglifo","quilômetro","maiúscula","minúscula","obstáculo","ortográfico",
    "psicólogo","reivindicar","satisfação","vicissitude","xilofone","jazigo","veículo","excêntrico","sublinhar","desenvolver","idiossincrasia","transatlântico","inconstitucional","desidratação","imprescindível","extraordinário","circunferência","perpendicular","eletrodoméstico","arquitetônico",
];

// Italian.
pub const IT_EASY: &[&str] = &[
    "sole","luna","casa","gatto","cane","pane","luce","mare","fiore","blu","verde","mano","piede","occhio","due","tre","acqua","tavolo","libro","carta",
    "dito","bocca","cielo","nuvola","pioggia","vento","fuoco","foglia","rana","anatra","orso","pesce","nido","latte","uovo","miele","sale","uva","rosa","naso",
];
pub const IT_MEDIUM: &[&str] = &[
    "finestra","cucina","giardino","montagna","cammino","famiglia","scuola","lavoro","città","numero","musica","bottiglia","arancia","banana","pomodoro","cipolla","cavallo","coniglio","giraffa","balena",
    "tartaruga","cappello","scarpa","camicia","vestito","chitarra","palla","giocattolo","castello","foresta","ponte","chiesa","mercato","cucchiaio","forchetta","coltello","mela","fragola","limone","ape",
];
pub const IT_HARD: &[&str] = &[
    "pipistrello","scheletro","bicicletta","calendario","avventura","dinosauro","hamburger","computer","telefono","televisione","ascensore","scala","camino","ventilatore","ombrello","zaino","quaderno","forbici","cintura","giacca",
    "sciarpa","calzino","cuscino","tenda","specchio","lampada","tappeto","armadio","colazione","carota","broccoli","spinaci","avocado","zucca","farfalla","coccodrillo","elefante","serpente","pinguino","biblioteca",
];
pub const IT_EXPERT: &[&str] = &[
    "ambiguità","vergogna","cicogna","bilingue","linguistica","esausto","esibizione","esuberante","esagerazione","eccezione","adiacente","giustapposizione","emorragia","ippopotamo","geroglifico","chilometro","maiuscola","minuscola","ostacolo","ortografico",
    "psicologo","rivendicare","soddisfazione","vicissitudine","xilofono","giacimento","veicolo","eccentrico","sottolineare","sviluppare","idiosincrasia","transatlantico","incostituzionale","disidratazione","imprescindibile","straordinario","circonferenza","perpendicolare","elettrodomestico","architettonico",
];

fn simple_tier<'a>(easy: &'a [&'a str], medium: &'a [&'a str], hard: &'a [&'a str], expert: &'a [&'a str], tier: &str) -> &'a [&'a str] {
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
    use crate::consts::{DE, ES, FR, IT, PT};
    match lang {
        ES => es_tier(tier),
        FR => simple_tier(FR_EASY, FR_MEDIUM, FR_HARD, FR_EXPERT, tier),
        DE => simple_tier(DE_EASY, DE_MEDIUM, DE_HARD, DE_EXPERT, tier),
        PT => simple_tier(PT_EASY, PT_MEDIUM, PT_HARD, PT_EXPERT, tier),
        IT => simple_tier(IT_EASY, IT_MEDIUM, IT_HARD, IT_EXPERT, tier),
        _ => en_tier(tier),
    }
}
