#!/usr/bin/env python3
"""CC-SOUNDALIKE-LADDER F1-F3: build the ladder table for fr and pt (called by tools/build_collisions.py).

Sets come from three places: the key function (tools/soundalike_keys.py) over the bank, the staged closed-list seeds in
data/<lang>/sound-alike-rules.json, and the closed pairs the spec names for fr. Every set carries a `ladder` object:

    status  missing -> machine_checked -> provisional -> verified     (verified needs a Gig B yes: not reachable here)
    reason  why a set did not advance (a code, never prose with IPA)
    origin  key | seed | closed-list

machine_checked = key + tests (the selftest) + seeds merged + loopback measured.
provisional     = machine_checked AND key, kaikki IPA (check only) and loopback all agree; a closed-list seed needs IPA + loopback.
The table `status` stays "missing" for these languages until every set is verified, so cues and the bonus stay off, and
Normal grading reads only sets the ladder allows (src/soundalikes.rs).
Nothing here reads or writes IPA: the IPA check (tools/soundalike_ipa_check.py) leaves one word per set (agree|differ|missing).
"""
import json, pathlib, re, sys, unicodedata, collections

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from soundalike_keys import KEYS
from soundalikes import layer1_voices, set_id

OUT = ROOT / "assets/words"
LADDER_LANGS = ("fr", "pt", "pl", "fil", "sw")
nfc = lambda s: unicodedata.normalize("NFC", s)
# F3: seeds from a closed rule list need only IPA + loopback (the spec names these for fr; es tilde is in accent-collisions.tsv)
CLOSED_LIST = {"fr": [("ou", "où"), ("a", "à"), ("la", "là"), ("sur", "sûr")], "pt": [], "pl": [], "fil": [], "sw": []}
# Decision 2026-11-07: sw is capped at machine_checked for v1 (92% of its bank has no kaikki IPA)
CAPPED = {"sw": "no_ipa_coverage"}
FR_INFLECT = re.compile(r"(ées?|és?|es|er|ez|ant|ent|e|s|x|t|d|z)$")


def bank(lang):
    ws = set()
    for t in ("easy", "medium", "hard", "expert"):
        for w in (OUT / lang / f"{t}.txt").read_text(encoding="utf-8").splitlines():
            w = nfc(w.strip().lower())
            if w and " " not in w:
                ws.add(w)
    return ws


def is_inflection(lang, members):
    """Same lemma, different grammatical form (technique/techniques): audibly identical in French, but typing the plural for
    the singular is a spelling error, not another sense. Such sets are never promoted past machine_checked."""
    if lang != "fr":
        return False
    bases = set()
    for w in members:
        b = w
        for _ in range(2):
            b = FR_INFLECT.sub("", b)
        bases.add(b)
    return len(bases) == 1


def json_or(p, d):
    return json.loads(p.read_text(encoding="utf-8")) if p.exists() else d


def candidates(lang):
    key, B = KEYS[lang], bank(lang)
    groups = collections.defaultdict(set)
    for w in B:
        k = key(w)
        if k:
            groups[k].add(w)
    cands = {}      # members tuple -> origin set
    for g in groups.values():
        if len(g) > 1:
            cands.setdefault(tuple(sorted(g)), set()).add("key")
    rules = json_or(ROOT / f"data/{lang}/sound-alike-rules.json", {"rules": [], "seeds": []})
    kinds = {r["id"]: r["kind"] for r in rules["rules"]}
    accents = []
    for s in rules["seeds"]:
        ms = tuple(sorted(nfc(w) for w in s["words"]))
        if kinds.get(s["rule"]) == "true":
            cands.setdefault(ms, set()).add("seed")
        else:
            accents.append((ms, s["rule"]))
    for a, b in CLOSED_LIST[lang]:
        cands.setdefault(tuple(sorted((nfc(a), nfc(b)))), set()).add("closed-list")
    return B, cands, accents


def build(lang, write=True):
    key = KEYS[lang]
    B, cands, accents = candidates(lang)
    loop = json_or(OUT / lang / "soundalike-loopback.json", {})
    loop_ok = bool(loop) and loop.get("voice") == layer1_voices().get(lang)
    loops = loop.get("sets", {}) if loop_ok else {}
    ipa = json_or(ROOT / f"data/{lang}/soundalike-ipa-check.json", {}).get("sets", {})
    sets, counts = [], collections.Counter()
    for ms, origin in sorted(cands.items()):
        sid = set_id(lang, list(ms))
        # a curated or closed-list seed is a lexical pair by decision (vers/vert/verre), never an inflection
        inflect = is_inflection(lang, ms) and not (origin & {"seed", "closed-list"})
        keys_agree = len({key(m) for m in ms}) == 1
        lr = loops.get(sid)
        ip = ipa.get(sid)
        measured = lr is not None and lr.get("flag") != "no-audio"
        status, reason = ("machine_checked" if measured else "missing"), None
        if not measured:
            reason = "loopback_unmeasured"
        elif inflect:
            reason = "same_lemma_inflection"
        elif lr.get("flag"):
            reason = "loopback_flag"
        elif "closed-list" not in origin and not keys_agree:
            reason = "key_disagrees"
        elif ip is None:
            reason = "ipa_unchecked"
        elif ip != "agree":
            reason = "ipa_differ" if ip == "differ" else "ipa_missing"
        elif lang in CAPPED:
            reason = CAPPED[lang]
        else:
            status = "provisional"
        counts[status] += 1
        sets.append({
            "setId": sid, "kind": "true", "mergerRules": [], "confirmed": False, "loopbackFlag": (lr or {}).get("flag") if lr and lr.get("flag") not in (None, "no-audio") else None,
            "members": [{"memberId": m, "wordId": m if m in B else None} for m in ms],
            "ladder": {"status": status, "reason": reason, "origin": sorted(origin)},
        })
    for ms, rule in accents:
        sets.append({"setId": set_id(lang, list(ms)), "kind": "accent", "mergerRules": [rule], "confirmed": False, "loopbackFlag": None,
                     "members": [{"memberId": m, "wordId": m if m in B else None} for m in ms],
                     "ladder": {"status": "machine_checked", "reason": "accent_rule", "origin": ["seed"]}})
    order = ["missing", "machine_checked", "provisional", "verified"]
    lowest = min((s["ladder"]["status"] for s in sets), key=order.index) if sets else "missing"
    doc = {
        "$doc": "GENERATED by tools/build_collisions.py -> tools/soundalike_ladder.py -- do not hand-edit. CC-SOUNDALIKE-LADDER. "
                "Read only through soundalikes() (src/soundalikes.rs, tools/soundalikes.py, scripts/lib/soundalikes.mjs).",
        "lang": lang,
        "status": "missing",
        "statusReason": "ladder language: `verified` needs a Gig B yes on every set (CC-SOUNDALIKE-LADDER F4); see ladderStatus",
        "ladderStatus": lowest,
        "ladderCounts": dict(sorted(counts.items())),
        "voice": layer1_voices().get(lang),
        "generator": f"soundalike_keys.key_{lang}@1",
        "bankRows": len(B),
        "sets": sets,
    }
    path = OUT / lang / "soundalikes.json"
    text = json.dumps(doc, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if write:
        path.write_text(text, encoding="utf-8")
    print(f"  {lang}: {len(sets)} sets {dict(counts)}; ladderStatus {lowest}")
    return text


if __name__ == "__main__":
    for l in [a for a in sys.argv[1:] if not a.startswith("-")] or LADDER_LANGS:
        build(l)
