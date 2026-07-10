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

// Dutch.
pub const NL_EASY: &[&str] = &["zon","maan","huis","kat","hond","boom","brood","kaas","melk","water","vuur","wind","bloem","vogel","vis","beer","wolf","muis","eend","rood","blauw","groen","geel","zwart","wit","hand","voet","oog","neus","mond","oor","dag","nacht","boek","tafel","stoel","deur","ei","appel","peer"];
pub const NL_MEDIUM: &[&str] = &["raam","keuken","tuin","school","auto","vliegtuig","paard","konijn","schildpad","aardappel","tomaat","wortel","aardbei","banaan","sinaasappel","chocolade","koek","bril","hemd","jas","schoen","gitaar","muziek","berg","rivier","dorp","markt","lepel","vork","mes","familie","kind","jongen","meisje","broer","tante","oom","kasteel","brood","vogel"];
pub const NL_HARD: &[&str] = &["bibliotheek","computer","telefoon","televisie","koelkast","lift","trap","paraplu","avontuur","dinosaurus","skelet","fiets","kalender","restaurant","bakkerij","apotheek","ziekenhuis","vliegveld","universiteit","verjaardag","vlinder","krokodil","olifant","slang","pingu\u{ef}n","giraffe","eekhoorn","kikker","schelp","zonnebloem","paddenstoel","framboos","servet","handschoen","woordenboek","regering","snelheid","gordijn","spiegel","tapijt"];
pub const NL_EXPERT: &[&str] = &["ritme","geweten","waarschijnlijk","verantwoordelijkheid","persoonlijkheid","vriendschap","wetenschap","maatschappij","verontschuldiging","verrassing","gezelligheid","ingewikkeld","nieuwsgierig","onmiddellijk","noodzakelijk","buitengewoon","psychologie","nijlpaard","kilometer","hoofdletter","obstakel","tegelijkertijd","ongelooflijk","tegenwoordig","oorspronkelijk","gemeenschap","aanwezigheid","gebeurtenis","herinnering","ademhaling","evenwicht","ervaring","gebruikelijk","karakteristiek","mogelijkheid","natuurlijk","overeenkomst","verantwoordelijk","verschillende","onafhankelijk"];

// Polish (\u{142}->l, and \u{f3}/\u{105}/\u{119}/\u{107}/\u{144}/\u{15b}/\u{17a}/\u{17c} fold via norm).
pub const PL_EASY: &[&str] = &["dom","kot","pies","oko","nos","ucho","r\u{119}ka","noga","woda","ogie\u{144}","wiatr","ptak","ryba","wilk","mysz","dzie\u{144}","noc","st\u{f3}\u{142}","drzwi","jajko","chleb","ser","mleko","s\u{f3}l","kwiat","drzewo","las","sok","ko\u{144}","krowa","kaczka","czerwony","zielony","\u{17c}\u{f3}\u{142}ty","bia\u{142}y"];
pub const PL_MEDIUM: &[&str] = &["okno","kuchnia","ogr\u{f3}d","szko\u{142}a","samoch\u{f3}d","samolot","kr\u{f3}lik","\u{17c}\u{f3}\u{142}w","ziemniak","pomidor","cebula","marchewka","truskawka","banan","pomara\u{144}cza","czekolada","ciasto","okulary","koszula","kurtka","gitara","muzyka","g\u{f3}ra","rzeka","wioska","rynek","\u{142}y\u{17c}ka","widelec","n\u{f3}\u{17c}","rodzina","dziecko","ch\u{142}opiec","dziewczyna","brat","siostra","ciotka","wujek","zamek","most","kot"];
pub const PL_HARD: &[&str] = &["biblioteka","komputer","telefon","telewizja","lod\u{f3}wka","winda","schody","parasol","przygoda","dinozaur","szkielet","rower","kalendarz","restauracja","piekarnia","apteka","szpital","lotnisko","uniwersytet","urodziny","motyl","krokodyl","s\u{142}o\u{144}","w\u{105}\u{17c}","pingwin","\u{17c}yrafa","wiewi\u{f3}rka","\u{17c}aba","muszla","s\u{142}onecznik","grzyb","malina","d\u{17c}em","serwetka","r\u{119}kawica","s\u{142}ownik","rz\u{105}d","pr\u{119}dko\u{15b}\u{107}","zas\u{142}ona","lustro"];
pub const PL_EXPERT: &[&str] = &["rytm","sumienie","odpowiedzialno\u{15b}\u{107}","osobowo\u{15b}\u{107}","przyja\u{17a}\u{144}","nauka","spo\u{142}ecze\u{144}stwo","przeprosiny","niespodzianka","skomplikowany","ciekawy","natychmiast","konieczny","nadzwyczajny","psychologia","hipopotam","kilometr","przeszkoda","jednocze\u{15b}nie","niewiarygodny","obecnie","pierwotny","r\u{f3}\u{17c}norodny","wsp\u{f3}lnota","obecno\u{15b}\u{107}","wydarzenie","wspomnienie","oddychanie","r\u{f3}wnowaga","do\u{15b}wiadczenie","zazwyczaj","sk\u{142}adnik","charakterystyczny","mo\u{17c}liwo\u{15b}\u{107}","naturalny","porozumienie","odpowiedzialny","oczekiwanie","uwaga","wystarczaj\u{105}cy"];

