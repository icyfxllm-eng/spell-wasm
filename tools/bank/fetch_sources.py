#!/usr/bin/env python3
"""CC-BANK-PURITY F2 — fetch the reference dictionaries named in config/bank-purity-sources.json.

    python3 tools/bank/fetch_sources.py [--verify]

Downloads each source to .corpus-cache/ (gitignored; the same pattern build-bigbank.py uses for corpora) and
checks its sha256 against the registry.  --verify only checks files already there.  A sha mismatch is an error:
upstream moved, so re-run the census (the registry hash is part of what a verdict means) before updating it.
Dictionaries are build inputs, never committed or shipped (P3, I7).
"""
import hashlib, json, os, sys, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
reg = json.load(open(f"{ROOT}/config/bank-purity-sources.json", encoding="utf-8"))
cache = os.environ.get("PURITY_CACHE") or f"{ROOT}/.corpus-cache"
bad = 0
for s in reg["sources"]:
    for url, rel, want in zip(s["url"], s["cache"], s["sha256"]):
        dest = f"{cache}/{rel}"
        if not os.path.exists(dest) and "--verify" not in sys.argv:
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            print(f"fetch {s['id']}: {url}")
            urllib.request.urlretrieve(url, dest)
        if not os.path.exists(dest):
            print(f"MISSING {s['id']}: {rel}"); bad += 1; continue
        got = hashlib.sha256(open(dest, "rb").read()).hexdigest()
        if got != want:
            print(f"SHA MISMATCH {s['id']}: {rel} is {got[:12]}…, registry says {want[:12]}…"); bad += 1
        else:
            print(f"ok {s['id']}: {rel}")
sys.exit(1 if bad else 0)
