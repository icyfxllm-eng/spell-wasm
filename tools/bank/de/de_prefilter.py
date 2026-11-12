#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""CC-DE-BANK F3 -- flag-only pre-filter for the German bank.

    python3 tools/bank/de/de_prefilter.py words.txt            # one word per line
    python3 tools/bank/de/de_prefilter.py --fixture FILE.xlsx  # the 200 batch-1 Words

FLAG ONLY. It never cuts, never approves; the output is a `prefilter_flags` string
per word ("SS_FOR_ESZETT;CASE") that goes into the locked audit export as a
read-only column. The auditor still decides (agent-flag-only doctrine).

Reference dictionary: hunspell de_DE_frami (the one tools/bank/purity_check.py
already uses). Nothing here branches on the language: the rules read their lists
and dictionary from config/bank-prefilter/<lang>.json, so a later language is a
new JSON file plus its rule-specific digraph table.
"""
import bisect, json, os, subprocess, sys, unicodedata

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
MAIN = os.path.expanduser("~/repos/spell-wasm")
HUNSPELL = "/opt/homebrew/bin/hunspell"
CODES = ["SS_FOR_ESZETT", "UMLAUT_DIGRAPH", "REMOVED_SPELLING", "FRAGMENT", "ABBREVIATION",
         "PROPER_NOUN", "CLIPPED", "REGIONAL", "CASE"]
# Digraph -> letter tables are orthography, not language branching: they are the
# rule's definition. (ss->ß, ae/oe/ue -> ä/ö/ü.)
DIGRAPHS = {"ae": "ä", "oe": "ö", "ue": "ü"}
VOWELS = set("aeiouyäöüáàâéèêíìîóòôúùû")


def find_dict(base):
    for root in [os.environ.get("PURITY_CACHE"), os.path.join(ROOT, ".corpus-cache"), os.path.join(MAIN, ".corpus-cache")]:
        if root and os.path.exists(os.path.join(root, "dicts", base + ".dic")):
            return os.path.join(root, "dicts", base)
    raise SystemExit("reference dictionary not found: dicts/%s.dic under .corpus-cache (set PURITY_CACHE). "
                     "No German source wired in -- stop and ask." % base)


class Reference:
    def __init__(self, cfg):
        self.base = find_dict(cfg["dictionary"]["hunspell"])
        self.cache = {}
        self.stems = None

    def _batch(self, words):
        todo = [w for w in dict.fromkeys(words) if w not in self.cache and w]
        if todo:
            r = subprocess.run([HUNSPELL, "-i", "utf-8", "-d", self.base, "-l"], input="\n".join(todo) + "\n",
                               capture_output=True, text=True, encoding="utf-8", check=True)
            bad = set(r.stdout.split("\n"))
            for w in todo:
                self.cache[w] = w not in bad
        return [self.cache.get(w, False) for w in words]

    def prime(self, words):
        self._batch(words)

    def ok(self, w):
        """Dictionary word exactly as written (hunspell is case-sensitive: 'haus' is rejected)."""
        return self._batch([w])[0]

    def ok_any_case(self, w):
        return self.ok(w) or self.ok(w[:1].upper() + w[1:]) or self.ok(w.lower())

    def has_longer(self, w):
        """Some dictionary headword starts with w and is longer (a compound head)."""
        if self.stems is None:
            lines = open(self.base + ".dic", encoding="utf-8", errors="replace").read().split("\n")[1:]
            self.stems = sorted({l.split("/")[0].split("\t")[0].lower() for l in lines if l.strip() and not l.startswith("#")})
        k = w.lower()
        i = bisect.bisect_right(self.stems, k)
        return i < len(self.stems) and self.stems[i].startswith(k)


def variants(w):
    """Every way of replacing digraphs/ss in w with their letters (ss->ß, ae/oe/ue->ä/ö/ü)."""
    lw = w.lower()
    out = set()
    if "ss" in lw:
        out.add(("SS_FOR_ESZETT", w.replace("ss", "ß").replace("SS", "ß")))
    for d, u in DIGRAPHS.items():
        if d in lw:
            out.add(("UMLAUT_DIGRAPH", w.replace(d, u).replace(d.capitalize(), u.upper())))
    return out


def flags_for(words, cfg, ref):
    sets = {k: set(x.lower() for x in cfg.get(k, [])) for k in
            ("removed_spellings", "abbreviations", "proper_nouns", "clipped", "regional")}
    prime = list(words) + [w[:1].upper() + w[1:] for w in words] + [w.lower() for w in words]
    for w in words:
        for _, v in variants(w):
            prime += [v, v[:1].upper() + v[1:]]
    ref.prime(prime)
    res = {}
    for w in words:
        lw, f = w.lower(), []
        is_word = ref.ok_any_case(w)
        # A valid word whose letters merely contain ss/ue (Masse, Quelle) is not an
        # error: the digraph rules fire only when the written form is NOT itself a word.
        for code, v in sorted(variants(w)):
            if not is_word and ref.ok_any_case(v) and code not in f:
                f.append(code)
        if lw in sets["removed_spellings"]:
            f.append("REMOVED_SPELLING")
        if not is_word and ref.has_longer(w) and not any(c in f for c in ("REMOVED_SPELLING",)):
            f.append("FRAGMENT")
        if not (set(lw) & VOWELS) or lw in sets["abbreviations"]:
            f.append("ABBREVIATION")
        if lw in sets["proper_nouns"]:
            f.append("PROPER_NOUN")
        if lw in sets["clipped"] or lw.endswith("'") or lw.endswith("’"):
            f.append("CLIPPED")
        if lw in sets["regional"]:
            f.append("REGIONAL")
        # A lowercase entry whose capitalised form is also a dictionary word. The
        # dictionary cannot say which reading the bank means (erde/Erde, kette/Kette,
        # licht/Licht are both a verb form and a noun), so this flags the AMBIGUITY
        # for the auditor; it is liberal by design and excluded from the precision bar.
        if w[:1].islower() and ref.ok(w[:1].upper() + w[1:]):
            f.append("CASE")
        res[w] = ";".join(f)
    return res


def load_cfg(lang="de"):
    return json.load(open(os.path.join(ROOT, "config", "bank-prefilter", lang + ".json"), encoding="utf-8"))


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)
    if argv[1] == "--fixture":
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from xlsx_min import read_sheet
        rows = read_sheet(argv[2], "Words")[1:]
        words = [r[1] for r in rows]
    else:
        words = [l.rstrip("\n") for l in open(argv[1], encoding="utf-8") if l.strip() and not l.startswith("#")]
    res = flags_for(words, load_cfg(), Reference(load_cfg()))
    for w in words:
        print("%s\t%s" % (w, res[w]))


if __name__ == "__main__":
    main(sys.argv)