// Swedish (\u{e5}/\u{e4}/\u{f6} fold via norm).
pub const SV_EASY: &[&str] = &["sol","m\u{e5}ne","hus","katt","hund","tr\u{e4}d","br\u{f6}d","ost","mj\u{f6}lk","vatten","eld","vind","blomma","f\u{e5}gel","fisk","bj\u{f6}rn","varg","mus","and","r\u{f6}d","bl\u{e5}","gr\u{f6}n","gul","svart","vit","hand","fot","\u{f6}ga","n\u{e4}sa","mun","\u{f6}ra","dag","natt","bok","bord","stol","d\u{f6}rr","\u{e4}gg","\u{e4}pple","p\u{e4}ron"];
pub const SV_MEDIUM: &[&str] = &["f\u{f6}nster","k\u{f6}k","tr\u{e4}dg\u{e5}rd","skola","bil","flygplan","h\u{e4}st","kanin","sk\u{f6}ldpadda","potatis","tomat","l\u{f6}k","morot","jordgubbe","banan","apelsin","choklad","kaka","glas\u{f6}gon","skjorta","jacka","sko","gitarr","musik","berg","flod","by","marknad","sked","gaffel","kniv","familj","barn","pojke","flicka","bror","syster","faster","farbror","slott"];
pub const SV_HARD: &[&str] = &["bibliotek","dator","telefon","television","kylsk\u{e5}p","hiss","trappa","paraply","\u{e4}ventyr","dinosaurie","skelett","cykel","kalender","restaurang","bageri","apotek","sjukhus","flygplats","universitet","f\u{f6}delsedag","fj\u{e4}ril","krokodil","elefant","orm","pingvin","giraff","ekorre","groda","sn\u{e4}cka","solros","svamp","hallon","sylt","servett","handske","ordbok","regering","hastighet","gardin","spegel"];
pub const SV_EXPERT: &[&str] = &["rytm","samvete","ansvar","personlighet","v\u{e4}nskap","vetenskap","samh\u{e4}lle","urs\u{e4}kt","\u{f6}verraskning","gemytlig","komplicerad","nyfiken","omedelbart","n\u{f6}dv\u{e4}ndig","utomordentlig","psykologi","flodh\u{e4}st","kilometer","versal","hinder","samtidigt","otrolig","nuf\u{f6}rtiden","ursprunglig","olika","gemenskap","n\u{e4}rvaro","h\u{e4}ndelse","minne","andning","j\u{e4}mvikt","erfarenhet","vanligtvis","ingrediens","karakteristisk","m\u{f6}jlighet","naturlig","\u{f6}verenskommelse","ansvarig","f\u{f6}rv\u{e4}ntan"];

// Norwegian Bokm\u{e5}l (\u{f8}/\u{e6} fold via norm; \u{e5} via NFD).
pub const NB_EASY: &[&str] = &["sol","m\u{e5}ne","hus","katt","hund","tre","br\u{f8}d","ost","melk","vann","ild","vind","blomst","fugl","fisk","bj\u{f8}rn","ulv","mus","and","r\u{f8}d","bl\u{e5}","gr\u{f8}nn","gul","svart","hvit","h\u{e5}nd","fot","\u{f8}ye","nese","munn","\u{f8}re","dag","natt","bok","bord","stol","d\u{f8}r","egg","eple","p\u{e6}re"];
pub const NB_MEDIUM: &[&str] = &["vindu","kj\u{f8}kken","hage","skole","bil","fly","hest","kanin","skilpadde","potet","tomat","l\u{f8}k","gulrot","jordb\u{e6}r","banan","appelsin","sjokolade","kake","briller","skjorte","jakke","sko","gitar","musikk","fjell","elv","landsby","marked","skje","gaffel","kniv","familie","barn","gutt","jente","bror","s\u{f8}ster","tante","onkel","slott"];
pub const NB_HARD: &[&str] = &["bibliotek","datamaskin","telefon","fjernsyn","kj\u{f8}leskap","heis","trapp","paraply","eventyr","dinosaur","skjelett","sykkel","kalender","restaurant","bakeri","apotek","sykehus","flyplass","universitet","bursdag","sommerfugl","krokodille","elefant","slange","pingvin","sjiraff","ekorn","frosk","skjell","solsikke","sopp","bringeb\u{e6}r","syltet\u{f8}y","serviett","hanske","ordbok","regjering","hastighet","gardin","speil"];
pub const NB_EXPERT: &[&str] = &["rytme","samvittighet","ansvar","personlighet","vennskap","vitenskap","samfunn","unnskyldning","overraskelse","koselig","komplisert","nysgjerrig","umiddelbart","n\u{f8}dvendig","enest\u{e5}ende","psykologi","flodhest","kilometer","hinder","samtidig","utrolig","n\u{e5}tiden","opprinnelig","forskjellige","fellesskap","tilstedev\u{e6}relse","hendelse","minne","pust","likevekt","erfaring","vanligvis","ingrediens","karakteristisk","mulighet","naturlig","avtale","ansvarlig","forventning","oppmerksomhet"];

