#!/usr/bin/env python3
"""CC-IMPOSSIBLE-COMPLETE v1 F1 -- the RULES NET: one deliberately coarse sound key per language.

A key merges everything the shipping voice might merge; two bank words with one key are
sound-alikes for Impossible's purposes and are excluded from the pool. Over-flagging costs a
pool word, under-flagging makes the game unfair (I1), so a key errs coarse. Whole bank (every
tier) is the comparison set: a competing form in any tier is a valid bank word.

Languages with a verified SENSE-CUE table do not use this net (es, ru, en); hi, vi, ar are held
(their loopback recogniser is too weak to trust) and get no gate.
"""
import collections
import re
import unicodedata

def nfc(s): return unicodedata.normalize("NFC", s)
def nfd(s): return unicodedata.normalize("NFD", s)
def strip_marks(s): return "".join(c for c in nfd(s) if not unicodedata.combining(c))
def collapse(s): return re.sub(r"(.)\1+", r"\1", s)
def subs(s, rules):
    for a, c in rules: s = re.sub(a, c, s)
    return s

def key_fr(w):
    w = strip_marks(w.lower())
    w = subs(w, [(r"eau|au", "o"), (r"ph", "f"), (r"qu|ck|c(?=[aou])|k", "k"), (r"c|ç", "s"), (r"ai|ei|e(?=[tz]$)|et$|ez$|er$", "e"),
                 (r"ain|ein|in|im|ym", "1"), (r"an|am|en|em", "a"), (r"on|om", "o"), (r"oi", "wa"), (r"ou", "u"),
                 (r"gu(?=[ei])", "g"), (r"g(?=[ei])|j", "z"), (r"h", ""), (r"y", "i"), (r"w", "v")])
    w = collapse(w)
    return re.sub(r"[stxdzpr]+$|e$", "", w) or w  # silent finals

def key_de(w):
    w = w.lower()
    w = subs(w, [("ß", "ss"), ("ä", "e"), ("äu", "eu"), ("ö", "o"), ("ü", "u"), ("ai|ay|ey", "ei"), ("äu", "eu"), ("ph", "f"), ("v", "f"),
                 ("dt|th", "t"), ("tz", "z"), ("ck", "k"), ("ie", "i"), ("chs", "x"), ("qu", "kw"), ("ah|eh|ih|oh|uh", "V")])
    w = w.replace("V", "")
    w = collapse(w)
    return re.sub(r"b$", "p", re.sub(r"d$", "t", re.sub(r"g$", "k", w)))

def key_pt(w):
    w = strip_marks(w.lower())
    w = subs(w, [(r"ss|c(?=[ei])|ç|x(?=[aeiou])|z", "s"), (r"ch|x", "S"), (r"lh", "L"), (r"nh", "N"), (r"qu|c|k", "k"), (r"g(?=[ei])", "j"),
                 (r"gu(?=[ei])", "g"), (r"h", ""), (r"[aeiou][mn](?=[^aeiou]|$)", "~"), (r"ao$|am$|oes$|aes$", "~"), (r"ph", "f"),
                 (r"w", "v"), (r"y", "i"), (r"o$", "u"), (r"e$", "i"), (r"s$", "")])
    return collapse(w)

def key_pl(w):
    w = w.lower()
    w = subs(w, [("ó", "u"), ("rz|ż", "Z"), ("sz", "S"), ("ch", "h"), ("ć|ci(?=[aąeęioóuy])", "C"), ("ś|si(?=[aąeęioóuy])", "s"), ("ź|zi(?=[aąeęioóuy])", "z"), ("ń|ni(?=[aąeęioóuy])", "n"),
                 ("ę", "e"), ("ą", "o"), ("ł", "l"), ("dz|dź|dż", "D"), ("cz", "c")])
    w = collapse(w)
    voice = str.maketrans("bdgwzZvD", "ptkfsSfT")
    return w.translate(voice)

def _vi_syl(w):
    w = subs(w, [(r"^(gi|d|r)", "z"), (r"^(s|x)", "s"), (r"^(ch|tr)", "c"), (r"^ngh", "ng"), (r"^gh", "g"), (r"^(k|q|c(?=[aăâouôơưy]))", "k"),
                 (r"^đ", "d"), (r"nh$", "n"), (r"(ch|c)$", "k")])
    return w
def key_vi(w):  # tones KEPT; consonant mergers applied per syllable
    return " ".join(_vi_syl(unicodedata.normalize("NFC", x)) for x in w.lower().split())

# --- Korean --------------------------------------------------------------
L = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"
V = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"
T = [""] + list("ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ")
CODA = {"ㄲ": "ㄱ", "ㅋ": "ㄱ", "ㄳ": "ㄱ", "ㄺ": "ㄱ", "ㅅ": "ㄷ", "ㅆ": "ㄷ", "ㅈ": "ㄷ", "ㅊ": "ㄷ", "ㅌ": "ㄷ", "ㅎ": "ㄷ", "ㄵ": "ㄴ", "ㄶ": "ㄴ",
        "ㄼ": "ㄹ", "ㄽ": "ㄹ", "ㄾ": "ㄹ", "ㅀ": "ㄹ", "ㄻ": "ㅁ", "ㅍ": "ㅂ", "ㅄ": "ㅂ", "ㄿ": "ㅂ"}
