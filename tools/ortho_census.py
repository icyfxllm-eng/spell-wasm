#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL v1 section 0 census (read-only over the banks).

Reads assets/words/<lang>/<tier>.txt for the eleven languages, runs the proposed normalize chains (C3),
the citation-form heuristics (C4), the script/character scans (C5) and the F-checks (C7), and writes
reports/ortho-census/*.csv plus reports/ortho-census/summary.json. Changes no bank, no profile, no grader.
Heuristics are named in the report; none of them is a verdict.
"""
import csv, json, os, re, sys, unicodedata
from collections import defaultdict, Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "reports", "ortho-census")
LANGS = "es fr pt pl ru vi ko fil zh sw hi".split()
TIERS = "easy medium hard expert".split()

def nfc(s): return unicodedata.normalize("NFC", s)
def load(lang):
    rows = []
    for t in TIERS:
        p = os.path.join(ROOT, "assets/words", lang, t + ".txt")
        if not os.path.exists(p): continue
        for i, line in enumerate(open(p, encoding="utf-8").read().split("\n"), 1):
            if line.strip() == "": continue
            rows.append({"id": f"{lang}/{t}/{i}", "tier": t, "raw": line, "ans": line.split("|")[0] if lang == "zh" else line,
                         "disp": line.split("|")[1] if lang == "zh" and "|" in line else None})
    return rows

# ---- normalize chains (profile vocabulary) ----
TONE_MARKS = "̣̀́̃̉"
def n_vi_tone(s):
    out = []
    for tok in unicodedata.normalize("NFD", s).split(" "):
        marks = [c for c in tok if c in TONE_MARKS]
        base = "".join(c for c in tok if c not in TONE_MARKS)
        out.append(unicodedata.normalize("NFC", base + "".join(marks)))
    return " ".join(out)
def fold_apos(s): return "".join("'" if c in "’ʼ‘" else c for c in s)
def collapse(s): return re.sub(r"\s+", " ", s).strip()
CHAIN = {
    "es": lambda s: collapse(nfc(s).casefold()),
    "fr": lambda s: collapse(nfc(fold_apos(s)).casefold()),
    "pt": lambda s: collapse(nfc(s).casefold()),
    "pl": lambda s: collapse(nfc(s).casefold()),
    # CC-ORTHO-SWEEP-ALL S3: the yo fold is one-way and acts on the TYPED side only (typed е matches stored ё), so the
    # stored-side chain used for I1 keeps ё and е apart. `RU_YO` below reports the rows a two-way fold would merge.
    "ru": lambda s: collapse(nfc(s).casefold()),
    "vi": lambda s: n_vi_tone(collapse(nfc(s).casefold())),
    "ko": lambda s: collapse(nfc(s)),
    "fil": lambda s: collapse(nfc(fold_apos(s)).casefold()),
    "zh": lambda s: collapse(nfc(s)).casefold(),   # characters: script_convert not run (no OpenCC here); pinyin: see PY
    "sw": lambda s: collapse(nfc(fold_apos(s)).casefold()),
    "hi": lambda s: collapse(nfc(s).replace("‌", "").replace("‍", "")),
}
def PY(s):  # fold_pinyin: tone marks -> digits, v/u: -> ü, drop separators, casefold
    s = unicodedata.normalize("NFD", s.casefold())
    toneof = {"̄": "1", "́": "2", "̌": "3", "̀": "4"}
    out, tone = [], ""
    for c in s:
        if c in toneof: tone = toneof[c]; continue
        if c == "̈": out.append("ü"); continue
        out.append(c)
    t = "".join(out).replace("u:", "ü").replace("v", "ü")
    t = re.sub(r"[\s'\-]", "", t)
    return unicodedata.normalize("NFC", t) + tone

# ---- script allow-lists (C5) ----
def letters_ok(lang, ch):
    n = unicodedata.name(ch, "")
    if lang in ("es", "fr", "pt", "pl", "fil", "sw", "vi", "zh"):
        if lang == "zh": return ch.isascii() and (ch.isalpha() or ch in "12345") or ch in "ü" or ch == "|"
        extra = {"es": "áéíóúüñ", "fr": "àâæçéèêëîïôœùûüÿ", "pt": "áàâãçéêíóôõúü", "pl": "ąćęłńóśźż",
                 "fil": "ñ", "sw": "", "vi": "đ"}[lang]
        if ch.casefold() in extra or (ch.isascii() and ch.isalpha()): return True
        if lang in ("fil",) and ch.casefold() in "áàâéèêíìîóòôúùû": return True
        if lang == "vi":
            b = unicodedata.normalize("NFD", ch)[0]
            return b.isascii() and b.isalpha()
        return False
    if lang == "ru": return "CYRILLIC" in n
    if lang == "ko": return "HANGUL" in n
    if lang == "hi": return "DEVANAGARI" in n
    return False
def odd_chars(lang, w):
    found = set()
    for ch in w:
        if ch in " '-’ʼ‘‌‍": continue
        if lang == "zh" and (ch.isdigit()): continue
        if not letters_ok(lang, ch): found.add(ch)
    return found
def special(w):
    s = set()
    if "'" in w or "’" in w or "ʼ" in w or "‘" in w: s.add("apostrophe")
    if "-" in w: s.add("hyphen")
    if " " in w: s.add("space")
    if "‍" in w: s.add("ZWJ")
    if "‌" in w: s.add("ZWNJ")
    return s

# ---- citation-form heuristics (C4) ----
def derive(bank, w, sufs, adds, minbase=3):
    """w looks derived from another row: strip a suffix, add a lemma ending, land on a different row."""
    for s in sufs:
        if w.endswith(s) and len(w) - len(s) >= minbase:
            base = w[: len(w) - len(s)]
            for a in adds:
                c = base + a
                if c != w and c in bank: return c
    return None
ES_SUF = ["os","as","es","s","ó","aron","ieron","aba","aban","ía","ían","amos","emos","imos","an","en","ando","iendo","ado","ada","ados","adas","ido","ida","idos","idas","o","a","é","í","ió","e","ará","erá","irá"]
ES_ADD = ["","o","a","ar","er","ir","e"]
FR_SUF = ["s","x","es","ée","ées","és","é","ent","ons","ez","ait","aient","ant","e","i","is","it","u","ue","aux"]
FR_ADD = ["","er","ir","re","e","al"]
PT_SUF = ["s","es","ões","ais","os","as","ou","aram","ava","avam","ando","ado","ada","ados","adas","am","emos","amos","eu","eram","endo","ido","ida","idos","idas","iu","iram","indo","o","a","e","em","ei"]
PT_ADD = ["","o","a","ar","er","ir","ão","al","e"]
PL_SUF = ["ów","om","ami","ach","ie","ę","ą","y","i","owi","em","iem","u","ego","emu","ym","ej","ych","ymi","ą","ły","ła","ło","li","ał","ała","ali","ają","ują","uje","ują","ym"]
PL_ADD = ["","a","o","e","y","i","ć","ek","ka"]
RU_SUF = ["а","у","ы","е","ом","ой","ов","ам","ами","ах","ого","ему","ым","ому","ая","ое","ые","ых","ую","ей","ем","ет","ют","ут","ит","ят","ал","ала","али","ало","ся","сь","и","ю","я","ь"]
RU_ADD = ["","а","я","ь","й","ть","ый","ой","ий","о","е","ся","ться"]
KO_SUF = ["은","는","을","를","이","가","에","에서","으로","로","와","과","도","만","의","한","된","인","할","될","하게","히","게","서","고","면","며","했다","하는","하여"]
KO_ADD = ["","다","하다","되다"]
FIL_PRE = ["mag","nag","pag","pinag","ipag","ipa","ma","na","ka","pa","i","um","in","kina","nang"]
HI_SUF = ["ता","ती","ते","ों","ाओं","ियाँ","ियां","ीं","ें","े","ी","ा"]
HI_ADD = ["ना","","ा"]
def cit_flag(lang, bank, w):
    lw = w.casefold() if lang not in ("ko", "hi", "vi", "zh") else w
    if lang == "es": return derive(bank, lw, ES_SUF, ES_ADD)
    if lang == "fr": return derive(bank, lw, FR_SUF, FR_ADD)
    if lang == "pt": return derive(bank, lw, PT_SUF, PT_ADD)
    if lang == "pl": return derive(bank, lw, PL_SUF, PL_ADD)
    if lang == "ru": return derive(bank, lw, RU_SUF, RU_ADD)
    if lang == "ko": return derive(bank, w, KO_SUF, KO_ADD, 1)
    if lang == "hi": return derive(bank, w, HI_SUF, HI_ADD, 2)
    if lang == "fil":
        for p in FIL_PRE:
            if lw.startswith(p) and len(lw) - len(p) >= 3 and lw[len(p):] in bank: return lw[len(p):]
        if len(lw) >= 4 and lw[:2] == lw[2:4] or (len(lw) >= 5 and lw[0] == lw[1] and lw[1:] in bank): return "reduplication"
        return None
    if lang == "sw":
        if re.match(r"^(ni|u|a|tu|m|wa)(na|me|li|ta|ku)[a-z']{3,}$", lw): return "tensed-verb-pattern"
        if lw.startswith("-"): return "bare-stem"
        return None
    return None

def write_csv(name, rows):
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, name), "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["row_id", "lang", "tier", "stored_answer", "check_id", "finding", "proposed_action"])
        w.writerows(rows)

def sample(lst, n=20):
    if len(lst) <= n: return list(lst)
    step = len(lst) / n
    return [lst[int(i * step)] for i in range(n)]

# ---- language-specific F-check data ----
ES_PRE2010 = "sólo éste ésta ése ésa aquél aquélla guión truhán fié fié dió vió fue guiais".split()
FR_PAIRS = [("oignon","ognon"),("nénuphar","nénufar"),("événement","évènement"),("coût","cout"),("paraître","paraitre"),
    ("maître","maitre"),("août","aout"),("ambiguë","ambigüe"),("week-end","weekend"),("porte-monnaie","portemonnaie"),("chariot","charriot"),
    ("connaître","connaitre"),("île","ile"),("traîner","trainer"),("entraîner","entrainer"),("plaît","plait"),("coûter","couter"),
    ("sûr","sur"),("allègement","allégement"),("règlementaire","réglementaire"),("évènement","événement")]
PT_AO90 = "idéia vôo pára lingüiça acção óptimo direcção anti-semita ação contacto facto".split()  # only the clearly pre-AO90 ones flagged below
PT_OLD = ["idéia","vôo","pára","lingüiça","acção","óptimo","direcção","ecónomo","pêra","côco","têem","vêem","crêem","dêem","lêem","sequência","freqüência","eqüino","tranqüilo","argüir","pólo","enjôo","abençôo","perdôo","voô","vôos","anti-semita","co-herdeiro","auto-estima","infra-estrutura"]
PT_EU = ["António","génio","fenómeno","cómodo","bebé","húmido","facto","receção","registo","equipa","facto","contacto","egípcio","secção","projecto","director","actual","objectivo","recepcionista","gestor","telemóvel","autocarro","comboio","pequeno-almoço","casa-de-banho","ecrã","rapariga","miúdo","fato","optimo","óptimo","conceção","deceção","perceção","aceitar","espetáculo","espectáculo","excepção","exceção","corrupto","baptismo","batismo","aspeto","aspecto","ótimo","tecto","teto","raptar","setembro"]
VI_IY = [("lí","lý"),("kĩ","kỹ"),("mĩ","mỹ"),("sĩ","sỹ"),("kí","ký"),("hí","hý"),("tí","tý"),("lí do","lý do"),("công ti","công ty")]
FIL_DR = ["rito","raw","rin","rine","ri","roon","rito","niyan"]
FIL_CONTR = ["kwento","bwan","bwat","pwede","kwarto","dyan","nyan","kwan","swerte","hwag","bwaya","lalake","babai","lalaki","babae"]
FIL_ENG = re.compile(r"(tion|ck|ph|th|ee|oo|sh|ough|ght|ing$|er$|ed$)")
SW_ENG = re.compile(r"(tion|ck|ph|th|ee|oo|sh$|ing$|ough|ght)")
HI_LOAN = re.compile(r"[ॉॅ]")
def pinyin_syllables(s): return len(re.findall(r"[a-zü]+[0-5]", s)) or 1
TRAD_ONLY = set("學習國會說這後當與開時來對點發無實過電長問話見種體說門關從車東頭機經義處應歲級報導麼為還邊離蘭氣萬億總鐵數樂響員決條臺灣個們們讀寫書師視覺親認區難雖須驗點龍馬鳥魚鳳")  # sample of Traditional-only chars

def main():
    summary = {}
    os.makedirs(OUT, exist_ok=True)
    for lang in LANGS:
        rows = load(lang)
        S = {"rows": len(rows), "by_tier": dict(Counter(r["tier"] for r in rows))}
        bank = set(r["ans"] for r in rows) if lang != "zh" else set(r["ans"] for r in rows)
        bankcf = set(r["ans"].casefold() for r in rows)
        # C3 collisions
        groups = defaultdict(list)
        for r in rows:
            # zh I1 carve-out (Eric, 2026-11-05): rows with the same toned pinyin and different hanzi are homophones,
            # exempt; identical hanzi is a collision. So the I1 key for zh is the hanzi side.
            key = (r["disp"] if lang == "zh" else CHAIN[lang](r["ans"]))
            groups[key].append(r)
        coll = [g for g in groups.values() if len(g) > 1]
        S["collision_groups"] = len(coll); S["collision_rows"] = sum(len(g) for g in coll)
        S["collision_pct"] = round(100 * S["collision_rows"] / max(1, len(rows)), 2)
        with open(os.path.join(OUT, f"c3-collisions-{lang}.tsv"), "w", encoding="utf-8") as f:
            f.write("normalized\trow_ids\tstored\ttiers\n")
            for g in sorted(coll, key=lambda g: g[0]["ans"]):
                f.write("\t".join([CHAIN[lang](g[0]["ans"]) if lang != "zh" else g[0]["disp"],
                                   " ".join(x["id"] for x in g), " | ".join(x["raw"] for x in g), ",".join(x["tier"] for x in g)]) + "\n")
        if lang == "ru":
            yo = defaultdict(list)
            for r in rows: yo[r["ans"].casefold().replace("ё", "е")].append(r)
            S["ru_yo_pairs_both_kept"] = sum(1 for g in yo.values() if len(g) > 1)
        if lang == "zh":
            pk = defaultdict(list)
            for r in rows: pk[PY(r["ans"])].append(r)
            S["zh_pinyin_homophone_groups_exempt"] = sum(1 for g in pk.values() if len({x["disp"] for x in g}) > 1)
            hz = defaultdict(list)
            for r in rows: hz[r["disp"]].append(r)
            S["zh_hanzi_collision_groups"] = sum(1 for g in hz.values() if len(g) > 1)
        # C4 citation heuristics
        derived, patt = [], []
        for r in rows:
            if lang == "zh" or lang == "vi": break
            c = cit_flag(lang, bankcf if lang not in ("ko", "hi") else bank, r["ans"])
            if c: derived.append((r, c))
        if lang == "ru":
            for r in rows:
                if r["ans"].endswith("тся"): patt.append(r)
        if lang == "ko":
            for r in rows:
                if re.search(r"(는|은|던|운|할|될|한|된)$", r["ans"]) and not r["ans"].endswith("다"): patt.append(r)
        if lang == "hi":
            inf = sum(1 for r in rows if r["ans"].endswith("ना"))
            S["hi_infinitive_rows"] = inf
        if lang == "sw":
            S["sw_ku_infinitives"] = sum(1 for r in rows if r["ans"].startswith("ku"))
        S["noncitation_derived_from_other_row"] = len(derived)
        S["noncitation_pattern_only"] = len(patt)
        S["noncitation_pct"] = round(100 * (len(derived) + len(patt)) / max(1, len(rows)), 1)
        S["noncitation_samples"] = [f'{r["raw"]} <- {c}' for r, c in sample(derived)] + [r["raw"] + " (pattern)" for r in sample(patt, 5)]
        # C5 chars
        odd = [(r, odd_chars(lang, r["ans"])) for r in rows]
        odd = [(r, o) for r, o in odd if o]
        sp = Counter()
        sprows = defaultdict(list)
        for r in rows:
            for k in special(r["ans"]): sp[k] += 1; sprows[k].append(r)
        S["outside_allowlist_rows"] = len(odd)
        S["outside_allowlist_chars"] = dict(Counter(c for _, o in odd for c in o).most_common(15))
        S["special"] = dict(sp)
        S["outside_samples"] = [f'{r["raw"]} {sorted(o)}' for r, o in sample(odd, 10)]
        # NFC / NFD
        notnfc = [r for r in rows if r["ans"] != nfc(r["ans"])]
        S["not_nfc"] = len(notnfc)
        checks = []   # (row, check_id, finding, action)
        def add(r, cid, finding, action): checks.append((r["id"], lang, r["tier"], r["raw"], cid, finding, action))
        for r, c in derived:
            add(r, f"F-{lang}-cit", f"looks derived from row '{c}' (heuristic)", "quarantine via PURITY ledger if approved")
        for r in patt: add(r, f"F-{lang}-cit", "inflection pattern, lemma not in bank (heuristic)", "review")
        for r in notnfc: add(r, f"F-{lang}-nfc", "stored form is not NFC", "normalize at build")
        for r, o in odd: add(r, f"F-{lang}-script", f"characters outside allow-list {sorted(o)}", "review")
        for g in coll:
            for r in g: add(r, f"F-{lang}-coll", "normalized form coincides with " + ", ".join(x["id"] for x in g if x is not r), "review")
        # language-specific
        L = lang
        if L == "es":
            for r in rows:
                if r["ans"].casefold() in ES_PRE2010: add(r, "F-es-1", "pre-2010 tilde form stored (only form)", "store RAE2010 form, accept the other")
                if re.search(r"[¿¡]", r["ans"]): add(r, "F-es-3", "contains ¿ or ¡", "quarantine")
        if L == "fr":
            for r in rows:
                if re.match(r"^(l|d|j|n|s|c|qu|m|t)['’]", r["ans"].casefold()) or "'" in r["ans"]: add(r, "F-fr-2", "elided or apostrophe form", "quarantine")
                for a, b in FR_PAIRS:
                    if r["ans"].casefold() in (a, b):
                        add(r, "F-fr-3", f"1990 pair {a}/{b}: stored '{r['ans']}'; other form {'present' if (b if r['ans'].casefold()==a else a) in bankcf else 'absent'}", "add rect1990 variant row")
        if L == "pt":
            for r in rows:
                w = r["ans"].casefold()
                if w in PT_OLD or re.search(r"(güe|güi|qüe|qüi|[ée]ia|[ói]ia|ôo\b|êem\b)", w) and w in PT_OLD: add(r, "F-pt-1", "pre-AO90 form stored", "replace with AO90 form, keep old as close")
                if re.search(r"(güe|güi|qüe|qüi)", w): add(r, "F-pt-1", "trema (pre-AO90 pattern)", "review")
                if w in set(x.casefold() for x in PT_EU): add(r, "F-pt-2", "form on the PT-PT/old-spelling watch list (stored must be BR)", "review: store BR, accept EU")
        if L == "pl":
            strip = lambda s: "".join(c for c in unicodedata.normalize("NFD", s) if not unicodedata.combining(c)).replace("ł", "l")
            by = defaultdict(list)
            for r in rows: by[strip(r["ans"].casefold())].append(r)
            for k, g in by.items():
                if len(g) > 1:
                    for r in g: add(r, "F-pl-2", "diacritic-stripped form coincides with " + ", ".join(x["raw"] for x in g if x is not r), "los/łoś rule: stripped typed = miss")
            for r in rows:
                if r["ans"] == r["ans"].casefold() and r["ans"].isascii() and re.search(r"zolw|zab|zaba|lodz|gora$", r["ans"]): add(r, "F-pl-2", "ASCII form of a diacritic word", "review")
        if L == "ru":
            for r in rows:
                if re.search(r"[A-Za-z]", r["ans"]): add(r, "F-ru-3", "Latin letters in row", "quarantine")
            S["ru_rows_with_yo"] = sum(1 for r in rows if "ё" in r["ans"])
            S["ru_f1"] = "could not run: no ё-headword dictionary in the repo; src/ru_stress_data.rs holds 973 stress entries only"
        if L == "vi":
            for r in rows:
                for a, b in VI_IY:
                    if r["ans"] in (a, b) and (b if r["ans"] == a else a) in bank: add(r, "F-vi-3", f"both i and y forms stored: {a}/{b}", "one row plus iy variant")
            tp = defaultdict(list)
            for r in rows: tp[n_vi_tone(collapse(nfc(r["ans"]).casefold()))].append(r)
            for g in tp.values():
                if len(g) > 1:
                    for r in g: add(r, "F-vi-2", "tone-placement duplicate of " + ", ".join(x["raw"] for x in g if x is not r), "keep one row")
            S["vi_multisyllable"] = sum(1 for r in rows if " " in r["ans"])
        if L == "ko":
            for r in rows:
                if " " in r["ans"]: add(r, "F-ko-2", "contains a space (headwords-only)", "report")
                if re.search(r"[一-鿿]|[A-Za-z]", r["ans"]): add(r, "F-ko-4", "Hanja or Latin", "quarantine")
            S["ko_ends_in_da"] = sum(1 for r in rows if r["ans"].endswith("다"))
            DUAL = ["자장면","짜장면","먹을거리","먹거리","남우세스럽다","남사스럽다","택견","태껸","품새","품세","눈초리","눈꼬리","복사뼈","복숭아뼈","만날","맨날","손자","손주","삐치다","삐지다","예쁘다","이쁘다","차지다","찰지다","마을","마실"]
            S["ko_dual_standard_in_bank"] = [w for w in DUAL if w in bank]
        if L == "fil":
            for r in rows:
                w = r["ans"].casefold()
                if re.search(r"[áàâéèêíìîóòôúùûÁÀÂÉÈÊÍÌÎÓÒÔÚÙÛ]", r["ans"]): add(r, "F-fil-2", "tuldik mark stored", "strip mark")
                if w in FIL_DR or w in FIL_CONTR: add(r, "F-fil-3", "r-/contracted/legacy form stored", "store KWF form, accept the other")
                if FIL_ENG.search(w) and w not in ("ngayon",): add(r, "F-fil-1", "English-looking spelling (heuristic)", "check against Filipinized-loan rule")
            S["fil_hyphen_rows"] = len(sprows.get("hyphen", []))
        if L == "zh":
            mono = [r for r in rows if pinyin_syllables(r["ans"]) == 1]
            S["zh_monosyllabic_rows"] = len(mono)
            key = Counter(PY(r["ans"]) for r in rows)
            for r in mono: add(r, "F-zh-1", f"monosyllable; pinyin key shared with {key[PY(r['ans'])]-1} other bank rows", "D7: keep only if unique")
            for r in rows:
                if r["disp"] is None: add(r, "F-zh-3", "no hanzi side", "report")
                elif re.search(r"[A-Za-z0-9\s\.,!?，。！？、；：]", r["disp"]): add(r, "F-zh-3", "Latin/punctuation in hanzi side", "quarantine")
                if r["disp"] and any(c in TRAD_ONLY for c in r["disp"]): add(r, "F-zh-2", "Traditional-only character in a Simplified bank", "review")
            set_sizes = defaultdict(Counter)
            for g in pk.values(): set_sizes["all"][len(g)] += 1
            S["zh_pinyin_key_set_sizes"] = dict(set_sizes["all"])
            S["zh_pinyin_key_sets_by_tier"] = {t: sum(1 for g in pk.values() if len(g) > 1 and any(x["tier"] == t for x in g)) for t in TIERS}
            S["zh_tone3_sandhi_applied"] = False
        if L == "sw":
            for r in rows:
                if r["ans"].startswith("-"): add(r, "F-sw-1", "bare stem", "quarantine or store ku- form")
                if "’" in r["ans"] or "‘" in r["ans"] or "ʼ" in r["ans"]: add(r, "F-sw-2", "curly apostrophe stored", "store U+0027")
                if SW_ENG.search(r["ans"].casefold()): add(r, "F-sw-3", "English-looking spelling (heuristic)", "check")
            S["sw_ng_apostrophe_rows"] = sum(1 for r in rows if "ng'" in r["ans"].casefold())
            S["sw_ngn_without_apostrophe_candidates"] = sum(1 for r in rows if re.match(r"^ng[aeiou]", r["ans"].casefold()) and ("'" not in r["ans"]))
        if L == "hi":
            nuk = defaultdict(list)
            for r in rows:
                if "‌" in r["ans"] or "‍" in r["ans"]: add(r, "F-hi-1", "contains ZWJ/ZWNJ", "strip at build")
                if HI_LOAN.search(r["ans"]): add(r, "F-hi-3", "ॉ/ॅ marks an English loan in Devanagari (heuristic)", "quarantine")
                nuk[r["ans"].replace("़", "").replace("ड़", "ड").replace("ढ़", "ढ")].append(r)
            for g in nuk.values():
                if len({x["ans"] for x in g}) > 1:
                    for r in g: add(r, "F-hi-4", "nukta/no-nukta pair stored: " + ", ".join(x["raw"] for x in g), "one row plus variant")
        # write per-check CSVs
        by_check = defaultdict(list)
        for c in checks: by_check[c[4]].append(c)
        S["checks"] = {k: len(v) for k, v in sorted(by_check.items())}
        S["check_pct_of_rows"] = {k: round(100 * len(v) / max(1, len(rows)), 1) for k, v in sorted(by_check.items())}
        write_csv(f"c7-{lang}.csv", [c for k in sorted(by_check) for c in by_check[k]])
        summary[lang] = S
    json.dump(summary, open(os.path.join(OUT, "summary.json"), "w"), ensure_ascii=False, indent=1)
    print("ok")
main()
