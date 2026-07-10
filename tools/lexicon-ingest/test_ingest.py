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

# --- validate rejects bad charset/POS ---
check(validate(Entry(word="café", lang="en"), lambda w: charset_ok(w, "en")), "validate flags é")
check(not validate(Entry(word="cat", lang="en"), lambda w: charset_ok(w, "en")), "validate accepts clean")

if FAILS:
    print(f"lexicon-ingest tests: {len(FAILS)} FAILED")
    for f in FAILS:
        print("  ✗ " + f)
    sys.exit(1)
print("lexicon-ingest tests: OK — all passed")
