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
import html, json, os, re, sys, threading, unicodedata, urllib.parse, urllib.request
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
# A form-of gloss is a POINTER, and the two kinds of pointer are not alike.
#
#   INFLECTION -- "plural of ano", "simple past of take", "feminine of dois".
#   The bank word is a DIFFERENT form of a different word. A player shown
#   "plural: a year" who writes anos has shown real knowledge, and 2,029 bank
#   rows across 8 languages had no definition at all while these were dropped.
#
#   VARIANT -- "alternative spelling of ki", "misspelling of", "romanization
#   of". The bank word is the SAME word spelled another way, which in a
#   SPELLING game means two correct spellings behind one meaning. Every
#   Vietnamese form-of row is of this kind, which is why vi gains nothing here
#   and must not: ky/ki is the hazard, not the opportunity.
#
# Order matters: VARIANT is tested first, so "alternative plural form of" is
# rejected rather than read as a plural.
FORM_OF_PARTS = re.compile(r"^(.*?)\s+of\s+(.+?)\s*$", re.S)
VARIANT_REL = (
    "alternative", "misspelling", "romanization", "pronunciation spelling",
    "clipping", "contraction", "abbreviation", "initialism", "acronym",
    "synonym", "apocopic", "obsolete form", "archaic form",
)
# The relation names a part-of-speech FAMILY, and the stem's gloss must belong
# to it. The cache holds ONE gloss per word, so a stem can be cached under a
# POS the relation cannot inflect: `used` ("simple past and past participle of
# use") resolved against the cached NOUN sense of `use` and composed as
# "simple past and past participle: The act of using." Caught in preview.
# A verb relation demands a verb stem; everything else demands a non-verb one.
VERB_REL = (
    "past", "present", "participle", "gerund", "person", "infinitive",
    "verbal noun", "imperative", "subjunctive", "indicative",
)
VERB_POS = {"verb", "participle"}
# Adverbs, conjunctions and prepositions do not take a plural or a gender, so
# a nominal relation pointing at one is a mis-keyed cache entry, not a form:
# `cuales` is the plural of the relative pronoun cual ("which"), but cual was
# cached as an ADVERB, and composing them read "plural: like, as, in the manner
# of". Caught in preview.
NOMINAL_POS = {
    "noun", "proper noun", "pronoun", "numeral", "determiner", "adjective",
}
# A stem gloss can itself be PART cross-reference, in a shape FORM_OF does not
# describe because the relation is not one it lists: `legen` glosses as
# "causative of liegen: to lay; to put", which composed into the nested
# "past participle: causative of liegen: to lay". One relation per row.
# The shape is a pointer followed by a colon and the real gloss, which is how
# Wiktionary writes a derived form. Anchoring on the COLON is what keeps an
# ordinary definition safe: "a unit of length" has no colon and is a meaning.
NESTED_REL = re.compile(r"^[^:]{0,60}\bof\b[^:]{0,60}:", re.I)
INFLECT_REL = (
    "plural", "singular", "verbal noun", "inflection", "feminine", "masculine",
    "neuter", "diminutive", "augmentative", "past", "present", "gerund",
    "participle", "person", "genitive", "nominative", "accusative", "dative",
    "comparative", "superlative", "agent noun", "attributive",
)
# Mirrors the JUNK table in scripts/def-pool-check.mjs. The check re-derives it
# from the shipped artifact on purpose (a hand-edit must not be able to promote
# a bad row); this copy keeps the builder from emitting one in the first place.
# It matters here because a composed definition inherits its STEM's gloss, and
# a stem can carry the same junk a bank word can.
JUNK = (
    re.compile(r"^(?:Former )?ISO \d", re.I),
    re.compile(r"\bISO 639\b", re.I),
    re.compile(r"^Symbol for\b", re.I),
    re.compile(r"mw-parser-output"),
    re.compile(r"\{[^}]*:[^}]*\}"),
    re.compile(r"^Terms relating to\b", re.I),
)


def junky(text):
    return not text.strip() or "\n" in text or any(p.search(text) for p in JUNK)


def resolve_form_of(gloss, real):
    """An inflection whose STEM has a real definition becomes a definition.

    Returns (relation, stem_definition) or None.

    The stem's WORD is deliberately left OUT of what the caller composes.
    `/api/defpool` feeds definition-match, where the definition is the PROMPT
    and the word is the answer, so "plural of ano" would make the card
    pickable by anyone who can spot a suffix without knowing any Spanish.
    "plural: a year" still names the meaning and still separates the row from
    its own stem's row, which is the whole job.
    """
    m = FORM_OF_PARTS.match(gloss)
    if not m:
        return None
    rel = re.sub(r"\s+", " ", m.group(1).strip().lower())
    stem = m.group(2).strip().strip(".")
    if not rel or any(v in rel for v in VARIANT_REL):
        return None
    if not any(i in rel for i in INFLECT_REL):
        return None
    base = real.get(stem)
    if not base:
        return None
    stem_pos = (base.get("pos") or "").lower()
    if any(v in rel for v in VERB_REL):
        # An unknown POS cannot be confirmed as a verb, so it is not one.
        if stem_pos not in VERB_POS:
            return None
    elif stem_pos and stem_pos not in NOMINAL_POS:
        return None
    text = base.get("definition", "")
    # A stem cached under the OLD looser regex carries form_of=False while its
    # text is still a pointer. Composing against it would nest one cross-
    # reference inside another ("genitive: plural of X").
    if junky(text) or FORM_OF.match(text) or NESTED_REL.match(text):
        return None
    return rel, text


