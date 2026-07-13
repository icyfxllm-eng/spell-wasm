# -*- coding: utf-8 -*-
"""Validate an audit/<lang>/findings.json against the declared schema.

Exit 0 iff every finding conforms and every Feature 1-6 is represented in the
report. Used as a cheap CI/PR gate so a malformed audit can't slip through.

  python3 tools/audit/validate_findings.py es
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
lang = sys.argv[1] if len(sys.argv) > 1 else "es"
base = ROOT / "audit" / lang

SEV = {"critical", "violation", "warning", "info"}
REQ = {"feature", "severity", "class", "file", "key", "detail", "proposed_fix"}
errs = []

for name in ("report.md", "findings.json", "auditor-packet.md"):
    if not (base / name).exists():
        errs.append(f"missing deliverable: audit/{lang}/{name}")

if (base / "findings.json").exists():
    doc = json.loads((base / "findings.json").read_text(encoding="utf-8"))
    fs = doc.get("findings", [])
    for i, f in enumerate(fs):
        if set(f) != REQ:
            errs.append(f"finding[{i}] fields {set(f)} != {REQ}")
        if f.get("severity") not in SEV:
            errs.append(f"finding[{i}] bad severity {f.get('severity')!r}")
        if not isinstance(f.get("feature"), int) or not (1 <= f["feature"] <= 6):
            errs.append(f"finding[{i}] bad feature {f.get('feature')!r}")

if (base / "report.md").exists():
    report = (base / "report.md").read_text(encoding="utf-8")
    for feat in range(1, 7):
        if f"## Feature {feat} —" not in report:
            errs.append(f"report.md missing Feature {feat} section")

if errs:
    print(f"INVALID audit/{lang}: {len(errs)} problem(s)")
    for e in errs:
        print("  -", e)
    sys.exit(1)
print(f"OK audit/{lang}: findings.json conforms; all Feature 1-6 sections present")
sys.exit(0)
