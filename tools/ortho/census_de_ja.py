#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-DE-JA v1 §0 + v1.1 C7-C12 census.  READ-ONLY: touches no bank row, registry entry or app code.

    python3 tools/ortho/census_de_ja.py

Writes reports/ortho-census/*.csv (one per check) and reports/ortho-census/_summary.json.
Dictionary authorities: Hunspell de_DE_frami (de), JMdict_e (ja), .corpus-cache/defs/de.jsonl (de POS, partial).
"""
import csv, gzip, importlib.util, json, os, re, sys, unicodedata
import xml.etree.ElementTree as ET
from collections import defaultdict

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "bank"))
spec = importlib.util.spec_from_file_location("pc", os.path.join(ROOT, "tools", "bank", "purity_check.py"))
pc = importlib.util.module_from_spec(spec); spec.loader.exec_module(pc)
OUT = os.path.join(ROOT, "reports", "ortho-census")
os.makedirs(OUT, exist_ok=True)
HDR = "row_id lang tier stored_answer check_id finding proposed_action".split()
findings = defaultdict(list)       # check_id -> rows
S = {}                             # summary

def flag(check, lang, tier, word, finding, action):
    findings[check].append([f"{lang}:{word}", lang, tier, word, check, finding, action])

de_rows = pc.load_bank("de"); ja_rows = pc.load_bank("ja")
S["rows"] = {"de": len(de_rows), "ja": len(ja_rows)}
S["tiers"] = {l: {t: sum(1 for x, _ in r if x == t) for t in pc.TIERS} for l, r in (("de", de_rows), ("ja", ja_rows))}
de_words = sorted({w for _, w in de_rows}); ja_words = sorted({w for _, w in ja_rows})
de_tier = {}; [de_tier.setdefault(w, t) for t, w in de_rows]
ja_tier = {}; [ja_tier.setdefault(w, t) for t, w in ja_rows]
acc = lambda ws: pc.hs_accepts("de", ws)

# ============================================================ F1
seed = [l.strip() for l in open(os.path.join(OUT, "struck-variants.seed.txt"), encoding="utf-8") if l.strip() and not l.startswith("#")]
sl = {s.lower(): s for s in seed}
for w in de_words:
    if w.lower() in sl:
        flag("F1c_struck", "de", de_tier[w], w, f"in struck seed list ({sl[w.lower()]})", "quarantine; current form not supplied by seed")
    if "ß" in w:
        ss = w.replace("ß", "ss")
        pass
cand_ss = [w for w in de_words if "ß" in w]
ok_w = acc(cand_ss); ok_ss = acc([w.replace("ß", "ss") for w in cand_ss])
for w in cand_ss:
    if w not in ok_w and w.replace("ß", "ss") in ok_ss:
        flag("F1a_pre_reform_ss", "de", de_tier[w], w, f"dictionary rejects it, accepts {w.replace('ß','ss')}", "quarantine")
    if "daß" in w.lower():
        flag("F1a_pre_reform_ss", "de", de_tier[w], w, "contains daß", "quarantine")
rej = [w for w in de_words if w not in acc(de_words)]
for w in rej:
    for m in re.finditer(r"([^aeiouyäöü\W\d])\1(?=[aeiouyäöü])", w):
        c = m.group(1); t = w[:m.start()] + c + w[m.start():]
        if t in acc([t]):
            flag("F1b_triple_consonant", "de", de_tier[w], w, f"dictionary rejects it, accepts {t}", "quarantine")
            break
S["de_hunspell_rejected"] = len(rej)

# ============================================================ F2
DIG = [("ae", "ä"), ("oe", "ö"), ("ue", "ü")]
def digraph_variants(w):
    sites = [(m.start(), dg) for dg, u in DIG for m in re.finditer(dg, w.lower())]
    out = []
    for i, dg in sites:
        u = dict(DIG)[dg]
        u = u.upper() if (w[i].isupper()) else u
        out.append(w[:i] + u + w[i + 2:])
    return out
okset = set(acc(de_words))
for w in de_words:
    if w in okset: continue
    for v in digraph_variants(w):
        if v in acc([v]):
            flag("F2a_umlaut_digraph", "de", de_tier[w], w, f"dictionary rejects it, accepts {v}", "quarantine; stored form -> " + v); break
    if "ss" in w:
        for m in re.finditer("ss", w):
            v = w[:m.start()] + "ß" + w[m.start() + 2:]
            if v in acc([v]):
                flag("F2b_ss_vs_eszett", "de", de_tier[w], w, f"dictionary rejects it, accepts {v}", "quarantine"); break
# noun case: Hunspell proxy (no POS tag in the .dic)
low_only = [w for w in de_words if w[0].islower()]
capd = [w[0].upper() + w[1:] for w in low_only]
okl, okc = acc(low_only), acc(capd)
for w, c in zip(low_only, capd):
    if w not in okl and c in okc:
        flag("F2c_noun_lowercase_proxy", "de", de_tier[w], w, f"dictionary accepts {c} only", "quarantine or recapitalise")
defs = {}
for l in open(os.path.expanduser("~/repos/spell-wasm/.corpus-cache/defs/de.jsonl"), encoding="utf-8"):
    d = json.loads(l); defs.setdefault(d["word"], d)
cov = [w for w in de_words if w in defs and defs[w].get("pos")]
S["de_pos_coverage"] = f"{len(cov)}/{len(de_words)}"
for w in cov:
    pos = defs[w]["pos"]
    if pos in ("noun", "proper noun") and w[0].islower():
        flag("F2d_pos_case", "de", de_tier[w], w, "POS noun, stored lowercase", "quarantine or recapitalise")
    if pos not in ("noun", "proper noun") and w[0].isupper() and not defs[w].get("form_of"):
        flag("F2d_pos_case", "de", de_tier[w], w, f"POS {pos}, stored capitalised", "recapitalise (review)")

# ============================================================ F3 candidates (no Regelwerk list available)
RULES3 = [("ph", "f"), ("f", "ph"), ("gh", "g"), ("g", "gh"), ("th", "t"), ("rh", "r"), ("ee", "é")]
lw = {w.lower(): w for w in de_words}
pairs3 = set()
for w in de_words:
    base = w
    for a, b in [("ph", "f"), ("f", "ph"), ("gh", "g"), ("th", "t"), ("rh", "r"), ("ä", "e"), ("äu", "eu"), ("ph", "f")]:
        for m in re.finditer(a, w.lower()):
            v = w[:m.start()] + (b.upper() if w[m.start()].isupper() and m.start() == 0 else b) + w[m.start() + len(a):]
            if v != w: pairs3.add((w, v, f"{a}~{b}"))
vs = sorted({v for _, v, _ in pairs3} | {w for w, _, _ in pairs3})
okv = set(acc(vs))
for w, v, r in sorted(pairs3):
    if w in okv and v in okv:
        flag("F3_double_spelling_candidate", "de", de_tier.get(w, "?"), w, f"dictionary accepts both {w} and {v} (rule {r}); {v} in bank: {v in de_tier}", "needs Regelwerk list to decide stored/accepted")

# ============================================================ F4
# No regional tag exists in frami .dic, defs/de.jsonl (word/pos/definition/form_of) or the English-only kaikki dump.
S["F4_could_not_run"] = "no Austrian/Swiss tag in any local de dictionary"
for w in de_words:
    d = defs.get(w)
    if d and re.search(r"\b(Austria|Austrian|Swiss|Switzerland)\b", d.get("definition", "")):
        flag("F4_weak_signal_definition_text", "de", de_tier[w], w, f"definition mentions region: {d['definition'][:80]}", "NOT a dictionary tag; verify before tagging")

# ============================================================ JMdict
def load_jm():
    rebs = defaultdict(list); entries = []
    with gzip.open(pc.find("ja/JMdict_e.gz"), "rb") as f:
        for _, el in ET.iterparse(f):
            if el.tag == "entry":
                r = [x.text for x in el.iter("reb")]
                pos = {x.text for x in el.iter("pos")}
                ls = [x for x in el.iter("lsource")]
                gairai = bool(ls) and not any(x.get("ls_wasei") for x in ls)
                for x in r: rebs[x].append((pos, gairai, set(r)))
                el.clear()
    return rebs
jm = load_jm()
kata = pc.kata
def jm_in(w): return w in jm
EX = {"ねえさん"}
SWAPS = [("おお", "おう"), ("おう", "おお"), ("こお", "こう"), ("こう", "こお"), ("とお", "とう"), ("とう", "とお"), ("ほお", "ほう"), ("ほう", "ほお"),
         ("えい", "ええ"), ("ええ", "えい")]
PAIRS_ZU = [("ぢ", "じ"), ("づ", "ず")]
seen_dual = set()
for w in ja_words:
    t = ja_tier[w]; inj = jm_in(w)
    # long-vowel rows
    for seq in ("おお", "おう", "こお", "こう", "とお", "とう", "ほお", "ほう", "えい", "ええ"):
        if seq in w:
            if inj or w in EX: break
            alts = [w.replace(a, b, 1) for a, b in SWAPS if a == seq and a in w]
            alt = next((x for x in alts if jm_in(x)), None)
            flag("F5a_long_vowel", "ja", t, w, "not a JMdict reading" + (f"; JMdict has {alt}" if alt else "; no swapped form either"),
                 "quarantine" if alt else "defer to purity (not a headword)")
            break
    for z, s in PAIRS_ZU:
        if z in w:
            alt = w.replace(z, s)
            if inj and jm_in(alt):
                flag("F5b_zu_di_both_listed", "ja", t, w, f"JMdict lists both {w} and {alt}", "add both to data/ja/accepted-variants.tsv")
            elif not inj:
                flag("F5b_zu_di_mismatch", "ja", t, w, "not a JMdict reading" + (f"; JMdict has {alt}" if jm_in(alt) else ""), "quarantine")
            break
    if inj and all(g for _, g, _ in jm[w]):
        flag("F5c_gairaigo_in_hiragana", "ja", t, w, "every JMdict entry for this reading has a foreign-language source", "quarantine (PURITY recheck)")
    if inj and any(kata(w) in rs for _, _, rs in jm[w]):
        flag("F5d_dual_script", "ja", t, w, f"same JMdict entry also writes {kata(w)}", "tag dual_script")
    for c in w:
        if c in "－—‐一－" or "｡" <= c <= "ﾟ":
            flag("F6_lookalike_or_halfwidth_stored", "ja", t, w, f"stored U+{ord(c):04X}", "quarantine")
S["F6_normalization_today"] = ("ja has no entry in norm.rs language_key_matches (only vi, fa); same_answer = fold_strict = NFC+lowercase+whitespace-drop; "
    "Kid Mode fold_lenient additionally maps ß->ss and strips combining marks")

# ============================================================ C7 de pairs
vowel = set("aeiouäöü")
lws = {w.lower() for w in de_words}
def de_neighbours(w):
    s = w.lower(); out = []
    for m in re.finditer("ß", s): out.append((s[:m.start()] + "ss" + s[m.end():], "ß->ss"))
    for m in re.finditer("ss", s): out.append((s[:m.start()] + "ß" + s[m.end():], "ss->ß"))
    for m in re.finditer("ie", s): out.append((s[:m.start()] + "i" + s[m.end():], "ie->i"))
    for m in re.finditer("i(?!e)", s): out.append((s[:m.start()] + "ie" + s[m.end():], "i->ie"))
    for m in re.finditer(r"([^aeiouäöüh\W\d])\1", s): out.append((s[:m.start()] + s[m.start() + 1:], "double->single"))
    for m in re.finditer(r"(?<!^)([^aeiouäöüh\W\d])(?!\1)", s): out.append((s[:m.end()] + m.group(1) + s[m.end():], "single->double"))
    for m in re.finditer(r"(?<=[aeiouäöü])h", s): out.append((s[:m.start()] + s[m.end():], "h-removed"))
    for m in re.finditer(r"(?<=[aeiouäöü])(?=[^h])", s): out.append((s[:m.start()] + "h" + s[m.start():], "h-added"))
    return out
c7 = {}
for w in de_words:
    for v, r in de_neighbours(w):
        if v in lws and v != w.lower():
            a, b = sorted((w.lower(), v)); c7.setdefault((a, b), r)
# extra rule the v1.1 seed list needs (Staat/Stadt), reported separately
extra = {}
for w in de_words:
    s = w.lower()
    for m in re.finditer("[td]", s):
        v = s[:m.start()] + ("d" if s[m.start()] == "t" else "t") + s[m.end():]
        for v2, tag in ((v, "t<->d"), (re.sub(r"(aa|ee|oo)", lambda x: x.group(1)[0], v, 1), "t<->d + vowel-doubling")):
            if v2 in lws and v2 != s: extra.setdefault(tuple(sorted((s, v2))), tag)
    for m in re.finditer("aa|ee|oo", s):
        v = s[:m.start()] + s[m.start()] + s[m.end():]
        if v in lws: extra.setdefault(tuple(sorted((s, v))), "vowel-doubling")
S["C7_pairs"] = len(c7); S["C7_extra_rule_pairs"] = len(extra)
seeds7 = [("masse", "maße"), ("busse", "buße"), ("miete", "mitte"), ("staat", "stadt"), ("ofen", "offen"), ("hüte", "hütte")]
S["C7_seeds"] = {f"{a}/{b}": ("rule-set" if (a, b) in c7 or (b, a) in c7 else "extra-rule" if tuple(sorted((a, b))) in extra else "absent from bank" if not (a in lws and b in lws) else "both in bank, no rule") for a, b in seeds7}
with open(os.path.join(OUT, "C7_de_near_sound_candidates.tsv"), "w", encoding="utf-8") as f:
    for (a, b), r in sorted(c7.items()): f.write(f"{a}\t{b}\t{r}\n")
    for (a, b), r in sorted(extra.items()): f.write(f"{a}\t{b}\tEXTRA:{r}\n")

# ============================================================ C8 ja pairs, devoicing
EXT = {"あ": "あ", "い": "い", "う": "う", "え": "え", "お": "お"}
ROW_V = {}
for row, v in (("あかさたなはまやらわがざだばぱ", "あ"), ("いきしちにひみりぎじぢびぴ", "い"), ("うくすつぬふむゆるぐずづぶぷ", "う"), ("えけせてねへめれげぜでべぺ", "え"), ("おこそとのほもよろをごぞどぼぽ", "お")):
    for c in row: ROW_V[c] = v
SMALL_PAIR = dict(zip("ゃゅょぁぃぅぇぉ", "やゆよあいうえお")); SMALL_REV = {v: k for k, v in SMALL_PAIR.items()}
NROW = {"あ": "な", "い": "に", "う": "ぬ", "え": "ね", "お": "の"}
jws = set(ja_words)
def ja_neighbours(w):
    out = []
    for i, c in enumerate(w):
        if i > 0 and c in "あいうえおー":
            pv = ROW_V.get(w[i - 1])
            if pv and (c == "ー" or (c == pv) or (pv == "え" and c == "い") or (pv == "お" and c == "う")):
                out.append((w[:i] + w[i + 1:], "long-vowel removed"))
        if c == "っ": out.append((w[:i] + w[i + 1:], "っ removed"))
        if c in SMALL_PAIR: out.append((w[:i] + SMALL_PAIR[c] + w[i + 1:], "small->full"))
        if c in SMALL_REV and i > 0: out.append((w[:i] + SMALL_REV[c] + w[i + 1:], "full->small"))
        if c == "ん" and i + 1 < len(w) and w[i + 1] in NROW: out.append((w[:i] + NROW[w[i + 1]] + w[i + 2:], "ん+vowel->n-row"))
        for v, n in NROW.items():
            if c == n: out.append((w[:i] + "ん" + v + w[i + 1:], "n-row->ん+vowel"))
        if i > 0:
            out.append((w[:i] + "っ" + w[i:], "っ added"))
            pv = ROW_V.get(w[i - 1])
            if pv: out.append((w[:i] + {"あ": "あ", "い": "い", "う": "う", "え": "い", "お": "う"}[pv] + w[i:], "long-vowel added"))
    return out
c8 = {}
for w in ja_words:
    for v, r in ja_neighbours(w):
        if v in jws and v != w: c8.setdefault(tuple(sorted((w, v))), r)
S["C8_pairs"] = len(c8)
seeds8 = [("おばさん", "おばあさん"), ("きて", "きって"), ("びょういん", "びよういん"), ("きんえん", "きねん")]
S["C8_seeds"] = {f"{a}/{b}": ("rule-set" if tuple(sorted((a, b))) in c8 else "absent from bank" if not (a in jws and b in jws) else "both in bank, no rule") for a, b in seeds8}
with open(os.path.join(OUT, "C8_ja_near_sound_candidates.tsv"), "w", encoding="utf-8") as f:
    for (a, b), r in sorted(c8.items()): f.write(f"{a}\t{b}\t{r}\n")
VL = set("かきくけこさしすせそたちつてとはひふへほぱぴぷぺぽ")
DEV_V = set("きしちひぴくすつふぷ")
def morae(w):
    out = []
    for c in w:
        if c in "ゃゅょぃぅぇぉぁ" and out and c in "ゃゅょ" or (c in "ぃぅぇぉぁ" and out and out[-1][0] in "ふうてでとどつ"): out[-1] += c
        else: out.append(c)
    return out
dev = {}
for w in ja_words:
    m = morae(w); sites = []
    for i, x in enumerate(m):
        cons_voiceless = x[0] in VL
        vowel_iu = (x[0] in DEV_V and len(x) == 1) or (len(x) == 2 and x[0] in "きしちひぴ" and x[1] == "ゅ")
        if not (cons_voiceless and vowel_iu): continue
        j = i + 1
        if j < len(m) and m[j] == "っ": j += 1
        if j < len(m) and m[j][0] in VL: sites.append(i + 1)
        elif i == len(m) - 1 and x in ("す", "し"): sites.append(i + 1)
    if sites: dev[w] = sites
S["C8_devoicing_tagged_rows"] = len(dev)
S["C8_T13_たくさん"] = f"{dev.get('たくさん')}" if "たくさん" in jws else "たくさん not in the ja bank"
with open(os.path.join(OUT, "C8_ja_devoicing_tags.tsv"), "w", encoding="utf-8") as f:
    for w, s in sorted(dev.items()): f.write(w + "\t" + ",".join(f"devoice_site:{x}" for x in s) + "\n")

# ============================================================ C9
sp = [(t, w) for t, w in de_rows if " " in w or "-" in w]
S["C9_rows_with_space_or_hyphen"] = {"space": sum(" " in w for _, w in sp), "hyphen": sum("-" in w for _, w in sp)}
for t, w in sp: flag("C9_space_or_hyphen_row", "de", t, w, "stored answer contains space/hyphen", "excluded from Easy/Jr/Kid/Little Speller by D15 (check)")
LEAD = "kennen sitzen stehen liegen bleiben lassen gehen fahren laufen spazieren lernen sein haben fallen schwimmen reiten".split()
cand9 = []
for w in de_words:
    if not w[0].islower() or not w.endswith(("en", "n")): continue
    for l in LEAD:
        if w.startswith(l) and len(w) > len(l) + 3: cand9.append((w, l, w[len(l):]))
okrest = set(acc([r for _, _, r in cand9]))
for w, l, r in cand9:
    if r in okrest:
        flag("C9_together_apart_candidate", "de", de_tier[w], w, f"{l} + {r}: rulebook may allow '{l} {r}'", "needs Regelwerk list (nothing is inferred)")
S["C9_note"] = "Today fold_strict DROPS whitespace on both sides, so 'kennen lernen' already grades equal to 'kennenlernen', and so does any split (v1.1 T14 'rejects an unlisted split' fails today)."

# ============================================================ C10
for w in ja_words:
    t = ja_tier[w]
    if "を" in w: flag("C10_particle", "ja", t, w, "contains を", "check JMdict; must stay を")
    elif (w.endswith("は") or w.endswith("へ") or re.search("(では|には|とは|へは|からは|までは)", w)) and len(w) >= 3:
        flag("C10_particle", "ja", t, w, "ends in / contains a particle-looking は・へ", "check JMdict expression")
    elif w in jm and any(("exp" in " ".join(p) or "int" in " ".join(p) or "expressions" in " ".join(p) or "interjection" in " ".join(p)) for p, _, _ in jm[w]) and re.search("[はへ]", w):
        flag("C10_particle", "ja", t, w, "JMdict expression/interjection containing は/へ", "check reading")
# ============================================================ C11
part = [w for w in de_words if re.fullmatch(r"ge[a-zäöü]+e?t", w)]
def infs(w):
    stem = w[2:]
    stem = stem[:-2] if stem.endswith("et") else stem[:-1]
    return [stem + "en", stem + "n", stem + "ieren"]
allinf = sorted({i for w in part for i in infs(w)})
okinf = set(acc(allinf))
for w in part:
    ok = [i for i in infs(w) if i in okinf]
    flag("C11_participle_form", "de", de_tier[w], w,
         f"past-participle pattern; infinitive in dictionary: {ok[0] if ok else 'NONE (loan-like?)'}; hunspell_accepts_form={w in okset}", "report only (D17)")
S["C11_participles_total"] = len(part)
S["C11_no_dictionary_infinitive"] = sum(1 for w in part if not any(i in okinf for i in infs(w)))
# ============================================================ write
summ = {}
for k in sorted(findings):
    rows = findings[k]
    with open(os.path.join(OUT, k + ".csv"), "w", newline="", encoding="utf-8") as f:
        wr = csv.writer(f); wr.writerow(HDR); wr.writerows(sorted(rows, key=lambda r: (r[1], r[2], r[3])))
    summ[k] = len(rows)
S["findings"] = summ
json.dump(S, open(os.path.join(OUT, "_summary.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(json.dumps(S, ensure_ascii=False, indent=1))
