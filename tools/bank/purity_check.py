#!/usr/bin/env python3
"""CC-BANK-PURITY F1 — the one purity checker.  Read-only: classifies bank rows.

    python3 tools/bank/purity_check.py <lang...>|--all [--out reports/bank-purity-rows.csv]

Every row gets exactly one class from the closed set in the spec, or "pass".
Rules run in the order the spec gives (script -> structure -> dictionary ->
diacritic restore -> English / proper noun / other language -> noise -> unconfirmed).
Dictionaries are build inputs under .corpus-cache/ (never committed, never shipped).
Output is sorted, so two runs on the same inputs are byte-identical.
"""
import csv, gzip, itertools, os, re, subprocess, sys, tarfile, unicodedata, zipfile
import xml.etree.ElementTree as ET
from collections import defaultdict

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
MAIN = os.path.expanduser("~/repos/spell-wasm")            # shared corpus cache lives here
CACHES = [os.path.join(ROOT, ".corpus-cache"), os.path.join(MAIN, ".corpus-cache")]
HUNSPELL = "/opt/homebrew/bin/hunspell"
ALL = "en es fr de pt pl ko ja fil zh ru ar hi sw".split()
TIERS = "easy medium hard expert".split()

DICT = {  # registry key -> hunspell dictionary base (no extension)
    "en": "en/en_US", "es": "es/es_ES", "fr": "fr/fr", "de": "de/de_DE_frami", "pt": "pt/pt_BR",
    "pt-PT": "pt-PT/pt_PT", "pl": "pl/pl_PL", "ru": "ru/ru_RU", "sw": "sw/sw_TZ", "ko": "ko/ko_KR",
    "ar": "ar/ar", "hi": "hi/hi_IN",
}
LATIN_OTHERS = ["en", "es", "fr", "de", "pt", "pl", "sw"]   # "accepted by another registry language"

SEVERITY = {
    "wrong_language_english": "A", "wrong_language_other": "A", "misspelled_missing_diacritic": "A",
    "abbreviation_or_noise": "A", "roman_numeral": "A", "fragment": "A", "traditional_character": "A",
    "wrong_script": "A", "proper_noun": "B", "particle_attached": "B", "inflected_sentence_form": "B",
    "loanword_in_hiragana": "B", "single_kana": "B", "variety_mismatch": "B",
    "not_a_dictionary_headword": "C", "attached_conjunction_or_preposition": "C",
    "noun_shown_lowercase": "C", "linker_attached_form": "C", "not_in_reference_dictionary": "C",
}
# Classes whose Phase A action is "quarantine" (misspelled / traditional / noun_shown_lowercase / linker /
# unconfirmed are handled separately by the callers that report on them).
QUARANTINE = {
    "wrong_language_english", "wrong_language_other", "proper_noun", "abbreviation_or_noise", "roman_numeral",
    "fragment", "particle_attached", "inflected_sentence_form", "not_a_dictionary_headword",
    "loanword_in_hiragana", "single_kana", "variety_mismatch", "attached_conjunction_or_preposition", "wrong_script",
}

# ---------------------------------------------------------------- bank + caches
def find(rel):
    for c in CACHES:
        p = os.path.join(c, rel)
        if os.path.exists(p):
            return p
    sys.exit(f"missing build input {rel} (looked in {CACHES})")

def load_bank(lang):
    rows = []
    for t in TIERS:
        p = os.path.join(ROOT, "assets", "words", lang, f"{t}.txt")
        if os.path.exists(p):
            for w in open(p, encoding="utf-8").read().split("\n"):
                w = w.strip()
                if w:
                    rows.append((t, w))
    return rows

_hs_cache = {}
def hs_accepts(key, words):
    """Subset of `words` that Hunspell dictionary `key` accepts."""
    words = sorted(set(words))
    todo = [w for w in words if (key, w) not in _hs_cache]
    if todo:
        r = subprocess.run([HUNSPELL, "-i", "utf-8", "-d", find("dicts/" + DICT[key] + ".dic")[:-4], "-l"],
                           input="\n".join(todo) + "\n", capture_output=True, text=True, encoding="utf-8")
        bad = set(r.stdout.split("\n"))
        for w in todo:
            _hs_cache[(key, w)] = w not in bad
    return {w for w in words if _hs_cache[(key, w)]}

