#!/usr/bin/env python3
"""Letter-name lexicons for voice spelling in EVERY language (Eric's directive
2026-07-27: "apply the Spell It microphone button on every language").

Each lexicons/letters/<lang>.json maps SPOKEN letter names (what the on-device
recognizer transcribes) to the graphemes the keyboard types, plus localized
edit commands. Machine-drafted from standard alphabet names, pending native
review (same posture as the banks/UI locales). en/es (hand-tuned) are NOT
regenerated here. The mic still only appears where iOS has on-device speech
for the language (the capabilities gate is unchanged) — these files make the
parser understand the letters wherever the mic CAN appear.
"""
import json, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "lexicons", "letters")

def latin(names, extra=None, multigraph=None, diacritics=None, commands=None, homophones=None):
    letters = {}
    for name, g in names:
        letters[name] = g
    if extra:
        letters.update(extra)
    return {
        "letterNames": letters,
        "homophones": homophones or {},
        "multigraph": multigraph or {},
        "diacritics": diacritics or {},
        "commands": commands or {},
    }

L = {}

L["fr"] = latin(
    [("a","a"),("bé","b"),("cé","c"),("dé","d"),("e","e"),("effe","f"),("gé","g"),
     ("ache","h"),("i","i"),("ji","j"),("ka","k"),("elle","l"),("emme","m"),
     ("enne","n"),("o","o"),("pé","p"),("ku","q"),("erre","r"),("esse","s"),
     ("té","t"),("u","u"),("vé","v"),("double vé","w"),("ixe","x"),("i grec","y"),("zède","z")],
    homophones={"bée":"b","say":"c","day":"d","œuf":"f","gué":"g","hache":"h","gi":"j","aile":"l","aime":"m","haine":"n","eau":"o","paie":"p","cul":"q","air":"r","est-ce":"s","thé":"t","eu":"u","vais":"v","iks":"x","zed":"z"},
    multigraph={"double l":"ll","deux l":"ll","double s":"ss","deux s":"ss","double m":"mm","double n":"nn","double t":"tt","double p":"pp","double r":"rr"},
    diacritics={"e accent aigu":"é","é":"é","e accent grave":"è","a accent grave":"à","u accent grave":"ù","e accent circonflexe":"ê","a accent circonflexe":"â","i accent circonflexe":"î","o accent circonflexe":"ô","u accent circonflexe":"û","c cédille":"ç","cé cédille":"ç","e tréma":"ë","i tréma":"ï","u tréma":"ü"},
    commands={"effacer":"delete","efface":"delete","retour":"delete","supprimer":"delete","tout effacer":"clear","recommencer":"clear","fini":"done","terminé":"done","c'est bon":"done"},
)

L["de"] = latin(
    [("a","a"),("be","b"),("ce","c"),("de","d"),("e","e"),("ef","f"),("ge","g"),
     ("ha","h"),("i","i"),("jot","j"),("ka","k"),("el","l"),("em","m"),("en","n"),
     ("o","o"),("pe","p"),("ku","q"),("er","r"),("es","s"),("te","t"),("u","u"),
     ("vau","v"),("we","w"),("iks","x"),("ypsilon","y"),("zett","z")],
    extra={"ä":"ä","a umlaut":"ä","ö":"ö","o umlaut":"ö","ü":"ü","u umlaut":"ü","eszett":"ß","scharfes s":"ß","scharfes es":"ß"},
    homophones={"beh":"b","zeh":"c","weh":"w","zet":"z","üpsilon":"y"},
    multigraph={"doppel s":"ss","doppel l":"ll","doppel m":"mm","doppel n":"nn","doppel t":"tt","doppel p":"pp","sch":"sch"},
    commands={"löschen":"delete","lösche":"delete","zurück":"delete","weg":"delete","alles löschen":"clear","neu anfangen":"clear","von vorne":"clear","fertig":"done","erledigt":"done"},
)

