#!/usr/bin/env python3
"""CC-HUMAN-AUDIO Phase A: Lingua Libre coverage census. Report only.

Build-time tool for Eric's machine. The Commons API is a network call, so none
of this runs in CI and nothing here is read by the app (I9).

Stages, each cached under --cache so a rerun resumes rather than refetching:

  titles    every file title in Category:Lingua Libre pronunciation-<iso>
  meta      for titles whose word matches a bank entry: license, speaker item,
            sha1, duration, read from Commons file metadata (never assumed, D1)
  speakers  each speaker's Lingua Libre item: native level and place of
            learning for that language (I8)
  places    each place's country, from Wikidata
  homographs  the D5 hold-out sets (ru kaikki, zh CC-CEDICT, ja JMdict,
            en CMUdict), then score every candidate into --out JSON

render.py turns that JSON into the tables of docs/census/human_audio_census.md.

The bank comes from `cargo test --lib human_audio_census -- --ignored`, which
reads it through words::tier_for, the accessor the game serves from.

Run:  python3 tools/human-audio-census/census.py --bank bank.tsv --cache DIR \
        --repo MAIN_CHECKOUT --kaikki-ru PATH --out census.json all
      python3 tools/human-audio-census/render.py census.json GAP_DIR > tables.md
"""
import argparse
import collections
import html
import json
import pathlib
import re
import sys
import time
import unicodedata
import urllib.parse
import urllib.request

UA = "SpellGameCensus/0.1 (build-time coverage census; icyfxllm@gmail.com)"
COMMONS = "https://commons.wikimedia.org/w/api.php"
WIKIDATA = "https://www.wikidata.org/w/api.php"
LL_ENTITY = "https://lingualibre.org/wiki/Special:EntityData/{}.json"
LL_NATIVE = "Q15"  # Lingua Libre item "native" (language level, P16)

AUDITOR_VARIETY = "auditor"  # variety decided in F3, not from speaker metadata

# App language -> (Lingua Libre ISO 639-3 categories, shipping variety tag,
# Wikidata country ids that match it, or None when a country cannot decide it).
# The variety tag is the backend TTS locale (backend/app.py LANG voice table).
LANGS = {
    "en": (["eng"], "en-US", {"Q30"}),
    "es": (["spa"], "es-ES", {"Q29"}),
    "fr": (["fra"], "fr-FR", {"Q142"}),
    "de": (["deu"], "de-DE", {"Q183"}),
    "pt": (["por"], "pt-BR", {"Q155"}),
    "pl": (["pol"], "pl-PL", {"Q36"}),
    "vi": (["vie"], "vi-VN", {"Q881"}),
    "ko": (["kor"], "ko-KR", {"Q884"}),
    "ja": (["jpn"], "ja-JP", {"Q17"}),
    # Tagalog-labelled recordings count as Filipino (Eric, 2026-09-18).
    "fil": (["fil", "tgl"], "fil-PH", {"Q928"}),
    "zh": (["cmn", "zho"], "cmn-CN", {"Q148"}),
    "ru": (["rus"], "ru-RU", {"Q159"}),
    # ar-XA is Modern Standard Arabic, kept as the shipping variety (Eric,
    # 2026-09-18). No country declares MSA, so the variety check moves to the
    # auditor: any native Arabic speaker's clip is a candidate, and F3 accepts
    # it only as a correct MSA citation form (Eric, 2026-09-18).
    "ar": (["ara"], "ar-XA (MSA)", AUDITOR_VARIETY),
    # sw-TZ per the backend voice table, not the sw-KE at backend/app.py:686
    # (Eric, 2026-09-18).
    "sw": (["swa"], "sw-TZ", {"Q924"}),
    "hi": (["hin"], "hi-IN", {"Q668"}),
}
TIERS = ["easy", "medium", "hard", "expert"]

ALLOWED = {"cc0": "CC0", "cc-by": "CC BY", "cc-by-sa": "CC BY-SA"}


