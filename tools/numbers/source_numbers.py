#!/usr/bin/env python3
"""CC-SPELLDOKU D3: source number words from a dictionary, never write them.

For each number 0-12 the English word (from config/numbers/en.json, itself cited
to Merriam-Webster) names an English Wiktionary page. Its translation table for
the cardinal-number sense lists each language's word. This tool reads that table,
applies the selection rules below, then fetches the chosen word's own Wiktionary
entry and keeps the row only if that entry exists with a section for the
language. Every kept row cites both pages. No word is typed by hand or by a model.

Selection rules (all visible in the report):
  * the first cardinal-table entry per language is the spelling;
  * Japanese: the hiragana readings the kanji's own entry tags as numerals
    ({{ja-pos|numeral|...}}) that the translation table ALSO lists in its tr=
    romanization, plus any kana the table itself gives -- all accepted (D4). The cross-check drops the
    katakana mahjong readings and archaic counting forms the entry also tags;
    the kana keyboard cannot type kanji, so kana is the spelling;
  * Chinese: the table's tr= pinyin, else the first Mandarin pinyin in the entry's
    {{zh-pron}} -- the citation tone, never sandhi (D8);
  * Korean: the native word (first entry without hanja) and the Sino-Korean word
    (first entry written with hanja), both cardinal numbers (D4 alternates; F5
    Twin Systems is Phase B). Pre-counter forms (한, 두, 세, 네) are not taken;
  * Russian stress marks and Arabic short-vowel marks are removed: they are not
    part of the written word, and the keyboards do not type them.
"""
import json, re, sys, time, unicodedata, urllib.parse, urllib.request, pathlib

UA = "SpellGame-number-sourcing/1.0 (icyfxllm@gmail.com)"
API = "https://en.wiktionary.org/w/api.php?action=parse&page={}&prop=wikitext&format=json&redirects=1"
LANGS = {  # app code -> (wiktionary code, entry section)
    "es": ("es", "Spanish"), "fr": ("fr", "French"), "de": ("de", "German"), "pt": ("pt", "Portuguese"),
    "pl": ("pl", "Polish"), "ru": ("ru", "Russian"), "vi": ("vi", "Vietnamese"), "ko": ("ko", "Korean"),
    "ja": ("ja", "Japanese"), "zh": ("cmn", "Chinese"), "fil": ("tl", "Tagalog"), "sw": ("sw", "Swahili"),
    "ar": ("ar", "Arabic"), "hi": ("hi", "Hindi"),
}
RU_STRESS = {"́", "̀"}
AR_HARAKAT = {chr(c) for c in range(0x064B, 0x0653)} | {"ٰ"}
KANA = re.compile(r"^[぀-ゟ゠-ヿー]+$")
PINYIN = re.compile(r"^[a-zāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜü']+$")
HANGUL = lambda s: bool(s) and all("가" <= c <= "힣" for c in s)
_cache = {}

HIRAGANA = re.compile(r"^[\u3040-\u309F]+$")
_K = {
 "あ":"a","い":"i","う":"u","え":"e","お":"o","か":"ka","き":"ki","く":"ku","け":"ke","こ":"ko",
 "さ":"sa","し":"shi","す":"su","せ":"se","そ":"so","た":"ta","ち":"chi","つ":"tsu","て":"te","と":"to",
 "な":"na","に":"ni","ぬ":"nu","ね":"ne","の":"no","は":"ha","ひ":"hi","ふ":"fu","へ":"he","ほ":"ho",
 "ま":"ma","み":"mi","む":"mu","め":"me","も":"mo","や":"ya","ゆ":"yu","よ":"yo","ら":"ra","り":"ri",
 "る":"ru","れ":"re","ろ":"ro","わ":"wa","を":"o","ん":"n","が":"ga","ぎ":"gi","ぐ":"gu","げ":"ge",
 "ご":"go","ざ":"za","じ":"ji","ず":"zu","ぜ":"ze","ぞ":"zo","だ":"da","で":"de","ど":"do","ば":"ba",
 "び":"bi","ぶ":"bu","べ":"be","ぼ":"bo","ぱ":"pa","ぴ":"pi","ぷ":"pu","ぺ":"pe","ぽ":"po",
}
_Y = {"ゃ":"a","ゅ":"u","ょ":"o"}

def to_romaji(kana):
    """Hepburn, used ONLY to cross-match two dictionary fields -- it never
    produces a spelling that is stored."""
    out = ""
    for ch in kana:
        if ch in _Y and out:
            out = out[:-1] + ("" if out.endswith(("sh", "ch", "j")) else "y") + _Y[ch]
            if out.endswith(("shy", "chy")):
                out = out.replace("shy", "sh").replace("chy", "ch")
        else:
            out += _K.get(ch, "?")
    return out

def romaji_set(tr):
    out = set()
    for t in tr.split(","):
        t = t.strip().lower()
        for a, b in (("ā", "aa"), ("ī", "ii"), ("ū", "uu"), ("ē", "ee")):
            t = t.replace(a, b)
        out.add(t.replace("ō", "oo"))
        out.add(t.replace("ō", "ou"))
    return out