L["pt"] = latin(
    [("á","a"),("bê","b"),("cê","c"),("dê","d"),("é","e"),("efe","f"),("gê","g"),
     ("agá","h"),("i","i"),("jota","j"),("cá","k"),("ele","l"),("eme","m"),
     ("ene","n"),("ó","o"),("pê","p"),("quê","q"),("erre","r"),("esse","s"),
     ("tê","t"),("u","u"),("vê","v"),("dáblio","w"),("xis","x"),("ípsilon","y"),("zê","z")],
    extra={"a":"a","e":"e","o":"o","ka":"k","dabliu":"w","ipsilon":"y"},
    homophones={"be":"b","ce":"c","de":"d","ge":"g","aga":"h","pe":"p","que":"q","te":"t","ve":"v","ze":"z"},
    multigraph={"erre duplo":"rr","dois erres":"rr","esse duplo":"ss","dois esses":"ss"},
    diacritics={"a com acento":"á","e com acento":"é","i com acento":"í","o com acento":"ó","u com acento":"ú","a com til":"ã","o com til":"õ","cê cedilha":"ç","ce cedilha":"ç","a com circunflexo":"â","e com circunflexo":"ê","o com circunflexo":"ô","a com crase":"à"},
    commands={"apagar":"delete","apaga":"delete","voltar":"delete","apagar tudo":"clear","limpar":"clear","recomeçar":"clear","pronto":"done","terminei":"done","feito":"done"},
)

L["pl"] = latin(
    [("a","a"),("be","b"),("ce","c"),("de","d"),("e","e"),("ef","f"),("gie","g"),
     ("ha","h"),("i","i"),("jot","j"),("ka","k"),("el","l"),("em","m"),("en","n"),
     ("o","o"),("pe","p"),("ku","q"),("er","r"),("es","s"),("te","t"),("u","u"),
     ("fał","v"),("wu","w"),("iks","x"),("igrek","y"),("zet","z")],
    extra={"ą":"ą","a z ogonkiem":"ą","ę":"ę","e z ogonkiem":"ę","ć":"ć","ce z kreską":"ć","ł":"ł","el z kreską":"ł","ń":"ń","en z kreską":"ń","ó":"ó","o z kreską":"ó","o kreskowane":"ó","ś":"ś","es z kreską":"ś","ź":"ź","zet z kreską":"ź","ż":"ż","zet z kropką":"ż"},
    multigraph={"podwójne l":"ll","podwójne n":"nn","podwójne s":"ss","podwójne t":"tt"},
    commands={"usuń":"delete","skasuj":"delete","cofnij":"delete","wyczyść":"clear","usuń wszystko":"clear","od nowa":"clear","gotowe":"done","koniec":"done","zrobione":"done"},
)

L["sw"] = latin(
    [("a","a"),("be","b"),("che","c"),("de","d"),("e","e"),("efu","f"),("ge","g"),
     ("he","h"),("i","i"),("je","j"),("ke","k"),("eli","l"),("emu","m"),("enu","n"),
     ("o","o"),("pe","p"),("kyu","q"),("re","r"),("esi","s"),("te","t"),("u","u"),
     ("ve","v"),("we","w"),("eksi","x"),("ye","y"),("zedi","z")],
    homophones={"bee":"b","chee":"c","dee":"d","gee":"g","echi":"h","jee":"j","kee":"k","pee":"p","ree":"r","tee":"t","vee":"v","wee":"w","zee":"z"},
    multigraph={},
    commands={"futa":"delete","ondoa":"delete","rudi nyuma":"delete","futa yote":"clear","anza upya":"clear","nimemaliza":"done","tayari":"done","imekamilika":"done"},
)

L["fil"] = latin(
    [("ey","a"),("bi","b"),("si","c"),("di","d"),("i","e"),("ef","f"),("dyi","g"),
     ("eyts","h"),("ay","i"),("dyey","j"),("key","k"),("el","l"),("em","m"),("en","n"),
     ("o","o"),("pi","p"),("kyu","q"),("ar","r"),("es","s"),("ti","t"),("yu","u"),
     ("vi","v"),("dobol yu","w"),("eks","x"),("way","y"),("zi","z")],
    extra={"enye":"ñ","a":"a","ba":"b","ka":"k","da":"d","ga":"g","ha":"h","la":"l","ma":"m","na":"n","pa":"p","ra":"r","sa":"s","ta":"t","wa":"w","ya":"y"},
    multigraph={"en dyi":"ng","nga":"ng"},
    commands={"burahin":"delete","bura":"delete","balik":"delete","burahin lahat":"clear","ulitin":"clear","tapos na":"done","yari na":"done"},
)

