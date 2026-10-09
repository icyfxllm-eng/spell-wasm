#!/usr/bin/env python3
"""CC-BANK-PURITY F2 — write config/bank-purity-sources.json and assets/words/purity/<lang>.tsv.

    python3 tools/bank/gen_ledger.py reports/bank-purity-rows-after.csv

The argument is an F1 run over the swept bank.  Verdicts (closed set): dictionary, exception, pending.
Every row F1 passes is `dictionary`; `not_in_reference_dictionary` and `linker_attached_form` rows
(auditor-sheet rows, P10) are `pending`; no row is ever written `exception` here (that needs a signer).
Dictionaries are only named and hashed here; nothing derived from them is written (I7).
"""
import csv, hashlib, json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import ALL, TIERS, load_bank, ROOT

C = f"{ROOT}/.corpus-cache"
LO = "https://raw.githubusercontent.com/LibreOffice/dictionaries/32b006a2c22a4ac7e8ed3f03346f7b3d85a970a4"
sha = lambda p: hashlib.sha256(open(p, "rb").read()).hexdigest()
def hs(id_, lang, dirn, base, lic, lo_dir):
    return {"id": id_, "langs": [lang], "kind": "hunspell", "licence": lic,
            "url": [f"{LO}/{lo_dir}/{base}.dic", f"{LO}/{lo_dir}/{base}.aff"],
            "cache": [f"dicts/{dirn}/{base}.dic", f"dicts/{dirn}/{base}.aff"]}
src = [
    hs("en_US", "en", "en", "en_US", "SCOWL (permissive)", "en"),
    hs("es_ES", "es", "es", "es_ES", "GPL-3+ / LGPL-3+ / MPL-1.1+", "es"),
    hs("fr", "fr", "fr", "fr", "MPL 2.0", "fr_FR"),
    hs("de_DE_frami", "de", "de", "de_DE_frami", "GPL 2 or 3", "de"),
    hs("pt_BR", "pt", "pt", "pt_BR", "LGPL-3 / MPL", "pt_BR"),
    hs("pt_PT", "pt", "pt-PT", "pt_PT", "GPL 2+ (variety check only)", "pt_PT"),
    hs("pl_PL", "pl", "pl", "pl_PL", "GPL2 / LGPL2.1 / MPL1.1 / Apache2 / CC BY 4.0", "pl_PL"),
    hs("ru_RU", "ru", "ru", "ru_RU", "BSD-style", "ru_RU"),
    hs("sw_TZ", "sw", "sw", "sw_TZ", "LGPL 2.1+", "sw_TZ"),
    hs("ko_KR", "ko", "ko", "ko_KR", "GPL-3.0 only", "ko_KR"),
    hs("ar", "ar", "ar", "ar", "GPL2 / LGPL2.1 / MPL1.1", "ar"),
    hs("hi_IN", "hi", "hi", "hi_IN", "GPL", "hi_IN"),
    {"id": "JMdict_e", "langs": ["ja"], "kind": "wordlist", "licence": "CC BY-SA 4.0 (EDRDG)", "url": ["http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"], "cache": ["ja/JMdict_e.gz"]},
    {"id": "CC-CEDICT", "langs": ["zh"], "kind": "wordlist", "licence": "CC BY-SA 4.0", "url": ["https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"], "cache": ["zh/cedict.txt.gz"]},
    {"id": "Unihan", "langs": ["zh"], "kind": "wordlist", "licence": "Unicode licence", "url": ["https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip"], "cache": ["zh/Unihan.zip"]},
    {"id": "kaikki-tagalog", "langs": ["fil"], "kind": "wiktionary-extract", "licence": "CC BY-SA", "url": ["https://kaikki.org/dictionary/Tagalog/kaikki.org-dictionary-Tagalog.jsonl"], "cache": ["fil/kaikki-tagalog.jsonl"]},
]
C = os.environ.get("PURITY_CACHE") or C
regp = f"{ROOT}/config/bank-purity-sources.json"
if os.path.exists(regp):          # the registry is written once; hashes change only by a deliberate re-census
    reg = json.load(open(regp, encoding="utf-8"))
    src = reg["sources"]
else:
    for s in src:
        s["sha256"] = [sha(f"{C}/{c}") for c in s["cache"]]
reg = reg if os.path.exists(regp) else {"_comment": "CC-BANK-PURITY F2. Reference dictionaries are build inputs only: downloaded to the gitignored .corpus-cache, never committed or shipped (P3, I7). Only verdicts live in assets/words/purity/.",
       "pinned_commit": "32b006a2c22a4ac7e8ed3f03346f7b3d85a970a4", "sources": src,
       "bank_source": {l: next(s["id"] for s in src if l in s["langs"]) for l in ALL}}
if not os.path.exists(regp):
    open(regp, "w", encoding="utf-8").write(json.dumps(reg, ensure_ascii=False, indent=2) + "\n")

pending = {(r["lang"], r["word"]) for r in csv.DictReader(open(sys.argv[1], encoding="utf-8"))
           if r["class"] in ("not_in_reference_dictionary", "linker_attached_form")}
other = [r for r in csv.DictReader(open(sys.argv[1], encoding="utf-8")) if r["class"] not in ("", "not_in_reference_dictionary", "linker_attached_form")]
if other:
    sys.exit(f"{len(other)} rows still carry a quarantine/fix class; run phase_a.py first: {other[:3]}")
th = {(a, b): c for a, b, c in re.findall(r'\("(\w+)", "(\w+)", (0x[0-9A-F]+)\)', open(f"{ROOT}/src/word_data.rs", encoding="utf-8").read())}
os.makedirs(f"{ROOT}/assets/words/purity", exist_ok=True)
for l in ALL:
    if l == "vi":
        continue
    sid = reg["bank_source"][l]
    out = ["# CC-BANK-PURITY F2 ledger: word<TAB>verdict<TAB>source. One line per bank row, bank order.",
           "# verdicts: dictionary | exception (signed) | pending (auditor sheet). No new row may ever enter as pending."]
    out += [f"#tier\t{t}\t{th[(l, t)]}" for t in TIERS if (l, t) in th]
    prev = {}   # signed `exception` lines survive a regeneration
    pp = f"{ROOT}/assets/words/purity/{l}.tsv"
    if os.path.exists(pp):
        for x in open(pp, encoding="utf-8").read().split("\n"):
            f = x.split("\t")
            if len(f) > 1 and f[1] == "exception":
                prev[f[0]] = x
    for t, w in load_bank(l):
        out.append(prev.get(w) or f"{w}\t{'pending' if (l, w) in pending else 'dictionary'}\t{sid}")
    open(f"{ROOT}/assets/words/purity/{l}.tsv", "w", encoding="utf-8").write("\n".join(out) + "\n")
print("ledger written:", {l: sum(1 for x in open(f'{ROOT}/assets/words/purity/{l}.tsv', encoding='utf-8') if x[0] != '#') for l in ALL})
