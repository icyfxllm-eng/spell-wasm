# -*- coding: utf-8 -*-
"""Language content audit (machine-checkable pass) — REVIEW-GATED, read-only.

Runs Features 1-6 of the content-audit spec for ONE language (parameterized, so
it's reusable for Thai next) and writes audit/<lang>/{report.md, findings.json,
auditor-packet.md, audio-manifest.txt}. It NEVER mutates content — mechanical
fixes live on a separate branch produced by apply_mechanical_fixes.py.

  python3 tools/audit/audit_lang.py es

Findings schema (mirrored at the top of findings.json):
  { feature:int, severity:"critical"|"violation"|"warning"|"info",
    class:str, file:str, key:str, detail:str, proposed_fix:str|null }
"""
from __future__ import annotations
import json, os, re, sys, unicodedata, urllib.request
from pathlib import Path
from statistics import mean, median

ROOT = Path(__file__).resolve().parent.parent.parent
TIERS = ["easy", "medium", "hard", "expert"]
SEV_ORDER = {"critical": 0, "violation": 1, "warning": 2, "info": 3}

lang = sys.argv[1] if len(sys.argv) > 1 else "es"
OUT = ROOT / "audit" / lang
OUT.mkdir(parents=True, exist_ok=True)
findings: list[dict] = []


def add(feature, severity, cls, file, key, detail, proposed_fix=None):
    findings.append({"feature": feature, "severity": severity, "class": cls,
                     "file": str(file), "key": str(key), "detail": detail, "proposed_fix": proposed_fix})


def nfc(s):
    return unicodedata.normalize("NFC", s)


def read_words(path):
    """Return [(lineno, raw)] of non-comment, non-blank lines (raw, unstripped)."""
    out = []
    if not path.exists():
        return out
    for i, raw in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
        if raw.strip() and not raw.strip().startswith("#"):
            out.append((i, raw))
    return out


# ---- load target-language lists -----------------------------------------------
WORDS_DIR = ROOT / "assets" / "words" / lang
tiers = {t: read_words(WORDS_DIR / f"{t}.txt") for t in TIERS}
# spoken form: zh stores "pinyin|hanzi"; else the word itself
def spoken(w):
    return w.split("|")[1] if "|" in w else w

# ============================================================ FEATURE 1: integrity
CHARSET = {"es": r"[a-záéíóúüñ]+(?:[- ][a-záéíóúüñ]+)*"}.get(lang, r"[a-z]+(?:[- ][a-z]+)*")
INPUT_LIMIT = 40  # generous cap; entries beyond this are suspect
seen_global = {}  # word -> tier (cross-tier dup)
for tier, rows in tiers.items():
    f = f"assets/words/{lang}/{tier}.txt"
    seen_tier = {}
    for ln, raw in rows:
        w = raw  # keep raw to test whitespace padding
        stripped = w.strip()
        # whitespace padding / length
        if w != stripped:
            add(1, "violation", "whitespace", f, ln, f"'{raw}' has leading/trailing whitespace", "trim whitespace")
        if not stripped:
            add(1, "violation", "empty", f, ln, "empty entry", "remove line")
            continue
        if len(stripped) > INPUT_LIMIT:
            add(1, "violation", "length", f, ln, f"'{stripped}' exceeds {INPUT_LIMIT} chars", None)
        # NFC
        if stripped != nfc(stripped):
            add(1, "violation", "nfc", f, ln, f"'{stripped}' is not NFC-normalized", "NFC-normalize")
        # charset (test the display token; for zh test the whole stored form loosely)
        token = spoken(stripped) if lang == "zh" else stripped
        if lang != "zh" and not re.fullmatch(CHARSET, stripped.lower()):
            bad = [c for c in stripped if not re.match(CHARSET, c.lower()) and c not in "- "]
            add(1, "violation", "charset", f, ln, f"'{stripped}' has out-of-set chars: {bad}", None)
        # dup within tier
        key = nfc(stripped).lower()
        if key in seen_tier:
            add(1, "violation", "dup-in-tier", f, ln, f"'{stripped}' duplicates line {seen_tier[key]}", "remove duplicate")
        else:
            seen_tier[key] = ln
        # dup across tiers
        if key in seen_global and seen_global[key][0] != tier:
            add(1, "violation", "dup-cross-tier", f, ln,
                f"'{stripped}' also in {seen_global[key][0]} tier (line {seen_global[key][1]})", None)
        elif key not in seen_global:
            seen_global[key] = (tier, ln)

