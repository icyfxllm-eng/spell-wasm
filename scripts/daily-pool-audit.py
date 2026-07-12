# -*- coding: utf-8 -*-
"""Daily Challenge pool-size audit (Feature 3).

For every (language, tier) in the daily arc, reports the pool size, the words
served per day (W), and the cycle length L = floor(|pool|/W) — the number of
consecutive dailies before that tier's deck recycles (its guaranteed no-repeat
horizon). Flags tiers that fall short of a 30-day horizon (|pool| >= W*30).

The no-repeat cycle walk holds for L days regardless of size, so a short L is a
"more words wanted" signal (pipeline batch request), not a correctness bug — and
word pools are out of scope for the selection change. This runs read-only and
does NOT fail the build; it prints the table so shortfalls are visible per
language instead of hidden.

  cargo test --lib tier_dump::dump -- --ignored    # refresh tools/tierhealth/tiers_dump.json
  python3 scripts/daily-pool-audit.py
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DUMP = ROOT / "tools" / "tierhealth" / "tiers_dump.json"
HORIZON = 30
# Words-per-day per tier, from daily.rs ARC (standard) — the stress case
# (Kid Mode's 5+5 over easy/medium recycles faster but has a shorter set).
ARC = {"easy": 2, "medium": 3, "hard": 3, "expert": 2}

data = json.loads(DUMP.read_text(encoding="utf-8"))
print(f"Daily Challenge no-repeat horizon (target: L >= {HORIZON} days)\n")
print(f"{'lang':4} {'tier':7} {'pool':>5} {'W':>2} {'L(days)':>8}  status")
print("-" * 44)
short = []
for lang, tiers in data.items():
    for tier, w in ARC.items():
        pool = len(tiers.get(tier, []))
        l = pool // w if w else 0
        ok = pool >= w * HORIZON
        if not ok:
            short.append((lang, tier, pool, w * HORIZON))
        print(f"{lang:4} {tier:7} {pool:5} {w:2} {l:8}  {'ok' if ok else 'SHORT (< %d)' % (w*HORIZON)}")

print()
if short:
    print(f"{len(short)} (lang,tier) below the {HORIZON}-day horizon — pipeline 'add words' requests:")
    for lang, tier, pool, need in short:
        print(f"  {lang}/{tier}: {pool} -> need {need} (+{need - pool})")
    print("\n(No-repeat still holds for each tier's L-day cycle; this is a depth target, not a defect.)")
else:
    print(f"All tiers sustain the {HORIZON}-day no-repeat horizon.")