L["vi"] = latin(
    [("a","a"),("bê","b"),("xê","c"),("dê","d"),("đê","đ"),("e","e"),("giê","g"),
     ("hát","h"),("i ngắn","i"),("ca","k"),("e lờ","l"),("em mờ","m"),("en nờ","n"),
     ("o","o"),("pê","p"),("quy","q"),("e rờ","r"),("ét xì","s"),("tê","t"),
     ("u","u"),("vê","v"),("ích xì","x"),("i dài","y")],
    extra={"ă":"ă","á":"ă","â":"â","ớ":"â","ê":"ê","ô":"ô","ơ":"ơ","ư":"ư","i":"i","y":"y","bờ":"b","cờ":"c","dờ":"d","đờ":"đ","gờ":"g","hờ":"h","lờ":"l","mờ":"m","nờ":"n","pờ":"p","rờ":"r","sờ":"s","tờ":"t","vờ":"v","xờ":"x"},
    multigraph={"en giê":"ng","giê hát":"gh","en giê hát":"ngh","ca hát":"kh","tê hát":"th","xê hát":"ch","phờ":"ph","tê rờ":"tr","en hát":"nh","cu u":"qu"},
    commands={"xóa":"delete","xoá":"delete","lùi lại":"delete","xóa hết":"clear","làm lại":"clear","xong":"done","xong rồi":"done","hoàn thành":"done"},
)

L["ru"] = latin(
    [("а","а"),("бэ","б"),("вэ","в"),("гэ","г"),("дэ","д"),("е","е"),("ё","ё"),
     ("жэ","ж"),("зэ","з"),("и","и"),("и краткое","й"),("ка","к"),("эль","л"),
     ("эм","м"),("эн","н"),("о","о"),("пэ","п"),("эр","р"),("эс","с"),("тэ","т"),
     ("у","у"),("эф","ф"),("ха","х"),("цэ","ц"),("че","ч"),("ша","ш"),("ща","щ"),
     ("твёрдый знак","ъ"),("ы","ы"),("мягкий знак","ь"),("э","э"),("ю","ю"),("я","я")],
    extra={"бе":"б","ве":"в","ге":"г","де":"д","же":"ж","зе":"з"," й":"й","эл":"л","пе":"п","ер":"р","ес":"с","те":"т","еф":"ф","це":"ц","твердый знак":"ъ","э оборотное":"э"},
    commands={"удалить":"delete","удали":"delete","назад":"delete","стереть":"delete","очистить":"clear","удалить всё":"clear","заново":"clear","готово":"done","всё":"done","закончил":"done"},
)

L["ar"] = latin(
    [("ألف","ا"),("باء","ب"),("تاء","ت"),("ثاء","ث"),("جيم","ج"),("حاء","ح"),
     ("خاء","خ"),("دال","د"),("ذال","ذ"),("راء","ر"),("زاي","ز"),("سين","س"),
     ("شين","ش"),("صاد","ص"),("ضاد","ض"),("طاء","ط"),("ظاء","ظ"),("عين","ع"),
     ("غين","غ"),("فاء","ف"),("قاف","ق"),("كاف","ك"),("لام","ل"),("ميم","م"),
     ("نون","ن"),("هاء","ه"),("واو","و"),("ياء","ي")],
    extra={"الف":"ا","با":"ب","تا":"ت","ثا":"ث","حا":"ح","خا":"خ","را":"ر","طا":"ط","ظا":"ظ","فا":"ف","ها":"ه","يا":"ي","همزة":"ء","تاء مربوطة":"ة","ألف مقصورة":"ى","الف مقصورة":"ى","ألف مد":"آ","لام ألف":"لا"},
    commands={"احذف":"delete","امسح":"delete","تراجع":"delete","امسح الكل":"clear","احذف الكل":"clear","من جديد":"clear","تم":"done","انتهيت":"done","خلاص":"done"},
)

L["hi"] = latin(
    [("क","क"),("ख","ख"),("ग","ग"),("घ","घ"),("च","च"),("छ","छ"),("ज","ज"),
     ("झ","झ"),("ट","ट"),("ठ","ठ"),("ड","ड"),("ढ","ढ"),("ण","ण"),("त","त"),
     ("थ","थ"),("द","द"),("ध","ध"),("न","न"),("प","प"),("फ","फ"),("ब","ब"),
     ("भ","भ"),("म","म"),("य","य"),("र","र"),("ल","ल"),("व","व"),("श","श"),
     ("ष","ष"),("स","स"),("ह","ह"),("अ","अ"),("आ","आ"),("इ","इ"),("ई","ई"),
     ("उ","उ"),("ऊ","ऊ"),("ए","ए"),("ऐ","ऐ"),("ओ","ओ"),("औ","औ")],
    extra={"का":"क","खा":"ख","गा":"ग","घा":"घ","चा":"च","छा":"छ","जा":"ज","झा":"झ","टा":"ट","ठा":"ठ","डा":"ड","ढा":"ढ","ता":"त","था":"थ","दा":"द","धा":"ध","ना":"न","पा":"प","फा":"फ","बा":"ब","भा":"भ","मा":"म","या":"य","रा":"र","ला":"ल","वा":"व","शा":"श","षा":"ष","सा":"स","हा":"ह"},
    commands={"हटाओ":"delete","मिटाओ":"delete","पीछे":"delete","सब हटाओ":"clear","सब मिटाओ":"clear","फिर से":"clear","हो गया":"done","पूरा":"done","बस":"done"},
)