# English contamination (flag, don't delete — D3)
en_words = set()
en_dir = ROOT / "assets" / "words" / "en"
for t in TIERS:
    for _, raw in read_words(en_dir / f"{t}.txt"):
        en_words.add(raw.strip().lower())
if lang != "en":
    for tier, rows in tiers.items():
        for ln, raw in rows:
            w = raw.strip().lower()
            if w in en_words:
                add(1, "warning", "english-contamination", f"assets/words/{lang}/{tier}.txt", ln,
                    f"'{raw.strip()}' also appears in the English word lists — confirm it's a legitimate {lang} word",
                    None)

# ============================================================ FEATURE 2: tier calibration
ACCENTS = set("áéíóúüñ")
def has_trap(w):
    lw = w.lower()
    return bool(re.search(r"h|[bv]|ll|y|z|c[ei]|g[ei]|j", lw) or any(c in ACCENTS for c in lw))
tier_stats = {}
prev_len = prev_trap = -1
for tier in TIERS:
    ws = [spoken(r.strip()) for _, r in tiers[tier]]
    n = len(ws)
    if n == 0:
        add(2, "critical", "empty-tier", f"assets/words/{lang}/{tier}.txt", tier, "tier is empty", None)
        tier_stats[tier] = None
        continue
    lens = [len(w) for w in ws]
    acc = sum(1 for w in ws if any(c in ACCENTS for c in w)) / n
    trap = sum(1 for w in ws if has_trap(w)) / n
    tier_stats[tier] = {"count": n, "mean_len": round(mean(lens), 2), "median_len": median(lens),
                        "accent_density": round(acc, 3), "trap_density": round(trap, 3)}
    # monotonic difficulty signal (mean length + trap density must not decrease)
    if prev_len >= 0 and mean(lens) < prev_len - 0.01:
        add(2, "violation", "non-monotonic-length", f"assets/words/{lang}/{tier}.txt", tier,
            f"mean length {mean(lens):.2f} < previous tier {prev_len:.2f}", None)
    if prev_trap >= 0 and trap < prev_trap - 0.001:
        add(2, "violation", "non-monotonic-trap", f"assets/words/{lang}/{tier}.txt", tier,
            f"trap density {trap:.3f} < previous tier {prev_trap:.3f}", None)
    prev_len, prev_trap = mean(lens), trap
# min-pool: no code constant exists (see report open questions)
add(2, "info", "min-pool-undefined", "src/consts.rs", "-",
    "No minimum-pool-size constant exists in code; the 'above minimum pool' invariant "
    "has no threshold to check against. Daily uses a W×30 horizon (scripts/daily-pool-audit). "
    "OPEN QUESTION for Eric: define the min tier pool size.", None)

# ============================================================ FEATURE 3: audio + TTS config
app_py = (ROOT / "backend" / "app.py").read_text(encoding="utf-8")
voice_defs = re.findall(r'"([a-z]{2})":\s*\(\s*"([^"]+)",\s*"([^"]+)"\)', app_py)
voice = next(((lc, vn) for code, lc, vn in voice_defs if code == lang), None)
# single-source check: count where a voice/locale for this lang is configured
occurrences = len(re.findall(rf'"{lang}":\s*\(', app_py))
if voice:
    add(3, "info", "tts-voice", "backend/app.py", "LANG_VOICES",
        f"{lang} voice = {voice[1]} (locale {voice[0]}); configured in {occurrences} place(s)", None)
    if occurrences != 1:
        add(3, "violation", "tts-voice-multi-source", "backend/app.py", "LANG_VOICES",
            f"{lang} voice configured in {occurrences} places (single-source doctrine)", None)