def cap(w):
    return w[:1].upper() + w[1:]

def leipzig_freq(name):
    tf = find(name + ".tar.gz")
    out = {}
    with tarfile.open(tf) as t:
        for m in t.getmembers():
            if m.name.endswith("-words.txt"):
                for line in t.extractfile(m).read().decode("utf-8").splitlines():
                    p = line.split("\t")
                    if len(p) >= 3 and p[2].isdigit():
                        out[p[1].lower()] = out.get(p[1].lower(), 0) + int(p[2])
    return out

# ---------------------------------------------------------------- noise rules
ROMAN = re.compile(r"^(?=[ivxlcdm])m{0,4}(cm|cd|d?c{0,3})(xc|xl|l?x{0,3})(ix|iv|v?i{0,3})e?s?$")
VOWELS = {
    "default": set("aeiouyáàâãäåæéèêëíìîïóòôõöøúùûüůœąęı"),
    "ru": set("аеёиоуыэюя"), "ar": set("اوي"), "hi": set(), "sw": set("aeiou"), "fil": set("aeiou"),
}
def noise_class(lang, w):
    s = w.lower()
    if lang not in ("ru", "ar", "hi") and ROMAN.match(s) and len(s) >= 2:
        return "roman_numeral"
    letters = [c for c in s if c.isalpha()]
    if not letters:
        return "abbreviation_or_noise"
    v = VOWELS.get(lang, VOWELS["default"])
    if v and not any(c in v for c in letters):
        return "abbreviation_or_noise"
    if len(letters) <= 3:
        return "abbreviation_or_noise"
    if re.fullmatch(r"(.{1,3}?)\1{2,}", s):          # repeated-syllable run (jajajaja, hehehehe)
        return "abbreviation_or_noise"
    return None

# ---------------------------------------------------------------- diacritic restore
SUBS = {
    "es": {"a": "á", "e": "é", "i": "í", "o": "ó", "u": "úü", "n": "ñ"},
    "fr": {"a": "àâ", "e": "éèêë", "i": "îï", "o": "ô", "u": "ùû", "c": "ç"},
    "de": {"a": "ä", "o": "ö", "u": "ü"},
    "pt": {"a": "áàâã", "e": "éê", "i": "í", "o": "óôõ", "u": "ú", "c": "ç"},
    "pl": {"a": "ą", "c": "ć", "e": "ę", "l": "ł", "n": "ń", "o": "ó", "s": "ś", "z": "źż"},
}
DIGRAPH = {"fr": {"oe": ["œ"], "ae": ["æ"]}, "de": {"ss": ["ß"], "ae": ["ä"], "oe": ["ö"], "ue": ["ü"]}}

def restore_candidates(lang, w, maxsub=3):
    """Yield (nsub, candidate) restorations of w with up to maxsub diacritic substitutions."""
    subs, dg = SUBS.get(lang, {}), DIGRAPH.get(lang, {})
    sites = []  # (start, length, options)
    i = 0
    while i < len(w):
        two = w[i:i + 2]
        if two in dg:
            sites.append((i, 2, dg[two]))
        if w[i] in subs:
            sites.append((i, 1, list(subs[w[i]])))
        i += 1
    seen = set()
    for k in range(1, maxsub + 1):
        for combo in itertools.combinations(sites, k):
            pos = sorted(combo, key=lambda s: s[0])
            if any(a[0] + a[1] > b[0] for a, b in zip(pos, pos[1:])):
                continue
            for picks in itertools.product(*[s[2] for s in pos]):
                out, last = [], 0
                for s, p in zip(pos, picks):
                    out += [w[last:s[0]], p]; last = s[0] + s[1]
                out.append(w[last:])
                c = "".join(out)
                if c != w and c not in seen:
                    seen.add(c); yield k, c