def nfc(s):
    return unicodedata.normalize("NFC", s)


PACE = 0.5  # seconds between API calls; Commons returns 429 when pushed


def get(url, params, tries=12):
    q = dict(params, format="json", formatversion="2")
    if "commons" in url or "wikidata" in url:
        q["maxlag"] = "5"
    full = url + "?" + urllib.parse.urlencode(q)
    for i in range(tries):
        time.sleep(PACE)
        try:
            req = urllib.request.Request(full, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=60) as r:
                d = json.loads(r.read())
            if isinstance(d, dict) and d.get("error", {}).get("code") == "maxlag":
                time.sleep(5 * (i + 1))
                continue
            return d
        except Exception as e:  # rate limits and blips: back off, then give up loudly
            if i == tries - 1:
                raise
            wait = min(300, 10 * 2 ** min(i, 5))
            ra = getattr(e, "headers", None) and e.headers.get("Retry-After")
            if ra and ra.isdigit():
                wait = max(wait, int(ra))
            print(f"  retry {i + 1} in {wait}s after {e!r}", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError(f"gave up after {tries} maxlag responses: {url}")


def load_json(p, default):
    return json.loads(p.read_text()) if p.exists() else default


def save_json(p, v):
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(v, ensure_ascii=False))
    tmp.replace(p)


# ---------------------------------------------------------------- bank

def load_bank(path):
    """lang -> tier -> [(entry, match_key)]. zh stores 'pinyin|hanzi'; Lingua
    Libre titles are hanzi, so hanzi is the key."""
    bank = collections.defaultdict(lambda: collections.defaultdict(list))
    for line in pathlib.Path(path).read_text().splitlines():
        lang, tier, entry = line.split("\t")
        key = entry.split("|")[1] if lang == "zh" and "|" in entry else entry
        bank[lang][tier].append((entry, nfc(key)))
    return bank


# ---------------------------------------------------------------- titles

def stage_titles(cache):
    for lang, (isos, _, _) in LANGS.items():
        for iso in isos:
            out = cache / f"titles-{iso}.json"
            if out.exists():
                continue
            part = cache / f"titles-{iso}.partial.json"  # checkpoint: resume mid-category
            st = load_json(part, {"titles": [], "cont": {}})
            titles, cont = st["titles"], st["cont"]
            while True:
                d = get(COMMONS, {"action": "query", "list": "categorymembers",
                                  "cmtitle": f"Category:Lingua Libre pronunciation-{iso}",
                                  "cmtype": "file", "cmlimit": "500",
                                  "cmprop": "title", **cont})
                titles += [m["title"] for m in d["query"]["categorymembers"]]
                if "continue" not in d:
                    break
                cont = d["continue"]
                if len(titles) % 10000 < 500:
                    save_json(part, {"titles": titles, "cont": cont})
                    print(f"  {iso}: {len(titles)}", file=sys.stderr)
            save_json(out, titles)
            part.unlink(missing_ok=True)
            print(f"titles {iso}: {len(titles)}", file=sys.stderr)


TITLE_RE = re.compile(r"^File:LL-Q\d+ \((?P<iso>[a-z-]+)\)-(?P<rest>.+)\.(?:wav|ogg|flac|mp3|webm)$", re.I)


def split_title(title):
    """'File:LL-Q7737 (rus)-User-word.wav' -> [(user, word), ...] for every
    hyphen split, first (most likely) first. Usernames and words can both
    contain hyphens, so the caller picks the split whose word is in the bank."""
    m = TITLE_RE.match(title)
    if not m:
        return []
    rest = m.group("rest")
    return [(rest[:i], rest[i + 1:]) for i, c in enumerate(rest) if c == "-"]


