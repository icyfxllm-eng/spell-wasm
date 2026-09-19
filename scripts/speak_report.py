#!/usr/bin/env python3
"""CC-TELEMETRY-FOUNDATION v1.1 F5 (server side, R8): TTS health per language.

Reads the daily speak-YYYY-MM-DD.jsonl files that backend/speak_metrics.py
writes and prints CSV, one row per (day, lang):

    day,lang,n,p50_ms,p95_ms,cache_hit_rate,error_rate

    python3 scripts/speak_report.py ~/spellgame-server/logs/speak            # last 14 days
    python3 scripts/speak_report.py DIR --days 90 --prune                    # and delete >90 days

The hit rate is for the Mac mini's own disk cache. Requests Cloudflare's edge
answers from its cache (audio is immutable) never reach the server, so they
don't appear here at all.
"""
import argparse
import csv
import datetime as dt
import glob
import json
import os
import sys

RETAIN_DAYS = 90  # D6


def percentile(sorted_vals, q):
    if not sorted_vals:
        return None
    k = (len(sorted_vals) - 1) * q
    lo, hi = int(k), min(int(k) + 1, len(sorted_vals) - 1)
    return round(sorted_vals[lo] + (sorted_vals[hi] - sorted_vals[lo]) * (k - lo), 1)


def day_of(path):
    name = os.path.basename(path)
    try:
        return dt.date.fromisoformat(name[len("speak-"):-len(".jsonl")])
    except ValueError:
        return None


def report(directory, days, today):
    rows = []
    for path in sorted(glob.glob(os.path.join(directory, "speak-*.jsonl"))):
        day = day_of(path)
        if day is None or (today - day).days >= days:
            continue
        by_lang = {}
        with open(path) as fh:
            for raw in fh:
                try:
                    r = json.loads(raw)
                except ValueError:
                    continue
                by_lang.setdefault(r.get("lang", "other"), []).append(r)
        for lang, recs in sorted(by_lang.items()):
            ok = sorted(r["ms"] for r in recs if r.get("status") == 200)
            served = [r for r in recs if r.get("status") == 200]
            errors = sum(1 for r in recs if r.get("status", 0) >= 500)
            rows.append({
                "day": day.isoformat(),
                "lang": lang,
                "n": len(recs),
                "p50_ms": percentile(ok, 0.50),
                "p95_ms": percentile(ok, 0.95),
                "cache_hit_rate": round(sum(1 for r in served if r.get("cache") == "hit") / len(served), 3) if served else None,
                "error_rate": round(errors / len(recs), 3),
            })
    return rows


def prune(directory, today, keep=RETAIN_DAYS):
    removed = []
    for path in glob.glob(os.path.join(directory, "speak-*.jsonl")):
        day = day_of(path)
        if day is not None and (today - day).days >= keep:
            os.remove(path)
            removed.append(path)
    return removed


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dir")
    ap.add_argument("--days", type=int, default=14)
    ap.add_argument("--prune", action="store_true", help=f"delete files older than {RETAIN_DAYS} days")
    a = ap.parse_args(argv)
    today = dt.datetime.now(dt.timezone.utc).date()
    if a.prune:
        for p in prune(a.dir, today):
            print(f"pruned {p}", file=sys.stderr)
    w = csv.DictWriter(sys.stdout, ["day", "lang", "n", "p50_ms", "p95_ms", "cache_hit_rate", "error_rate"])
    w.writeheader()
    w.writerows(report(a.dir, a.days, today))


if __name__ == "__main__":
    main()
