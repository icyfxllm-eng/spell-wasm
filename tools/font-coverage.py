#!/usr/bin/env python3
"""CC-VISUAL-ECLIPSE AT-5 / I3: no glyph renders as tofu.

For every language with a locale (src/i18n/locales/<lang>.json) plus every
language with a compiled bank, collect each codepoint a player can be shown:
the word banks (the generated consts compiled into the wasm: src/word_data.rs,
src/words.rs for zh, both halves of "pinyin|hanzi"), the Impossible pools, and
the UI strings. NFC, whitespace and Cc controls dropped.

Each codepoint must be drawable by the font chain the stylesheet gives it, in
the role it appears in:

  ANSWER  the typed letters and the reveal: the script face for ar/fa/ur/hi,
          then Space Mono, then the system monospace
  WORD    #meaning / #hintLine: the script face for ar/fa/ur/hi, then the UI chain
  UI      chrome: Instrument Sans (Vietnamese: Bricolage Body), then system-ui

Family names come from design/tokens.json (type.family.*, type.face.*). A
bundled face counts only inside its declared unicode-range, because the browser
never uses it outside. The system fallback is checked against the macOS system
fonts, a PROXY for iOS (same Apple families); the CI job runs on macOS.

Fails (exit 1) if any codepoint has no font at all. Reports, without failing,
codepoints that only a system font draws (fine on iOS, worth knowing) and words
that mix a bundled and a system face.

  python3 -I tools/font-coverage.py [--report out.json]
"""
import json
import logging
import os
import re
import sys
import unicodedata
from collections import OrderedDict, defaultdict

try:
    from fontTools.ttLib import TTCollection, TTFont
except ImportError:
    sys.exit("font-coverage: needs fontTools (pip install fonttools)")
logging.getLogger("fontTools").setLevel(logging.ERROR)  # benign "extra bytes in post" noise from system fonts

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
SF = "/System/Library/Fonts/"
if not os.path.isdir(SF):
    sys.exit("font-coverage: the system-font check needs macOS (/System/Library/Fonts); run it on a Mac or the macOS CI job")

# ---- family names, from the token file (the single source)
tokens = json.load(open(os.path.join(ROOT, "design/tokens.json"), encoding="utf-8"))
def first_family(value):
    return value.split(",")[0].strip().strip("'\"")
FACE = {k: first_family(v["$value"]) for k, v in tokens["type"]["face"].items() if not k.startswith("$")}
FAM = {k: first_family(v["$value"]) for k, v in tokens["type"]["family"].items() if not k.startswith("$")}
SCRIPT_FACE = {"ar": FACE["arabic"], "fa": FACE["arabic"], "ur": FACE["urdu"], "hi": FACE["devanagari"]}

# ---- bundled faces: cmap within declared unicode-range
html = open(os.path.join(ROOT, "index.html"), encoding="utf-8").read()
def cmap_of(path):
    if path.endswith(".ttc"):
        s = set()
        for f in TTCollection(path).fonts:
            s |= set(f.getBestCmap() or {})
        return s
    return set(TTFont(path).getBestCmap() or {})

bundled = defaultdict(set)
for m in re.finditer(r"@font-face\{([^}]*)\}", html):
    body = m.group(1)
    fam = re.search(r"font-family:'([^']+)'", body).group(1)
    src = re.search(r"url\('\./fonts/([^']+)'\)", body).group(1)
    ur = re.search(r"unicode-range:([^;}]+)", body)
    ranges = None
    if ur:
        ranges = []
        for part in ur.group(1).split(","):
            a, _, b = part.strip()[2:].partition("-")
            ranges.append((int(a, 16), int(b or a, 16)))
    for cp in cmap_of(os.path.join(ROOT, "fonts", src)):
        if ranges is None or any(a <= cp <= b for a, b in ranges):
            bundled[fam].add(cp)

# ---- macOS system fonts (iOS proxy). Every file listed must exist: a missing
# one would make the check pass for the wrong reason.
SYSTEM = OrderedDict([
    ("system-ui", SF + "SFNS.ttf"), ("ui-monospace", SF + "SFNSMono.ttf"),
    ("monospace", SF + "Courier.ttc"), ("sans-serif", SF + "Helvetica.ttc"),
    ("Apple SD Gothic Neo", SF + "AppleSDGothicNeo.ttc"), ("Hiragino Sans", SF + "ヒラギノ角ゴシック W3.ttc"),
    ("PingFang", "/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc"),
    ("SF Arabic", SF + "SFArabic.ttf"), ("Geeza Pro", SF + "GeezaPro.ttc"),
    ("Kohinoor", SF + "Kohinoor.ttc"), ("Apple Symbols", SF + "Apple Symbols.ttf"),
    ("CJK Symbols Fallback", SF + "CJKSymbolsFallback.ttc"), ("Apple Color Emoji", SF + "Apple Color Emoji.ttc"),
    ("Zapf Dingbats", SF + "ZapfDingbats.ttf"),
])
missing_files = [p for p in SYSTEM.values() if not os.path.exists(p)]
if missing_files:
    sys.exit("font-coverage: expected system fonts are missing (the proxy would lie): " + ", ".join(missing_files))
