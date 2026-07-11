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


EXTRACTORS = {"en": en, "zh": zh, "vi": vi}

# Stubs — features from the spec table, to implement when each language is scored.
TODO_FEATURES = {
    "es": ["b_v", "silent_h", "ll_y", "c_s_z", "g_j", "accent_placement"],
    "fr": ["silent_finals", "verb_ending_homophones", "accents", "tion_family"],
    "de": ["ss_vs_ss", "doubled_consonant", "long_i_spellings", "compound", "end_devoicing"],
    "ja": ["long_vowel_mark", "small_tsu", "dzu_zu_ji", "rare_reading"],
    "ko": ["sound_change", "ae_e_merger", "silent_h", "sai_siot"],
    "th": ["silent_letters", "inherent_vowel", "class_tone", "consonant_homophones"],
    # … it, nl, pl, sv, nb, tr, fil, pt per the spec table.
}
