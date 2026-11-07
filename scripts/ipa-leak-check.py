#!/usr/bin/env python3
"""CC-SOUNDALIKE-LADDER constraint: nothing derived from a BY-SA source ships. Wiktionary/kaikki IPA is a check signal only.
Fails if any ladder data file carries a character from the IPA blocks (U+0250-02AF, U+1D00-1DBF) or an IPA-looking /.../ string,
or any key naming IPA. Words of fr/pt/pl/fil/sw/ko/vi do not use these characters, so a hit is a leak, not a false alarm."""
import glob, json, os, re, sys
R = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FILES = (glob.glob(f"{R}/assets/words/*/soundalike*.json") + glob.glob(f"{R}/data/*/soundalike-*.json") +
         glob.glob(f"{R}/data/*/sound-alike-rules.json") + glob.glob(f"{R}/assets/words/*/homophones.seed.txt"))
IPA = re.compile("[ɐ-ʯᴀ-ᶿʰ-˿̀-ͯ]")
bad = []
# letters these languages use legitimately: combining marks are allowed in NFC-decomposed text only if the file never mixes them
for f in FILES:
    t = open(f, encoding="utf-8").read()
    if f.endswith(".json"):
        try:
            d = json.loads(t)
        except ValueError:
            bad.append(f"{f}: not JSON"); continue
        def walk(x, path=""):
            if isinstance(x, dict):
                for k, v in x.items():
                    if k.lower() in ("ipa", "pron", "pronunciation", "sounds"): bad.append(f"{f}: key {k!r}")
                    walk(v, path + "/" + k)
            elif isinstance(x, list):
                for v in x: walk(v, path)
            elif isinstance(x, str):
                if re.search(r"/[^/\s]{1,20}/", x) and re.search("[ɐ-ʯʰ-˿]", x): bad.append(f"{f}: IPA-looking string {x[:40]!r}")
        walk(d)
    for m in re.finditer("[ɐ-ʯᴀ-ᶿ]", t):
        bad.append(f"{f}: IPA character U+{ord(m.group()):04X}"); break
if bad:
    print("ipa-leak-check: FAILED -- IPA must never ship (CC-SOUNDALIKE-LADDER):")
    for b in bad[:20]: print("  " + b)
    sys.exit(1)
print(f"ipa-leak-check: OK ({len(FILES)} files)")
