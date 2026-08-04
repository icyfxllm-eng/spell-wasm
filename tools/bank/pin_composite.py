#!/usr/bin/env python3
"""CC-BANK-COMPLETE F2 — the composite pin law (D2 signed).

pin:   python3 tools/bank/pin_composite.py <lang> <artifact.jsonl> \
              --wordfreq-ver X --freqwords-snap DATE --leipzig-year YYYY
unpin: python3 tools/bank/pin_composite.py <lang> --unpin "reason"
       -> appends an ACK-REQUIRED line to docs/BANK-DECISIONS.log; CI
          (verify_pins.py in the gate) stays RED until a matching
          "ACK <lang>" line signed by Eric lands below it.
Re-pinning is an Eric-acknowledged EVENT, never a silent drift.
"""
import hashlib, json, pathlib, sys

FLOORS = pathlib.Path("config/bank_floors.json")
LOG = pathlib.Path("docs/BANK-DECISIONS.log")

def main():
    lang = sys.argv[1]
    d = json.loads(FLOORS.read_text())
    assert lang in d["languages"], f"unknown language {lang}"
    if "--unpin" in sys.argv:
        reason = sys.argv[sys.argv.index("--unpin") + 1]
        old = d["languages"][lang]["compositePin"]
        d["languages"][lang]["compositePin"] = None
        FLOORS.write_text(json.dumps(d, ensure_ascii=False, indent=1))
        with LOG.open("a") as f:
            f.write(f"UNPIN {lang} (was {old and old['hash'][:12]}): {reason} — ACK-REQUIRED\n")
        print(f"unpinned {lang}; CI is RED until Eric ACKs in {LOG}")
        return
    artifact = pathlib.Path(sys.argv[2])
    h = hashlib.sha256(artifact.read_bytes()).hexdigest()
    def arg(flag):
        return sys.argv[sys.argv.index(flag) + 1] if flag in sys.argv else None
    d["languages"][lang]["compositePin"] = {
        "hash": h,
        "artifact": str(artifact),
        "wordfreq": arg("--wordfreq-ver"),
        "frequencyWords": arg("--freqwords-snap"),
        "leipzig": arg("--leipzig-year"),
    }
    FLOORS.write_text(json.dumps(d, ensure_ascii=False, indent=1))
    print(f"pinned {lang}: {h[:16]}…")

if __name__ == "__main__":
    main()