# ---------------------------------------------------------------- Latin / Cyrillic flow
def classify_hunspell(lang, rows):
    words = sorted({w for _, w in rows})
    res = {}
    lo = hs_accepts(lang, words)
    rest = [w for w in words if w not in lo]
    pt_pt = hs_accepts("pt-PT", rest) if lang == "pt" else set()
    caps = hs_accepts(lang, [cap(w) for w in rest])
    # diacritic restore
    cand_of = {}
    if lang in SUBS or lang in DIGRAPH:
        allc = {}
        for w in rest:
            for k, c in restore_candidates(lang, w):
                allc.setdefault(c, []).append((k, w))
        ok = hs_accepts(lang, list(allc))
        best = {}
        for c in sorted(ok):
            for k, w in allc[c]:
                if w not in best or k < best[w][0]:
                    best[w] = (k, c)
        cand_of = {w: v[1] for w, v in best.items()}
    en_lo = hs_accepts("en", rest) if lang != "en" else set()
    others = {}
    if rest:
        for o in ([] if lang == "ru" else LATIN_OTHERS):
            if o == lang:
                continue
            others[o] = (hs_accepts(o, rest), hs_accepts(o, [cap(w) for w in rest]))
    for w in words:
        if w in lo:
            res[w] = ("pass", "")
        elif lang == "pt" and w in pt_pt:
            res[w] = ("variety_mismatch", "VARIETY_PT_PT")
        elif cap(w) in caps:
            res[w] = (("noun_shown_lowercase", "") if lang == "de" else ("proper_noun", ""))
        elif w in cand_of and w not in en_lo:
            res[w] = ("misspelled_missing_diacritic", "→ " + cand_of[w])
        elif w in en_lo:
            res[w] = ("wrong_language_english", "")
        else:
            cls = None
            for o, (olo, ocap) in others.items():
                if cap(w) in ocap and w not in olo:
                    cls = ("proper_noun", ""); break
            if not cls:
                for o, (olo, ocap) in others.items():
                    if w in olo:
                        cls = ("wrong_language_other", o); break
            if not cls:
                n = noise_class(lang, w)
                cls = (n, "") if n else ("not_in_reference_dictionary", "")
            res[w] = cls
    return res

# ---------------------------------------------------------------- Korean
PARTICLES = sorted("""은 는 이 가 을 를 의 에 에서 에게 에게서 으로 로 와 과 도 만 까지 부터 으로부터 로부터 보다 처럼 에서는 에서도
에는 에도 으로는 으로도 로는 이다 이었다 였다 이라고 이라는 이라 라고 라는 들 들은 들이 들을 들의 들에게 들과 들에 이나 나
으로서 로서 으로써 로써 에서의 와의 과의 만의 에서부터 까지의 마저 조차 이라도 라도 이며 이고 이지만 인""".split(), key=len, reverse=True)
ENDINGS = sorted("""었다 았다 였다 했다 하였다 하였고 하였으며 하였으나 되었다 되었고 되었으며 하다 한다 하는 하고 하며 하면서 하면 하지만 하기도
했으며 했지만 했는데 하는데 한다고 한다는 합니다 입니다 습니다 십니다 해요 했어요 어요 아요 지만 으며 는데 면서 고 며 니다 어서 아서
은 는 을 ㄹ 던 도록 게 기 지 다 라 요 죠 네 군 까 까요 래요 거나 으니 니 나 자 세요""".split(), key=len, reverse=True)

def load_ko_stems():
    stems = set()
    for l in open(find("dicts/ko/ko_KR.dic"), encoding="utf-8").read().splitlines()[1:]:
        stems.add(unicodedata.normalize("NFC", l.split("/")[0].strip()))
    return stems

def classify_ko(rows):
    stems = load_ko_stems()
    res = {}
    for w in sorted({w for _, w in rows}):
        if not all("가" <= c <= "힣" for c in w):
            res[w] = ("wrong_script", ""); continue
        if w in stems:
            res[w] = ("pass", ""); continue
        if w + "다" in stems:
            res[w] = ("fragment", w + "다"); continue
        hit = None
        for p in PARTICLES:
            if w.endswith(p) and len(w) > len(p) and w[:-len(p)] in stems:
                hit = ("particle_attached", f"{w[:-len(p)]}+{p}"); break
        if not hit:
            for e in ENDINGS:
                if w.endswith(e) and len(w) > len(e):
                    r = w[:-len(e)]
                    if r in stems or r + "다" in stems or (r.endswith("하") and r[:-1] in stems) or (r.endswith("되") and r[:-1] in stems):
                        hit = ("inflected_sentence_form", f"{r}+{e}"); break
        res[w] = hit or ("not_a_dictionary_headword", "")
    return res

