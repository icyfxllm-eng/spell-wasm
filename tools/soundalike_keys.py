#!/usr/bin/env python3
"""CC-SOUNDALIKE-LADDER F2: sound keys for fr and pt, and their tests.

    python3 tools/soundalike_keys.py --selftest

A key maps a spelling to an approximate pronunciation string; words with the same key are CANDIDATE sound-alikes. The keys are
deliberately conservative: when a rule is unsure it keeps the letter, so two words get different keys rather than a wrong merge
(a missed set costs nothing, a wrong set in accept-any marks a wrong spelling correct). A key is never trusted alone: a set
reaches "provisional" only when the key, an independent IPA check and the voice loopback all agree (ladder F3).

Pure standard library; no IPA is read or written here (Wiktionary IPA is a check signal only, tools/soundalike_ipa_check.py).
"""
import re, sys, unicodedata

nfc = lambda s: unicodedata.normalize("NFC", s)
V = "aeiouáàâãäéèêëíîïóòôõöúùûü"


def _squeeze(s):
    return re.sub(r"([bcdfgklmnprstvz])\1", r"\1", s)


# ----------------------------------------------------------------------------- French
_FV = "aeiouyàâäéèêëîïôöùûüœ"


def _groups(s):
    return len(re.findall(r"[aeiouyàâäéèêëîïôöùûüœ]+", s))


def key_fr(word):
    s = nfc(word.lower())
    s = re.sub(r"[^a-zàâäéèêëîïôöùûüÿçœæ]", "", s).replace("ÿ", "i")
    if not s:
        return ""
    # 1. silent endings, decided on the letters (so a closed "é" is never confused with a mute "e")
    s = re.sub(r"ées?$", "é", s)
    s = re.sub(r"(?<=[aeiouyàâéèêîôûù])es?$", "", s) if len(s) > 3 else s                 # voie, joie, vie
    if _groups(s) > 1:
        s = re.sub(r"(?<=[^aeiouyàâäéèêëîïôöùûüœ])es?$", "_", s)                        # cane, canne, portes: a mute e
    if re.search(r"[aeiouyàâäéèêëîïôöùûüœ].*[^aeiouyàâäéèêëîïôöùûüœ]?er$", s) and _groups(s) > 1:
        s = s[:-2] + "é"                                                                  # aimer = aimé = aimez
    s = re.sub(r"ez$", "é", s) if _groups(s) > 1 else s
    s = re.sub(r"[sxz]$", "", s) if len(s) > 2 else s                   # a x before a mute e is pronounced (annexe), so only a bare final one goes
    if _groups(s.replace("_", "")) == 1 and s.endswith("e"):
        s = s[:-1] + "ə"                                           # de, le, me: a schwa, not a closed e
    # 2. consonant digraphs
    s = s.replace("ch", "ʃ").replace("ph", "f").replace("th", "t").replace("gn", "ɲ").replace("œu", "ø").replace("oeu", "ø").replace("œ", "ø").replace("æ", "è")
    s = re.sub(r"qu", "k", s)
    s = re.sub(r"gu(?=[eiyéèêî])", "G", s)                  # a hard g, kept apart from the soft g below
    s = s.replace("h", "")
    # 3. nasals (a vowel group + n/m before a consonant or the end)
    nas = r"(?=[^aeiouyàâäéèêëîïôöùûüœhnm_]|$)"
    s = re.sub(r"(ain|aim|ein|eim)" + nas, "I", s)
    s = re.sub(r"ien" + nas, "jI", s)
    s = re.sub(r"(?<![aeiouéèê])(in|im|yn|ym)" + nas, "I", s)
    s = re.sub(r"(un|um)" + nas, "U", s)                      # brun is not brin: an accent difference, kept apart
    s = re.sub(r"(an|am|en|em)" + nas, "A", s)
    s = re.sub(r"(on|om)" + nas, "N", s)
    # 4. vowels. "ou" is /u/ and "u" is /y/: different markers
    s = re.sub(r"(eau|au|ô)", "O", s)
    s = re.sub(r"(où|oû|ou)", "W", s)
    s = re.sub(r"(oi|oî)", "wa", s)
    s = re.sub(r"oy(?=[aeiou])", "waj", s)
    s = re.sub(r"(ai|aî|ei|ê|è|ë)", "E", s)
    s = re.sub(r"e(?=rr|ll|tt|nn|ss|mm|ff|pp|ct|st|x)", "E", s)       # open e before a double or a cluster
    if _groups(s.replace("A", "a").replace("I", "i").replace("N", "o").replace("U", "u")) == 1:
        s = re.sub(r"e(?=r[td]?$|[lcstfdnmpbvgz]$)", "E", s)        # a closed monosyllable: mer, vert, sel, bec
    s = s.replace("é", "e").replace("ç", "s")
    s = re.sub(r"c(?=[eiyEeIjw])", "S", s)
    s = s.replace("c", "k")
    s = re.sub(r"g(?=[eiyEeI])", "ʒ", s).replace("j", "ʒ")
    s = re.sub(r"ss", "S", s).replace("s", "s")
    s = re.sub(r"(?<=[aeiouyAEINOUWy])s(?=[aeiouyAEINOUWy])", "z", s)
    s = s.replace("S", "s")
    s = re.sub(r"x", "ks", s)
    s = s.replace("y", "i").replace("à", "a").replace("â", "a").replace("ä", "a").replace("î", "i").replace("ï", "i").replace("ù", "y").replace("û", "y").replace("ü", "y").replace("u", "y").replace("ô", "O").replace("ö", "O")
    s = _squeeze(s)
    # 5. a final consonant that is silent in the common words, only where the rest ends in a vowel or a nasal
    s = re.sub(r"(?<=[AINUOWEeaiyøw])[tdp]$", "", s)
    s = re.sub(r"(?<=[AINU])g$", "", s)                              # sang, long: a g after a nasal is silent
    s = re.sub(r"(?<=r)[td]$", "", s)
    return s.replace("_", "")