def fetch(page):
    if page in _cache:
        return _cache[page]
    req = urllib.request.Request(API.format(urllib.parse.quote(page)), headers={"User-Agent": UA})
    for _ in range(4):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                d = json.load(r)
            time.sleep(0.4)
            _cache[page] = d.get("parse", {}).get("wikitext", {}).get("*")
            return _cache[page]
        except Exception:
            time.sleep(2)
    return None

def strip_marks(s, marks):
    return unicodedata.normalize("NFC", "".join(c for c in unicodedata.normalize("NFD", s) if c not in marks))

def cardinal_block(wt):
    blocks = re.split(r"\{\{trans-top", wt)
    for b in blocks[1:]:
        head = b.split("}}", 1)[0].lower()
        if "cardinal" in head:
            return b.split("{{trans-bottom", 1)[0]
    return blocks[1].split("{{trans-bottom", 1)[0] if len(blocks) > 1 else ""

def entries(block, code):
    out = []
    for m in re.finditer(r"\{\{tt?\+?\|" + re.escape(code) + r"\|([^}]*)\}\}", block):
        parts = m.group(1).split("|")
        term = re.sub(r"\[\[|\]\]", "", parts[0]).strip()
        tr = next((p[3:] for p in parts[1:] if p.startswith("tr=")), "")
        out.append((term, tr))
    return out

def section(wt, name):
    if f"=={name}==" not in wt:
        return None
    return re.split(r"\n==[^=]", wt.split(f"=={name}==", 1)[1])[0]

def choose(app, ents):
    """-> (spellings, the entry page title to verify)"""
    if not ents:
        return [], None
    if app == "ko":
        native = next((t for t, _ in ents if "(" not in t and HANGUL(t)), None)
        sino = next((re.sub(r"\(.*?\)", "", t).strip() for t, _ in ents if "(" in t), None)
        sp = [h for h in (native, sino) if HANGUL(h or "")]
        return sp, (native or sino)
    head = re.sub(r"\(.*?\)", "", ents[0][0]).strip()
    if app == "ja":
        ja = section(fetch(head) or "", "Japanese") or ""
        listed = romaji_set(ents[0][1])
        readings = []
        for r in re.findall(r"\{\{ja-pos\|numeral\|([^}|]+)", ja):
            r = r.strip()
            if HIRAGANA.match(r) and r not in readings and to_romaji(r) in listed:
                readings.append(r)
        # The table's own kana readings count as well: it is the same dictionary.
        for t in ents[0][1].split(","):
            t = t.strip()
            if HIRAGANA.match(t) and t not in readings:
                readings.append(t)
        return readings, head
    if app == "zh":
        tr = ents[0][1].strip()
        if tr and PINYIN.match(tr):
            return [tr], head
        for m in re.findall(r"\|m=([^|\n}]+)", fetch(head) or ""):
            cand = m.split(",")[0].strip()
            if PINYIN.match(cand):
                return [cand], head
        return [], head
    if app == "ru":
        head = strip_marks(head, RU_STRESS)
    if app == "ar":
        head = strip_marks(head, AR_HARAKAT)
    return [head], head

def main(out_dir):
    en = json.loads(pathlib.Path("config/numbers/en.json").read_text())
    pages = {r["n"]: r["spellings"][0] for r in en["rows"] if r["n"] <= 12}
    tables = {app: [] for app in LANGS}
    report = {app: [] for app in LANGS}
    for n, word in sorted(pages.items()):
        trans_page = f"{word}/translations"
        wt = fetch(trans_page)
        if not wt or "trans-top" not in wt:
            trans_page, wt = word, fetch(word) or ""
        block = cardinal_block(wt)
        for app, (code, sec) in LANGS.items():
            ents = entries(block, code)
            listed = [e[0] + (f" (tr {e[1]})" if e[1] else "") for e in ents]
            sp, head = choose(app, ents)
            if not sp:
                report[app].append([n, None, listed, "no usable spelling in the dictionary"])
                continue
            if section(fetch(head) or "", sec) is None:
                report[app].append([n, sp, listed, f"entry '{head}' has no {sec} section"])
                continue
            loc = (f"https://en.wiktionary.org/wiki/{urllib.parse.quote(trans_page)} ; "
                   f"https://en.wiktionary.org/wiki/{urllib.parse.quote(head)}#{sec}")
            tables[app].append({"n": n, "spellings": sp, "status": "sourced", "source": "Wiktionary", "locator": loc})
            report[app].append([n, sp, listed, "kept"])
        print(f"{n} {word}: done", file=sys.stderr)
    out = pathlib.Path(out_dir); out.mkdir(parents=True, exist_ok=True)
    for app, rows in tables.items():
        doc = {"lang": app, "authority": "Wiktionary",
               "note": "Sourced by tools/numbers/source_numbers.py from English Wiktionary's cardinal-number translation tables, each row verified against its own entry. status stays 'sourced' until the Fiverr audit ingest.",
               "rows": rows}
        (out / f"{app}.json").write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n")
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    for app, rows in report.items():
        kept = {r[0]: r[1] for r in rows if r[3] == "kept"}
        print(f"{app:3} " + "  ".join(f"{n}:{'/'.join(kept[n])}" for n in sorted(kept)))
        for r in rows:
            if r[3] != "kept":
                print(f"      MISS {r[0]}: {r[3]} | listed {r[2][:3]}")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "numbers-out")