# ---------------------------------------------------------------- Japanese
SMALL = set("ぁぃぅぇぉゃゅょゎ")
SENTENCE_TAILS = ("とは", "という", "として", "ものなり", "について", "による", "なった", "ている", "でした", "ました", "は", "が", "を")
def kata(s):
    return "".join(chr(ord(c) + 0x60) if "ぁ" <= c <= "ゖ" else c for c in s)

def load_jmdict():
    rebs = set()
    with gzip.open(find("ja/JMdict_e.gz"), "rb") as f:
        for _, el in ET.iterparse(f):
            if el.tag == "reb" and el.text:
                rebs.add(el.text)
            elif el.tag == "entry":
                el.clear()
    return rebs

def classify_ja(rows):
    rebs = load_jmdict()
    res = {}
    for w in sorted({w for _, w in rows}):
        if len(w) == 1 and ("ぁ" <= w <= "ゖ"):
            res[w] = ("fragment", "") if w in SMALL else ("single_kana", ""); continue
        if w[0] in SMALL or w[-1] == "っ" or w == "ん" * len(w):
            res[w] = ("fragment", ""); continue
        if w in rebs:
            res[w] = ("pass", ""); continue
        if len(w) >= 5 and w.endswith(SENTENCE_TAILS):   # a run of Wikipedia text, not a word
            res[w] = ("fragment", "sentence tail"); continue
        if kata(w) in rebs:
            res[w] = ("loanword_in_hiragana", kata(w)); continue
        if len(w) >= 8:                                    # unknown and this long: a phrase, not a headword
            res[w] = ("fragment", "long unknown run"); continue
        res[w] = ("not_in_reference_dictionary", "")
    return res

# ---------------------------------------------------------------- Chinese
def load_cedict():
    simp, trad = set(), {}
    for l in gzip.open(find("zh/cedict.txt.gz"), "rt", encoding="utf-8"):
        if l.startswith("#"):
            continue
        p = l.split(" ", 2)
        if len(p) > 2:
            simp.add(p[1])
            if p[0] != p[1]:
                trad.setdefault(p[0], p[1])
    return simp, trad

def classify_zh(rows):
    """Word-level: a hanzi string CC-CEDICT knows only as a traditional spelling is traditional.
    (Unihan alone over-flags 著/蒙/覆, which are valid simplified characters.)"""
    simp, trad = load_cedict()
    schars = {c for w in simp for c in w}
    res = {}
    for w in sorted({w for _, w in rows}):
        py, _, han = w.partition("|")
        sylls = re.findall(r"[a-zü]+[1-5]", py)
        if not han or not re.fullmatch(r"(?:[a-züv]+[1-5])+", py) or len(sylls) != len(han):
            res[w] = ("fragment", "malformed pinyin"); continue
        if han in simp:
            res[w] = ("pass", "")
        elif han in trad:
            res[w] = ("traditional_character", trad[han])
        elif any(c not in schars for c in han):
            res[w] = ("traditional_character", "".join(c for c in han if c not in schars))
        else:
            res[w] = ("pass", "")
    return res

# ---------------------------------------------------------------- Hindi / Arabic
def classify_hi(rows):
    words = sorted({w for _, w in rows})
    ok = hs_accepts("hi", words)
    return {w: (("pass", "") if w in ok else ("not_in_reference_dictionary", "")) for w in words}

AR_PREFIX = ["وال", "بال", "كال", "فال", "لل", "ولل", "و", "ف", "ب", "ل", "ك"]
AR_MARKS = set(range(0x064B, 0x0660)) | {0x0670, 0x0640}
AR_AFFIX_TEMPLATES = {"af", "affix", "prefix", "pre", "com", "compound", "suffix", "suf"}

