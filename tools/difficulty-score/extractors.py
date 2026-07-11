"""Per-language difficulty feature extractors (word-quality spec, Part 1).

Each extractor returns the `hardBecause` list for a word — the machine-verifiable
reasons its SPELLING (from hearing) is hard in that language's writing system.
An Expert word must carry ≥1 feature. This is the algorithmic core; it needs no
external data (frequency/graded lists refine the *score*, not the features).

Pilot set proves the three hardest extractor styles:
  en  — Latin orthographic irregularity
  zh  — character recall / stroke complexity / homophone density
  vi  — tone-mark + diacritic + initial/final ambiguity (diacritic-heavy Latin)

The remaining 14 languages plug in the same way (one function each) — stubs
below list the features from the spec table to implement when their data lands.
"""
from __future__ import annotations

import unicodedata

# ---------------------------------------------------------------- English

_SILENT = ["kn", "mb", "gh", "wr", "mn", "bt", "gn", "pn", "ps"]
_LOAN = ["eau", "ough", " psy", "eur", "oir", "aut"]
_AMBIG_SUFFIX = ["able", "ible", "ance", "ence", "ant", "ent", "tion", "sion", "cian"]


def en(word: str) -> list[str]:
    w = word.lower()
    f = []
    if any(s in w for s in _SILENT):
        f.append("silent_letters")
    # doubled consonant
    if any(w[i] == w[i + 1] and w[i] not in "aeiou" for i in range(len(w) - 1)):
        f.append("doubled_consonant")
    if any(w.endswith(s) or s in w for s in _AMBIG_SUFFIX):
        f.append("unstressed_vowel_ambiguity")
    if any(l in w for l in _LOAN):
        f.append("loanword_spelling")
    return f


# ---------------------------------------------------------------- 中文

# Homophone-dense syllables (typed input = character recall; many chars share the
# sound so recalling THE character is hard). Small representative set.
def zh(word: str) -> list[str]:
    # For "pinyin|hanzi" entries we score the hanzi side.
    hanzi = word.split("|")[-1]
    f = []
    if any(ord(c) > 0x4E00 for c in hanzi):
        # multi-character or rarer characters are harder to recall
        if len(hanzi) >= 2:
            f.append("character_recall")
        # crude stroke-complexity proxy: codepoint in the higher CJK range
        if any(ord(c) >= 0x9000 for c in hanzi):
            f.append("high_stroke_count")
        f.append("homophone_density")  # Mandarin is homophone-dense by nature
    return f


# ---------------------------------------------------------------- Tiếng Việt

_HOI_NGA = set("ảẢỉỈủỦỏỎẻẺỷỶễỄ") | {c for c in "ãẫẵẽễỗỡữ"}  # hỏi/ngã-bearing
_DIGRAPH_INITIALS = ["ch", "tr", "gi", "ng", "nh", "ph", "kh", "th", "qu"]


def vi(word: str) -> list[str]:
    w = word.lower()
    f = []
    marks = [c for c in unicodedata.normalize("NFD", w) if unicodedata.combining(c)]
    # hỏi (U+0309) / ngã (U+0303) — the classic native-speaker dictation trap
    if "̉" in marks or "̃" in marks:
        f.append("hoi_nga_tone")
    if len(marks) >= 3:
        f.append("dense_diacritics")
    if any(w.startswith(d) or d in w for d in _DIGRAPH_INITIALS):
        f.append("initial_final_ambiguity")
    return f


import unicodedata as _ud


def _has_double(w: str) -> bool:
    return any(w[i] == w[i + 1] and w[i].isalpha() and w[i] not in "aeiouáàâãéêíóôõúü" for i in range(len(w) - 1))


def _accented(w: str) -> int:
    return sum(1 for c in _ud.normalize("NFD", w) if _ud.combining(c))


# ---- Latin-script European extractors (spec feature table) ----
def es(w):
    w = w.lower(); f = []
    if "ll" in w or "y" in w: f.append("ll_y")
    if any(x in w for x in ("ge", "gi", "ja", "je", "ji", "jo", "ju")): f.append("g_j")
    if "h" in w and not w.startswith("ch"): f.append("silent_h")
    if any(x in w for x in ("z", "ce", "ci")): f.append("c_s_z")
    if _accented(w): f.append("accent_placement")
    return f


def fr(w):
    w = w.lower(); f = []
    if w[-1:] in "sdtxznp" and not w.endswith(("tion", "sion")): f.append("silent_finals")
    if any(w.endswith(x) for x in ("er", "é", "ez", "ait", "ai")): f.append("verb_ending_homophones")
    if _has_double(w): f.append("doubled_consonant")
    if _accented(w): f.append("accent_choice")
    if any(x in w for x in ("tion", "ssion", "cien")): f.append("tion_family")
    return f