# ----------------------------------------------------------------------------- Portuguese (Brazil)
def key_pt(word):
    s = nfc(word.lower())
    s = re.sub(r"[^a-záàâãéêíóôõúüçñ]", "", s)
    if not s:
        return ""
    s = s.replace("h", "") if not re.search(r"[cln]h", s) else re.sub(r"(?<![cln])h", "", s)
    s = s.replace("lh", "ʎ").replace("nh", "ɲ").replace("ch", "ʃ")
    s = re.sub(r"^ex(?=[aeiouáéêíóôú])", "ez", s)           # exame; x before a consonant is left alone below
    s = re.sub(r"ex(?=[pctfq])", "es", s)                    # expiar, excesso: es
    s = re.sub(r"^x", "ʃ", s)                                # xá, xeque (word-initial x is /ʃ/)
    s = re.sub(r"(?<=[ei])x(?=[aeiou])", "ʃ", s) if False else s
    s = re.sub(r"qu(?=[eiéêí])", "k", s).replace("qu", "kw")
    s = re.sub(r"gu(?=[eiéêí])", "g", s)
    s = re.sub(r"sc(?=[eiéêí])", "S", s).replace("sç", "S")
    s = s.replace("ç", "S")
    s = re.sub(r"c(?=[eiéêí])", "S", s).replace("c", "k")
    s = re.sub(r"ss", "S", s)
    s = re.sub(r"(?<=[aeiouáàâãéêíóôõúü])s(?=[aeiouáàâãéêíóôõúü])", "z", s)
    s = s.replace("S", "s")
    s = re.sub(r"[sz]$", "s", s)
    s = re.sub(r"g(?=[eiéêí])", "ʒ", s).replace("j", "ʒ")
    s = re.sub(r"^r|(?<=[nls])r|rr", "R", s)
    # written accents are kept in the key (vera is not verá, avó is not avô); only the diaeresis is dropped
    s = s.replace("ü", "u")      # every other written accent is kept: stress and open/closed quality differ between words
    s = _squeeze(s)
    s = s.replace("ñ", "ɲ")
    return s


# ----------------------------------------------------------------------------- Polish
def key_pl(word):
    """Polish: ó=u, rz=ż, ch=h are true (CC-SOUNDALIKE-LADDER key notes); word-final obstruents devoice (kot=kod). rz after a
    voiceless consonant is sz (trzy, krzak). Nasals ą/ę, palatals and every other letter are left alone: fewer merges, never wrong ones."""
    s = nfc(word.lower())
    s = re.sub(r"[^a-ząćęłńóśźż]", "", s)
    if not s:
        return ""
    s = s.replace("ch", "h")
    s = re.sub(r"(?<=[ptkhcsf])rz", "sz", s).replace("rz", "ż")
    s = s.replace("ó", "u")
    s = re.sub(r"dż$", "cz", s)
    s = re.sub(r"dź$", "ć", s)
    s = re.sub(r"dz$", "c", s)
    s = re.sub(r"ż$", "sz", s)
    s = re.sub(r"ź$", "ś", s)
    s = re.sub(r"(?<![sc])z$", "s", s)
    s = s[:-1] + {"b": "p", "d": "t", "g": "k", "w": "f"}.get(s[-1], s[-1])
    return _squeeze(s)


# ----------------------------------------------------------------------------- Filipino
def key_fil(word):
    """Filipino is near-phonetic: the key is the normalized spelling plus a short list of spelling mergers for loan letters
    (c=k/s, qu=k, ph=f, x=ks, ñ=ny, doubled letters). e/i, o/u, d/r, p/f and b/v are NOT merged: they are accent or
    variant questions, handled by accepted-variants, never by an accept-any set."""
    s = nfc(word.lower())
    s = re.sub(r"[^a-zñáàâéèêíìîóòôúùû]", "", s)
    s = s.translate(str.maketrans("áàâéèêíìîóòôúùû", "aaaeeeiiiooouuu"))
    if not s:
        return ""
    s = s.replace("ñ", "ny").replace("ph", "f").replace("x", "ks")
    s = re.sub(r"qu(?=[ei])", "k", s).replace("qu", "kw").replace("q", "k")
    s = re.sub(r"c(?=[ei])", "s", s)
    s = re.sub(r"c(?!h)", "k", s)
    return _squeeze(s)


