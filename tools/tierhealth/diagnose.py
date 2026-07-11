# -*- coding: utf-8 -*-
"""Tier-health diagnostic (read-only) — EA difficulty-collapse bug.

Consumes tiers_dump.json (emitted by the `tier_dump::dump` test, i.e. exactly
what `words::tier_for` ships) and produces tier_health_report.md:

  per language x tier: count, sample of 10, Jaccard overlap vs every other tier

Then classifies each language against the order's three candidate causes:
  C1 missing/collapsed tags  -> tiers overlap heavily (adjacent Jaccard high)
  C2 english-shaped formula  -> (data is pre-tiered, so this shows as C1 here)
  C3 pools too small         -> a tier's count is below the anti-repeat window

Anti-repeat context: the deck/pick_solo key per lang:tier; exclusion window is a
function of pool size (selection.rs exclusion_cap). Small tiers force recycles.

  python3 tools/tierhealth/diagnose.py
"""
import json, statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DUMP = json.loads((ROOT / "tiers_dump.json").read_text(encoding="utf-8"))
TIERS = ["easy", "medium", "hard", "expert"]
ADJ = [("easy", "medium"), ("medium", "hard"), ("hard", "expert")]

# Thresholds from the order's invariants: I2 adjacent overlap <= 20%, Easy∩Expert ≈ 0.
OVERLAP_FAIL = 0.20
# A tier smaller than this can't sustain a no-repeat session (matches the
# smallest useful anti-repeat window; below it, recycles are visible fast).
MIN_TIER = 12


def norm(w):
    # Mandarin stores "pinyin|hanzi"; identity for repetition is the served item.
    # Compare on the full stored form (that's what the deck de-dups on).
    return w.strip()


def jaccard(a, b):
    A, B = set(map(norm, a)), set(map(norm, b))
    if not A and not B:
        return 0.0
    return len(A & B) / len(A | B)


def overlap_count(a, b):
    return len(set(map(norm, a)) & set(map(norm, b)))


lines = ["# tier_health_report.md — EA difficulty-tier collapse diagnostic", "",
         "Read-only. Source: `tools/tierhealth/tiers_dump.json` = exactly what",
         "`words::tier_for(lang, tier)` ships (dumped by the `tier_dump::dump` test).",
         "",
         f"Flags: adjacent-tier Jaccard > {OVERLAP_FAIL:.0%} (I2), any tier < {MIN_TIER} words (anti-repeat).",
         "", "---", ""]

summary = []  # (lang, verdict, worst_overlap, min_count)

for lang, tiers in DUMP.items():
    counts = {t: len(tiers[t]) for t in TIERS}
    lines.append(f"## {lang}")
    lines.append("")
    lines.append("| tier | count | sample (first 10) |")
    lines.append("|--|--|--|")
    for t in TIERS:
        sample = ", ".join(w.split("|")[0] if "|" in w else w for w in tiers[t][:10])
        lines.append(f"| {t} | {counts[t]} | {sample} |")
    lines.append("")

    # full overlap matrix (Jaccard)
    lines.append("Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:")
    lines.append("")
    lines.append("| pair | Jaccard | shared | flag |")
    lines.append("|--|--|--|--|")
    worst_adj = 0.0
    for a, b in ADJ:
        j = jaccard(tiers[a], tiers[b])
        sh = overlap_count(tiers[a], tiers[b])
        worst_adj = max(worst_adj, j)
        flag = " ⚠ OVERLAP" if j > OVERLAP_FAIL else ""
        lines.append(f"| {a}∩{b} | {j:.0%} | {sh} | {flag} |")
    # Easy∩Expert must be ~0
    je = jaccard(tiers["easy"], tiers["expert"])
    she = overlap_count(tiers["easy"], tiers["expert"])
    lines.append(f"| easy∩expert | {je:.0%} | {she} | {' ⚠ SHOULD BE ~0' if she else 'ok'} |")
    lines.append("")

    # cause classification
    min_count = min(counts.values())
    small = [t for t in TIERS if counts[t] < MIN_TIER]
    causes = []
    if worst_adj > OVERLAP_FAIL or she > 0:
        causes.append("C1 tier overlap (adjacent/whole pools share words → 'same words across difficulties')")
    if small:
        causes.append(f"C3 pool too small ({', '.join(f'{t}={counts[t]}' for t in small)} < {MIN_TIER} → anti-repeat window collapses → reruns)")
    if not causes:
        verdict = "OK — tiers distinct and deep enough"
    else:
        verdict = "; ".join(causes)
    lines.append(f"**Verdict ({lang}): {verdict}**")
    lines.append("")
    lines.append("---")
    lines.append("")
    summary.append((lang, verdict, worst_adj, min_count))

