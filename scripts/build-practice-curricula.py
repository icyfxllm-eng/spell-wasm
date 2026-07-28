#!/usr/bin/env python3
"""CC-PRACTICE D4 — draft per-language curricula (20 first-contact + 5
difficult words + trap intro cards) for Eric's review.

AGENT-DRAFTED per D4's proposed path: trap classes come from
config/trap-registry.json (ru/ar existing; the rest added alongside this
script as data-only registry entries); words are selected FROM THE LANGUAGE'S
OWN BANK (never authored) by per-trap heuristics — first-contact words from
the easy tier, difficult words from medium/hard, one per introduced trap.
Intro strings live in the curriculum file (≤90 chars hard cap, one line per
trap class, shown once per run) pending the standing audit round.

Output: config/practice/<lang>.json
Validate: node scripts/practice-check.mjs   (schema gate, CI)
"""
import json, os, re, unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "config", "practice")
TIERS = ["easy", "medium", "hard", "expert"]

def bank(lang):
    words = {}
    if lang == "zh":
        src = open(os.path.join(ROOT, "src", "words.rs"), encoding="utf-8").read()
        for tier in TIERS:
            m = re.search(rf"pub const ZH_{tier.upper()}: &\[&str\] = &\[(.*?)\];", src, re.S)
            words[tier] = [e for e in re.findall(r'"([^"]+)"', m.group(1))]
        return words
    for tier in TIERS:
        p = os.path.join(ROOT, "assets", "words", lang, f"{tier}.txt")
        words[tier] = [w.strip() for w in open(p, encoding="utf-8") if w.strip()]
    return words

# (trap_id, word_matcher, intro ≤90 chars) per language. Matchers pick REAL
# bank words that exhibit the trap. Order = teaching sequence.
def m(rx):
    r = re.compile(rx)
    return lambda w: r.search(w) is not None