else:
    add(3, "critical", "tts-voice-missing", "backend/app.py", "LANG_VOICES", f"no TTS voice configured for {lang}", None)
# generation manifest (all words) — verification of the cache is server-side
manifest = sorted({spoken(r.strip()) for rows in tiers.values() for _, r in rows})
(OUT / "audio-manifest.txt").write_text("\n".join(manifest) + "\n", encoding="utf-8")
add(3, "info", "audio-coverage", "-", "-",
    f"{len(manifest)} unique words emitted to audio-manifest.txt. Cache coverage lives on the "
    "server (audio_cache, keyed md5('{lang}:'+word)); not verifiable from the repo and NOT probed "
    "(probing /api/speak would bulk-generate). Verify against the cache in a separate approved run.", None)

# ============================================================ FEATURE 4: homophone map
def fold_phonetic(w):
    s = w.lower()
    s = s.replace("ll", "y")            # yeísmo
    s = s.replace("h", "")              # silent h
    s = s.replace("v", "b")            # b/v merge (fold to one)
    s = re.sub(r"c([ei])", r"s\1", s)  # ce/ci -> se/si (seseo)
    s = s.replace("z", "s")            # z -> s (seseo)
    s = s.replace("c", "k")            # remaining c -> k (casa/kasa)
    s = re.sub(r"g([ei])", r"j\1", s)  # ge/gi -> je/ji
    s = re.sub(r"qu([ei])", r"k\1", s) # qu -> k
    for a, b in zip("áéíóúü", "aeiouu"):
        s = s.replace(a, b)
    return s
def fold_accent(w):
    s = w.lower()
    for a, b in zip("áéíóú", "aeiou"):
        s = s.replace(a, b)
    return s
# reference set: all list words + a common-Spanish frequency list (the "outside twin")
list_words = {spoken(r.strip()) for rows in tiers.values() for _, r in rows}
common = set()
if lang == "es":
    try:
        data = urllib.request.urlopen(
            "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/es/es_50k.txt",
            timeout=25).read().decode("utf-8")
        for line in data.splitlines()[:20000]:
            wd = line.split(" ")[0].strip().lower()
            if re.fullmatch(CHARSET, wd) and 2 <= len(wd) <= 15:
                common.add(wd)
    except Exception as e:
        add(4, "info", "freq-fetch-failed", "-", "-", f"couldn't fetch es frequency list ({e}); homophone pass limited to list-internal pairs", None)
universe = list_words | common
# Eric-confirmed accept-any pairs already wired into grading (assets/words/es/homophones.txt).
CONFIRMED = set()
hp = ROOT / "assets" / "words" / lang / "homophones.txt"
if hp.exists():
    for line in hp.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            CONFIRMED |= set(line.split())
CLASSES = [("phonetic", fold_phonetic, "b/v · silent-h · s/z/c (seseo) · ll/y (yeísmo) · g/j"),
           ("accent", fold_accent, "accent-only pairs (papa/papá)")]
for cls_name, fold, desc in CLASSES:
    buckets: dict[str, set] = {}
    for w in universe:
        buckets.setdefault(fold(w), set()).add(w)
    for key, group in sorted(buckets.items()):
        involving_list = group & list_words
        if len(group) >= 2 and involving_list:
            # a genuine ambiguity a player could hit: a list word + ≥1 other real word
            others = sorted(group - involving_list) or sorted(group - {list(involving_list)[0]})
            # proposed bucket (review-gated; native auditor confirms):
            if group & CONFIRMED:
                bucket = "accept-any — CONFIRMED (Eric), already wired in homophones.txt"
            elif cls_name == "accent":
                bucket = ("no-action — the accent is a stress difference the audio CAN carry, so this is a "
                          "legitimate spelling test (many 'twins' here are just unaccented corpus typos)")
            elif all(o in common for o in others):
                bucket = "accept-any (both members common) — PROPOSED, confirm"
            else:
                bucket = "remove-rarer if the rare twin is ever added (not in lists today) — PROPOSED, confirm"
            add(4, "warning", cls_name,
                f"assets/words/{lang}/", ",".join(sorted(involving_list)),
                f"[{desc}] sounds identical to: {sorted(group)} | PROPOSED BUCKET: {bucket} | "
                f"Twins in lists: {sorted(involving_list)}; other real words: {others}",
                "Eric to pick homophone policy (D2): accept-any / remove-rarer / require-sentence")