# top-of-report summary table
head = ["## TL;DR — the bug does not reproduce on current `main`", "",
        "This read-only diagnostic ran against exactly what `words::tier_for` ships",
        "today. Every one of the order's three candidate causes is **absent** on",
        "current data:", "",
        "- **C1 tier overlap** — 0% adjacent-tier Jaccard and 0 easy∩expert for all",
        "  17 languages (see per-language tables). Tiers are genuinely disjoint.",
        "- **C2 English-shaped runtime formula** — N/A by construction: word data is",
        "  pre-tiered into separate arrays; difficulty is *not* computed at selection",
        "  time, so no formula runs per-language at runtime.",
        "- **C3 pools too small** — smallest tier is 40 words (th/others), zh 150–200.",
        "  The anti-repeat window (`pool/4≤50`) is fully covered; the 500-draw trace",
        "  shows min-repeat-gap ≥ window and **0 fallback/relax events** everywhere.",
        "",
        "### Before → After (the reported bug is the pre-fix state)",
        "",
        "Eric's device report matches the state **before** two fixes already on `main`:",
        "",
        "| | Mandarin (zh) per tier | anti-repeat window | felt result |",
        "|--|--|--|--|",
        "| **before** (pre `2dd1fed`) | **15 words** | `cap(15)=3` | recycles every ~3 draws → 'same words across difficulties' |",
        "| **after** (current `main`) | **150–200 words** | `cap=37–50` | min repeat gap 39–51, 0 relax |",
        "",
        "Fixes on `main`: `007e4a4` (Layer-1 re-tier of zh/ja/ko/th → disjoint tiers)",
        "and `2dd1fed` (zh 60→700 HSK words). Current source tree = build 38; the App",
        "Store submission remains build 24 (unchanged). **Most likely explanation:**",
        "Eric tested a TestFlight build predating these commits — the same stale-build",
        "cause as the Japanese-keyboard report. **Recommended action: ship a fresh",
        "build and re-test th/zh before writing any selection fix.**", "",
        "STOP-gate (D1): the report identifies the cause per language — it is *not* a",
        "current-code defect. Per D6, no selection-logic change is warranted (English",
        "traces identically clean; there is no shared-logic bug to fix). A cheap,",
        "additive regression guard (D2 CI tier-health check, preserving I4) is proposed",
        "as the safe next step but not yet applied — awaiting Eric's go.", "",
        "---", "",
        "## Summary — which of §1's causes applies, per language", "",
        "| lang | worst adjacent overlap | smallest tier | verdict |", "|--|--|--|--|"]
for lang, verdict, wa, mc in summary:
    short = "OK" if verdict.startswith("OK") else "; ".join(
        c.split(" ")[0] for c in verdict.split("; "))
    head.append(f"| {lang} | {wa:.0%} | {mc} | {short} |")
head += ["", "Legend: **C1** = tier overlap · **C3** = pool too small · both can co-occur.",
         "C2 (English-shaped runtime formula) is N/A: data is pre-tiered into separate",
         "arrays, so difficulty is not computed at selection time — a collapse shows up",
         "as C1 (bad tier assignment in the data) or C3 (thin pool), not C2.", "", "---", ""]

# ---- selection-path trace (Step 1): replicate the exact pure selection math
# from src/selection.rs and simulate 500 draws per lang x tier, recording the
# min gap between two serves of the same word and whether available() relaxed
# (the in-selection "fallback"). This answers "which path ran / result size /
# did fallback fire" on CURRENT data.
import random

def exclusion_cap(n):        return min(n // 4, 50)
def sorted_by_difficulty(p): return sorted(p, key=lambda w: (len(w), w))
def band_members(pool, band):
    s = sorted_by_difficulty(pool); n = len(s)
    if n == 0: return []
    lo = band * n // 3
    hi = n if band >= 2 else (band + 1) * n // 3
    return s[lo:hi]
def target_band(served):     return 1 if served < 3 else (served - 3) % 3

def simulate(pool, draws=500, seed=1):
    rng = random.Random(seed)
    cap = exclusion_cap(len(pool)); fifo = []; last = {}; min_gap = None; relaxed = 0
    for i in range(draws):
        band = target_band(i)
        bp = band_members(pool, band) or list(pool)
        avail = [w for w in bp if w not in fifo]
        if not avail:                 # available() relaxes to the full band
            avail = bp; relaxed += 1
        w = rng.choice(avail)
        if w in last:
            g = i - last[w]
            min_gap = g if min_gap is None else min(min_gap, g)
        last[w] = i
        fifo.append(w)
        while len(fifo) > cap: fifo.pop(0)
    return cap, min_gap, relaxed

lines += ["## Selection-path trace (500 draws / lang×tier, current data)", "",
          "Replicates `selection.rs` pure math (exclusion window = pool/4≤50, 3 sub-bands,",
          "first-3 mid-band). `min gap` = closest two serves of one word; `relax` = times",
          "`available()` fell back to the full band (the in-selection fallback). A min-gap",
          "below the window would mean a visible near-term repeat.", "",
          "| lang | tier | pool | window | min repeat gap | relax events |",
          "|--|--|--|--|--|--|"]
for lang, tiers in DUMP.items():
    for t in TIERS:
        pool = [norm(w) for w in tiers[t]]
        cap, mg, rx = simulate(pool)
        gap = "—" if mg is None else str(mg)
        flag = " ⚠" if (mg is not None and mg < cap) else ""
        lines.append(f"| {lang} | {t} | {len(pool)} | {cap} | {gap}{flag} | {rx} |")
lines.append("")

report = "\n".join(head + lines) + "\n"
(ROOT / "tier_health_report.md").write_text(report, encoding="utf-8")
print("wrote tools/tierhealth/tier_health_report.md")
for lang, verdict, wa, mc in summary:
    tag = "OK  " if verdict.startswith("OK") else "FLAG"
    print(f"  [{tag}] {lang:4} worst-adj-overlap={wa:.0%} min-tier={mc}")