system = {k: cmap_of(p) for k, p in SYSTEM.items()}
SYSTEM_ANY = set().union(*system.values())

def chain(role, lang):
    face = [SCRIPT_FACE[lang]] if lang in SCRIPT_FACE else []
    ui = [FACE["vietnamese-ui"] if lang == "vi" else FAM["ui"]]
    if role == "ANSWER":
        return face + [FAM["answer"]]
    if role == "WORD":
        return face + ui
    return ui

def serve(cp, role, lang):
    for fam in chain(role, lang):
        if cp in bundled.get(fam, ()):
            return "bundled"
    return "system" if cp in SYSTEM_ANY else None

# ---- text sources
def rust_strings(chunk):
    for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', chunk):
        s = re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda x: chr(int(x.group(1), 16)), m.group(1))
        yield s.replace('\\"', '"').replace("\\\\", "\\")

WD = open(os.path.join(ROOT, "src/word_data.rs"), encoding="utf-8").read()
WS = open(os.path.join(ROOT, "src/words.rs"), encoding="utf-8").read()
def bank(lang):
    src = WS if lang == "zh" else WD
    out = []
    for tier in ("EASY", "MEDIUM", "HARD", "EXPERT"):
        head = f"pub const {lang.upper()}_{tier}: &[&str] = &["
        i = src.find(head)
        if i >= 0:
            out += list(rust_strings(src[i + len(head):src.find("];", i)]))
    return [p for w in out for p in w.split("|")] if lang == "zh" else out

def impossible(lang):
    p = os.path.join(ROOT, f"assets/impossible/impossible_pool.{lang}.json")
    if not os.path.exists(p):
        return []
    return [w if isinstance(w, str) else (w.get("word") or w.get("w") or "") for w in json.load(open(p, encoding="utf-8")).get("words", [])]

def ui(lang):
    p = os.path.join(ROOT, f"src/i18n/locales/{lang}.json")
    return [v for v in json.load(open(p, encoding="utf-8")).values() if isinstance(v, str)] if os.path.exists(p) else []

def cps(strings):
    out = set()
    for s in strings:
        for ch in unicodedata.normalize("NFC", s):
            if not ch.isspace() and unicodedata.category(ch) != "Cc":
                out.add(ord(ch))
    return out

locales = sorted(f[:-5] for f in os.listdir(os.path.join(ROOT, "src/i18n/locales")) if f.endswith(".json"))
bank_langs = sorted(set(m.group(1).lower() for m in re.finditer(r"pub const ([A-Z]{2,3})_EASY: &\[&str\]", WD)) | {"zh"})
langs = sorted(set(locales) | set(bank_langs))

report = OrderedDict()
fail = []
for lang in langs:
    words = bank(lang) + impossible(lang)
    sets = [("ANSWER", words), ("WORD", words), ("UI", ui(lang))]
    if lang == "zh":
        sets.append(("ANSWER", ["āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜü"]))
    entry = OrderedDict()
    for role, strings in sets:
        c = cps(strings)
        if not c:
            continue
        served = {cp: serve(cp, role, lang) for cp in c}
        none = sorted(cp for cp, s in served.items() if s is None)
        sys_only = sorted(cp for cp, s in served.items() if s == "system")
        mixed = sum(1 for s in strings if len({served.get(ord(ch)) for ch in unicodedata.normalize("NFC", s) if ord(ch) in served}) > 1)
        key = f"{role} ({len(strings)} strings)"
        entry[key] = dict(codepoints=len(c), no_font=[f"U+{cp:04X} {chr(cp)}" for cp in none],
                          system_only=len(sys_only), strings_mixing_faces=mixed)
        for cp in none:
            fail.append(f"{lang} {role}: U+{cp:04X} {chr(cp)} {unicodedata.name(chr(cp), '?')}")
    report[lang] = entry

if "--report" in sys.argv:
    json.dump(report, open(sys.argv[sys.argv.index("--report") + 1], "w", encoding="utf-8"), indent=1, ensure_ascii=False)

if fail:
    print(f"font-coverage: FAILED, {len(fail)} codepoint(s) have no font (tofu):")
    for f in fail[:60]:
        print("  ✗ " + f)
    sys.exit(1)
total = sum(e["codepoints"] for r in report.values() for e in r.values())
print(f"font-coverage: OK, {len(langs)} languages, {total} role/codepoint pairs, 0 without a font "
      f"(bundled faces + macOS system fonts as the iOS proxy).")