# ============================================================ FEATURE 5: profanity coverage
def load_terms(p):
    return {ln.strip().lower() for ln in p.read_text(encoding="utf-8").split("\n")
            if ln.strip() and not ln.strip().startswith("#")} if p.exists() else set()
prof_dir = ROOT / "assets" / "words" / "profanity"
seed = load_terms(prof_dir / f"{lang}.txt")
union = {}  # term -> [langs it's seeded in], for cross-language context only
for p in prof_dir.glob("*.txt"):
    for t in load_terms(p):
        union.setdefault(t, []).append(p.stem)
# Universal hard slurs (mirrors src/profanity.rs BANNED_ROOTS/EXACT): profane in
# essentially every language, so they DO disqualify any word list. Everything
# else is judged against THIS language's own seed only — a word that is profane
# only in another language (e.g. es "negro"=black, on the en/fr slur seed;
# "leche"=milk, a Filipino expletive) is legitimate here and must NOT be flagged
# (decision addendum, 2026-07: puzzle words are seen only inside their own mode).
UNIVERSAL_ROOTS = ("fuck", "shit", "cunt", "nigg", "faggot")
UNIVERSAL_EXACT = {"cunt", "faggot", "rape", "rapist"}
add(5, "info", "filter-layers", f"assets/words/profanity/{lang}.txt", "-",
    f"{lang} seed layer: {len(seed)} terms. Curation scan below is language-scoped ({lang} seed + "
    f"universal hard slurs). Runtime My Words screening (src/profanity.rs is_blocked) separately uses "
    f"the {len(union)}-term all-language union — that over-block is intentional for user imports.", None)
if not seed:
    add(5, "critical", "no-seed", f"assets/words/profanity/{lang}.txt", "-", f"no {lang} profanity seed layer", None)
# cross-check every curated word against the LANGUAGE-SCOPED filter — a hit is critical
for tier, rows in tiers.items():
    for ln, raw in rows:
        w = nfc(spoken(raw.strip())).lower()
        universal = any(r in w for r in UNIVERSAL_ROOTS) or w in UNIVERSAL_EXACT
        if w in seed or universal:
            add(5, "critical", "profanity-in-list", f"assets/words/{lang}/{tier}.txt", ln,
                f"'{raw.strip()}' is on the {lang} profanity seed / universal slur set but present in a word list",
                "remove from list (Eric approves)")
        elif w in union:  # profane only in OTHER languages -> context, not a violation
            add(5, "info", "cross-lang-profanity", f"assets/words/{lang}/{tier}.txt", ln,
                f"'{raw.strip()}' is a valid {lang} word but is on the profanity seed for: "
                f"{sorted(set(union[w]))}. Not flagged for {lang} (kept per decision addendum). "
                f"Note: it stays blocked in free-text usernames via the global/English path.", None)
# regional-vulgar innocents present in lists → auditor flag
REGIONAL = {"es": {"coger": "vulgar (sexual) in Mexico/Argentina", "concha": "vulgar in Rioplatense",
                    "pico": "vulgar in Chile", "polla": "vulgar in Spain", "pija": "vulgar in Rioplatense",
                    "cajeta": "vulgar in Argentina", "chucha": "vulgar in Andean", "verga": "vulgar widely"}}.get(lang, {})
for tier, rows in tiers.items():
    for ln, raw in rows:
        w = raw.strip().lower()
        if w in REGIONAL:
            add(5, "warning", "regional-vulgar", f"assets/words/{lang}/{tier}.txt", ln,
                f"'{w}' — {REGIONAL[w]}. Innocent in some varieties; auditor to judge.", None)