def de(w):
    f = []
    if "ß" in w: f.append("ss_vs_ss")
    if _has_double(w): f.append("doubled_consonant")
    if any(x in w for x in ("ieh", "ih", "ie")): f.append("long_i_spelling")
    if len(w) >= 11: f.append("compound_length")
    if w[-1:] in "dgb": f.append("end_devoicing")
    return f


def pt(w):
    w = w.lower(); f = []
    if "ç" in w: f.append("cedilha")
    if any(x in w for x in ("ão", "õe", "ã", "õ")): f.append("nasal_marks")
    if "x" in w: f.append("x_ambiguity")
    if any(x in w for x in ("ss", "sc", "ç", "z")): f.append("s_sound_set")
    if _accented(w): f.append("accent_placement")
    return f


def it(w):
    w = w.lower(); f = []
    if _has_double(w): f.append("double_consonant")
    if "gli" in w or "gn" in w: f.append("gli_gn")
    if "cu" in w or "qu" in w: f.append("cu_qu")
    if "sce" in w or "sci" in w: f.append("sce_scie")
    return f


def nl(w):
    w = w.lower(); f = []
    if "ei" in w or "ij" in w: f.append("ei_ij")
    if "ou" in w or "au" in w: f.append("ou_au")
    if w.endswith(("d", "dt", "t")): f.append("d_t_dt")
    if _has_double(w): f.append("vowel_length")
    return f


def pl(w):
    w = w.lower(); f = []
    if "ż" in w or "rz" in w: f.append("z_rz")
    if "ó" in w or "u" in w: f.append("u_o")
    if "ch" in w or "h" in w: f.append("h_ch")
    if "ą" in w or "ę" in w: f.append("nasal_vowels")
    if any(c not in "aeiouyąę" for c in w[:3]) and len(set(w[:4]) & set("bcdfgklmnprstwzżźćśń")) >= 3: f.append("consonant_cluster")
    return f


def sv(w):
    w = w.lower(); f = []
    if any(x in w for x in ("sj", "skj", "stj", "sk", "sch")): f.append("sj_sound")
    if any(x in w for x in ("tj", "kj")): f.append("tj_sound")
    if any(c in w for c in "åäö"): f.append("vowel_ambiguity")
    if _has_double(w): f.append("double_consonant")
    return f


def nb(w):
    w = w.lower(); f = []
    if w.startswith("hv") or w.endswith(("g", "d")): f.append("silent_letters")
    if "o" in w or "å" in w: f.append("o_a_ambiguity")
    if "kj" in w or "skj" in w: f.append("kj_skj")
    if _has_double(w): f.append("double_consonant")
    return f


def tr(w):
    w = w.lower(); f = []
    if "ğ" in w: f.append("soft_g")
    if "â" in w: f.append("circumflex")
    # loanword violating vowel harmony (mix of front+back vowels)
    front, back = set("eiöü"), set("aıou")
    vowels = [c for c in w if c in front | back]
    if vowels and (set(vowels) & front) and (set(vowels) & back): f.append("harmony_violation")
    return f


# ---- Non-Latin extractors ----
def ja(w):
    f = []
    if "ー" in w: f.append("long_vowel_mark")
    if any(c in w for c in "っゃゅょ"): f.append("small_kana")
    if "づ" in w or "ぢ" in w: f.append("dzu_ji")
    if any(c in w for c in "おう") and "う" in w: f.append("long_vowel_spelling")
    return f


_KO_DOUBLE_BATCHIM = set(range(3, 20))  # simplistic: many double-final indices


def ko(w):
    f = []
    for c in w:
        u = ord(c)
        if 0xAC00 <= u <= 0xD7A3:
            fin = (u - 0xAC00) % 28
            if fin in (3, 5, 6, 9, 10, 11, 12, 13, 14, 15, 18): f.append("double_batchim"); break
    if len(w) >= 3: f.append("multi_syllable")
    return list(dict.fromkeys(f))


def th(w):
    f = []
    if "์" in w: f.append("garan_silent")  # thanthakhat killer mark
    if "ๆ" in w: f.append("repetition_mark")
    if any(c in w for c in "ศษส") and w.count("ส") + w.count("ศ") + w.count("ษ"): pass
    if len([c for c in w if c in "ศษ"]): f.append("s_consonant_set")
    if "ทร" in w: f.append("irregular_cluster")
    if len(w) >= 7: f.append("length")
    return f


def fil(w):
    w = w.lower(); f = []
    if "-" in w: f.append("hyphen_glottal")
    if any(x in w for x in ("ts", "sy", "dy", "ky", "ny")): f.append("adapted_loan_spelling")
    if "ng" in w: f.append("ng_digraph")
    if "iy" in w or "uw" in w: f.append("glide_spelling")
    return f


EXTRACTORS = {
    "en": en, "es": es, "fr": fr, "de": de, "pt": pt, "it": it, "nl": nl, "pl": pl,
    "sv": sv, "nb": nb, "tr": tr, "vi": vi, "ko": ko, "ja": ja, "zh": zh, "th": th, "fil": fil,
}
