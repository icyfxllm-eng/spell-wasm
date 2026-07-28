#!/usr/bin/env python3
"""Definition pools for CC-DEF-MATCH (Eric's directive 2026-07-27: "add the
definitions not already added... then proceed with the definition match mode").

For every bank word in every language: fetch its gloss from en.wiktionary's
REST definition endpoint (the same source the app's runtime /api/meaning proxy
uses; zh comes from the bundled CC-CEDICT extract instead), then apply the
MECHANICAL prescreen and emit backend/def_pools/<lang>.json for the backend's
/api/defpool route and the Rust core's Round engine.

PRESCREEN (provisional stand-in for the Gig A prompt-grade column and the
CC-DEF-PRECHECK sweep — Eric-approved interim content, formal native audit
still open, same posture as the ar/hi banks):
  * prompt_grade=False when: definition empty, >90 chars, contains the target
    word itself (spelling leak / self-reference), or byte-identical to another
    word's definition in the same (lang, tier) — ambiguous rounds.
  * rows whose definition hits backend/blocklist.txt are DROPPED entirely.
  * kid_register: easy/medium rows with definitions <= 60 chars.
  * exclusions (near-miss stand-in): A excludes B when B's word appears as a
    whole token inside A's definition, or their definitions are identical.

Incremental: fetched glosses cache to .corpus-cache/defs/<lang>.jsonl — reruns
only fetch words not yet cached. ~10 concurrent requests, polite UA.

Usage: python3 scripts/build-def-pools.py [langs...]   (default: all)
"""
import json, os, re, sys, threading, unicodedata, urllib.parse, urllib.request
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, ".corpus-cache", "defs")
OUT = os.path.join(ROOT, "backend", "def_pools")
TIERS = ["easy", "medium", "hard", "expert"]
SECTION = {
    "en": "en", "es": "es", "fr": "fr", "de": "de", "pt": "pt", "pl": "pl",
    "vi": "vi", "ko": "ko", "ja": "ja", "ru": "ru", "ar": "ar", "hi": "hi",
    "sw": "sw", "fil": "tl",
}
# Grammar cross-references are not definitions. Matches compound leads too
# ("simple past AND past participle of…" — caught live 2026-07-27).
FORM_OF = re.compile(
    r"^(?:(?:simple |archaic |dated |obsolete |informal |nonstandard )*"
    r"(?:verbal noun|plural|singular|inflection|alternative(?: form| spelling)?|"
    r"romanization|feminine|masculine|neuter|diminutive|augmentative|misspelling|"
    r"past(?: tense)?|past participle|present(?: tense| participle)?|gerund|"
    r"third-person singular|first-person singular|second-person singular|"
    r"genitive|nominative|accusative|dative|comparative|superlative|"
    r"agent noun|attributive form|clipping|contraction|abbreviation|initialism|"
    r"acronym|synonym|apocopic form|pronunciation spelling)"
    r"(?:\s*(?:,|and|or)\s*)?)+ of\b",
    re.IGNORECASE,
)
TAG = re.compile(r"<[^>]+>")
DEF_URL = "https://en.wiktionary.org/api/rest_v1/page/definition/{}"

_print_lock = threading.Lock()


def bank_words(lang):
    out = {}
    if lang == "zh":
        src = open(os.path.join(ROOT, "src", "words.rs"), encoding="utf-8").read()
        # tier -> hanzi from the ZH_* consts, in order.
        for tier in TIERS:
            m = re.search(rf"pub const ZH_{tier.upper()}: &\[&str\] = &\[(.*?)\];", src, re.S)
            out[tier] = [e.split("|")[1] for e in re.findall(r'"([^"]+)"', m.group(1)) if "|" in e]
        return out
    for tier in TIERS:
        p = os.path.join(ROOT, "assets", "words", lang, f"{tier}.txt")
        out[tier] = [w.strip() for w in open(p, encoding="utf-8") if w.strip() and not w.startswith("#")]
    return out


def strip_html(t):
    import html
    return html.unescape(TAG.sub("", t or "")).strip()


def fetch_one(lang, word):
    """None = transient failure (NOT cached, retried next run); found=False is
    cached ONLY for a definitive 404."""
    import time, urllib.error
    section = SECTION[lang]
    url = DEF_URL.format(urllib.parse.quote(word, safe=""))
    req = urllib.request.Request(url, headers={"User-Agent": "SpellGame/1.0 (spellgame.net; defpool build; contact icyfxllm@gmail.com)"})
    data = None
    for attempt in range(6):
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            break
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return {"word": word, "found": False}
            time.sleep(2 ** attempt)  # 429/5xx: back off 1..32s
        except Exception:
            time.sleep(2 ** attempt)
    if data is None:
        return None
    fallback = None
    for entry in data.get(section, []):
        pos = (entry.get("partOfSpeech") or "").lower()
        for d in entry.get("definitions", []):
            definition = strip_html(d.get("definition", ""))
            if not definition:
                continue
            if FORM_OF.match(definition):
                fallback = fallback or {"word": word, "found": True, "pos": pos, "definition": definition, "form_of": True}
                continue
            return {"word": word, "found": True, "pos": pos, "definition": definition, "form_of": False}
    return fallback or {"word": word, "found": False}