TAG = re.compile(r"<[^>]+>")
# Wiktionary wraps a topical label around the real senses ("Terms relating to
# animals." before the Felidae gloss) and inlines CSS for its date/usage tags.
# Both reach the player as definition text if they are not removed — 883 rows
# shipped that way, caught by the CC-CONTRIBUTE census 2026-09-30.
STYLE = re.compile(r"<style\b.*?</style>", re.S | re.I)
LABEL = re.compile(r'<span[^>]*class="[^"]*use-with-mention[^"]*"[^>]*>.*?</span>', re.S | re.I)
# A bank word that doubles as an ISO code, an SI symbol or an affix has that
# sense listed FIRST in the en section, so first-sense-wins picked it: "cat"
# came out as the language code for Catalan, "sun" for Sundanese, and 185 more
# of the commonest words in the game. Never the sense a speller means — these
# are a fallback, used only when the word has no lexical sense at all.
NON_LEXICAL = {"symbol", "letter", "abbreviation", "initialism", "acronym",
               "punctuation mark", "romanization", "syllable", "prefix",
               "suffix", "infix", "interfix", "character", "number"}
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
    return html.unescape(TAG.sub("", t or "")).strip()


def sense_text(raw):
    """The first real sense: topical label and inline CSS removed, sub-senses
    left behind. A few function words keep their whole gloss inside the label,
    so if stripping it leaves nothing with a letter in it, take the label back."""
    for candidate in (LABEL.sub("", STYLE.sub("", raw or "")), STYLE.sub("", raw or "")):
        for line in html.unescape(TAG.sub("", candidate)).split("\n"):
            line = " ".join(line.split())
            if any(ch.isalpha() for ch in line):
                return line
    return ""


def stale(r):
    """Cached before the label/CSS/symbol fixes (2026-09-30) — re-fetch it."""
    if not r.get("found"):
        return False
    d = r.get("definition", "")
    return ("\n" in d or "mw-parser-output" in d
            or (r.get("pos") or "").lower() in NON_LEXICAL)


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
            definition = sense_text(d.get("definition", ""))
            if not definition:
                continue
            if FORM_OF.match(definition) or pos in NON_LEXICAL:
                fallback = fallback or {"word": word, "found": True, "pos": pos, "definition": definition,
                                        "form_of": FORM_OF.match(definition) is not None}
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
        todo = [w for w in dict.fromkeys(all_words)
                if w not in cache or stale(cache[w])]
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

    # The stems an inflection can resolve against: every cached gloss that is
    # a MEANING rather than a pointer. Keyed on the whole cache, not the bank,
    # because a stem need not itself be a bank word (134 of them are not).
    real = {w: r for w, r in cache.items()
            if r.get("found") and not r.get("form_of") and r.get("definition")}

    pools, exclusions = {}, {}
    for tier in TIERS:
        rows = []
        stems = {}
        for w in words[tier]:
            r = cache.get(w)
            if not r or not r.get("found"):
                continue
            d = unicodedata.normalize("NFC", r["definition"])
            composed = False
            # Keyed on the TEXT, not only the flag: rows cached under the old,
            # looser regex carry form_of=False while still reading as pointers,
            # and both kinds get the same treatment -- resolved, or dropped.
            if r.get("form_of") or FORM_OF.match(d):
                res = resolve_form_of(d, real)
                if not res:
                    continue  # an unresolvable pointer is not a definition
                rel, base = res
                d = f"{rel}: {unicodedata.normalize('NFC', base)}"
                composed = True
                stems[w] = FORM_OF_PARTS.match(
                    unicodedata.normalize("NFC", r["definition"])).group(2).strip().strip(".")
            if blocked(d) or junky(d):
                continue
            wl = w.lower()
            leak = re.search(rf"(?<!\w){re.escape(wl)}(?!\w)", d.lower()) is not None
            prompt = bool(d) and len(d) <= 90 and not leak
            rows.append({
                "word": w,
                "definition": d,
                "pos": r.get("pos", ""),
                "prompt_grade": prompt,
                # Composed rows stay OUT of the kid register. The relation is
                # the player's only clue to which form is wanted, and it is
                # grammar jargon -- "third-person singular present" is not a
                # prompt for a six-year-old in Spell Jr. Eric's to relax.
                "kid_register": (not composed) and tier in ("easy", "medium") and len(d) <= 60,
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
        # A composed inflection and its own stem are ONE meaning in two forms,
        # and the near-miss rule above cannot see it: the stem's word is kept
        # out of the composed text on purpose, so no token ever matches. Pair
        # them here, or "a year" and "plural: a year" sit in one tier as two
        # defensible answers to each other's card.
        present = {row["word"] for row in rows}
        for w, stem in stems.items():
            if stem != w and stem in present:
                exclusions.setdefault(w, set()).add(stem)
                exclusions.setdefault(stem, set()).add(w)
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
