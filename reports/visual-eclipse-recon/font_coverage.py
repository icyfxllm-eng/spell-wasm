#!/usr/bin/env python3
"""R4 glyph-coverage probe for SpellGame (read-only; writes only to OUT dir).

Usage: python -I font_coverage.py <worktree> <outdir>

For each registered language it collects every codepoint in
  * the shipped word banks (parsed from the GENERATED Rust consts that are compiled
    into the wasm: src/word_data.rs XX_{EASY,MEDIUM,HARD,EXPERT}, src/words.rs ZH_*),
  * the Impossible pools (assets/impossible/impossible_pool.<lang>.json "words"),
  * the UI locale strings (src/i18n/locales/<lang>.json values),
NFC-normalised, dropping whitespace and Cc controls (Cf format chars such as ZWNJ
are kept and reported), and checks them against the font chain that CSS actually
assigns (index.html; computed-style probe in computed_fonts.json):

  UI      body / chrome          'Instrument Sans' -> system-ui -> sans-serif
  ANSWER  .ltr letters, .reveal  'Space Mono' -> ui-monospace -> monospace
  WORD    #meaning / #hintLine   ar,fa: 'Noto Naskh Arabic'; ur: 'Noto Nastaliq Urdu';
                                 hi: 'Noto Sans Devanagari'; else #meaning = UI chain,
                                 #hintLine = ANSWER chain (reported as UI chain here)

Bundled coverage of a family = union over its @font-face rules of
(cmap of that file) INTERSECT (its declared unicode-range), because a browser never
uses a face for a codepoint outside its unicode-range.

System fallback is approximated with macOS system fonts as a PROXY for iOS.
"""
import json, os, re, sys, unicodedata
from collections import defaultdict, OrderedDict
from fontTools.ttLib import TTFont, TTCollection

WT, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)

LANGS = ['en', 'es', 'fr', 'de', 'pt', 'pl', 'vi', 'ko', 'ja', 'fil', 'zh', 'ru', 'ar', 'sw', 'hi']
EXTRA_BANK_ONLY = ['fa']  # compiled into word_data.rs but not in the registry

# ---------- unicode-range parsing from index.html ----------
html = open(os.path.join(WT, 'index.html'), encoding='utf-8').read()
faces = []  # (family, file, ranges, line)
for m in re.finditer(r"@font-face\{([^}]*)\}", html):
    body = m.group(1)
    fam = re.search(r"font-family:'([^']+)'", body).group(1)
    src = re.search(r"url\('\./fonts/([^']+)'\)", body).group(1)
    ur = re.search(r"unicode-range:([^;}]+)", body)
    ranges = None
    if ur:
        ranges = []
        for part in ur.group(1).split(','):
            part = part.strip()[2:]
            a, _, b = part.partition('-')
            ranges.append((int(a, 16), int(b or a, 16)))
    line = html[:m.start()].count('\n') + 1
    faces.append((fam, src, ranges, line))

def in_ranges(cp, ranges):
    return ranges is None or any(a <= cp <= b for a, b in ranges)

def cmap_of(path, idx=None):
    if path.endswith('.ttc'):
        s = set()
        for f in TTCollection(path).fonts:
            s |= set(f.getBestCmap() or {})
        return s
    return set(TTFont(path).getBestCmap())

bundled = defaultdict(set)  # family -> effective codepoints
face_detail = []
for fam, src, ranges, line in faces:
    cm = cmap_of(os.path.join(WT, 'fonts', src))
    eff = {c for c in cm if in_ranges(c, ranges)}
    bundled[fam] |= eff
    face_detail.append(dict(family=fam, file=src, index_html_line=line, cmap=len(cm), effective=len(eff),
                            cmap_outside_range=len(cm) - len(eff)))

# ---------- macOS system fonts as iOS proxy ----------
SF = '/System/Library/Fonts/'
SYS = OrderedDict([
    ('SF Pro (SFNS.ttf) [system-ui]', SF + 'SFNS.ttf'),
    ('SF Mono (SFNSMono.ttf) [ui-monospace]', SF + 'SFNSMono.ttf'),
    ('Courier (Courier.ttc) [monospace]', SF + 'Courier.ttc'),
    ('Times (Times.ttc) [serif]', SF + 'Times.ttc'),
    ('Helvetica (Helvetica.ttc) [sans-serif]', SF + 'Helvetica.ttc'),
    ('Apple SD Gothic Neo', SF + 'AppleSDGothicNeo.ttc'),
    ('Hiragino Sans W3', SF + 'ヒラギノ角ゴシック W3.ttc'),
    ('PingFang (PingFangUI.ttc)', '/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc'),
    ('SF Arabic', SF + 'SFArabic.ttf'),
    ('Geeza Pro', SF + 'GeezaPro.ttc'),
    ('Noto Nastaliq Urdu (system)', SF + 'NotoNastaliq.ttc'),
    ('Kohinoor (Devanagari)', SF + 'Kohinoor.ttc'),
    ('SF India', SF + 'SFIndia.ttc'),
    ('Apple Symbols', SF + 'Apple Symbols.ttf'),
    ('CJK Symbols Fallback', SF + 'CJKSymbolsFallback.ttc'),
    ('Apple Color Emoji', SF + 'Apple Color Emoji.ttc'),
    ('Zapf Dingbats', SF + 'ZapfDingbats.ttf'),
    ('Menlo', SF + 'Menlo.ttc'),
])
sys_cmaps = OrderedDict()
for k, p in SYS.items():
    try:
        sys_cmaps[k] = cmap_of(p)
    except Exception as e:  # noqa
        print('WARN cannot read', p, e)