# ----------------------------------------------------------------------------- tests
# Pairs that must share a key: the staged closed-list seeds that this key is meant to derive (a missing merge is a
# false negative, reported, not wrong), and pairs the key must NEVER merge (a merge would be a wrong accept-any set).
MUST_MERGE = {
    "fr": [("sang", "sans"), ("foi", "fois"), ("foi", "foie"), ("pain", "pin"), ("saint", "sein"), ("mère", "mer"), ("mère", "maire"),
           ("chant", "champ"), ("cou", "coup"), ("voie", "voix"), ("cane", "canne"), ("cygne", "signe"), ("ancre", "encre"),
           ("tante", "tente"), ("vers", "verre"), ("vert", "verre"), ("cour", "cours"), ("cour", "court"), ("air", "aire"), ("aimer", "aimé"), ("aimer", "aimez")],
    "pl": [("kot", "kod"), ("może", "morze"), ("grób", "grup"), ("hart", "chart"), ("ług", "łuk"), ("żaba", "rzaba"), ("bóg", "buk"),
           ("góra", "gura"), ("mur", "mór"), ("tuż", "tusz")],
    "fil": [("kasa", "casa"), ("kilo", "quilo"), ("sine", "cine"), ("kuwarto", "cuwarto"), ("taksi", "taxi"), ("senyor", "señor"), ("fotograpiya", "photograpiya"), ("kamatis", "kammatis")],
    "pt": [("caça", "cassa"), ("cozer", "coser"), ("chá", "xá"), ("cheque", "xeque"), ("concerto", "conserto"), ("acento", "assento"),
           ("senso", "censo"), ("sessão", "cessão"), ("hora", "ora"), ("cela", "sela"), ("espiar", "expiar"), ("laço", "lasso"),
           ("cerrar", "serrar"), ("paço", "passo"), ("cinto", "sinto"), ("ruço", "russo"), ("apreçar", "apressar")],
}
MUST_NOT_MERGE = {
    "fr": [("brin", "brun"), ("empreinte", "emprunte"), ("pêche", "péché"), ("poisson", "poison"), ("dessus", "dessous"),
           ("pain", "pan"), ("son", "sang"), ("bon", "banc"), ("vin", "vent"), ("main", "mon"), ("mur", "mou"), ("rue", "roue"),
           ("peur", "pour"), ("lit", "lu"), ("guet", "jet"), ("fig", "fis"), ("annexe", "année"), ("sol", "sel"), ("dé", "de"), ("mal", "mol"), ("jeu", "jou"), ("rose", "rase")],
    "pl": [("rok", "róg"), ("żar", "zar"), ("czas", "cas"), ("szal", "sal"), ("ćma", "cma"), ("świat", "swiat"), ("łapa", "lapa"),
           ("koń", "kon"), ("są", "sa"), ("rzeka", "reka"), ("ból", "bol"), ("sól", "sol"), ("baba", "papa"), ("dom", "tom"),
           ("bąk", "bak"), ("kura", "kora"), ("wąż", "waż"), ("rzeka", "zeka")],
    "fil": [("bata", "beta"), ("lalaki", "lalake"), ("dito", "rito"), ("pilipino", "filipino"), ("baso", "vaso"), ("bote", "buti"), ("mano", "manu"), ("ako", "aku"),
            ("kama", "gama"), ("aso", "asu"), ("tama", "tana"), ("bata", "bat"), ("mata", "mataa")],
    "pt": [("mal", "mau"), ("pode", "pôde"), ("avó", "avô"), ("caça", "casa"), ("assar", "azar"), ("por", "pôr"), ("pera", "pêra"),
           ("ouço", "ouso"), ("são", "sou"), ("lã", "la"), ("pão", "pau"), ("mãe", "me"), ("vá", "vê"), ("rio", "rico"), ("caro", "carro"),
           ("cura", "curra"), ("ara", "arra"), ("gato", "jato")],
}
KEYS = {"fr": key_fr, "pt": key_pt, "pl": key_pl, "fil": key_fil}


def selftest():
    bad = 0
    for lang, fn in KEYS.items():
        for a, b in MUST_NOT_MERGE[lang]:
            if fn(a) == fn(b):
                print(f"FAIL {lang}: {a}/{b} merged under key {fn(a)!r}"); bad += 1
        for a, b in MUST_MERGE[lang]:
            if fn(a) != fn(b):
                print(f"MISS {lang}: {a}={fn(a)!r} {b}={fn(b)!r} (a seed the key does not derive: it stays a seed, not a rule-derived set)")
        # keys are stable, non-empty for real words, and never contain spaces
        for w in ("casa", "maison", "você", "élève"):
            k = fn(w)
            if not k or " " in k:
                print(f"FAIL {lang}: key of {w} is {k!r}"); bad += 1
    print("soundalike_keys selftest:", "OK" if not bad else f"{bad} problem(s)")
    return bad


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(1 if selftest() else 0)
    for a in sys.argv[1:]:
        print(a, "fr:", key_fr(a), "pt:", key_pt(a), "pl:", key_pl(a))
