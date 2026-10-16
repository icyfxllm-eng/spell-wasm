"""CC-EN-EXPERT-GROWTH shared helpers. Build-time only: nothing here ships in the app."""
import hashlib, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TIERS = ["easy", "medium", "hard", "expert"]
CMU = os.path.join(ROOT, "tools/wordpipe/sources/cmudict.dict")
WORDFREQ_VERSION = "3.1.1"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_text(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def bank_rows(root=ROOT):
    """tier -> list of words, bank order, from assets/words/en/<tier>.txt."""
    out = {}
    for t in TIERS:
        p = os.path.join(root, "assets/words/en", t + ".txt")
        out[t] = [l.strip() for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")]
    return out


def bank_hash(root=ROOT):
    b = bank_rows(root)
    return sha256_text("\n".join(f"{t}\t{w}" for t in TIERS for w in b[t]))


def load_cmu(path=CMU):
    """word -> sorted list of pronunciations (tuples of ARPAbet, stress stripped)."""
    d = {}
    for line in open(path, encoding="utf-8", errors="ignore"):
        if not line.strip() or line.startswith(";;;"):
            continue
        p = line.split()
        w = p[0].lower()
        if w.endswith(")"):
            w = w[: w.rindex("(")]
        if not w.isalpha():
            continue
        d.setdefault(w, set()).add(tuple(re.sub(r"\d", "", x) for x in p[1:]))
    return {w: sorted(v) for w, v in d.items()}


def zipf_fn():
    """wordfreq is a pinned build-time input (Apache-2.0 code, CC BY-SA 4.0 data); it never ships."""
    try:
        import wordfreq
        from importlib.metadata import version
    except ImportError:
        sys.exit("wordfreq missing: use .corpus-cache/wf/v/bin/python (see tools/en_growth/README.md)")
    v = version("wordfreq")
    if v != WORDFREQ_VERSION:
        sys.exit(f"wordfreq {v} != pinned {WORDFREQ_VERSION}")
    return lambda w: wordfreq.zipf_frequency(w, "en")