def jamo(w):
    out = []
    for c in nfc(w):
        o = ord(c) - 0xAC00
        if 0 <= o < 11172: out.append([L[o // 588], V[(o % 588) // 28], T[o % 28]])
        else: out.append([c, "", ""])
    return out
def key_ko(w):
    s = jamo(w)
    for i, (l, v, t) in enumerate(s):
        # liaison: coda moves to a following empty onset
        if i + 1 < len(s) and s[i + 1][0] == "ㅇ" and t and t != "ㅇ":
            if t in "ㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ":
                a, c = {"ㄳ": ("ㄱ", "ㅅ"), "ㄵ": ("ㄴ", "ㅈ"), "ㄶ": ("ㄴ", "ㅎ"), "ㄺ": ("ㄹ", "ㄱ"), "ㄻ": ("ㄹ", "ㅁ"), "ㄼ": ("ㄹ", "ㅂ"),
                        "ㄽ": ("ㄹ", "ㅅ"), "ㄾ": ("ㄹ", "ㅌ"), "ㄿ": ("ㄹ", "ㅍ"), "ㅀ": ("ㄹ", "ㅎ"), "ㅄ": ("ㅂ", "ㅅ")}[t]
                s[i][2] = a; s[i + 1][0] = c
            elif t != "ㅎ":
                s[i + 1][0] = t; s[i][2] = ""
            else: s[i][2] = ""
    # palatalisation 이 + ㄷ/ㅌ
    for i in range(len(s) - 1):
        if s[i][2] in ("ㄷ", "ㅌ") and s[i + 1][0] == "ㅇ" and s[i + 1][1] == "ㅣ":
            s[i + 1][0] = "ㅈ" if s[i][2] == "ㄷ" else "ㅊ"; s[i][2] = ""
    for x in s:
        x[2] = CODA.get(x[2], x[2])
    # nasalisation / lateralisation / tensification (coarse: neutralise to the merged class)
    for i in range(len(s) - 1):
        t, n = s[i][2], s[i + 1][0]
        if t in ("ㄱ", "ㄷ", "ㅂ") and n in ("ㄴ", "ㅁ"): s[i][2] = {"ㄱ": "ㅇ", "ㄷ": "ㄴ", "ㅂ": "ㅁ"}[t]
        if (t == "ㄴ" and n == "ㄹ") or (t == "ㄹ" and n == "ㄴ"): s[i][2] = "ㄹ"; s[i + 1][0] = "ㄹ"
        if t in ("ㄱ", "ㄷ", "ㅂ") and n in "ㄱㄷㅂㅅㅈ":  # tensification: fold plain/tense
            pass
    merge_v = {"ㅐ": "ㅔ", "ㅒ": "ㅖ", "ㅙ": "ㅞ", "ㅚ": "ㅞ", "ㅟ": "ㅟ"}
    tense = {"ㄲ": "ㄱ", "ㅋ": "ㄱ", "ㄸ": "ㄷ", "ㅌ": "ㄷ", "ㅃ": "ㅂ", "ㅍ": "ㅂ", "ㅆ": "ㅅ", "ㅉ": "ㅈ", "ㅊ": "ㅈ"}  # coarse: plain/tense/aspirate share a key
    return "".join(tense.get(l, l) + merge_v.get(v, v) + t for l, v, t in s)

def key_fil(w):
    w = strip_marks(w.lower())
    w = subs(w, [("ñ", "ny"), ("ng", "N"), ("qu", "k"), ("c(?=[ei])", "s"), ("c", "k"), ("ph|f", "p"), ("v", "b"), ("j", "dy"), ("z", "s"),
                 ("e", "i"), ("o", "u"), ("'|-", ""), ("h", "")])
    return collapse(w)

def key_ar(w):
    w = "".join(c for c in nfd(w) if not unicodedata.combining(c))
    w = subs(w, [("[أإآءؤئ]", "ا"), ("ة", "ه"), ("[ىي]", "ي"), ("ث", "س"), ("ص", "س"), ("[ذظضز]", "ز"), ("ض", "د"), ("ط", "ت"), ("ق", "ك"), ("ـ", "")])
    w = w.replace("ز", "د") if False else w
    return collapse(w)

def key_sw(w):
    w = w.lower()
    w = subs(w, [("ng'", "ng"), ("l", "r"), ("dh", "d"), ("th", "t"), ("gh", "g"), ("kh", "k")])
    return collapse(w)

def key_hi(w):
    w = nfd(w)
    w = subs(w, [("़", ""), ("ँ", "ं"), ("[ङञणन]्", "ं"), ("[ङञण]", "न"), ("ष", "श"), ("[ीि]", "ि"), ("[ूु]", "ु"), ("ऋ", "रि"), ("ः", "ह"), ("ड़", "ड"), ("ढ़", "ढ"), ("ज़", "ज"), ("फ़", "फ")])
    return nfc(w)

NETS = {"fr": key_fr, "de": key_de, "pt": key_pt, "pl": key_pl, "vi": key_vi, "ko": key_ko, "fil": key_fil, "ar": key_ar, "sw": key_sw, "hi": key_hi}


NETS = {"fr": key_fr, "de": key_de, "pt": key_pt, "pl": key_pl, "vi": key_vi, "ko": key_ko, "fil": key_fil, "ar": key_ar, "sw": key_sw, "hi": key_hi}


def flagged(lang, expert, whole_bank):
    """Expert words sharing a key with another DIFFERENT word anywhere in the bank."""
    key = NETS[lang]
    groups = collections.defaultdict(set)
    for w in whole_bank:
        groups[key(nfc(w))].add(nfc(w))
    out = set()
    for w in expert:
        if len(groups[key(nfc(w))]) > 1:
            out.add(nfc(w))
    return out
