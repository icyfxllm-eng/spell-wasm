#!/usr/bin/env python3
"""Bundle the scan library into the shipping manifest (path data only —
references never ship, I6). Coordinates quantized to 0.5px: below the
0.05px residual epsilon's visibility and it halves the payload."""
import json, pathlib
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
MANIFESTS = ROOT / "content-pipeline/wordpic/manifests"
OUT = ROOT / "config/wordpic/scans.json"
TIER_ORDER = {"easy": 0, "medium": 1, "hard": 2, "expert": 3}
subjects = {}
for f in sorted(SCANS.glob("*.json")):
    d = json.loads(f.read_text())
    paths = []
    kept = []  # scan-doc index of each bundled path, in bundle order
    for i, e in enumerate(d["paths"]):
        pts = [[round(x * 2) / 2, round(y * 2) / 2] for x, y in e["points"]]
        ded = [pts[0]]
        for p in pts[1:]:
            if p != ded[-1]:
                ded.append(p)
        if len(ded) < 2:
            continue
        kept.append(i)
        paths.append({
            "p": ded,
            "s": [round(t, 4) for t in e["segments"]],
            "f": (1 if e["sub_floor"] else 0) | (2 if e["decorative_thin"] else 0)
                 | (4 if e.get("micro_feature") else 0),
        })
    # CC-PICTURE-BANK F3: the runtime ladder is the manifest's ladder,
    # copied — not re-derived — so device, manifest, and manifest_check all
    # speak about the same strata. Manifest path_ids index the SCAN DOC;
    # quantization above can drop degenerate paths, so ids are remapped
    # through the kept list rather than assumed equal.
    mf = MANIFESTS / f"{d['subject']}.json"
    if not mf.exists():
        raise SystemExit(f"{d['subject']}: no manifest — run build_manifests.py first "
                         "(F3 ships each subject's layer ladder from its manifest)")
    remap = {old: new for new, old in enumerate(kept)}
    layers = [{"n": L["name"], "p": [remap[i] for i in L["path_ids"] if i in remap]}
              for L in json.loads(mf.read_text())["layers"]]
    covered = sorted(i for L in layers for i in L["p"])
    if covered != list(range(len(paths))):
        raise SystemExit(f"{d['subject']}: manifest layers must cover every bundled "
                         f"path exactly once (got {covered}, want 0..{len(paths) - 1})")
    subjects[d["subject"]] = {
        "tier": d["tier"],
        "attr": d.get("attribution", ""),
        "req": d.get("required_micro", []),
        "layers": layers,
        "paths": paths,
    }
OUT.write_text(json.dumps({"v": "8.2", "subjects": subjects}, separators=(",", ":")))
kb = OUT.stat().st_size / 1024
print(f"bundled {len(subjects)} subjects -> {OUT} ({kb:.0f} KB)")