# ============================================================ FEATURE 6: i18n completeness
loc_dir = ROOT / "src" / "i18n" / "locales"
en_json = json.loads((loc_dir / "en.json").read_text(encoding="utf-8"))
tgt_json = json.loads((loc_dir / f"{lang}.json").read_text(encoding="utf-8"))
ALLOW_IDENTICAL = {"No", "OK", "Total"}
def placeholders(s):
    return set(re.findall(r"\{[^}]+\}|%[sd]", str(s)))
for k in en_json:
    if k not in tgt_json:
        add(6, "critical", "missing-key", f"src/i18n/locales/{lang}.json", k, "key present in en, missing in target", "add translation")
    else:
        if placeholders(en_json[k]) != placeholders(tgt_json[k]):
            add(6, "violation", "interpolation", f"src/i18n/locales/{lang}.json", k,
                f"placeholders differ: en={sorted(placeholders(en_json[k]))} vs {lang}={sorted(placeholders(tgt_json[k]))}", None)
        if str(en_json[k]) == str(tgt_json[k]) and str(en_json[k]) not in ALLOW_IDENTICAL and len(str(en_json[k])) > 2:
            add(6, "warning", "untranslated", f"src/i18n/locales/{lang}.json", k,
                f"value identical to English: '{en_json[k]}'", None)
for k in tgt_json:
    if k not in en_json:
        add(6, "warning", "orphan-key", f"src/i18n/locales/{lang}.json", k, "key in target not in en", "remove or add to en")
# recent-feature sweep: confirm the newest strings exist
RECENT = ["coming.badge", "coming.notice", "coming.notify", "top.headToHead", "settings.kid", "mode.tricky_words.title", "share.button"]
for k in RECENT:
    if k in en_json and k not in tgt_json:
        add(6, "critical", "missing-recent", f"src/i18n/locales/{lang}.json", k, "recent-feature key missing", "add translation")
# static-analysis: language-conditional branches outside the i18n layer
for rs in (ROOT / "src").glob("*.rs"):
    if rs.name in ("i18n.rs", "consts.rs"):
        continue
    for i, line in enumerate(rs.read_text(encoding="utf-8").split("\n"), 1):
        if re.search(rf'==\s*"{lang}"|"{lang}"\s*==', line) and "//" not in line.split('"' + lang + '"')[0]:
            add(6, "warning", "lang-branch", f"src/{rs.name}", i,
                f"possible language-conditional on '{lang}' outside the i18n layer: {line.strip()[:80]}", None)

# ============================================================ OUTPUTS
schema = {"fields": {"feature": "int 1-6", "severity": "critical|violation|warning|info",
                     "class": "str", "file": "str", "key": "str", "detail": "str", "proposed_fix": "str|null"}}
