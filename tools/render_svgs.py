#!/usr/bin/env python3
"""Word-picture SVGs from scanlock placements (Eric's re-grade set)."""
import json, math, pathlib, subprocess, sys, unicodedata
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
POOLS = ROOT / "content-pipeline/wordpic/pools"
BIN = ROOT / "target/release/scanlock-render"
OUT = pathlib.Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)
BAND_MAX = {"easy": 40.0, "medium": 32.0, "hard": 24.0, "expert": 30.0}
ONLY = set(sys.argv[2:])  # optional subject filter, for quick re-checks
for f in sorted(SCANS.glob("*.json")):
    if ONLY and f.stem not in ONLY: continue
    sub = f.stem
    doc = json.loads(f.read_text())
    tier = doc["tier"]
    pool = json.loads((POOLS/f"en-{tier}.json").read_text()) + json.loads((POOLS/"en-easy.json").read_text())
    lines = [f"WORD {w} {len(unicodedata.normalize('NFC', w))}" for w in pool if " " not in w]
    inp = [f"PARAMS 13.0 {BAND_MAX[tier]} 0.5257 1.0 1"]
    for e in doc["paths"]:
        segs = " ".join(str(t) for t in e["segments"])
        pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
        inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} {int(e.get('micro_feature', False))} | {segs} | {pts}")
    r = subprocess.run([str(BIN)], input="\n".join(inp+lines).encode(), capture_output=True)
    j = json.loads(r.stdout)
    if "error" in j:
        print(sub, "BLOCKED/ERR", j["error"]); continue
    if "render_blocked" in j:
        print(sub, "RENDER_BLOCKED", j["render_blocked"]); continue
    if not j.get("placements"):
        print(sub, "RENDER_BLOCKED: zero words planned"); continue
    svg = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512"><rect width="512" height="512" fill="#101623"/>']
    # v8.2 F5 as written: ink that words CANNOT host renders as the plain
    # pinned stroke — micro paths (snowman eyes/nose), sub-floor and
    # decorative paths, and any run left unhosted by keep-outs or size.
    # Where words go, no stroke. Erase the letters AND the tagged strokes
    # and the pinned scan comes back whole.
    import math as _m
    def _plen(p):
        return sum(_m.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))
    placed_by_path = {}
    for pl in j["placements"]:
        placed_by_path.setdefault(pl["path"], []).append((pl["t0"], pl["t1"]))
    stroke_w = max(1.6, (j["placements"][0]["size"] if j["placements"] else 13) * 0.13)
    for pi, e in enumerate(doc["paths"]):
        pts = e["points"]
        if len(pts) < 2:
            continue
        cum = [0.0]
        for a, b in zip(pts, pts[1:]):
            cum.append(cum[-1] + _m.hypot(b[0]-a[0], b[1]-a[1]))
        tot = cum[-1] or 1.0
        spans = sorted(placed_by_path.get(pi, []))
        gaps = []
        cur = 0.0
        for t0, t1 in spans:
            if t0 > cur + 1e-4:
                gaps.append((cur, t0))
            cur = max(cur, t1)
        if cur < 1.0 - 1e-4:
            gaps.append((cur, 1.0))
        # A small CLOSED unworded loop is a solid feature in the source
        # ink (snowman eyes, nose): render it filled, not as a hairline
        # ring — an eye must read as an eye.
        closed = _m.hypot(pts[0][0]-pts[-1][0], pts[0][1]-pts[-1][1]) < 3.0
        if closed and not spans and _plen(pts) < 120.0:
            d2 = "M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in pts) + " Z"
            svg.append(f'<path d="{d2}" fill="#e8ecf5" fill-opacity="0.95" stroke="#e8ecf5" '
                       f'stroke-width="{stroke_w:.1f}" stroke-linejoin="round"/>')
            continue
        for g0, g1 in gaps:
            seg = [p for p, c in zip(pts, cum) if g0 * tot <= c <= g1 * tot]
            if len(seg) < 2 or _plen(seg) < 3.0:
                continue
            d2 = "M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in seg)
            svg.append(f'<path d="{d2}" fill="none" stroke="#e8ecf5" stroke-opacity="0.92" '
                       f'stroke-width="{stroke_w:.1f}" stroke-linecap="round" stroke-linejoin="round"/>')
    for k, pl in enumerate(j["placements"]):
        d = "M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in pl["baseline"])
        svg.append(f'<defs><path id="{sub}-b{k}" d="{d}"/></defs>')
        # justified: per-glyph placement via textLength over the full segment
        seg_len = sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(pl["baseline"], pl["baseline"][1:]))
        svg.append(f'<text font-size="{pl["size"]:.1f}" fill="#e8ecf5" font-family="Helvetica">'
                   f'<textPath href="#{sub}-b{k}" textLength="{seg_len:.1f}" lengthAdjust="spacing">{pl["word"]}</textPath></text>')
    # v8.2.1 (2) — required-micro gate: every named feature in the scan
    # file must be present in the drawn output or nothing displays.
    req = doc.get("required_micro", [])
    if req:
        import math as _mm
        drawn = []
        for pi, e in enumerate(doc["paths"]):
            if not e.get("micro_feature"):
                continue
            if any(pl["path"] == pi for pl in j["placements"]):
                continue  # hosted by words, not drawn as a feature
            cx = sum(x for x, _ in e["points"]) / len(e["points"])
            cy = sum(y for _, y in e["points"]) / len(e["points"])
            drawn.append((cx, cy))
        missing = [r["name"] for r in req
                   if not any(_mm.hypot(cx - r["near"][0], cy - r["near"][1]) <= r["tol"]
                              for cx, cy in drawn)]
        if missing:
            print(f"{sub} RENDER_BLOCKED: required micro features missing: {missing}")
            continue
    svg.append("</svg>")
    (OUT/f"{sub}-scanlock.svg").write_text("".join(svg))
    print(sub, "ok", len(j["placements"]), "words")
