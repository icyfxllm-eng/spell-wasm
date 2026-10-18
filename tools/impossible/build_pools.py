#!/usr/bin/env python3
"""CC-IMPOSSIBLE v1 F2, as amended by v1.1 (sound-alike filter) and v1.2 section 10
(the pool is 100% of Expert) -- build each language's Impossible pool artifact.

    python3 tools/impossible/build_pools.py           # write assets/impossible/impossible_pool.<lang>.json
    python3 tools/impossible/build_pools.py --check   # gate: committed artifacts match a fresh build

A language's pool is every row of its live Expert tier that passes, in order:
  1. PURITY: not on the awaiting-c6 list (every other bank row already passes the
     PURITY build gate, so the bank itself is the PURITY-passing set).
  2. The v1.1 sound-alike filter: drop every member of every kind "true" or
     "accent" set, read through tools/soundalikes.py (never the raw table file).
  3. The proper-noun assertion (v1 F2 step 6): a capitalised row fails the build.

The source is the compiled tier the app serves (src/word_data.rs; English reads
EN_RETIER_EXPERT from src/word_data_en_retier.rs). English draws from the re-tier tier and
the re-tier table, because v1.2 P2 serves English only while en.retier is on.

Every registry language gets an artifact, so "locked" is something the file SAYS:
zh and ja are locked by v1 D4, a `missing` table locks the language (v1.1 A3), and
a filtered pool under 150 rows is locked as thin (v1 D3). The artifact records
the source tier's wordListHash (the same FNV the bank's TIER_HASHES use), so a
bank edit without a rebuild fails `cargo test` (v1 I9, test A9).

No ranking, no slice, no new rows. This script never writes a bank file.
"""
import json
import pathlib
import re
import sys
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import soundalikes as sa  # noqa: E402  (the one Python API for sound-alike tables)

OUT = ROOT / "assets" / "impossible"
FLOOR = 150  # v1 D3, kept by v1.2 P3
DRAWN_ENTRY = ("zh", "ja")  # v1 D4
KINDS = ("true", "accent")  # v1.1 A2: Impossible is stricter than the bonus

_FNV64_OFF = 0xCBF29CE484222325
_FNV64_PRIME = 0x100000001B3
_MASK64 = (1 << 64) - 1


def nfc(s):
    return unicodedata.normalize("NFC", s)


def word_id(word):
    h = _FNV64_OFF
    for b in nfc(word).encode("utf-8"):
        h = ((h ^ b) * _FNV64_PRIME) & _MASK64
    return h


def list_hash(words):
    """Byte-identical to src/wordid.rs compute_list_hash and build-wordlists.py."""
    h = _FNV64_OFF
    for w in words:
        for b in word_id(w).to_bytes(8, "big"):
            h = ((h ^ b) * _FNV64_PRIME) & _MASK64
    return h


def compiled_tier(lang, root):
    """The Expert tier exactly as the app serves it: the generated const in
    src/word_data.rs (sorted, length-capped, deduped by build-wordlists.py), or
    EN_RETIER_EXPERT for English. Reading the .txt would include rows the game
    never serves."""
    if lang == "en":
        path, name = root / "src" / "word_data_en_retier.rs", "EN_RETIER_EXPERT"
    else:
        path, name = root / "src" / "word_data.rs", f"{lang.upper()}_EXPERT"
    m = re.search(rf"pub const {name}: &\[&str\] = &\[(.*?)\];", path.read_text(encoding="utf-8"), re.S)
    if not m:
        raise SystemExit(f"impossible pools: {name} not found in {path.relative_to(root)}")
    return json.loads("[" + m.group(1) + "]")


def lines(path):
    if not path.exists():
        return []
    return [l.strip() for l in path.read_text(encoding="utf-8").splitlines() if l.strip() and not l.startswith("#")]


def build(lang, root=ROOT):
    variant, label = ("retier", "en-retier-expert") if lang == "en" else (None, "expert")
    table = sa.soundalikes(lang, root=root, variant=variant)
    doc = {
        "lang": lang,
        "source": label,
        "bankHash": None,
        "expertRows": 0,
        "table": table["status"],
        "removedPurity": 0,
        "removedSoundalike": 0,
        "status": "locked",
        "reason": None,
        "words": [],
    }
    if lang in DRAWN_ENTRY:
        doc["reason"] = "drawn-entry"
        return doc
    expert = compiled_tier(lang, root)
    doc["bankHash"] = f"0x{list_hash(expert):016X}"
    doc["expertRows"] = len(expert)
    if table["status"] == "missing":
        doc["reason"] = "table-missing"
        return doc
    awaiting = {nfc(w) for w in lines(root / "assets" / "words" / "purity" / "awaiting-c6" / f"{lang}.txt")}
    members = {nfc(m).lower() for s in sa.sets_of_kind(lang, KINDS, root=root, variant=variant) for m in s}
    pool = []
    for w in expert:
        if nfc(w) in awaiting:
            doc["removedPurity"] += 1
        elif nfc(w).lower() in members:
            doc["removedSoundalike"] += 1
        else:
            pool.append(nfc(w))
    proper = [w for w in pool if w[:1] != w[:1].lower()]
    if proper:
        raise SystemExit(f"impossible pools: {lang} has capitalised rows (proper-noun assertion, v1 F2 step 6): {proper[:5]}")
    doc["words"] = pool
    if len(pool) < FLOOR:
        doc["reason"] = "thin"
    else:
        doc["status"] = "pool"
    return doc


def render(doc):
    return json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


def main(argv):
    check = "--check" in argv
    bad = []
    for lang in sa.LANGS:
        path = OUT / f"impossible_pool.{lang}.json"
        text = render(build(lang))
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                bad.append(f"{path.relative_to(ROOT)} is stale: run python3 tools/impossible/build_pools.py")
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    # v1.1 A2.5: the removal count equals the shared census report's, exactly.
    report = json.loads((ROOT / "reports" / "soundalikes-report.json").read_text(encoding="utf-8"))["languages"]
    for lang in sa.LANGS:
        doc = build(lang)
        want = report.get("en+retier" if lang == "en" else lang, {}).get("impossibleFilterRemovesExpert")
        if doc["table"] != "missing" and lang not in DRAWN_ENTRY and want != doc["removedSoundalike"]:
            bad.append(f"{lang}: the pool removed {doc['removedSoundalike']} sound-alike rows; the shared report says {want} "
                       "(run tools/soundalikes_report.py, then this script)")
    if bad:
        print("impossible pools: FAILED", *bad, sep="\n  ", file=sys.stderr)
        return 1
    if not check:
        for lang in sa.LANGS:
            d = json.loads((OUT / f"impossible_pool.{lang}.json").read_text(encoding="utf-8"))
            print(f"{lang:4} {d['status']:6} {str(d['reason'] or ''):14} expert={d['expertRows']:5} "
                  f"purity-={d['removedPurity']} soundalike-={d['removedSoundalike']} pool={len(d['words'])}")
    else:
        print("impossible pools: OK (all 15 artifacts match a fresh build)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