def candidates(cache, bank):
    """lang -> key -> [title]. A title is a CANDIDATE for every hyphen split
    that leaves exactly a bank key (NFC, case-sensitive: the bank already
    carries natural case, e.g. German nouns). Which split is real is decided
    after the meta stage by the uploader's name (title_word_ok), so
    '...-Arlo Barnes-it-it.wav' and '...-Vealhurl--able.wav' never count as
    recordings of 'it' or 'able'."""
    out = {}
    for lang, (isos, _, _) in LANGS.items():
        keys = {k for tier in bank[lang].values() for _, k in tier}
        hits = collections.defaultdict(list)
        n = 0
        for iso in isos:
            for t in load_json(cache / f"titles-{iso}.json", []):
                n += 1
                for word in {w for _, w in split_title(nfc(t)) if w in keys}:
                    hits[word].append(t)
        out[lang] = (hits, n)
    return out


# ---------------------------------------------------------------- meta

SPEAKER_RE = re.compile(r"Speaker:\s*<a[^>]*lingualibre:(Q\d+)")


def license_key(md):
    raw = (md.get("License", {}).get("value") or "").lower()
    for k in ("cc-by-sa", "cc-by", "cc0"):
        if raw.startswith(k) and "-nc" not in raw and "-nd" not in raw:
            return k, raw
    return None, raw or "unknown"


