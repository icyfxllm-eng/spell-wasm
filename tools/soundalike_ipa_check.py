#!/usr/bin/env python3
"""CC-SOUNDALIKE-LADDER F3: the independent IPA check for a ladder language's sets.

    python3 tools/soundalike_ipa_check.py --lang fr --dump-dir <dir holding kaikki-<lang>.jsonl>

Wiktionary/kaikki IPA is a CHECK SIGNAL ONLY (CC BY-SA, Eric 2026-11-07): it is read here and compared, and the only thing
written is one word per set -- agree | differ | missing -- to data/<lang>/soundalike-ipa-check.json. No IPA string is ever
written to a shipped file; scripts/ipa-leak-check.py fails the gate if one appears.

A set AGREES when its members share at least one normalized pronunciation. pt reads Brazilian-tagged entries only (the voice is
pt-BR). Normalization drops stress, syllable dots, liaison ties, length marks and optional (parenthesized) sounds.
"""
import argparse, json, pathlib, re, sys, unicodedata, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
nfc = lambda s: unicodedata.normalize("NFC", s)


def norm_ipa(s):
    s = re.sub(r"\([^)]*\)", "", s)
    s = s.strip("/[] ")
    s = re.sub(r"[ˈˌ.‿ː\s]", "", s)
    return unicodedata.normalize("NFC", s)


def collect(dump, words, lang):
    out = {w: set() for w in words}
    for line in open(dump, encoding="utf-8"):
        d = json.loads(line)
        w = nfc(d.get("word", "")).lower()
        if w not in out:
            continue
        for x in d.get("sounds", []):
            ipa = x.get("ipa")
            if not ipa or not ipa.startswith(("/", "[") if lang == "ko" else "/"):   # ko entries carry narrow [..] transcriptions only
                continue
            tags = x.get("tags") or []
            if lang == "pt" and "Brazil" not in tags:
                continue
            if lang == "fr" and "‿" in ipa:
                continue
            n = norm_ipa(ipa)
            if n:
                out[w].add(n)
    return out


def run(lang, dump_dir):
    table = json.loads((ROOT / f"assets/words/{lang}/soundalikes.json").read_text(encoding="utf-8"))
    sets = [s for s in table["sets"] if s["kind"] == "true"]
    words = {nfc(m["memberId"]) for s in sets for m in s["members"]}
    pron = collect(pathlib.Path(dump_dir) / f"kaikki-{lang}.jsonl", words, lang)
    res = {}
    for s in sets:
        ms = [nfc(m["memberId"]) for m in s["members"]]
        if any(not pron[m] for m in ms):
            res[s["setId"]] = "missing"
        else:
            common = set.intersection(*(pron[m] for m in ms))
            res[s["setId"]] = "agree" if common else "differ"
    doc = {"$doc": "CC-SOUNDALIKE-LADDER F3 check results. One word per set; no IPA is stored here. Written by tools/soundalike_ipa_check.py.",
           "lang": lang, "checked": time.strftime("%Y-%m-%d"), "sets": dict(sorted(res.items()))}
    p = ROOT / f"data/{lang}/soundalike-ipa-check.json"
    p.write_text(json.dumps(doc, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    from collections import Counter
    print(lang, dict(Counter(res.values())), "of", len(res), "true sets")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", required=True)
    ap.add_argument("--dump-dir", required=True)
    a = ap.parse_args()
    run(a.lang, a.dump_dir)