def ar_strip(w):
    return "".join(c for c in w if ord(c) not in AR_MARKS)

AR_PREFIX_LETTERS = {"و", "ف", "ب", "ل", "ك", "لل", "بال", "وال"}

def ar_prefix_template(t):
    """True when a Wiktionary etymology template analyses the word as <prefix letter> + <word>."""
    name, args = t.get("name"), t.get("args", {})
    vals = [str(v) for _, v in sorted(args.items(), key=lambda kv: str(kv[0]))]
    if name == "ety" and any(v.lstrip(":") in AR_AFFIX_TEMPLATES for v in vals):
        vals = [v for v in vals if v.lstrip(":") not in AR_AFFIX_TEMPLATES]
    elif name not in AR_AFFIX_TEMPLATES:
        return False
    vals = [v for v in vals if v not in ("ar", "+", "") and not v.isdigit()]
    if not vals:
        return False
    first = re.sub(r"<[^>]*>", "", ar_strip(vals[0])).replace("ـ", "").strip()
    return first in AR_PREFIX_LETTERS

def ar_wikt(words):
    """For each candidate row: 'lemma' when Wiktionary lists it as a plain lemma with no affix etymology
    (a real word whose first letter merely looks like a prefix, e.g. بذرة 'seed'), 'affixed' when Wiktionary
    itself analyses it as prefix + word (بالفعل, كهذا), else absent.  Keys are compared without vowel marks."""
    import json
    path = None
    for c in CACHES:
        if os.path.exists(os.path.join(c, "wikt/Arabic.jsonl")):
            path = os.path.join(c, "wikt/Arabic.jsonl")
    if not path:
        return {}
    want = set(words)
    out = {}
    for line in open(path, encoding="utf-8"):
        d = json.loads(line)
        k = ar_strip(d.get("word", ""))
        if k not in want:
            continue
        if any(f.get("form_of") for f in d.get("senses", [])) or d.get("pos") in ("name",):
            continue
        affixed = any(ar_prefix_template(t) for t in d.get("etymology_templates", []))
        if affixed:
            out[k] = "affixed"
        elif out.get(k) != "affixed" and d.get("pos") in ("noun", "adj", "verb"):
            out[k] = "lemma"                     # function words (adv/prep/conj/pron) keep the prefix analysis
    return out

def classify_ar(rows):
    words = sorted({w for _, w in rows})
    freq = leipzig_freq("ara_wikipedia_2021_100K")
    cands = {w: [(p, w[len(p):]) for p in AR_PREFIX if w.startswith(p) and len(w) - len(p) >= 2] for w in words}
    rem = sorted({r for v in cands.values() for _, r in v} | {"ا" + r for v in cands.values() for _, r in v})
    ok_rem = hs_accepts("ar", rem)
    ok = hs_accepts("ar", words)
    wk = ar_wikt([w for w in words if cands[w]])
    res = {}
    for w in words:
        hit = None
        if wk.get(w) == "lemma":
            cands[w] = []                        # a real lemma: the leading letter is part of the word
        for p, r in cands[w]:
            base = r if r in ok_rem else None
            if p in ("وال", "بال", "كال", "فال", "لل", "ولل"):
                if base or ("ال" + r) in ok_rem or r in ok_rem:
                    hit = p
            elif base and (freq.get(r, 0) > freq.get(w, 0) or wk.get(w) == "affixed"):
                hit = p
            if hit:
                break
        if hit:
            res[w] = ("attached_conjunction_or_preposition", "prefix " + hit)
        elif w in ok:
            res[w] = ("pass", "")
        else:
            res[w] = ("not_in_reference_dictionary", "")
    return res

# ---------------------------------------------------------------- Filipino
def load_kaikki():
    import json
    lower, names = set(), set()
    for l in open(find("fil/kaikki-tagalog.jsonl"), encoding="utf-8"):
        d = json.loads(l)
        w = d.get("word", "")
        if not w or " " in w:
            continue
        (names if d.get("pos") == "name" else lower).add(w.lower())
        for f in d.get("forms", []):
            fw = f.get("form", "")
            if fw and " " not in fw and d.get("pos") != "name":
                lower.add(fw.lower())
    return lower, names