def stage_meta(cache, cands):
    out = cache / "meta.json"
    meta = load_json(out, {})
    todo = [t for lang in cands for ts in cands[lang][0].values() for t in ts if t not in meta]
    print(f"meta: {len(todo)} to fetch ({len(meta)} cached)", file=sys.stderr)
    for i in range(0, len(todo), 50):
        batch = todo[i:i + 50]
        d = get(COMMONS, {"action": "query", "prop": "imageinfo", "titles": "|".join(batch),
                          "iiprop": "extmetadata|user|sha1|size",
                          "iiextmetadatafilter": "License|LicenseShortName|Artist|Categories"})
        norm = {n["to"]: n["from"] for n in d["query"].get("normalized", [])}
        for p in d["query"]["pages"]:
            title = norm.get(p["title"], p["title"])
            if p.get("missing") or not p.get("imageinfo"):
                meta[title] = {"missing": True}
                continue
            ii = p["imageinfo"][0]
            md = ii.get("extmetadata", {})
            lk, raw = license_key(md)
            sp = SPEAKER_RE.search(md.get("Artist", {}).get("value", ""))
            meta[title] = {"license": lk, "license_raw": raw,
                           "license_short": md.get("LicenseShortName", {}).get("value"),
                           "speaker": sp.group(1) if sp else None,
                           "uploader": ii.get("user"), "sha1": ii.get("sha1"),
                           "duration": ii.get("duration")}
        for t in batch:  # a title the API silently dropped is recorded, not lost
            meta.setdefault(t, {"missing": True})
        if (i // 50) % 40 == 0:
            save_json(out, meta)
            print(f"  meta {i + len(batch)}/{len(todo)}", file=sys.stderr)
    save_json(out, meta)
    return meta


# ---------------------------------------------------------------- speakers

LL_SPARQL = "https://lingualibre.org/bigdata/namespace/wdq/sparql"
LLP = "https://lingualibre.org/prop/"
SPEAKER_Q = """SELECT ?s ?lang ?level ?learn ?res WHERE {{
  VALUES ?s {{ {ids} }}
  OPTIONAL {{ ?s <{p}P4> ?st . ?st <{p}statement/P4> ?lang .
             OPTIONAL {{ ?st <{p}qualifier/P16> ?level }}
             OPTIONAL {{ ?st <{p}qualifier/P15> ?learn }} }}
  OPTIONAL {{ ?s <{p}direct/P14> ?res }}
}}"""


def tail_q(v):
    m = re.search(r"(Q\d+)$", v or "")
    return m.group(1) if m else None


def stage_speakers(cache, meta):
    """Speaker items via Lingua Libre's SPARQL endpoint. Its wiki now 301s
    entity pages to archive.lingualibre.org, which refuses connections
    (2026-09-18), but the query service still serves the same statements."""
    out = cache / "speakers.json"
    sp = {k: v for k, v in load_json(out, {}).items() if "error" not in v}
    todo = sorted({m["speaker"] for m in meta.values() if m.get("speaker")} - set(sp))
    print(f"speakers: {len(todo)} to fetch", file=sys.stderr)
    for i in range(0, len(todo), 80):
        batch = todo[i:i + 80]
        q = SPEAKER_Q.format(ids=" ".join(f"<https://lingualibre.org/entity/{x}>" for x in batch), p=LLP)
        body = urllib.parse.urlencode({"query": q}).encode()
        for attempt in range(6):
            time.sleep(PACE)
            try:
                req = urllib.request.Request(LL_SPARQL, data=body, headers={
                    "User-Agent": UA, "Accept": "application/sparql-results+json",
                    "Content-Type": "application/x-www-form-urlencoded"})
                with urllib.request.urlopen(req, timeout=120) as r:
                    rows = json.loads(r.read())["results"]["bindings"]
                break
            except Exception as e:
                if attempt == 5:
                    raise
                print(f"  sparql retry {attempt + 1} after {e!r}", file=sys.stderr)
                time.sleep(10 * (attempt + 1))
        got = {x: {"langs": {}, "residence": []} for x in batch}
        for b in rows:
            s = got[tail_q(b["s"]["value"])]
            if "res" in b and tail_q(b["res"]["value"]) not in s["residence"]:
                s["residence"].append(tail_q(b["res"]["value"]))
            if "lang" in b:
                li = s["langs"].setdefault(tail_q(b["lang"]["value"]), {"level": [], "learned": []})
                for k, f in (("level", "level"), ("learn", "learned")):
                    v = tail_q(b.get(k, {}).get("value"))
                    if v and v not in li[f]:
                        li[f].append(v)
        sp.update(got)
        save_json(out, sp)
    print(f"speakers: {len(sp)} cached", file=sys.stderr)
    return sp


# ------------------------------------------------------------------ places

def stage_places(cache, sp):
    out = cache / "places.json"
    pl = load_json(out, {})
    qs = set()
    for s in sp.values():
        for l in s.get("langs", {}).values():
            qs.update(l["learned"])
        qs.update(s.get("residence", []))
    todo = sorted(q for q in qs if isinstance(q, str) and re.fullmatch(r"Q\d+", q) and q not in pl)
    print(f"places: {len(todo)} to fetch", file=sys.stderr)
    for i in range(0, len(todo), 50):
        batch = todo[i:i + 50]
        d = get(WIKIDATA, {"action": "wbgetentities", "ids": "|".join(batch),
                           "props": "claims|labels", "languages": "en"})
        for q, e in d.get("entities", {}).items():
            cl = e.get("claims", {})
            # Current country only: a preferred-rank P17 if the place has one,
            # else the normal-rank claims without an end date (P582). Historic
            # states (Republic of New Granada, Kingdom of England) drop out.
            p17 = [c for c in cl.get("P17", []) if c.get("rank") != "deprecated"]
            pref = [c for c in p17 if c.get("rank") == "preferred"]
            cur = pref or [c for c in p17 if "P582" not in c.get("qualifiers", {})] or p17
            country = [c["mainsnak"].get("datavalue", {}).get("value", {}).get("id") for c in cur]
            is_country = any(c["mainsnak"].get("datavalue", {}).get("value", {}).get("id")
                             in ("Q6256", "Q3624078") for c in cl.get("P31", []))
            pl[q] = {"label": e.get("labels", {}).get("en", {}).get("value"),
                     # P17 wins when present: Galicia is typed "country" (Q6256,
                     # in the nation sense) but its P17 is Spain.
                     "country": [c for c in country if c] or ([q] if is_country else [])}
    missing = sorted({c for v in pl.values() for c in v["country"]} - set(pl))
    for i in range(0, len(missing), 50):
        d = get(WIKIDATA, {"action": "wbgetentities", "ids": "|".join(missing[i:i + 50]),
                           "props": "labels", "languages": "en"})
        for q, e in d.get("entities", {}).items():
            pl[q] = {"label": e.get("labels", {}).get("en", {}).get("value"), "country": [q]}
    save_json(out, pl)
    return pl


# ------------------------------------------------------------------ homographs (D5)

RU_STRESS = "́"


def ru_homographs(bank_keys, kaikki):
    """Bank spellings that carry two or more stress placements anywhere in
    kaikki-ru (headwords AND inflected forms): за́мок/замо́к, ру́ки/руки́."""
    stresses = collections.defaultdict(set)
    want = set(bank_keys)

    def note(form):
        f = nfc(form)
        if RU_STRESS not in unicodedata.normalize("NFD", f) and "ё" not in f.lower():
            return
        bare = nfc(unicodedata.normalize("NFD", f).replace(RU_STRESS, ""))
        if bare in want:
            stresses[bare].add(f)

    with open(kaikki, encoding="utf-8") as fh:
        for line in fh:
            try:
                e = json.loads(line)
            except ValueError:
                continue
            if e.get("lang_code") != "ru":
                continue
            for h in e.get("head_templates", []) or []:
                for a in (h.get("args") or {}).values():
                    if isinstance(a, str):
                        note(a)
            for f in e.get("forms", []) or []:
                if isinstance(f.get("form"), str):
                    note(f["form"])
    return {w for w, s in stresses.items() if len(s) >= 2}


def zh_polyphones(bank_keys, cedict):
    """Bank hanzi whose CC-CEDICT entries carry two or more distinct readings
    (tone-sensitive, case-folded): 行 xíng/háng, 长 cháng/zhǎng."""
    readings = collections.defaultdict(set)
    want = set(bank_keys)
    for line in open(cedict, encoding="utf-8"):
        if line.startswith("#"):
            continue
        m = re.match(r"(\S+) (\S+) \[([^\]]+)\]", line)
        if m and m.group(2) in want:
            readings[m.group(2)].add(m.group(3).lower().replace("u:", "v"))
    return {w for w, r in readings.items() if len(r) >= 2}


def en_heteronyms(bank_keys, cmudict):
    """Bank words whose CMUdict pronunciations differ in primary-stress
    position (record, present) or in the primary-stressed vowel (read, lead,
    wind, bow). Variants that differ only in unstressed vowels (accept,
    always) are free variation and not flagged. Still conservative: it also
    catches accent splits (cost, drop), so English usable coverage is a floor."""
    prons = collections.defaultdict(list)
    want = {k.lower() for k in bank_keys}
    for line in open(cmudict, encoding="latin-1"):
        if not line.strip() or line.startswith(";;;"):
            continue
        head, *ph = line.split("#")[0].split()
        w = re.sub(r"\(\d+\)$", "", head).lower()
        if w in want:
            prons[w].append(ph)

    def sig(ph):
        stress = [p[-1] for p in ph if p[-1].isdigit()]
        prim = tuple(p[:-1] for p in ph if p[-1:] == "1")
        return (stress.index("1") if "1" in stress else None), prim

    return {w for w, ps in prons.items() if len({sig(p) for p in ps}) >= 2}


def ja_homographs(bank_keys, jmdict_dir):
    """Kana bank entries that JMdict lists under two or more common entries
    (news1/ichi1/spec1/gai1 priority), e.g. はし 橋/箸/端. A kana-titled
    recording cannot say which one, or which pitch accent, it carries."""
    import xml.etree.ElementTree as ET
    path = pathlib.Path(jmdict_dir) / "JMdict"
    if not path.exists():
        return None
    want = set(bank_keys)
    count = collections.Counter()
    for _, el in ET.iterparse(str(path), events=("end",)):
        if el.tag != "entry":
            continue
        for r in el.findall("r_ele"):
            reb = r.findtext("reb")
            pri = {p.text for p in r.findall("re_pri")}
            if reb in want and pri & {"news1", "ichi1", "spec1", "gai1", "spec2", "news2", "ichi2"}:
                count[reb] += 1
        el.clear()
    return {w for w, c in count.items() if c >= 2}


# ------------------------------------------------------------------ classify

def title_word_ok(title, word, uploader):
    """True when the title is exactly LL-Q.. (iso)-<uploader>-<word>.<ext>."""
    m = TITLE_RE.match(nfc(title))
    return bool(m and uploader and m.group("rest") == nfc(uploader) + "-" + word)


def classify(lang, title, word, meta, sp, pl, lang_items, residence_ok=False):
    """One candidate clip -> (usable: bool, reason, speaker, license).
    reason None means the title is not a recording of this word at all."""
    m = meta.get(title, {"missing": True})
    if m.get("missing"):
        return False, "missing on Commons", None, None
    if not title_word_ok(title, word, m.get("uploader")):
        return False, None, None, None
    lic = m["license"]
    if lic not in ALLOWED:
        return False, f"license {m['license_raw']}", m.get("speaker"), None
    q = m.get("speaker")
    s = sp.get(q) if q else None
    if not s or "error" in s:
        return False, "speaker item unreadable", q, lic
    info = None
    for li in lang_items:
        if li in s["langs"]:
            info = s["langs"][li]
            break
    if info is None:
        return False, "speaker declares no level for this language", q, lic
    if LL_NATIVE not in info["level"]:
        return False, "speaker not native", q, lic
    if LANGS[lang][2] == AUDITOR_VARIETY:
        return True, "usable (variety checked by auditor)", q, lic
    places = info["learned"]
    if not places and residence_ok:
        # Alternate reading for Eric's ruling: residence stands in for an
        # undeclared place of learning. Not I8 as written.
        places = s.get("residence", [])
    if not places:
        return False, "variety undeclared (no place of learning)", q, lic
    countries = {c for p in places for c in pl.get(p, {}).get("country", [])}
    if not countries:
        return False, "variety undeclared (place has no country)", q, lic
    ok = LANGS[lang][2]
    if not ok:
        return False, "variety not decidable by country (" + LANGS[lang][1] + ")", q, lic
    if not countries & ok:
        return False, "variety mismatch", q, lic
    return True, "usable", q, lic


# ------------------------------------------------------------------ main

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bank", required=True)
    ap.add_argument("--cache", required=True)
    ap.add_argument("--repo", default=str(pathlib.Path(__file__).resolve().parents[2]),
                    help="checkout holding the gitignored sources under tools/")
    ap.add_argument("--kaikki-ru")
    ap.add_argument("--out", default="human_audio_census.json")
    ap.add_argument("stage", choices=["titles", "meta", "speakers", "places", "homographs", "all"])
    a = ap.parse_args()
    cache = pathlib.Path(a.cache)
    cache.mkdir(parents=True, exist_ok=True)
    repo = pathlib.Path(a.repo)
    bank = load_bank(a.bank)

    if a.stage in ("titles", "all"):
        stage_titles(cache)
    cands = candidates(cache, bank)
    meta = stage_meta(cache, cands) if a.stage in ("meta", "all") else load_json(cache / "meta.json", {})
    sp = stage_speakers(cache, meta) if a.stage in ("speakers", "all") else load_json(cache / "speakers.json", {})
    pl = stage_places(cache, sp) if a.stage in ("places", "all") else load_json(cache / "places.json", {})

    hg_path = cache / "homographs.json"
    hg = load_json(hg_path, {})
    if a.stage in ("homographs", "all") or not hg:
        keys = lambda l: [k for t in bank[l].values() for _, k in t]
        src = repo / "tools"
        hg = {"en": sorted(en_heteronyms(keys("en"), src / "wordpipe/sources/cmudict.dict")),
              "zh": sorted(zh_polyphones(keys("zh"), src / "langdata/sources/zh/cedict.txt"))}
        ja = ja_homographs(keys("ja"), src / "langdata/sources/ja")
        if ja is not None:
            hg["ja"] = sorted(ja)
        if a.kaikki_ru:
            hg["ru"] = sorted(ru_homographs(keys("ru"), a.kaikki_ru))
        save_json(hg_path, hg)

    # Lingua Libre language items per app language, for reading speaker levels.
    ll_lang = {"eng": "Q22", "spa": "Q386", "fra": "Q21", "deu": "Q24", "por": "Q126",
               "pol": "Q298", "vie": "Q208", "kor": "Q207", "jpn": "Q389", "fil": "Q75",
               "tgl": "Q169", "cmn": "Q113", "zho": None, "rus": "Q129", "ara": "Q219",
               "swa": "Q36", "hin": "Q123"}
    ll_lang.update(load_json(cache / "ll-lang-items.json", {}))

    result = {}
    for lang, (isos, variety, _) in LANGS.items():
        hits, n_titles = cands[lang]
        lang_items = [ll_lang[i] for i in isos if ll_lang.get(i)]
        homog = set(hg.get(lang, []))
        tiers = {}
        lic_mix = collections.Counter()
        excl = collections.Counter()
        speakers_usable = collections.Counter()
        country_spread = collections.Counter()
        gap = []
        for tier in TIERS:
            entries = bank[lang][tier]
            raw = usable = usable_res = homog_n = 0
            for entry, key in entries:
                ts = [t for t in hits.get(key, [])
                      if title_word_ok(t, key, meta.get(t, {}).get("uploader"))]
                if ts:
                    raw += 1
                if key in homog:
                    homog_n += 1
                    if ts:
                        excl["homograph/polyphone (D5)"] += len(ts)
                    gap.append((tier, entry))
                    continue
                any_ok = False
                for t in ts:
                    ok, why, q, lic = classify(lang, t, key, meta, sp, pl, lang_items)
                    lic_mix[ALLOWED.get(lic, "excluded")] += 1
                    if ok:
                        any_ok = True
                        speakers_usable[q] += 1
                    else:
                        excl[why] += 1
                        if why.startswith("variety"):
                            s = sp.get(q, {})
                            for li in lang_items:
                                for p in s.get("langs", {}).get(li, {}).get("learned", []):
                                    for c in pl.get(p, {}).get("country", []):
                                        country_spread[c] += 1
                if any_ok:
                    usable += 1
                else:
                    gap.append((tier, entry))
                if any(classify(lang, t, key, meta, sp, pl, lang_items, residence_ok=True)[0] for t in ts):
                    usable_res += 1
            tiers[tier] = {"n": len(entries), "raw": raw, "usable": usable,
                           "usable_residence": usable_res, "homographs": homog_n}
        top = speakers_usable.most_common(1)
        uploader_of = collections.Counter(
            (m.get("speaker"), m.get("uploader")) for m in meta.values() if m.get("speaker"))
        name_of = lambda q: max(((n, u) for (s_, u), n in uploader_of.items() if s_ == q), default=(0, q))[1]
        result[lang] = {
            "isos": isos, "variety": variety, "commons_titles": n_titles,
            "tiers": tiers, "license_mix": dict(lic_mix), "exclusions": dict(excl),
            "speakers_usable": len(speakers_usable),
            "top_speaker": ({"id": top[0][0], "name": name_of(top[0][0]),
                             "clips": top[0][1],
                             "share": top[0][1] / sum(speakers_usable.values())} if top else None),
            "variety_countries_excluded": {pl.get(c, {}).get("label", c): n
                                           for c, n in country_spread.most_common(8)},
            "gap": len(gap),
            "gap_entries": gap,
        }
    pathlib.Path(a.out).write_text(json.dumps(result, ensure_ascii=False, indent=1))
    print(f"wrote {a.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