CURRICULA = {
    "en": [
        ("silent-e",        m(r"[a-z][bcdfgklmnprstvz]e$"),      "A silent e at the end often makes the vowel before it say its name."),
        ("double-consonant",m(r"(.)\1"),                          "Some words double a consonant — listen for the short vowel before it."),
        ("ie-ei",           m(r"ie|ei"),                          "i before e except after c — usually!"),
        ("hard-c-k",        m(r"c[aou]|ck|k"),                    "The k sound can be written c, k, or ck."),
        ("wh-th-sh",        m(r"wh|th|sh|ch"),                    "Two letters, one sound: sh, ch, th, wh stick together."),
    ],
    "es": [
        ("b-v",       m(r"[bv]"),        "b y v suenan igual — hay que memorizar cuál lleva cada palabra."),
        ("h-muda",    m(r"h"),           "La h no suena: se escribe pero no se pronuncia."),
        ("ll-y",      m(r"ll|y"),        "ll e y pueden sonar igual — fíjate cuál usa la palabra."),
        ("tildes",    m(r"[áéíóú]"),     "La tilde marca la sílaba fuerte: á é í ó ú."),
        ("c-s-z",     m(r"ce|ci|z|s"),   "c (ante e/i), s y z pueden sonar parecido según la región."),
    ],
    "fr": [
        ("finales-muettes", m(r"[tdsxpz]$"),  "Beaucoup de lettres finales sont muettes : on les écrit sans les dire."),
        ("accents",         m(r"[éèêàùôîç]"), "Les accents changent le son : é è ê — écoute bien."),
        ("eau-au-o",        m(r"eau|au"),     "Le son o peut s'écrire o, au ou eau."),
        ("doubles",         m(r"(.)\1"),      "Certaines consonnes se doublent — la prononciation ne le dit pas toujours."),
        ("ou-u",            m(r"ou|u"),       "u et ou sont deux sons différents en français."),
    ],
    "de": [
        ("grossschreibung", m(r"^[A-ZÄÖÜ]"),  "Alle Nomen beginnen groß — immer!"),
        ("ie-ei",           m(r"ie|ei"),      "ie klingt wie ein langes i, ei klingt wie ai."),
        ("umlaute",         m(r"[äöü]"),      "Die Punkte zählen: a und ä sind verschiedene Laute."),
        ("sch-ch",          m(r"sch|ch"),     "sch und ch sind feste Teams aus mehreren Buchstaben."),
        ("doppel",          m(r"(.)\1"),      "Nach kurzem Vokal wird der Konsonant oft verdoppelt."),
    ],
    "pt": [
        ("nasais",     m(r"[ãõ]|am$|em$"),  "O til (~) deixa a vogal nasal: ã e õ saem pelo nariz."),
        ("acentos",    m(r"[áéíóúâêô]"),    "O acento marca a sílaba forte: á é í ó ú."),
        ("cedilha",    m(r"ç"),             "O ç soa como s antes de a, o, u."),
        ("lh-nh",      m(r"lh|nh"),         "lh e nh são sons próprios do português — duas letras, um som."),
        ("s-z",        m(r"s|z"),           "Entre vogais, o s soa como z."),
    ],
    "pl": [
        ("rz-z",       m(r"rz|ż"),      "rz i ż brzmią tak samo — trzeba zapamiętać pisownię."),
        ("o-u",        m(r"ó|u"),       "ó i u brzmią identycznie — to pułapka ortograficzna nr 1."),
        ("ch-h",       m(r"ch|h"),      "ch i h brzmią tak samo — pisownia z historii słowa."),
        ("nosowe",     m(r"[ąę]"),      "ą i ę to samogłoski nosowe — charakterystyczne dla polskiego."),
        ("miekkie",    m(r"[śćźń]|si|ci|zi|ni"), "Miękkie ś ć ź ń można też zapisać jako si ci zi ni."),
    ],
    "vi": [
        ("thanh-dieu", m(r"[àáảãạằắẳẵặầấẩẫậèéẻẽẹềếểễệìíỉĩịòóỏõọồốổỗộờớởỡợùúủũụừứửữựỳýỷỹỵ]"), "Dấu thanh đổi nghĩa của từ — nghe kỹ thanh điệu."),
        ("d-gi-r",     m(r"^d|^gi|^r"),  "d, gi và r có thể nghe giống nhau — chú ý chữ đầu."),
        ("ch-tr",      m(r"ch|tr"),      "ch và tr dễ lẫn — mỗi từ dùng một cách viết."),
        ("s-x",        m(r"^s|^x"),      "s và x nghe gần giống nhau — ghi nhớ từng từ."),
        ("n-ng",       m(r"n$|ng$|nh$"), "Âm cuối n, ng, nh khác nhau — nghe đuôi từ."),
    ],
    "ko": [
        ("batchim",    m(r"[가-힣]"),    "받침(끝소리)이 철자를 결정해요 — 끝을 잘 들어 보세요."),
        ("ae-e",       m(r"[가-힣]"),    "ㅐ와 ㅔ는 소리가 거의 같아요 — 단어마다 외워야 해요."),
        ("ssang",      m(r"[가-힣]"),    "된소리 ㄲ ㄸ ㅃ ㅆ ㅉ는 더 세게 나는 소리예요."),
        ("yeorin",     m(r"[가-힣]"),    "ㅕ ㅛ ㅠ 같은 이중 모음을 주의하세요."),
        ("ieung",      m(r"[가-힣]"),    "첫 ㅇ은 소리가 없어요 — 자리만 지켜요."),
    ],
    "ja": [
        ("chiisai-tsu", m(r"っ"),        "小さい「っ」は音をつまらせます。"),
        ("chouon",      m(r"ー|う|お"),  "のばす音に注意 — 「おう」「ー」で長くなります。"),
        ("dakuten",     m(r"[がぎぐげござじずぜぞだぢづでどばびぶべぼ]"), "てんてん（゛）で音がにごります。"),
        ("youon",       m(r"[ゃゅょ]"),  "小さい「ゃゅょ」は前の字とくっつきます。"),
        ("ha-wa",       m(r"は|わ"),     "「は」と「わ」— 音が同じでも字がちがうことがあります。"),
    ],
    "zh": [
        ("shengdiao",  m(r"[1-5]"),      "声调数字要记牢 — 1 2 3 4 5 声调不同意思不同。"),
        ("zh-z",       m(r"zh|ch|sh|[zcs]"), "zh ch sh 是卷舌音，z c s 不卷舌 — 拼写不同。"),
        ("n-ng",       m(r"n[1-5]|ng[1-5]|n$|ng$"), "韵尾 n 和 ng 很容易混 — 听清楚结尾。"),
        ("u-v",        m(r"[uv]"),       "ü 在拼音输入里写作 v。"),
        ("qingsheng",  m(r"5"),          "轻声用 5 表示 — 又轻又短。"),
    ],
    "fil": [
        ("ng-digraph", m(r"ng"),         "Ang ng ay isang tunog — dalawang letra, isang bigkas."),
        ("o-u",        m(r"[ou]"),       "Nagpapalitan minsan ang o at u — tandaan ang baybay ng bawat salita."),
        ("d-r",        m(r"[dr]"),       "Nag-iiba minsan ang d at r sa gitna ng salita."),
        ("iw-ay",      m(r"iw|ay|oy|aw"),"Mag-ingat sa mga kambal-patinig: ay, aw, iw, oy."),
        ("mga-clusters",m(r"ts|dy|sy"),  "May mga bagong kumpol-katinig: ts, dy, sy."),
    ],
    "sw": [
        ("ng-apostrophe", m(r"ng'|ng"),  "ng' (yenye alama) na ng ni sauti mbili tofauti."),
        ("m-n-prefix",    m(r"^m|^n"),   "Viambishi m- na n- huanza maneno mengi — sikiliza mwanzo."),
        ("double-vowel",  m(r"(aa|ee|ii|oo|uu)"), "Vokali mbili hurefusha sauti: saa, juu."),
        ("dh-th",         m(r"dh|th|gh|sh"), "dh, th, gh, sh ni sauti moja kwa herufi mbili."),
        ("l-r",           m(r"[lr]"),    "l na r zinakaribiana kwa sauti — kumbuka tahajia."),
    ],
    "hi": [
        ("matra",      m(r"[ािीुूेैोौ]"), "मात्राएँ स्वर बदलती हैं — क, का, कि, की सब अलग हैं।"),
        ("retroflex",  m(r"[टठडढण]"),     "ट ठ ड ढ ज़बान मोड़कर बोलते हैं — त थ द ध से अलग।"),
        ("aspirate",   m(r"[खघछझठढथधफभ]"), "ख घ छ झ जैसे अक्षरों में हवा ज़्यादा निकलती है।"),
        ("anusvara",   m(r"[ंँ]"),         "बिंदु (ं) नाक से बोलने का निशान है।"),
        ("halant",     m(r"्"),            "हलंत (्) स्वर हटाता है — अक्षर जुड़ जाते हैं।"),
    ],
    # ru / ar trap classes already live in config/trap-registry.json.
    "ru": [
        ("unstressed-vowel-reduction", m(r"[оа]"), "Безударные о и а звучат одинаково — проверяй написание."),
        ("soft-hard-sign", m(r"[ьъ]"),  "ь смягчает согласный, ъ разделяет — сами они не звучат."),
        ("yo",         m(r"[её]"),      "ё часто пишут как е — но это разные буквы."),
        ("doubled-consonant", m(r"(.)\1"), "Двойные согласные слышно не всегда — запоминай их."),
        ("hushing-confusion", m(r"[жшчщ]"), "После ж и ш пиши и — жи-ши пиши с буквой и."),
    ],
    "ar": [
        ("hamza",      m(r"[ءأإئؤ]"),   "للهمزة مقاعد مختلفة: أ إ ئ ؤ ء — حسب الحركات حولها."),
        ("taa-marbuta",m(r"ة"),         "التاء المربوطة (ة) تأتي آخر الكلمة وتُنطق هاء عند الوقف."),
        ("alif-maqsura",m(r"ى"),        "الألف المقصورة (ى) تشبه الياء لكن بلا نقاط."),
        ("sun-moon",   m(r"^ال"),       "لام التعريف تُدغم قبل الحروف الشمسية: الشمس تُنطق أش-شمس."),
        ("long-vowels",m(r"[اوي]"),     "حروف المد ا و ي تطيل الصوت."),
    ],
}