# Per-language preference for attribution (all proxy fonts still count for (b)).
SCRIPT_PREF = {
    'ko': ['Apple SD Gothic Neo'], 'ja': ['Hiragino Sans W3'], 'zh': ['PingFang (PingFangUI.ttc)'],
    'ar': ['SF Arabic', 'Geeza Pro'], 'fa': ['SF Arabic', 'Geeza Pro'],
    'hi': ['Kohinoor (Devanagari)', 'SF India'],
}
CHAIN_SYS = {
    'UI': ['SF Pro (SFNS.ttf) [system-ui]', 'Helvetica (Helvetica.ttc) [sans-serif]'],
    'ANSWER': ['SF Mono (SFNSMono.ttf) [ui-monospace]', 'Courier (Courier.ttc) [monospace]'],
    'WORD': ['Times (Times.ttc) [serif]'],
}

def chain(role, lang):
    if role == 'UI':
        return ['Instrument Sans']
    if role == 'ANSWER':
        return ['Space Mono']
    if role == 'WORD':
        return {'ar': ['Noto Naskh Arabic'], 'fa': ['Noto Naskh Arabic'], 'ur': ['Noto Nastaliq Urdu'],
                'hi': ['Noto Sans Devanagari']}.get(lang, ['Instrument Sans'])

def sys_order(role, lang):
    r = role
    if role == 'WORD' and lang not in ('ar', 'fa', 'ur', 'hi'):
        r = 'UI'  # #meaning falls through Instrument Sans -> system-ui
    pref = CHAIN_SYS[r] + SCRIPT_PREF.get(lang, [])
    return pref + [k for k in sys_cmaps if k not in pref]

def first_font(cp, role, lang):
    for fam in chain(role, lang):
        if cp in bundled[fam]:
            return 'bundled:' + fam
    for k in sys_order(role, lang):
        if cp in sys_cmaps.get(k, ()):
            return 'system:' + k
    return None

# ---------- text sources ----------
def rust_strings(chunk):
    out = []
    for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', chunk):
        s = m.group(1)
        s = re.sub(r'\\u\{([0-9a-fA-F]+)\}', lambda x: chr(int(x.group(1), 16)), s)
        s = s.replace('\\"', '"').replace('\\\\', '\\')
        out.append(s)
    return out

wd = open(os.path.join(WT, 'src/word_data.rs'), encoding='utf-8').read()
ws = open(os.path.join(WT, 'src/words.rs'), encoding='utf-8').read()

def bank_words(lang):
    words = []
    src = ws if lang == 'zh' else wd
    for tier in ('EASY', 'MEDIUM', 'HARD', 'EXPERT'):
        name = f'pub const {lang.upper()}_{tier}: &[&str] = &['
        i = src.find(name)
        if i < 0:
            continue
        j = src.find('];', i)
        words += rust_strings(src[i + len(name):j])
    if lang == 'zh':  # "pinyin|hanzi": both sides are shown/typed
        words = [p for w in words for p in w.split('|')]
    return words

def impossible_words(lang):
    p = os.path.join(WT, f'assets/impossible/impossible_pool.{lang}.json')
    if not os.path.exists(p):
        return []
    d = json.load(open(p, encoding='utf-8'))
    out = []
    for w in d.get('words', []):
        out.append(w if isinstance(w, str) else (w.get('word') or w.get('w') or ''))
    return out

def ui_strings(lang):
    p = os.path.join(WT, f'src/i18n/locales/{lang}.json')
    if not os.path.exists(p):
        return []
    return [v for v in json.load(open(p, encoding='utf-8')).values() if isinstance(v, str)]

def codepoints(strings):
    cps = set()
    for s in strings:
        for ch in unicodedata.normalize('NFC', s):
            if ch.isspace() or unicodedata.category(ch) == 'Cc':
                continue
            cps.add(ord(ch))
    return cps

