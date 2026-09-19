#!/usr/bin/env python3
"""CC-HUMAN-AUDIO F1: harvest and filter Lingua Libre clips for one language.

Build-time tool; the Commons download is a network call, so it runs on Eric's
machine, never in CI, and the app never fetches from Commons (I9).

Reads the Phase A census cache (tools/human-audio-census/census.py) so the
filters are the census's own: exact NFC match bound to the uploader, the
D1 license allowlist read from Commons metadata, native speaker, strict I8
variety (place of learning only, R1), and the D5 hold-outs. Every candidate
the filters reject is written to the exclusion log with its reason; nothing
is dropped silently.

Then it downloads each accepted file and checks it against the SHA-1 Commons
publishes for it. A file whose bytes do not match is excluded, not kept.

Outputs, under --out:
  clips/raw/<sha1>.wav    the original recordings, named by content
  manifest.json           one record per accepted clip (provenance for F7)
  exclusions.tsv          every rejected candidate, with the reason

Run: python3 tools/human-audio/harvest.py --lang en \\
       --census-cache ~/repos/ha-census-cache --out ~/repos/ha-census-cache/phase-b/en
"""
import argparse
import hashlib
import json
import pathlib
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "human-audio-census"))
import census  # noqa: E402  (the Phase A filters, reused rather than re-derived)

UA = census.UA
# upload.wikimedia.org answers bursts with 429 + Retry-After. The pace adapts:
# every 429 slows it (and waits out Retry-After), every success speeds it back
# up a little, never below PACE_MIN. Politeness costs minutes; being blocked
# costs the run.
PACE_MIN, PACE_MAX = 1.0, 8.0
pace = [2.0]

# Lingua Libre language items per app language (as census.main uses them).
LL_LANG = {"en": ["Q22"], "es": ["Q386"], "fr": ["Q21"], "de": ["Q24"], "pt": ["Q126"],
           "pl": ["Q298"], "vi": ["Q208"], "ko": ["Q207"], "ja": ["Q389"],
           "fil": ["Q75", "Q169"], "zh": ["Q113"], "ru": ["Q129"], "ar": ["Q219"],
           "sw": ["Q36"], "hi": ["Q123"]}


def upload_url(title):
    """Commons stores File:X at /<md5[0]>/<md5[:2]>/X, with spaces as underscores."""
    name = title.split(":", 1)[1].replace(" ", "_")
    h = hashlib.md5(name.encode("utf-8")).hexdigest()
    return f"https://upload.wikimedia.org/wikipedia/commons/{h[0]}/{h[:2]}/{urllib.parse.quote(name)}"


def select(lang, cache):
    """Apply the census filters. Returns (accepted, excluded)."""
    bank = census.load_bank(cache / "bank.tsv")
    cands = census.candidates(cache, bank)
    meta = census.load_json(cache / "meta.json", {})
    sp = census.load_json(cache / "speakers.json", {})
    pl = census.load_json(cache / "places.json", {})
    held = set(census.load_json(cache / "homographs.json", {}).get(lang, []))
    hits = cands[lang][0]

    tiers = {}
    for tier, rows in bank[lang].items():
        for entry, key in rows:
            tiers.setdefault(key, {"entry": entry, "tiers": []})["tiers"].append(tier)

    accepted, excluded = [], []
    for key, info in sorted(tiers.items()):
        for title in hits.get(key, []):
            m = meta.get(title, {})
            if not census.title_word_ok(title, key, m.get("uploader")):
                continue  # the title is not a recording of this word at all
            if key in held:
                excluded.append((key, title, "homograph/polyphone (D5)"))
                continue
            ok, why, q, lic = census.classify(lang, title, key, meta, sp, pl, LL_LANG[lang])
            if not ok:
                excluded.append((key, title, why))
                continue
            s = sp.get(q, {})
            learned = [x for li in LL_LANG[lang] for x in s.get("langs", {}).get(li, {}).get("learned", [])]
            accepted.append({
                "entry": info["entry"], "key": key, "tiers": info["tiers"],
                "title": title, "source_url": upload_url(title),
                "page_url": "https://commons.wikimedia.org/wiki/" + urllib.parse.quote(title.replace(" ", "_"), safe=":"),
                "license": census.ALLOWED[lic], "license_raw": m.get("license_raw"),
                "license_short": m.get("license_short"),
                "speaker_item": q, "speaker": m.get("uploader"),
                "variety_places": [{"id": p, "label": pl.get(p, {}).get("label"),
                                    "country": pl.get(p, {}).get("country")} for p in learned],
                "commons_sha1": m.get("sha1"), "duration": m.get("duration"),
            })
    return accepted, excluded


