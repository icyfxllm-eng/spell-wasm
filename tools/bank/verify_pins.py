#!/usr/bin/env python3
"""F2 gate step: every non-null compositePin's artifact must exist and
hash-match; every UNPIN in the decisions log must carry Eric's ACK line
after it. Null pins are legal (pre-recipe era) and report as such."""
import hashlib, json, pathlib, sys

d = json.loads(pathlib.Path("config/bank_floors.json").read_text())
log = pathlib.Path("docs/BANK-DECISIONS.log")
fail = False
for lang, row in d["languages"].items():
    pin = row.get("compositePin")
    if pin is None:
        continue
    art = pathlib.Path(pin["artifact"])
    if not art.exists():
        print(f"PIN FAIL {lang}: artifact {art} missing"); fail = True; continue
    h = hashlib.sha256(art.read_bytes()).hexdigest()
    if h != pin["hash"]:
        print(f"PIN FAIL {lang}: hash drift (the pin is law — D2)"); fail = True
if log.exists():
    lines = log.read_text().splitlines()
    for i, line in enumerate(lines):
        if "ACK-REQUIRED" in line:
            lang = line.split()[1]
            if not any(l.startswith(f"ACK {lang}") for l in lines[i + 1:]):
                print(f"PIN FAIL: {line.strip()} — awaiting Eric's ACK {lang} line"); fail = True
sys.exit(1 if fail else 0)