def load_cache(lang):
    path = os.path.join(CACHE, f"{lang}.jsonl")
    cache = {}
    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            try:
                r = json.loads(line)
                cache[r["word"]] = r
            except ValueError:
                pass
    return cache


def build_lang(lang):
    os.makedirs(CACHE, exist_ok=True)
    os.makedirs(OUT, exist_ok=True)
    words = bank_words(lang)
    all_words = [w for tier in TIERS for w in words[tier]]

    if lang == "zh":
        glosses = json.load(open(os.path.join(ROOT, "backend", "zh_glosses.json"), encoding="utf-8"))
        cache = {w: ({"word": w, "found": True, "pos": "", "definition": glosses[w]["definition"], "form_of": False}
                     if w in glosses else {"word": w, "found": False}) for w in all_words}
    else:
        cache = load_cache(lang)
        todo = [w for w in dict.fromkeys(all_words) if w not in cache]
        if todo:
            path = os.path.join(CACHE, f"{lang}.jsonl")
            failed = 0
            with open(path, "a", encoding="utf-8") as f, ThreadPoolExecutor(4) as ex:
                done = 0
                for r in ex.map(lambda w: fetch_one(lang, w), todo):
                    done += 1
                    if r is None:
                        failed += 1
                        continue  # transient — NOT cached; a rerun retries it
                    cache[r["word"]] = r
                    f.write(json.dumps(r, ensure_ascii=False) + "\n")
                    if done % 500 == 0:
                        with _print_lock:
                            print(f"    {lang}: {done}/{len(todo)} ({failed} transient)", flush=True)
            if failed:
                with _print_lock:
                    print(f"    {lang}: {failed} transient failures — rerun to retry", flush=True)

    # Blocklist screen for definition TEXT (words themselves already screened).
    block = set()
    bl = os.path.join(ROOT, "backend", "blocklist.txt")
    if os.path.exists(bl):
        block = {l.strip().lower() for l in open(bl, encoding="utf-8") if l.strip() and not l.startswith("#")}

    def blocked(text):
        toks = re.findall(r"[\w']+", text.lower())
        return any(t in block for t in toks)

    pools, exclusions = {}, {}
    for tier in TIERS:
        rows = []
        for w in words[tier]:
            r = cache.get(w)
            if not r or not r.get("found") or r.get("form_of"):
                continue
            d = unicodedata.normalize("NFC", r["definition"])
            if FORM_OF.match(d):
                continue  # cached under the old, looser regex — drop now
            if blocked(d):
                continue
            wl = w.lower()
            leak = re.search(rf"(?<!\w){re.escape(wl)}(?!\w)", d.lower()) is not None
            prompt = bool(d) and len(d) <= 90 and not leak
            rows.append({
                "word": w,
                "definition": d,
                "pos": r.get("pos", ""),
                "prompt_grade": prompt,
                "kid_register": tier in ("easy", "medium") and len(d) <= 60,
            })
        # Ambiguity: identical definitions in one tier → none is prompt-grade,
        # and each excludes the others (they'd be two right answers).
        seen = {}
        for row in rows:
            seen.setdefault(row["definition"].lower(), []).append(row)
        for group in seen.values():
            if len(group) > 1:
                for row in group:
                    row["prompt_grade"] = False
                    exclusions.setdefault(row["word"], set()).update(
                        g["word"] for g in group if g["word"] != row["word"])
        # Near-miss: candidate's word appearing inside the target's definition.
        vocab = {row["word"].lower(): row["word"] for row in rows}
        for row in rows:
            for tok in re.findall(r"[\w']+", row["definition"].lower()):
                hit = vocab.get(tok)
                if hit and hit != row["word"]:
                    exclusions.setdefault(row["word"], set()).add(hit)
                    exclusions.setdefault(hit, set()).add(row["word"])
        pools[tier] = rows

    out = {
        "lang": lang,
        "tiers": pools,
        "exclusions": {k: sorted(v) for k, v in exclusions.items()},
    }
    with open(os.path.join(OUT, f"{lang}.json"), "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False)
    counts = {t: sum(1 for r in pools[t] if r["prompt_grade"]) for t in TIERS}
    with _print_lock:
        print(f"  {lang}: prompt-grade per tier {counts} (floor 40)", flush=True)
    return counts


def main():
    langs = sys.argv[1:] or list(SECTION) + ["zh"]
    summary = {}
    for lang in langs:
        summary[lang] = build_lang(lang)
    print("\nDEF-POOL SUMMARY (prompt-grade rows; D3 floor = 40/tier):")
    for lang, c in summary.items():
        active = [t for t in TIERS if c[t] >= 40]
        print(f"  {lang}: {c}  active tiers: {active or 'NONE'}")


if __name__ == "__main__":
    main()
