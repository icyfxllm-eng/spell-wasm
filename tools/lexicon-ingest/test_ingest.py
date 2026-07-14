"""Scaffold tests — run: python3 test_ingest.py (no pytest dependency).

Covers: normalization per language, charset validation, content-filter drop,
merge, determinism, and the homophone-collision grouping the P1 pass needs.
Uses tiny in-memory fixtures + the shipped lists (migration path).
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from schema import Entry, merge, validate
from normalize import normalize, charset_ok
from parsers.kaikki import compress_etymology, ETY_MAX_LEN
import ingest

FAILS = []


def check(cond, msg):
    if not cond:
        FAILS.append(msg)


# --- normalization mirrors the Rust engines ---
check(normalize("Ping2Guo3", "zh") == "ping2guo3", "zh case+form")
check(normalize("lv4", "zh") == "lü4", "zh v->ü")
check(normalize("de5", "zh") == "de", "zh neutral-5 dropped")
check(normalize("Mag-Aral", "fil") == "mag-aral", "fil case, keep hyphen")
check(normalize("piña", "fil") == normalize("piña", "fil"), "fil ñ NFC fold")
check(normalize("한국", "ko") == "한국", "ko NFC passthrough")

# --- charset validation ---
check(charset_ok("ping2guo3", "zh"), "zh pinyin typeable")
check(not charset_ok("café", "en"), "en rejects é")
check(charset_ok("mag-aral", "fil"), "fil hyphen ok")
check(charset_ok("น้ำ", "th"), "th combining marks ok")
check(charset_ok("한국", "ko"), "ko syllable ok")

# --- content filter drops blocklist words ---
roots, exact = ingest.load_exclusions("fil")
check(ingest.is_blocked("puta", roots, exact), "fil blocks 'puta'")
check(not ingest.is_blocked("puti", roots, exact), "fil keeps 'puti' (boundary-safe)")

# --- merge unions list fields, keeps best rank ---
a = Entry(word="w", lang="en", pos=["noun"], freq_rank=None, sources=["x"])
b = Entry(word="w", lang="en", pos=["verb"], freq_rank=42, gloss="g", sources=["y"])
m = merge(a, b)
check(set(m.pos) == {"noun", "verb"}, "merge unions pos")
check(m.freq_rank == 42, "merge keeps rank")
check(m.gloss == "g" and set(m.sources) == {"x", "y"}, "merge scalars+sources")

# --- homophone grouping (P1): same pron, different word ---
entries = [
    Entry(word="their", lang="en", pron="DH EH R"),
    Entry(word="there", lang="en", pron="DH EH R"),
    Entry(word="here", lang="en", pron="HH IH R"),
]
groups = {}
for e in entries:
    groups.setdefault(e.pron, []).append(e.word)
homophones = {p: ws for p, ws in groups.items() if len(ws) > 1}
check(homophones == {"DH EH R": ["their", "there"]}, "homophone group detected")

# --- determinism: migrate ja twice, identical bytes ---
e1, _ = ingest.ingest("ja", [("plainlist", None)])
e2, _ = ingest.ingest("ja", [("plainlist", None)])
check([x.to_json() for x in e1] == [x.to_json() for x in e2], "deterministic ingest")
check(len(e1) > 100, "ja migration non-empty")

# --- F5 word stories: etymology compression (mirrors src/word_stories.rs) ---
check(compress_etymology(None) is None, "ety None -> None")
check(compress_etymology("") is None, "ety empty -> None")
# One first-hop sentence: cut at the em dash / semicolon.
check(compress_etymology("From Dutch 'jacht' (a hunt) — a fast ship; kept Dutch.")
      == "From Dutch 'jacht' (a hunt)", "ety first hop only")
# Chains: keep only up to the second 'from'.
_chain = "From Latin iudicare from Proto-Italic from Proto-Indo-European deik"
check(compress_etymology(_chain) == "From Latin iudicare", "ety drops chains")
check(compress_etymology(_chain).lower().count("from") == 1, "ety one hop")
# 120-char cap on a word boundary + ellipsis.
_long = "From Latin " + ("verylongrootword " * 20)
_out = compress_etymology(_long)
check(len(_out) <= ETY_MAX_LEN, "ety <=120 chars")
check(_out.endswith("…"), "ety ellipsis on truncation")
# NFC: decomposed é folds to precomposed.
check("\u00e9" in compress_etymology("From French e\u0301tude"), "ety NFC folds e-acute")
check("\u0301" not in compress_etymology("From French e\u0301tude"), "ety no combining mark")
# Etymology rides through the schema (merge prefers first non-None).
_a = Entry(word="w", lang="en", etymology=None)
_b = Entry(word="w", lang="en", etymology="From Latin x")
check(merge(_a, _b).etymology == "From Latin x", "merge carries etymology")
check("etymology" in _b.to_json(), "etymology serialized")

# --- validate rejects bad charset/POS ---
check(validate(Entry(word="café", lang="en"), lambda w: charset_ok(w, "en")), "validate flags é")
check(not validate(Entry(word="cat", lang="en"), lambda w: charset_ok(w, "en")), "validate accepts clean")

if FAILS:
    print(f"lexicon-ingest tests: {len(FAILS)} FAILED")
    for f in FAILS:
        print("  ✗ " + f)
    sys.exit(1)
print("lexicon-ingest tests: OK — all passed")
