#!/usr/bin/env python3
"""CC-MASTERPIECE-TONAL / FACEPASS — mona's tonal EXPORT (staged).

The flip is atomic at export: extractionClass LINE -> TONAL +
requiredFeatures, paths replaced by the tonal trace. This tool builds
the registry entry and REFUSES to arm the real flip until every gate
in the face session is recorded — D4 is law: export BLOCKS until the
operator explicitly touches the smile ("it looked fine" is not an
exception).

Gates read from facepass-verdicts.json (written at Eric's session):
  contourApproved, landmarksConfirmed, perFeaturePass, handsLasso,
  smileTouched  — all must be true; smileTouched also flips the trace's
  facePass.focalTouched.

Modes:
  --stage  write staged-mona-entry.json + gate report (always allowed)
  --flip   apply to config/wordpic/pictures.json (only when gates green)

Selection law (v1, documented for the review):
  hosts  = 11 iconic vocabulary strokes (band 2, budget [1,4], the
           closed FACEPASS set; lipParting focal:true, order = MAX)
         + flow paths per band, longest first, budgeted so estimated
           slots stay inside mona's 12..=110 slot law
  guides = band boundary paths (the outline look) + unselected flow
  order  = flow by band (lightest first, words-as-shading builds the
           shadows last), then iconic strokes, then the smile — F4.
"""
import json, math, pathlib, sys

SCRATCH = pathlib.Path("/private/tmp/claude-501/-Users-eric/d5d833c7-982c-4f16-9353-8fe37d983a28/scratchpad")
ROOT = pathlib.Path("/Users/eric/Desktop/xcode-backup/repos/spell-wasm")
REG = ROOT / "config/wordpic/pictures.json"
TRACE = SCRATCH / "mona-tonal.json"
VERDICTS = SCRATCH / "facepass-verdicts.json"

VOCAB = ["leftEyeLid", "leftEyeCrease", "leftBrow", "rightEyeLid",
         "rightEyeCrease", "rightBrow", "noseBridge", "noseBase",
         "lipParting", "lowerLipShadow", "chinCrescent"]
# The ledger says TWELVE features arrive with the export; the trace
# carries these eleven. The twelfth is NOT invented here — it is an
# open question ON the face-session checklist. Export blocks until the
# list is confirmed (or amended) by Eric.
TWELFTH_OPEN = True

GATES = ["contourApproved", "landmarksConfirmed", "perFeaturePass",
         "handsLasso", "smileTouched"]

SLOT_TARGET = 96          # inside mona's 12..=110 law with headroom
EST_PX_PER_SLOT = 64.0

def arclen(p):
    return sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))

def dstr(p):
    return "M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in p)

def gate_report():
    v = json.loads(VERDICTS.read_text()) if VERDICTS.exists() else {}
    rows = [(g, bool(v.get(g))) for g in GATES]
    rows.append(("twelfthFeatureNamed", bool(v.get("twelfthFeatureNamed"))))
    return rows, all(ok for _, ok in rows)

def build_entry():
    t = json.loads(TRACE.read_text())
    cands = t["candidates"]
    feats = {c["feature"]: c for c in cands if c["pathKind"] == "feature"}
    missing = [f for f in VOCAB if f not in feats]
    assert not missing, f"vocabulary strokes missing from trace: {missing}"

    flows = [c for c in cands if c["pathKind"] == "flow"]
    bounds = [c for c in cands if c["pathKind"] == "boundary"]

    # flow selection: per band, longest first, global slot budget
    by_band = {}
    for c in flows:
        by_band.setdefault(c["bandIndex"], []).append(c)
    for b in by_band.values():
        b.sort(key=lambda c: arclen(c["points"]), reverse=True)
    face_slots = len(VOCAB)  # one word per iconic stroke
    budget = SLOT_TARGET - face_slots
    chosen, est = [], 0.0
    bands_sorted = sorted(by_band)  # lightest..darkest by index order
    i = 0
    while est < budget:
        took = False
        for b in bands_sorted:
            if by_band[b][i:i+1]:
                c = by_band[b][i]
                s = max(1.0, arclen(c["points"]) / EST_PX_PER_SLOT)
                if est + s > budget:
                    continue
                chosen.append(c)
                est += s
                took = True
        if not took:
            break
        i += 1

    chosen.sort(key=lambda c: (c["bandIndex"], -arclen(c["points"])))
    paths, order = [], 0
    for c in chosen:
        order += 1
        L = arclen(c["points"])
        paths.append({"mode": "flow", "d": dstr(c["points"]), "order": order,
                      "budget": [2, 10] if L < 420 else [3, 20],
                      "band": min(4, int(c["bandIndex"]) + 1),
                      "arch": "tonal-flow", "feature": f"band{c['bandIndex']}"})
    for name in VOCAB:
        if name == "lipParting":
            continue
        order += 1
        paths.append({"mode": "flow", "d": dstr(feats[name]["points"]),
                      "order": order, "budget": [1, 4], "band": 2,
                      "arch": "tonal-feature", "feature": name})
    order += 1  # F4: the smile schedules LAST — max export order, focal
    paths.append({"mode": "flow", "d": dstr(feats["lipParting"]["points"]),
                  "order": order, "budget": [1, 4], "band": 2,
                  "arch": "tonal-feature", "feature": "lipParting",
                  "focal": True})

    guide = [dstr(c["points"]) for c in bounds][:200]
    return {
        "extractionClass": "TONAL",
        "requiredFeatures": VOCAB[:],   # + the confirmed twelfth at flip
        "paths": paths,
        "guide": guide,
    }, {"flows": len(chosen), "estSlots": round(face_slots + est),
        "guides": min(len(bounds), 200)}

def main():
    rows, green = gate_report()
    patch, stats = build_entry()
    (SCRATCH / "staged-mona-entry.json").write_text(json.dumps(patch, indent=1))
    print("STAGED staged-mona-entry.json —", stats)
    print("GATES:")
    for g, ok in rows:
        print(f"  [{'x' if ok else ' '}] {g}")
    if "--flip" not in sys.argv:
        return
    if not green:
        sys.exit("EXPORT BLOCKED: the face session gates above are not all "
                 "recorded. D4 is law — the smile must be touched.")
    reg = json.loads(REG.read_text())
    mona = next(p for p in reg["pictures"] if p["id"] == "mona")
    mona.update(patch)
    REG.write_text(json.dumps(reg, ensure_ascii=False, indent=1))
    print("FLIPPED: mona is TONAL. Run the scoped sweep + mona_bands_and_density.")

if __name__ == "__main__":
    main()
