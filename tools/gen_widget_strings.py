#!/usr/bin/env python3
"""BD-1 I4 — the hard-capped audited widget string pool. Reads the keys
from the app's audited locale files and emits Localizable.strings per
locale for the extension. Adding a widget string = adding the key to the
pool here AND the locales; the cap check fails the build on drift."""
import json, glob, pathlib, sys

POOL = ["widget.daily.name", "widget.daily.desc", "widget.daily.done",
        "widget.daily.notyet", "widget.streak.name", "widget.streak.desc",
        "widget.streak.unit"]
CAP = 8  # hard cap: grow only with an audit event

assert len(POOL) <= CAP, "widget string pool exceeded its audited cap"
root = pathlib.Path("ios/App/SpellWidgets")
missing = []
for f in sorted(glob.glob("src/i18n/locales/*.json")):
    code = pathlib.Path(f).stem
    d = json.load(open(f))
    lines = []
    for k in POOL:
        if k not in d:
            missing.append(f"{code}:{k}")
            continue
        v = d[k].replace('"', '\\"')
        lines.append(f'"{k}" = "{v}";')
    out = root / f"{code}.lproj"
    out.mkdir(parents=True, exist_ok=True)
    (out / "Localizable.strings").write_text("\n".join(lines) + "\n")
if missing:
    print("WIDGET POOL AUDIT FAIL — keys missing:", missing)
    sys.exit(1)
print(f"widget strings: {len(POOL)} keys x 16 locales, cap {CAP}")