def pick(words, matcher, used, n=1):
    out = []
    for w in words:
        if len(out) == n:
            break
        key = unicodedata.normalize("NFC", w)
        if key in used:
            continue
        if matcher(key.lower() if key.isascii() else key):
            out.append(key)
            used.add(key)
    return out

def build(lang):
    traps = CURRICULA[lang]
    b = bank(lang)
    easy = b["easy"]
    harder = b["medium"] + b["hard"]
    used = set()
    words, intro_at = [], {}
    # 4 easy words per trap class = 20, trap order = teaching order.
    for trap_id, matcher, _ in traps:
        got = pick(easy, matcher, used, 4)
        # Backfill from medium if the easy tier runs dry for this matcher.
        if len(got) < 4:
            got += pick(b["medium"], matcher, used, 4 - len(got))
        if len(got) < 4:
            got += pick(easy, lambda w: True, used, 4 - len(got))
        intro_at[trap_id] = len(words)  # index where this class first appears
        words += got
    # 5 difficult words: one per trap, from medium/hard.
    difficult = []
    for trap_id, matcher, _ in traps:
        got = pick(harder, matcher, used, 1) or pick(harder, lambda w: True, used, 1)
        difficult += got
    intros = {t: s for t, _, s in traps}
    assert len(words) == 20 and len(difficult) == 5, (lang, len(words), len(difficult))
    for t, s in intros.items():
        assert len(s) <= 90, (lang, t, len(s))
    return {
        "lang": lang,
        "$draft": "Agent-drafted per D4 (2026-07-27) — words from the language's own bank, intro strings pending the standing audit round.",
        "words": words,
        "difficult": difficult,
        "traps": [{"id": t, "firstWord": intro_at[t], "intro": s} for t, _, s in traps],
    }

def main():
    os.makedirs(OUT, exist_ok=True)
    for lang in CURRICULA:
        cur = build(lang)
        with open(os.path.join(OUT, f"{lang}.json"), "w", encoding="utf-8") as f:
            json.dump(cur, f, ensure_ascii=False, indent=2)
        print(f"  {lang}: 20+5 drafted, traps: {[t['id'] for t in cur['traps']]}")

if __name__ == "__main__":
    main()