L["ko"] = latin(
    [("기역","ㄱ"),("니은","ㄴ"),("디귿","ㄷ"),("리을","ㄹ"),("미음","ㅁ"),
     ("비읍","ㅂ"),("시옷","ㅅ"),("이응","ㅇ"),("지읒","ㅈ"),("치읓","ㅊ"),
     ("키읔","ㅋ"),("티읕","ㅌ"),("피읖","ㅍ"),("히읗","ㅎ"),
     ("쌍기역","ㄲ"),("쌍디귿","ㄸ"),("쌍비읍","ㅃ"),("쌍시옷","ㅆ"),("쌍지읒","ㅉ"),
     ("아","ㅏ"),("야","ㅑ"),("어","ㅓ"),("여","ㅕ"),("오","ㅗ"),("요","ㅛ"),
     ("우","ㅜ"),("유","ㅠ"),("으","ㅡ"),("이","ㅣ"),("애","ㅐ"),("에","ㅔ"),
     ("얘","ㅒ"),("예","ㅖ"),("와","ㅘ"),("워","ㅝ"),("위","ㅟ"),("의","ㅢ"),("외","ㅚ"),("웨","ㅞ")],
    extra={"기억":"ㄱ","시읏":"ㅅ"},
    commands={"지워":"delete","지우기":"delete","뒤로":"delete","삭제":"delete","다 지워":"clear","전부 지워":"clear","처음부터":"clear","다 했어":"done","끝":"done","완료":"done"},
)

_kana = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん"
_kana_extra = {"が":"が","ぎ":"ぎ","ぐ":"ぐ","げ":"げ","ご":"ご","ざ":"ざ","じ":"じ","ず":"ず","ぜ":"ぜ","ぞ":"ぞ",
               "だ":"だ","ぢ":"ぢ","づ":"づ","で":"で","ど":"ど","ば":"ば","び":"び","ぶ":"ぶ","べ":"べ","ぼ":"ぼ",
               "ぱ":"ぱ","ぴ":"ぴ","ぷ":"ぷ","ぺ":"ぺ","ぽ":"ぽ","ゃ":"ゃ","ゅ":"ゅ","ょ":"ょ","っ":"っ","ー":"ー"}
L["ja"] = latin([(k, k) for k in _kana], extra=_kana_extra,
    commands={"けして":"delete","消して":"delete","もどる":"delete","戻る":"delete","ぜんぶけして":"clear","全部消して":"clear","やりなおし":"clear","できた":"done","おわり":"done","終わり":"done"},
)

L["zh"] = latin(
    [("诶","a"),("比","b"),("西","c"),("迪","d"),("伊","e"),("艾弗","f"),("吉","g"),
     ("艾尺","h"),("艾","i"),("杰","j"),("开","k"),("艾勒","l"),("艾马","m"),
     ("艾娜","n"),("欧","o"),("屁","p"),("吉吾","q"),("艾儿","r"),("艾丝","s"),
     ("提","t"),("伊吾","u"),("维","v"),("豆贝尔维","w"),("艾克斯","x"),("吾艾","y"),("贼德","z")],
    extra={"a":"a","b":"b","c":"c","d":"d","e":"e","f":"f","g":"g","h":"h","i":"i","j":"j","k":"k","l":"l","m":"m","n":"n","o":"o","p":"p","q":"q","r":"r","s":"s","t":"t","u":"u","v":"v","w":"w","x":"x","y":"y","z":"z",
           "一":"1","二":"2","三":"3","四":"4","五":"5","衣":"1","1":"1","2":"2","3":"3","4":"4","5":"5"},
    commands={"删除":"delete","删掉":"delete","退格":"delete","返回":"delete","全部删除":"clear","清空":"clear","重来":"clear","好了":"done","完成":"done","结束":"done"},
)

for lang, data in L.items():
    data = {"lang": lang,
            "_doc": "Machine-drafted letter lexicon (2026-07-27, mic-everywhere directive) — spoken names -> typed graphemes. Pending native review.",
            **data}
    with open(os.path.join(OUT, f"{lang}.json"), "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"  {lang}: {len(data['letterNames'])} letter names")
print("done")