// Turkish (\u{131}->i via norm; \u{e7}/\u{11f}/\u{f6}/\u{15f}/\u{fc} fold via norm).
pub const TR_EASY: &[&str] = &["g\u{fc}ne\u{15f}","ay","ev","kedi","k\u{f6}pek","a\u{11f}a\u{e7}","ekmek","peynir","s\u{fc}t","su","ate\u{15f}","r\u{fc}zgar","\u{e7}i\u{e7}ek","ku\u{15f}","bal\u{131}k","ay\u{131}","kurt","fare","\u{f6}rdek","k\u{131}rm\u{131}z\u{131}","mavi","ye\u{15f}il","sar\u{131}","siyah","beyaz","el","ayak","g\u{f6}z","burun","a\u{11f}\u{131}z","kulak","g\u{fc}n","gece","kitap","masa","sandalye","kap\u{131}","yumurta","elma","armut"];
pub const TR_MEDIUM: &[&str] = &["pencere","mutfak","bah\u{e7}e","okul","araba","u\u{e7}ak","at","tav\u{15f}an","kaplumba\u{11f}a","patates","domates","so\u{11f}an","havu\u{e7}","\u{e7}ilek","muz","portakal","\u{e7}ikolata","kek","g\u{f6}zl\u{fc}k","g\u{f6}mlek","ceket","ayakkab\u{131}","gitar","m\u{fc}zik","da\u{11f}","nehir","k\u{f6}y","market","ka\u{15f}\u{131}k","\u{e7}atal","b\u{131}\u{e7}ak","aile","\u{e7}ocuk","o\u{11f}lan","k\u{131}z","karde\u{15f}","teyze","amca","kale","orman"];
pub const TR_HARD: &[&str] = &["k\u{fc}t\u{fc}phane","bilgisayar","telefon","televizyon","buzdolab\u{131}","asans\u{f6}r","merdiven","\u{15f}emsiye","macera","dinozor","iskelet","bisiklet","takvim","restoran","f\u{131}r\u{131}n","eczane","hastane","havaalan\u{131}","\u{fc}niversite","do\u{11f}umg\u{fc}n\u{fc}","kelebek","timsah","fil","y\u{131}lan","penguen","z\u{fc}rafa","sincap","kurba\u{11f}a","kabuk","ay\u{e7}i\u{e7}e\u{11f}i","mantar","ahududu","re\u{e7}el","pe\u{e7}ete","eldiven","s\u{f6}zl\u{fc}k","h\u{fc}k\u{fc}met","h\u{131}z","perde","ayna"];
pub const TR_EXPERT: &[&str] = &["ritim","vicdan","sorumluluk","ki\u{15f}ilik","arkada\u{15f}l\u{131}k","bilim","toplum","\u{f6}z\u{fc}r","s\u{fc}rpriz","karma\u{15f}\u{131}k","merakl\u{131}","hemen","gerekli","ola\u{11f}an\u{fc}st\u{fc}","psikoloji","suayg\u{131}r\u{131}","kilometre","engel","inan\u{131}lmaz","g\u{fc}n\u{fc}m\u{fc}zde","\u{f6}zg\u{fc}n","\u{e7}e\u{15f}itli","topluluk","varl\u{131}k","olay","hat\u{131}ra","nefes","denge","deneyim","genellikle","malzeme","karakteristik","olas\u{131}l\u{131}k","do\u{11f}al","anla\u{15f}ma","sorumlu","beklenti","dikkat","yeterli","ba\u{11f}\u{131}ms\u{131}zl\u{131}k"];

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
    use crate::consts::{DE, ES, FR, IT, NB, NL, PL, PT, SV, TR};
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
        _ => en_tier(tier),
    }
}
