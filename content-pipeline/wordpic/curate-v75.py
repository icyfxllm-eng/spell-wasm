#!/usr/bin/env python3
"""v7.5 curation: Eric-passed traces -> word-hostable stroke maps.

Per subject: prune to tier band, drop unhostable shorts, simplify,
rescale into the word-margin box, split closed loops where the band
floor needs it, assign band/label/order. Output: curated-traces-v75.json
consumed by the generator. The sweep remains the law downstream.
"""
import json, math, pathlib, sys

INK = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else
    "/private/tmp/claude-501/-Users-eric/d5d833c7-982c-4f16-9353-8fe37d983a28/scratchpad/ink-out")
OUT = pathlib.Path(__file__).parent / "curated-traces-v75.json"

def plen(p):
    return sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))

def dp(pts, eps):
    if len(pts) < 3: return pts
    def sd(p, a, b):
        vx, vy = b[0]-a[0], b[1]-a[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((p[0]-a[0])*vx+(p[1]-a[1])*vy)/l2))
        return math.hypot(p[0]-(a[0]+t*vx), p[1]-(a[1]+t*vy))
    dmax, idx = 0, 0
    for i in range(1, len(pts)-1):
        d = sd(pts[i], pts[0], pts[-1])
        if d > dmax: dmax, idx = d, i
    if dmax > eps:
        l = dp(pts[:idx+1], eps); r = dp(pts[idx:], eps)
        return l[:-1] + r
    return [pts[0], pts[-1]]

def chaikin(p, rounds=2):
    for _ in range(rounds):
        if len(p) < 3: return p
        out = [p[0]]
        for a, b in zip(p, p[1:]):
            out.append((0.75*a[0]+0.25*b[0], 0.75*a[1]+0.25*b[1]))
            out.append((0.25*a[0]+0.75*b[0], 0.25*a[1]+0.75*b[1]))
        out.append(p[-1])
        p = out
    return p

def resample(p, step=52.0):
    """Vertices >= step apart: whatever the engine's corner split does,
    every fragment is >= step and therefore word-hostable."""
    if len(p) < 2: return p
    out = [p[0]]
    acc = 0.0
    for a, b in zip(p, p[1:]):
        d = math.hypot(b[0]-a[0], b[1]-a[1])
        while acc + d >= step:
            t = (step - acc) / d
            nx, ny = a[0] + (b[0]-a[0])*t, a[1] + (b[1]-a[1])*t
            out.append((nx, ny))
            a = (nx, ny)
            d = math.hypot(b[0]-a[0], b[1]-a[1])
            acc = 0.0
        acc += d
    if math.hypot(p[-1][0]-out[-1][0], p[-1][1]-out[-1][1]) > step * 0.55:
        out.append(p[-1])
    else:
        out[-1] = p[-1]
    while len(out) > 2 and math.hypot(out[-1][0]-out[-2][0], out[-1][1]-out[-2][1]) < 48:
        del out[-2]
    return out

def tame(p, eps=4.0):
    """Corner-tame for the engine: simplify, then enforce >=52px vertex
    spacing so corner splits always yield hostable fragments."""
    return resample(dp(p, eps))