findings.sort(key=lambda x: (x["feature"], SEV_ORDER[x["severity"]]))
(OUT / "findings.json").write_text(json.dumps({"schema": schema, "lang": lang, "findings": findings}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

def by(feat=None, sev=None):
    return [x for x in findings if (feat is None or x["feature"] == feat) and (sev is None or x["severity"] == sev)]

FEAT_NAMES = {1: "Word list integrity", 2: "Difficulty tier calibration", 3: "Audio & TTS config",
              4: "Homophone / hearing-ambiguity map", 5: "Profanity filter coverage", 6: "UI localization completeness"}
rep = [f"# Spanish content audit — `{lang}`  (machine pass, REVIEW-GATED)", "",
       f"Totals: **{len(by(sev='critical'))} critical · {len(by(sev='violation'))} violation · "
       f"{len(by(sev='warning'))} warning · {len(by(sev='info'))} info**", ""]
CAVEATS = {4: "_Note: `accent` twins are bucketed against a web frequency corpus, which contains "
              "unaccented misspellings (e.g. `arbol` for `árbol`). Treat those as noise; genuine minimal "
              "pairs like `camino/caminó`, `tomate/tómate`, `trabajo/trabajó` are the ones to rule on._"}
for feat in range(1, 7):
    rep.append(f"## Feature {feat} — {FEAT_NAMES[feat]}")
    if feat in CAVEATS:
        rep.append("\n" + CAVEATS[feat])
    fs = by(feat)
    if feat == 2:
        rep.append("\nTier stats:\n\n| tier | count | mean len | median len | accent% | trap% |\n|--|--|--|--|--|--|")
        for t in TIERS:
            s = tier_stats.get(t)
            rep.append(f"| {t} | {s['count']} | {s['mean_len']} | {s['median_len']} | {s['accent_density']*100:.0f}% | {s['trap_density']*100:.0f}% |" if s else f"| {t} | 0 | — | — | — | — |")
        rep.append("")
    if not fs:
        rep.append("\n**0 findings.**\n")
        continue
    for sev in ("critical", "violation", "warning", "info"):
        sub = by(feat, sev)
        if sub:
            rep.append(f"\n**{sev.upper()} ({len(sub)})**\n")
            for x in sub[:200]:
                rep.append(f"- `{x['file']}:{x['key']}` [{x['class']}] {x['detail']}")
    rep.append("")
(OUT / "report.md").write_text("\n".join(rep) + "\n", encoding="utf-8")

# auditor packet: the judgment-call subset
pk = [f"# {lang} native-speaker auditor packet", "",
      "Machine checks are done; these need a human who reads the language.", ""]
# What the 2026-07 decision addendum already resolved (for the auditor to ratify).
resolved = [x for x in findings if x["class"] == "cross-lang-profanity"]
pk.append(f"## Already resolved (decision addendum 2026-07 — please ratify)  ({len(resolved) + 1})\n")
for x in resolved:
    pk.append(f"- **{x['key']}** — {x['detail']}")
pk.append("- **negro (username)** — now in `backend/blocklist.txt`; rejected as a username in ANY locale "
          "(`backend/test_usernames.py`), while staying a valid Spanish puzzle word. Checklist item: "
          "“any word inappropriate in your region?”")
pk.append("- **accept-any homophones** — casa/caza, botar/votar, cocer/coser are wired into grading "
          "(`assets/words/es/homophones.txt`, consumed by `src/homophones.rs`): typing either spelling "
          "scores correct. Confirm these three and rule on the proposed buckets below.\n")
groups = [("Regional vocabulary / English cognates to confirm", [x for x in findings if x["class"] in ("english-contamination",)]),
          ("Homophone pairs — confirm the PROPOSED BUCKET on each (accept-any / remove-rarer / no-action; "
           "require-sentence is parked, see docs/features/homophone-carrier-sentences.md)",
           [x for x in findings if x["feature"] == 4 and x["severity"] == "warning"]),
          ("Regionally-vulgar innocent words (Kid Mode risk)", [x for x in findings if x["class"] == "regional-vulgar"]),
          ("Open questions", [x for x in findings if x["class"] in ("min-pool-undefined", "audio-coverage", "tts-voice")])]
for title, items in groups:
    pk.append(f"## {title}  ({len(items)})\n")
    for x in items[:300]:
        pk.append(f"- **{x['key']}** — {x['detail']}")
    pk.append("")
pk.append("## Voice note\nThe Spanish TTS voice is **es-ES (Castilian, Spain)**. The seseo/yeísmo homophone "
          "pairs above (casa/caza, valla/vaya) are only homophones for *Latin American* speakers — a Castilian "
          "voice distinguishes s/z/c. If the target audience is Latin American (recommendation on file: es-419), "
          "the voice choice itself is an auditor/Eric decision.")
(OUT / "auditor-packet.md").write_text("\n".join(pk) + "\n", encoding="utf-8")

print(f"audit[{lang}]: {len(by(sev='critical'))} critical, {len(by(sev='violation'))} violation, "
      f"{len(by(sev='warning'))} warning, {len(by(sev='info'))} info -> audit/{lang}/")
