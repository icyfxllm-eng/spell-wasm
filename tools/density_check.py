#!/usr/bin/env python3
"""CC-MASTERPIECE-RECOG — the density floor and the ratchet.

Two checks. The FLOOR is an absolute minimum, and it reaches only the seven
masters because no threshold generalizes past them. The RATCHET is relative
and covers all 382 traces: no picture may ever get thinner than it already is.
The floor says how much substance a masterpiece needs; the ratchet says nobody
quietly takes substance away from anything.


The Aug 11 review blamed noise. The bank says starvation. Measured across all
382 traces, hand-traced pieces run a median of 857 points and suggester output
runs 60 — a 14x gap — and the two masterpieces that failed review are exactly
the two suggester-derived masterpieces:

    greatwave    2076  hand-traced                recognizable
    scream       1128  hand-traced                marginal
    rhino         786  hand-traced                (not reviewed)
    mona          445  hand-traced   TONAL        the reference success
    redfuji       214  suggested-then-approved    read as Starry Night
    starrynight   138  suggested-then-approved    the piece it was mistaken for
    sunflowers      -  no scan at all             rendered as a scribble

Red Fuji's three anchors carry 8, 15 and 8 points. No stroke-weighting rule
can rescue that; there is nothing there to weight. So the F2 hierarchy lints
are necessary and not sufficient — they check the SHAPE of a trace, and this
checks whether there is enough of one to shape. Red Fuji passes every F2 lint
today.

WHY THE FLOOR IS MASTERS-ONLY. Category is not complexity: a 200-point floor
would fail 15 of 17 landmarks, because "landmark" covers both a signpost and a
Hokusai. The bank's own spread makes any global floor either vacuous or
absurd; `masters` is the one category whose members are all inherently dense
subjects. A per-anchor floor was tried and abandoned — it cannot separate the
failures from the successes, since Mona's thinnest legitimate anchor (13
points) is thinner than Red Fuji's (15).

WHY TONAL IS EXEMPT rather than tuned to pass. On a TONAL piece substance
lives in posterized bands, not vertices; counting points measures the wrong
thing. Mona would happen to clear a 350-point floor, but that is a
coincidence of her trace, not a property of the medium. TONAL carries its own
substance check already: every TONAL subject must declare requiredFeatures,
asserted in src/wordpic.rs.
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
PICTURES = ROOT / "config/wordpic/pictures.json"
QUARANTINE = ROOT / "config/wordpic/density_quarantine.json"
BASELINE = ROOT / "config/wordpic/density_baseline.json"

# The ratchet's tolerance. Re-bundling can shift a point count slightly through
# quantization; a real regression is not subtle — Red Fuji sits 14x under the
# hand-traced median. 5% absorbs the former and cannot hide the latter.
RATCHET_SLACK = 0.95

# Calibrated to the recorded verdicts, not chosen round. The sparsest LINE
# master that reads correctly is the Rhinoceros at 786; the densest that fails
# is Red Fuji at 214. 500 sits between them with 57% headroom under Rhino and
# fails Fuji by more than half. Raising it above 786 would condemn a working
# piece; dropping it below 215 would admit a known failure.
MASTERS_FLOOR = 500

# Masters must be traced by hand. Every hand-traced master read acceptably;
# both suggester-derived masters failed. The suggester is a fine starting
# point for a cactus and is not a deliverable for a Hokusai.
MASTERS_AUTHORING = {"hand-traced"}

# THE SHIPPED-LAYOUT FLOOR, added 2026-08-15.
#
# Everything above measures content-pipeline/wordpic/scans — the authoring
# INPUT. Nothing rendered on a device has ever read that file. What ships is
# the "d" strings in config/wordpic/pictures.json, and the two are not close:
#
#     greatwave   scan 2076 -> ships   8 paths      scream  scan 1128 -> 10
#     mona        scan  445 -> ships  21 paths      rhino   scan  786 -> 39
#     redfuji     scan  214 -> ships   1 path       starrynight  138 ->  1
#
# So Great Wave clears a 500-point floor on a number nobody renders. The floor
# was guarding the wrong artifact, which is the same class of mistake the
# density work exists to catch, aimed one file to the left.
#
# Vertices do not transfer as the shipped measure: the layout deliberately
# reduces a contour to a word carrier, and rhino reads correctly on 103
# vertices spread over 39 short paths. The quantity that separates the
# recorded successes from the recorded failures is the NUMBER OF DISTINCT
# PATHS the layout ships:
#
#     reads correctly   rhino 39, mona 21, scream 10, greatwave 8
#     failed Aug 11     redfuji 1, starrynight 1        (sunflowers 3)
#
# 6 sits in that gap with room on both sides: it clears Great Wave, the
# sparsest piece Eric graded as correct, by 2, and fails a single-skeleton
# masterpiece by 5. A piece that ships as one line cannot be a masterpiece
# whatever its scan says, and that is precisely what shipped twice.
SHIPPED_PATHS_FLOOR = 6


def load_pictures() -> list[dict]:
    return json.loads(PICTURES.read_text())["pictures"]


def trace_of(pid: str) -> dict | None:
    f = SCANS / f"{pid}.json"
    return json.loads(f.read_text()) if f.exists() else None


def points(doc: dict) -> int:
    return sum(len(p["points"]) for p in doc["paths"])


def authoring(doc: dict) -> str:
    return doc.get("authoring") or doc.get("source", {}).get("authoring", "(unset)")


def quarantine() -> dict:
    if not QUARANTINE.exists():
        return {}
    return json.loads(QUARANTINE.read_text()).get("starved", {})


def evaluate(pid: str, pic: dict, doc: dict | None) -> list[str]:
    """The masters substance rules, over data so --selftest can feed them."""
    bad: list[str] = []
    if "masters" not in pic.get("categories", []):
        return bad

    # The shipped layout is checked first and for every class, because it is
    # the only geometry a player ever sees. A TONAL piece gets no exemption
    # here: bands are why its vertex count is meaningless, not a reason for
    # it to ship as two strokes.
    shipped = len(pic.get("paths") or [])
    if shipped < SHIPPED_PATHS_FLOOR:
        bad.append(f"{pid}: ships {shipped} layout path(s), under the "
                   f"{SHIPPED_PATHS_FLOOR}-path shipped floor — this is what "
                   f"renders on the device, whatever the scan holds")

    if doc is None:
        return [f"{pid}: a master with no trace at all — it falls back to the "
                f"legacy solver, which is what rendered Sunflowers as a scribble"]

    auth = authoring(doc)
    if auth not in MASTERS_AUTHORING:
        bad.append(f"{pid}: authored by {auth!r} — masters must be hand-traced. "
                   f"Both suggester-derived masters failed the Aug 11 review")

    if pic.get("extractionClass") == "TONAL":
        # Substance is bands, not vertices; requiredFeatures is TONAL's floor
        # and src/wordpic.rs already asserts it is non-empty.
        return bad

    n = points(doc)
    if n < MASTERS_FLOOR:
        bad.append(f"{pid}: {n} points, under the {MASTERS_FLOOR}-point masters "
                   f"floor — too starved to be recognizable, not too noisy")
    return bad


def ratchet(current: dict[str, int]) -> tuple[list[str], list[str]]:
    """Density may fall for no picture, anywhere in the bank.

    The floor above only reaches the seven masters, because category is not
    complexity — a 200-point floor would fail 15 of 17 landmarks, since
    "landmark" covers both a signpost and a Hokusai. This covers all 382 by
    asking a question that needs no threshold: is this trace thinner than it
    used to be?

    That is the guard the bank actually lacked. Red Fuji is a hand-traced
    subject carrying suggester-grade density, which is what a re-run of the
    suggester over an existing trace produces. Nothing would have caught it.
    """
    if not BASELINE.exists():
        return [], [f"no baseline yet — run {rel(BASELINE)} --accept to record one"]
    return ratchet_eval(json.loads(BASELINE.read_text())["points"], current)


def ratchet_eval(base: dict[str, int], current: dict[str, int]) -> tuple[list[str], list[str]]:
    bad, notes = [], []
    for pid, was in sorted(base.items()):
        now = current.get(pid)
        if now is None:
            bad.append(f"{pid}: had a trace of {was} points, now has none — a "
                       f"deleted or renamed scan is a silent recognizability loss")
        elif now < was * RATCHET_SLACK:
            bad.append(f"{pid}: density fell {was} -> {now} points "
                       f"({100 * (was - now) / was:.0f}% thinner). If this is "
                       f"deliberate, re-record with --accept and say why")
    new = sorted(set(current) - set(base))
    if new:
        notes.append(f"{len(new)} picture(s) not yet in the baseline: "
                     f"{new[:6]}{' ...' if len(new) > 6 else ''} — --accept to record")
    return bad, notes


def rel(p: pathlib.Path) -> str:
    try:
        return str(p.relative_to(ROOT))
    except ValueError:
        return str(p)


def measure() -> dict[str, int]:
    out = {}
    for f in SCANS.glob("*.json"):
        out[f.stem] = points(json.loads(f.read_text()))
    return out


def accept() -> int:
    cur = measure()
    old = json.loads(BASELINE.read_text())["points"] if BASELINE.exists() else {}
    BASELINE.write_text(json.dumps({
        "_comment": "Per-picture density floor: no trace may lose points. "
                    "Generated by tools/density_check.py --accept. A drop is a "
                    "gate failure; re-record deliberately and justify it.",
        "points": {k: cur[k] for k in sorted(cur)},
    }, indent=1) + "\n")
    down = [k for k in cur if k in old and cur[k] < old[k]]
    print(f"density baseline: recorded {len(cur)} pictures "
          f"({len(set(cur) - set(old))} new, {len(down)} LOWERED)")
    for k in sorted(down):
        print(f"  lowered {k}: {old[k]} -> {cur[k]}")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    if "--accept" in sys.argv:
        return accept()

    held = quarantine()
    pics = load_pictures()
    failures, stale, checked = [], [], 0

    for pic in sorted(pics, key=lambda p: p["id"]):
        pid = pic["id"]
        if "masters" not in pic.get("categories", []):
            continue
        checked += 1
        bad = evaluate(pid, pic, trace_of(pid))
        if pid in held:
            # Quarantine shrinks, never grows: once a piece is re-authored the
            # entry must go, or the next regression hides behind a stale pass.
            if not bad:
                stale.append(pid)
        else:
            failures += bad

    for f in failures:
        print(f"FAIL {f}")
    for pid in stale:
        print(f"FAIL {pid}: quarantined as starved but now passes — remove it "
              f"from {QUARANTINE.relative_to(ROOT)}")
    unknown = sorted(set(held) - {p["id"] for p in pics})
    for pid in unknown:
        print(f"FAIL {pid}: quarantined but not in the bank — remove the entry")

    current = measure()
    slipped, notes = ratchet(current)
    for f in slipped:
        print(f"FAIL {f}")
    if failures or stale or unknown or slipped:
        return 1

    print(f"density-floor: OK — {checked} masters, {checked - len(held)} meet the "
          f"{MASTERS_FLOOR}-point scan floor hand-traced and ship at least "
          f"{SHIPPED_PATHS_FLOOR} layout paths")
    if held:
        print(f"  quarantined pending re-authoring: {sorted(held)}")
        for pid, why in sorted(held.items()):
            print(f"    {pid}: {why}")
    print(f"density-ratchet: OK — {len(current)} traces, none thinner than recorded")
    for n in notes:
        print(f"  note: {n}")
    return 0


def _pic(pid, cls="LINE", cats=("masters",), shipped=10):
    # `shipped` is the number of layout paths in pictures.json. It defaults
    # clear of SHIPPED_PATHS_FLOOR so the older cases keep testing the one
    # rule they were written for.
    return {"id": pid, "extractionClass": cls, "categories": list(cats),
            "paths": [{"d": "M0 0 L1 1"}] * shipped}


def _doc(n, auth="hand-traced"):
    return {"paths": [{"points": [[0, 0]] * n}], "authoring": auth}


def selftest() -> int:
    """Prove each rule still bites. Cases assert on the message, not on mere
    truthiness — the F2 selftest shipped blind because it did not."""
    cases = [
        ("starved LINE master", _pic("x"), _doc(214), "under the"),
        ("suggester-derived master", _pic("x"), _doc(2000, "suggested-then-approved"),
         "hand-traced"),
        ("master with no trace", _pic("x"), None, "no trace at all"),
        ("dense hand-traced LINE master", _pic("x"), _doc(786), None),
        # TONAL is exempt from the point floor but not from the authoring rule.
        ("sparse TONAL master (Mona)", _pic("x", "TONAL"), _doc(445), None),
        ("sparse suggester TONAL master", _pic("x", "TONAL"),
         _doc(445, "suggested-then-approved"), "hand-traced"),
        # A 60-point cactus is fine; the floor must not reach outside masters.
        ("thin non-master", _pic("x", "LINE", ("learn",)), _doc(11), None),
        # The shipped-layout floor. Red Fuji's exact shape: a fat scan that
        # clears the point floor, rendering as one line. This is the case the
        # scan-only floor could never see.
        ("single-skeleton master with a fat scan", _pic("x", shipped=1), _doc(2000),
         "shipped floor"),
        ("shipped floor reaches TONAL too", _pic("x", "TONAL", shipped=2), _doc(445),
         "shipped floor"),
        ("Great Wave's shipped width passes", _pic("x", shipped=8), _doc(2076), None),
        ("one path under the floor fails", _pic("x", shipped=5), _doc(786),
         "shipped floor"),
        # Same as above but outside masters: a 3-stroke cactus is not this
        # rule's business either.
        ("narrow non-master layout", _pic("x", "LINE", ("learn",), shipped=3), _doc(60),
         None),
    ]
    bad = []
    for name, pic, doc, want in cases:
        got = evaluate("x", pic, doc)
        if want is None:
            if got:
                bad.append(f"  {name}: expected clean, got {got}")
        elif not any(want in g for g in got):
            bad.append(f"  {name}: expected a failure mentioning {want!r}, "
                       f"got {got or 'clean'}")

    # The ratchet, over the same style of fixture.
    ratchet_cases = [
        ("halved trace", {"a": 800}, {"a": 400}, "density fell"),
        ("deleted trace", {"a": 800}, {}, "now has none"),
        ("trace grew", {"a": 800}, {"a": 2000}, None),
        ("unchanged", {"a": 800}, {"a": 800}, None),
        # Quantization noise must not cry wolf; a real regression is not 3%.
        ("3% quantization drift", {"a": 1000}, {"a": 970}, None),
        ("6% drop", {"a": 1000}, {"a": 940}, "density fell"),
    ]
    for name, base, cur, want in ratchet_cases:
        got, _ = ratchet_eval(base, cur)
        if want is None:
            if got:
                bad.append(f"  ratchet/{name}: expected clean, got {got}")
        elif not any(want in g for g in got):
            bad.append(f"  ratchet/{name}: expected a failure mentioning {want!r}, "
                       f"got {got or 'clean'}")
    cases = cases + ratchet_cases
    if bad:
        print("FAIL density-floor selftest — the gate no longer bites:")
        print("\n".join(bad))
        return 1
    print(f"density-floor selftest: OK — {len(cases)} deliberate cases behave")
    return 0


if __name__ == "__main__":
    sys.exit(main())