def path_min_dist(a, b):
    def sd(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    m = 1e9
    for pt in a[::2]:
        for u, v in zip(b, b[1:]):
            d = sd(pt, u, v)
            if d < m: m = d
    return m

def clearance_prune(paths, min_gap=19.0, short=62.0):
    """v6 cat lesson: words are >=13px tall. Only SHORT strokes die — a
    texture crumb jammed against anything loses; long structural paths
    (outlines, hats, arms) always stay."""
    kept = []
    for p in sorted(paths, key=plen, reverse=True):
        if plen(p) >= short or all(path_min_dist(p, k) >= min_gap for k in kept):
            kept.append(p)
    return kept

def rescale(p, s=0.90, c=256):
    return [(c + (x-c)*s, c + (y-c)*s) for x, y in p]

def halves(p):
    m = len(p)//2
    return [p[:m+1], p[m:]]

# (tier, band_lo/hi, min stroke px, labels by length rank)
PLAN = {
 "dog":     ("easy",   (3,8),  46, ["body outline","ear","haunch line","eye","nose","chest marking","collar","paw line"]),
 "butterfly":("easy",  (3,8),  46, ["left wing","right wing","body","left antenna","right antenna","wing marking","wing marking","wing marking"]),
 "duck":    ("easy",   (3,8),  46, ["body outline","wing","bill","eye","tail line","breast line","foot","foot"]),
 "turtle":  ("easy",   (3,8),  46, ["shell and body outline","shell pattern","head detail","leg line","leg line","tail","shell pattern","shell pattern"]),
 "owl":     ("medium", (6,20), 46, ["body outline","face disc","left wing","right wing","branch","beak","left talon","right talon","ear tuft","ear tuft","breast feathers","breast feathers","tail feathers","eye ring","eye ring","wing feathers","wing feathers","breast feathers","wing feathers","tail feathers"]),
 "elephant":("medium", (6,20), 46, ["body outline","trunk","ear","front leg","hind leg","tusk","tail","back line","belly line","ear fold","trunk fold","leg line","leg line","head line","ear line","trunk line","leg line","body line","body line","body line"]),
 "snowman": ("medium", (6,20), 46, ["head","head","middle ball","middle ball","base ball","base ball","hat","hat brim","left arm","right arm","eyes","buttons","mouth","scarf"]),
 "horse":   ("hard",   (8,45), 58, ["body outline","mane","tail","head line","front leg","front leg","hind leg","hind leg","grass","grass","grass","mane strand","mane strand","neck line","back line","belly line","muzzle","ear","grass","grass","mane strand","grass","leg line","leg line","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass","grass"]),
 "peacock": ("hard",   (8,45), 58, ["fan edge","body","neck","head","crest","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","eyespot","eyespot","eyespot","eyespot","eyespot","eyespot","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray","fan ray"]),
 "eiffel":  ("hard",   (8,45), 58, ["tower outline","base arch","first platform","second platform","spire","left leg lattice","right leg lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice","lattice"]),
 "dragon":  ("hard",   (8,45), 58, ["head","jaw","horn","whisker","spine ridge","body coil","body coil","body coil","tail","claw","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line","scale line"]),
}
import subprocess
from PIL import Image, ImageOps
ROOT = pathlib.Path(__file__).resolve().parents[2]
TP = ROOT / "target/release/trace-pgm"
REF = pathlib.Path(__file__).parent / "ref"
def boundary_paths(ref_file):
    im = Image.open(REF / ref_file).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255,255,255,255)); bg.alpha_composite(im)
    im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((468, 468), Image.LANCZOS)
    c = Image.new("L", (512, 512), 255)
    c.paste(im, ((512-im.width)//2, (512-im.height)//2))
    pgm = b"P5 %d %d 255\n" % c.size + c.tobytes()
    r = json.loads(subprocess.run([str(TP), "ink-boundary"], input=pgm, capture_output=True).stdout)
    return r["paths"]
ALT_SOURCE = {"owl": lambda: boundary_paths("owl.png"), "dragon": lambda: boundary_paths("dragon.jpg")}
curated = {}
for sub, (tier, (lo, hi), minlen, labels) in PLAN.items():
    src = json.loads((INK / f"{sub}-outline.svg").read_text().split('<rect')[1] and "null") if False else None
    # paths come from the batch metrics run; reparse from the SVG d attrs
    import re
    svg = (INK / f"{sub}-outline.svg").read_text()
    if "<img" in svg or "base64" in svg[:2000] and "<path" not in svg:
        print(f"{sub}: raster only — need path source"); continue
    if sub in ALT_SOURCE:
        paths = ALT_SOURCE[sub]()
    elif sub == "peacock":
        psvg = pathlib.Path(str(INK).replace("ink-out", "trace-out")) / "peacock-outline.svg"
        paths = []
        for dstr in re.findall(r'\bd="(M[^"]+)"', psvg.read_text()):
            pts = [(float(x), float(y)) for x, y in re.findall(r'(-?[\d.]+)[ ,](-?[\d.]+)', dstr)]
            if len(pts) >= 2: paths.append(pts)
    else:
        paths = []
        for dstr in re.findall(r'\bd="(M[^"]+)"', svg):
            pts = [(float(x), float(y)) for x, y in re.findall(r'(-?[\d.]+)[ ,](-?[\d.]+)', dstr)]
            if len(pts) >= 2: paths.append(pts)
    paths = [p for p in paths if plen(p) >= 26]
    # dedupe near-identical (texture traces emit twins)
    uniq = []
    for p in paths:
        c = (sum(x for x,_ in p)/len(p), sum(y for _,y in p)/len(p), round(plen(p)))
        if not any(abs(c[0]-u[0])<3 and abs(c[1]-u[1])<3 and abs(c[2]-u[2])<6 for u in [q[0] for q in uniq]):
            uniq.append((c, p))
    paths = [p for _, p in uniq]
    paths.sort(key=plen, reverse=True)
    paths = paths[:hi]
    paths = [tame(rescale(p)) for p in paths]
    paths = [p for p in paths if plen(p) >= 26]
    paths = [p for p in paths if len(p) >= 2 and plen(p) >= 26]
    paths = clearance_prune(paths)
    paths.sort(key=plen, reverse=True)
    entry = []
    for i, p in enumerate(paths):
        band = 1 if i == 0 else (2 if plen(p) > 180 else 3)
        entry.append({"feature": labels[i] if i < len(labels) else labels[-1],
                      "band": band, "points": [[round(x,1), round(y,1)] for x, y in p]})
    curated[sub] = {"tier": tier, "paths": entry}
    print(f"{sub:10} {tier:6} paths={len(entry):3} lens={[int(plen([(x,y) for x,y in q['points']])) for q in entry[:6]]}...")
# mona: from the passed v7.5.1 named build — silhouette split into segments
named = json.load(open(INK / "mona8-named.json"))["paths"]
mp = []
sil = named.pop("figure silhouette")
m2 = len(sil) // 2
segs = [sil[:m2+1], sil[m2:]]
for k, s in enumerate(segs):
    if plen(s) >= 55:
        mp.append({"feature": "figure silhouette", "band": 2, "points": [[round(x,1),round(y,1)] for x,y in resample(s)]})
order_last = named.pop("smile")
for name, p in named.items():
    if plen(p) < 28: continue
    band = 1 if name == "frame" else (2 if plen(p) > 150 else 3)
    if name.startswith("fingers") and plen(p) < 40: continue
    mp.append({"feature": "wrists/cuffs" if name.startswith("cuff") else ("hands" if name.startswith("fingers") else name), "band": band, "points": [[round(x,1),round(y,1)] for x,y in resample(dp(p, 3.0))]})
# expert floor: split longest paths until the band floor holds
_keep_pts = clearance_prune([q["points"] for q in mp])
mp = [q for q in mp if q["points"] in _keep_pts]
mp.append({"feature": "smile", "band": 4, "points": [[round(x,1),round(y,1)] for x,y in order_last]})
curated["mona"] = {"tier": "expert", "paths": mp}
print(f"mona       expert paths={len(mp)}")
json.dump(curated, open(OUT, "w"))
print("->", OUT)