def fmt(cp):
    ch = chr(cp)
    show = ch if unicodedata.category(ch)[0] not in 'MC' else '\u25cc' + ch
    return f'U+{cp:04X} {show} {unicodedata.name(ch, "?")}'

# ---------- run ----------
summary = OrderedDict()
detail = OrderedDict()
for lang in LANGS + EXTRA_BANK_ONLY:
    bw = bank_words(lang)
    iw = impossible_words(lang)
    us = ui_strings(lang) if lang in LANGS else []
    sets = OrderedDict([
        ('bank+impossible @ ANSWER (.ltr/.reveal)', ('ANSWER', bw + iw)),
        ('bank+impossible @ WORD (#meaning/#hintLine)', ('WORD', bw + iw)),
        ('UI strings @ UI (body)', ('UI', us)),
    ])
    if lang == 'zh':  # tone-marked pinyin the grader accepts (src/pinyin.rs:88-)
        pin = 'āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜü'
        sets['zh tone-marked pinyin (typed) @ ANSWER'] = ('ANSWER', [pin])
    ls = OrderedDict(); ld = OrderedDict()
    for label, (role, strings) in sets.items():
        cps = codepoints(strings)
        if not cps:
            continue
        attr = {cp: first_font(cp, role, lang) for cp in cps}
        miss_a = sorted(cp for cp, f in attr.items() if not (f or '').startswith('bundled:'))
        miss_b = sorted(cp for cp, f in attr.items() if f is None)
        by_font = defaultdict(int)
        for f in attr.values():
            by_font[f or 'NONE'] += 1
        # combining marks left after NFC (no precomposed form) whose serving font lacks the mark
        marks = sorted(cp for cp in cps if unicodedata.category(chr(cp)) in ('Mn', 'Mc'))
        # per-string font mixing between bundled and system within one word/string
        mixed = 0; total = 0; mixed_ex = []
        for s in strings:
            s2 = [c for c in unicodedata.normalize('NFC', s) if not c.isspace() and unicodedata.category(c) != 'Cc']
            if not s2:
                continue
            total += 1
            kinds = {(attr.get(ord(c)) or 'NONE').split(':')[0] for c in s2}
            if 'bundled' in kinds and len(kinds) > 1:
                mixed += 1
                if len(mixed_ex) < 8:
                    mixed_ex.append(s)
        ls[label] = dict(role=role, chain=chain(role, lang), strings=len(strings), codepoints=len(cps),
                         missing_bundled_only=len(miss_a), missing_bundled_plus_system=len(miss_b),
                         served_by=dict(sorted(by_font.items(), key=lambda x: -x[1])),
                         strings_mixing_bundled_and_system=f'{mixed}/{total}', mixed_examples=mixed_ex,
                         combining_marks_after_nfc=[fmt(c) for c in marks][:30])
        ld[label] = dict(missing_bundled_only=[fmt(c) for c in miss_a],
                         missing_bundled_plus_system=[fmt(c) for c in miss_b],
                         cf_format_chars=[fmt(c) for c in sorted(cps) if unicodedata.category(chr(c)) == 'Cf'])
    summary[lang] = ls
    detail[lang] = ld

json.dump(dict(faces=face_detail, system_proxy_fonts={k: len(v) for k, v in sys_cmaps.items()}, summary=summary),
          open(os.path.join(OUT, 'coverage_summary.json'), 'w'), indent=1, ensure_ascii=False)
json.dump(detail, open(os.path.join(OUT, 'coverage_missing_full.json'), 'w'), indent=1, ensure_ascii=False)

# human-readable
with open(os.path.join(OUT, 'coverage_report.txt'), 'w') as f:
    for fd in face_detail:
        f.write(f"face {fd}\n")
    for lang, ls in summary.items():
        f.write(f"\n===== {lang} =====\n")
        for label, s in ls.items():
            f.write(f"-- {label}: chain={s['chain']} strings={s['strings']} cps={s['codepoints']} "
                    f"miss(a bundled)={s['missing_bundled_only']} miss(b +system)={s['missing_bundled_plus_system']} "
                    f"mixing={s['strings_mixing_bundled_and_system']}\n   served_by={s['served_by']}\n")
            if s['mixed_examples']:
                f.write(f"   mixed e.g. {s['mixed_examples']}\n")
            d = detail[lang][label]
            f.write("   (a) " + '; '.join(d['missing_bundled_only'][:50]) + ('' if len(d['missing_bundled_only']) <= 50 else ' ...') + "\n")
            f.write("   (b) " + '; '.join(d['missing_bundled_plus_system']) + "\n")
            if d['cf_format_chars']:
                f.write("   Cf: " + '; '.join(d['cf_format_chars']) + "\n")
print(open(os.path.join(OUT, 'coverage_report.txt')).read()[:200])