def classify_fil(rows):
    words = sorted({w for _, w in rows})
    heads, names = load_kaikki()
    tgl, eng = leipzig_freq("tgl_wikipedia_2021_100K"), leipzig_freq("eng_wikipedia_2016_100K")
    tn, en_ = sum(tgl.values()) or 1, sum(eng.values()) or 1
    en_lo = hs_accepts("en", words)
    en_cap = hs_accepts("en", [cap(w) for w in words])
    others = {o: hs_accepts(o, words) for o in ("es", "fr", "de", "pt", "pl")}
    res = {}
    for w in words:
        if w in heads:
            res[w] = ("pass", ""); continue
        ratio_tgl = tgl.get(w, 0) / tn
        ratio_en = eng.get(w, 0) / en_
        if w in en_lo and not ratio_tgl > 5 * ratio_en:
            res[w] = ("wrong_language_english", ""); continue
        if w in names or (cap(w) in en_cap and w not in en_lo):
            res[w] = ("proper_noun", ""); continue
        if w.endswith("ng") and len(w) > 4 and (w[:-2] in heads or w[:-1] in heads or w[:-3] in heads or w[:-2] + "a" in heads):
            res[w] = ("linker_attached_form", "-ng linker attached"); continue
        oth = [o for o, s in others.items() if w in s]
        if oth and w not in heads:
            res[w] = ("wrong_language_other", oth[0]); continue
        n = noise_class("fil", w)
        res[w] = (n, "") if n else ("not_in_reference_dictionary", "")
    return res

# ---------------------------------------------------------------- driver
def classify(lang, rows):
    return {
        "ko": lambda: classify_ko(rows), "ja": lambda: classify_ja(rows), "zh": lambda: classify_zh(rows),
        "hi": lambda: classify_hi(rows), "ar": lambda: classify_ar(rows), "fil": lambda: classify_fil(rows),
    }.get(lang, lambda: classify_hunspell(lang, rows))()

def overrides(lang):
    p = os.path.join(ROOT, "tools", "bank", "purity_overrides.csv")
    out = {}
    if os.path.exists(p):
        for r in csv.DictReader(open(p, encoding="utf-8")):
            if r["lang"] == lang and r["class"] in QUARANTINE and r["signed_by"]:
                out[r["word"]] = (r["class"], "signed override: " + r["signed_by"])
    return out

def main(argv):
    out = os.path.join(ROOT, "reports", "bank-purity-rows.csv")
    args = [a for a in argv if not a.startswith("--out")]
    if "--out" in argv:
        out = argv[argv.index("--out") + 1]; args = [a for a in args if a != out]
    langs = ALL if "--all" in args else [a for a in args if not a.startswith("--")]
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from census_c6 import fold_lenient, load  # profanity column, same fold as src/norm.rs
    out_rows = []
    for lang in langs:
        rows = load_bank(lang)
        res = classify(lang, rows)
        for w, (c, e) in overrides(lang).items():   # signed C1 rulings; only ever move a row into a quarantine class
            if w in res and res[w][0] not in QUARANTINE:
                res[w] = (c, e)
        prof = load(f"{ROOT}/assets/words/profanity/{lang}.txt") or set()
        for t, w in rows:
            cls, ev = res[w]
            p = "yes" if fold_lenient(w.rsplit("|", 1)[-1]) in prof else ""
            if cls != "pass" or p:
                out_rows.append((lang, t, w, "" if cls == "pass" else cls, "" if cls == "pass" else SEVERITY.get(cls, ""), p, ev))
        print(f"{lang}: {len(rows)} rows", file=sys.stderr)
    out_rows.sort(key=lambda r: (ALL.index(r[0]), TIERS.index(r[1]), r[2]))
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", newline="", encoding="utf-8") as f:
        cw = csv.writer(f, lineterminator="\n")
        cw.writerow("lang,tier,word,class,severity,on_repo_profanity_list,evidence".split(","))
        cw.writerows(out_rows)

if __name__ == "__main__":
    main(sys.argv[1:])