def download(rec, raw_dir):
    """Fetch one file and verify it against the Commons SHA-1. Returns an
    exclusion reason, or None when the file is on disk and verified."""
    dest = raw_dir / f"{rec['commons_sha1']}.wav"
    if dest.exists() and hashlib.sha1(dest.read_bytes()).hexdigest() == rec["commons_sha1"]:
        return None
    for attempt in range(10):
        time.sleep(pace[0])
        try:
            req = urllib.request.Request(rec["source_url"], headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=60) as r:
                data = r.read()
            pace[0] = max(PACE_MIN, pace[0] * 0.95)
            break
        except urllib.error.HTTPError as e:
            if attempt == 9:
                return f"download failed: HTTP {e.code}"
            if e.code == 429:
                pace[0] = min(PACE_MAX, pace[0] * 1.5)
                ra = e.headers.get("Retry-After", "")
                time.sleep(int(ra) if ra.isdigit() else 15)
            else:
                time.sleep(min(120, 5 * 2 ** attempt))
        except Exception as e:
            if attempt == 9:
                return f"download failed: {e!r}"
            time.sleep(min(120, 5 * 2 ** attempt))
    got = hashlib.sha1(data).hexdigest()
    if got != rec["commons_sha1"]:
        return f"sha1 mismatch (Commons {rec['commons_sha1']}, got {got})"
    tmp = dest.with_suffix(".part")
    tmp.write_bytes(data)
    tmp.replace(dest)
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", required=True)
    ap.add_argument("--census-cache", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--limit", type=int, help="download only the first N (smoke test)")
    a = ap.parse_args()
    cache, out = pathlib.Path(a.census_cache).expanduser(), pathlib.Path(a.out).expanduser()
    raw = out / "clips" / "raw"
    raw.mkdir(parents=True, exist_ok=True)

    accepted, excluded = select(a.lang, cache)
    print(f"{a.lang}: {len(accepted)} accepted, {len(excluded)} excluded by filters", file=sys.stderr)
    todo = accepted[: a.limit] if a.limit else accepted
    kept = []
    for i, rec in enumerate(todo, 1):
        why = download(rec, raw)
        if why:
            excluded.append((rec["key"], rec["title"], why))
        else:
            rec["raw_path"] = f"clips/raw/{rec['commons_sha1']}.wav"
            kept.append(rec)
        if i % 100 == 0:
            print(f"  downloaded {i}/{len(todo)} (pace {pace[0]:.1f}s)", file=sys.stderr, flush=True)

    (out / "manifest.json").write_text(json.dumps(
        {"lang": a.lang, "generated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
         "clips": kept}, ensure_ascii=False, indent=1))
    with open(out / "exclusions.tsv", "w", encoding="utf-8") as fh:
        fh.write("key\ttitle\treason\n")
        for key, title, why in sorted(excluded):
            fh.write(f"{key}\t{title}\t{why}\n")
    print(f"{a.lang}: {len(kept)} clips on disk, {len(excluded)} excluded "
          f"(log: {out / 'exclusions.tsv'})", file=sys.stderr)


if __name__ == "__main__":
    main()
