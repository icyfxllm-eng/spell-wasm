#!/usr/bin/env python3
"""v8 scan library: pinned batch traces -> render-library scan files.

Paths are VERBATIM (the tracer is deterministic; the pin hash proves it).
Scan-acceptance work happens here, once: pre-marked segment boundaries
(F2's ONLY split points), D-A sub-floor merge tags, and the raw
measurements F4's gate reads. Layout never touches geometry again.
"""
import json, math, pathlib, subprocess, sys
from PIL import Image, ImageOps

ROOT = pathlib.Path(__file__).resolve().parents[1]
TP = ROOT / "target/release/trace-pgm"
REF = ROOT / "content-pipeline/wordpic/ref"
OUT = ROOT / "content-pipeline/wordpic/scans"
OUT.mkdir(exist_ok=True)
CANVAS = 512
# Proposed constants (D1-D3 pending Eric):
FLOOR = 13.0
MIN_WORD_CHARS = 3
CORNER_DEG = 35.0
SMALL_FEATURE_MAX = 130.0
SEG_MAX = 100000.0  # v8.1: pack per path; corner marks only   # a segment longer than this gets an interior mark

# Eric's per-path sign-off (D4): features that MUST be present, named.
# Position is (x, y) in canvas coords with a tolerance, so the gate binds
# to the feature, not to a path index that could renumber.
# Eric's authoring exclusions: features that trace cleanly but read as
# blobs in the render and are better dropped (his call, per subject).
DROP_FEATURES = {
    "owl": [{"near": [214, 383], "tol": 26}, {"near": [296, 383], "tol": 26}],
}

REQUIRED_MICRO = {
    # Verified against the recovered closed features (build output):
    # eyes at (233,137)/(267,138), nose wedge at (258,160), buttons below.
    "snowman": [
        {"name": "left eye", "near": [233, 137], "tol": 20},
        {"name": "right eye", "near": [267, 138], "tol": 20},
        {"name": "nose", "near": [258, 160], "tol": 24},
    ],
}

SUBJ = {  # subject -> (ref, mode, tier)
 "dog": ("dog.png", "ink", "easy"), "butterfly": ("butterfly.png", "ink", "easy"),
 "duck": ("duck.png", "ink", "easy"), "turtle": ("turtle.png", "ink", "easy"),
 "owl": ("owl.png", "ink", "medium"), "elephant": ("elephant.png", "ink", "medium"),
 "snowman": ("snowman.png", "ink", "medium"), "horse": ("horse.png", "ink", "hard"),
 "fish": ("fish.png", "ink", "easy"), "eiffel": ("eiffel.png", "ink", "hard"),
 "dragon": ("dragon.png", "ink", "hard"), "peacock": ("peacock.png", "ink", "hard"),
 # v8.3 — the remaining eight (Eric). Geometric subjects (smiley, star,
 # house) are trace-EXEMPT per v7 F8 and carry authored scans; Mona rides
 # her v7.5.1 engraving build; these four come from references.
 "cat": ("cat.png", "ink", "medium"), "rocket": ("rocket.png", "ink", "medium"),
 "snail": ("snail.png", "ink", "easy"),
 # Wave 1 batch 1 (Eric approved 2026-08-01): suggested-then-approved.
 "balloon": ("balloon.png", "ink", "easy"),
 "mug": ("mug.png", "ink", "easy"),
 # kite + ladybug: revised after solver gates (decorative budget,
 # starvation); parked pending Eric's re-review of the new geometry.
 "kite": ("kite.png", "ink", "easy"),
 "mushroom": ("mushroom.png", "ink", "easy"),
 "sailboat": ("sailboat.png", "ink", "easy"),
 "cactus": ("cactus.png", "ink", "easy"),
 "ladybug": ("ladybug.png", "ink", "easy"),
 "moonstar": ("moonstar.png", "ink", "easy"),
 # Wave 1 batch 2 (Eric: "cut the trex and send the rest"): the
 # intermediate eight, suggested-then-approved. Trex cut.
 "bicycle": ("bicycle.png", "ink", "medium"),
 "windmill": ("windmill.png", "ink", "medium"),
 "hotair": ("hotair.png", "ink", "medium"),
 "lighthouse": ("lighthouse.png", "ink", "medium"),
 "violin": ("violin.png", "ink", "medium"),
 "hummingbird": ("hummingbird.png", "ink", "medium"),
 "rooster": ("rooster.png", "ink", "medium"),
 "seahorse": ("seahorse.png", "ink", "medium"),
 # Wave 1 batch 3 (advanced): sourced PD art, Eric-approved.
 "wolf": ("wolf.png", "ink", "hard"),
 "koi": ("koi.png", "ink", "hard"),
 # Wave 2 batch 1 (Eric: "both pass"): sourced PD.
 "octopus": ("octopus.png", "ink", "hard"),
 "oak": ("oak.png", "ink", "hard"),
    "starrynight": ("starrynight.png", "ink", "expert"),
    "crane": ("crane.png", "ink", "medium"),
    "dallah": ("dallah.png", "ink", "medium"),
    "basil": ("basil.png", "ink", "hard"),
    "panda": ("panda.png", "ink", "easy"),
    "calavera": ("calavera.png", "ink", "medium"),
    "tajmahal": ("tajmahal.png", "ink", "hard"),
    "stork": ("stork.png", "ink", "medium"),
    "caravel": ("caravel.png", "ink", "hard"),
    "bigben": ("bigben.png", "ink", "medium"),
    "turtleship": ("turtleship.png", "ink", "hard"),
    "baobab": ("baobab.png", "ink", "medium"),
    "nonla": ("nonla.png", "ink", "medium"),
    "cuckoo": ("cuckoo.png", "ink", "medium"),
    "torii": ("torii.png", "ink", "medium"),
    "hanbok": ("hanbok.png", "ink", "medium"),
    "matryoshka": ("matryoshka.png", "ink", "easy"),
    "greatwall": ("greatwall.png", "ink", "hard"),
    "oud": ("oud.png", "ink", "medium"),
    "orion": ("orion.png", "ink", "medium"),
    "papelpicado": ("papelpicado.png", "ink", "medium"),
    "redfuji": ("redfuji.png", "ink", "expert"),
    "anubis": ("anubis.png", "ink", "hard"),
    "athenaowl": ("athenaowl.png", "ink", "medium"),
    "bamboo": ("bamboo.png", "ink", "medium"),
    "girih": ("girih.png", "ink", "medium"),
    "gyenyame": ("gyenyame.png", "ink", "medium"),
    "arcdetriomphe": ("arcdetriomphe.png", "ink", "medium"),
    "rickshaw": ("rickshaw.png", "ink", "medium"),
    "dhow": ("dhow.png", "ink", "medium"),
    "galo": ("galo.png", "ink", "medium"),
    "pyramids": ("pyramids.png", "ink", "medium"),
    "eyeofhorus": ("eyeofhorus.png", "ink", "medium"),
    "ankh": ("ankh.png", "ink", "easy"),
    "waweldragon": ("waweldragon.png", "ink", "hard"),
    "doubledecker": ("doubledecker.png", "ink", "medium"),
    "halongbay": ("halongbay.png", "ink", "medium"),
    "carabao": ("carabao.png", "ink", "medium"),
    "fleurdelis": ("fleurdelis.png", "ink", "easy"),
    "jeepney": ("jeepney.png", "ink", "medium"),
    "manekineko": ("manekineko.png", "ink", "easy"),
    "acacia": ("acacia.png", "ink", "easy"),
    "samovar": ("samovar.png", "ink", "medium"),
    "lantern": ("lantern.png", "ink", "easy"),
    "sitar": ("sitar.png", "ink", "medium"),
    "brandenburg": ("brandenburg.png", "ink", "medium"),
    "colosseum": ("colosseum.png", "ink", "hard"),
    "fan": ("fan.png", "ink", "medium"),
    "stonehenge": ("stonehenge.png", "ink", "medium"),
    "pagoda": ("pagoda.png", "ink", "medium"),
    "daruma": ("daruma.png", "ink", "easy"),
    "balalaika": ("balalaika.png", "ink", "medium"),
    "sombrero": ("sombrero.png", "ink", "easy"),
    "djembe": ("djembe.png", "ink", "medium"),
    "angkorwat": ("angkorwat.png", "ink", "medium"),
    "accordion": ("accordion.png", "ink", "medium"),
    "quetzal": ("quetzal.png", "ink", "medium"),
    "grandpiano": ("grandpiano.png", "ink", "medium"), "rhino": ("rhino.png", "ink", "expert"),
}

# ---- per-subject AUTHORING recipes (content work, not renderer tuning:
# the renderer stays picture-agnostic; this is how a scan is authored) ----
def author_horse(im):
    """v8.2.1 horse re-author (Eric): the block was MicroBudget 0.234 —
    grass tufts and mane/tail wisps fragment into arcs too small to host
    words. Recipe: drop the grass by colour, then close the subject mask
    so mane and tail spikes merge into solid mass. Interior identity
    lines (leg separations, mane edge, jaw, eye) survive untouched."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    # 1. subject = not near-white, not green-dominant (grass)
    mask = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            r, g, b = px[x, y]
            if r > 235 and g > 235 and b > 235:
                continue
            if g > r + 12 and g > b + 12:      # grass green
                continue
            mask[y][x] = True
    # 2. keep the largest component (drops isolated grass blades)
    seen = [[False]*W for _ in range(H)]
    best, bestsz = None, 0
    for y0 in range(H):
        for x0 in range(W):
            if not mask[y0][x0] or seen[y0][x0]:
                continue
            comp, stack = [], [(x0, y0)]
            seen[y0][x0] = True
            while stack:
                x, y = stack.pop()
                comp.append((x, y))
                for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = True
                        stack.append((nx, ny))
            if len(comp) > bestsz:
                best, bestsz = comp, len(comp)
    keep = [[False]*W for _ in range(H)]
    for x, y in best:
        keep[y][x] = True
    # 3. morphological CLOSE (r=5): mane/tail spikes merge into mass
    R = 5
    dil = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if not keep[y][x]:
                continue
            for dy in range(-R, R+1):
                for dx in range(-R, R+1):
                    if dx*dx + dy*dy > R*R:
                        continue
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H:
                        dil[ny][nx] = True
    closed = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if not dil[y][x]:
                continue
            ok = True
            for dy in range(-R, R+1):
                for dx in range(-R, R+1):
                    if dx*dx + dy*dy > R*R:
                        continue
                    nx, ny = x+dx, y+dy
                    if not (0 <= nx < W and 0 <= ny < H and dil[ny][nx]):
                        ok = False
                        break
                if not ok:
                    break
            closed[y][x] = ok
    # 4. paint: subject ink dark, filled gaps dark, everything else white
    out = Image.new("L", (W, H), 255)
    o = out.load()
    g = ImageOps.autocontrast(rgb.convert("L"), cutoff=1).load()
    for y in range(H):
        for x in range(W):
            if closed[y][x] or keep[y][x]:
                o[x, y] = min(g[x, y], 90) if keep[y][x] else 40
    return out


# ---- shared authoring primitives ----
def _largest_component(mask, W, H):
    seen = [[False]*W for _ in range(H)]
    best, bestsz = [], 0
    for y0 in range(H):
        for x0 in range(W):
            if not mask[y0][x0] or seen[y0][x0]:
                continue
            comp, stack = [], [(x0, y0)]
            seen[y0][x0] = True
            while stack:
                x, y = stack.pop()
                comp.append((x, y))
                for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = True
                        stack.append((nx, ny))
            if len(comp) > bestsz:
                best, bestsz = comp, len(comp)
    out = [[False]*W for _ in range(H)]
    for x, y in best:
        out[y][x] = True
    return out

def _close(mask, W, H, R):
    dil = [[False]*W for _ in range(H)]
    off = [(dx, dy) for dy in range(-R, R+1) for dx in range(-R, R+1) if dx*dx+dy*dy <= R*R]
    for y in range(H):
        for x in range(W):
            if mask[y][x]:
                for dx, dy in off:
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H:
                        dil[ny][nx] = True
    out = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if not dil[y][x]:
                continue
            ok = True
            for dx, dy in off:
                nx, ny = x+dx, y+dy
                if not (0 <= nx < W and 0 <= ny < H and dil[ny][nx]):
                    ok = False
                    break
            out[y][x] = ok
    return out

def _fill_holes(mask, W, H):
    """Flood the outside; anything unreached and unmasked is an interior
    hole (feather gaps, texture) — filled, so texture stops fragmenting."""
    outside = [[False]*W for _ in range(H)]
    stack = [(x, 0) for x in range(W)] + [(x, H-1) for x in range(W)] + \
            [(0, y) for y in range(H)] + [(W-1, y) for y in range(H)]
    stack = [(x, y) for x, y in stack if not mask[y][x]]
    for x, y in stack:
        outside[y][x] = True
    while stack:
        x, y = stack.pop()
        for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
            nx, ny = x+dx, y+dy
            if 0 <= nx < W and 0 <= ny < H and not mask[ny][nx] and not outside[ny][nx]:
                outside[ny][nx] = True
                stack.append((nx, ny))
    return [[mask[y][x] or not outside[y][x] for x in range(W)] for y in range(H)]

def _paint(mask, holes, W, H):
    im = Image.new("L", (W, H), 255)
    p = im.load()
    for y in range(H):
        for x in range(W):
            if mask[y][x] and not (holes and holes[y][x]):
                p[x, y] = 20
    return im


def author_owl(im):
    """v8.2.1 owl re-author (Eric: 'easier owl picture'). The detailed
    eagle-owl drowned in feather stippling; this pictogram is already
    clean, so the recipe is minimal: solid subject, and the white eyes
    and beak stay as holes so each traces as its own closed feature."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    mask = [[not (px[x, y][0] > 200 and px[x, y][1] > 200 and px[x, y][2] > 200)
             for x in range(W)] for y in range(H)]
    mask = _close(mask, W, H, 2)
    return _paint(mask, None, W, H)


def author_dragon(im):
    """v8.2.1 dragon re-author (Eric supplied a target look): the Hokusai
    panel was tonal and untraceable. This is clean clipart — solidify the
    creature so scale texture stops fragmenting, keep the silhouette
    (serpentine body, horned head, clawed feet, flame fins)."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    mask = [[not (px[x, y][0] > 236 and px[x, y][1] > 236 and px[x, y][2] > 236)
             for x in range(W)] for y in range(H)]
    mask = _close(mask, W, H, 3)
    mask = _largest_component(mask, W, H)
    mask = _fill_holes(mask, W, H)
    return _paint(mask, None, W, H)


def author_rhino(im):
    """Duerer's Rhinoceros was the right animal and the wrong reference.
    Even redrawn as vector lines its plate ornament is dense enough that
    every threshold floods: the legs merged with the ground shadow and the
    eye disappeared, which is exactly what Eric saw. The reference is now
    a drawn silhouette (tools/draw_ref.py) with real air between the legs
    and the eye as a hole, so both survive by construction rather than by
    tuning. Recipe is therefore the plain one -- solid subject, holes kept.
    """
    return author_pictogram(im)


def author_pictogram(im):
    """Clean CC0 pictograms (cat, rocket, snail): the subject is already a
    solid shape — take it as-is, keeping interior holes (the rocket's
    window, the snail's shell spiral) as their own contours."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    mask = [[not (px[x, y][0] > 205 and px[x, y][1] > 205 and px[x, y][2] > 205)
             for x in range(W)] for y in range(H)]
    mask = _close(mask, W, H, 2)
    return _paint(mask, None, W, H)


def author_peacock(im):
    """v8.2.1 peacock re-author (Eric: the photo's translucent fan was
    untraceable; swapped to the PD road-sign pictogram). The bird is the
    LIGHT shape inside a dark rounded square: take the light pixels
    within the sign, drop the plate, and every part — fan eyespots, body,
    crest — arrives as a clean closed contour."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    g = ImageOps.autocontrast(rgb.convert("L"), cutoff=1).load()
    # the plate is the dark rounded square; the bird is light inside it
    plate = [[not (px[x, y][0] > 238 and px[x, y][1] > 238 and px[x, y][2] > 238)
              for x in range(W)] for y in range(H)]
    plate = _largest_component(plate, W, H)
    plate = _fill_holes(plate, W, H)
    bird = [[plate[y][x] and g[x, y] > 150 for x in range(W)] for y in range(H)]
    bird = _close(bird, W, H, 2)
    return _paint(bird, None, W, H)

def author_solid_pictogram(im):
    """Pictograms whose INTERIOR detail is finer than a word is tall (the
    snail's spiral whorls, the rocket's stripes): fill the interior so the
    subject reads as its outer form. Eric's snail direction — outer shell,
    body, eyes, smile — is exactly this shape."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    mask = [[not (px[x, y][0] > 205 and px[x, y][1] > 205 and px[x, y][2] > 205)
             for x in range(W)] for y in range(H)]
    mask = _close(mask, W, H, 3)
    mask = _largest_component(mask, W, H)
    mask = _fill_holes(mask, W, H)
    return _paint(mask, None, W, H)


AUTHOR = {"horse": author_pictogram, "owl": author_owl, "peacock": author_peacock,
          "dragon": author_dragon, "rhino": author_rhino,
          "cat": author_pictogram,
          # interior detail finer than a word is tall -> fill it
          "snail": author_solid_pictogram,
          # not solid_pictogram: that fills holes, and the window is a hole
          "rocket": author_pictogram}


def rings(path, subject=None):
    """Ring-based scan source for boundary art: threshold -> boundary
    pixels -> connected components -> ordered rings (greedy walk). The
    outer ring is the word path; interior rings are the strokes' far
    sides / hole contours."""
    im0 = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im0.size, (255, 255, 255, 255)); bg.alpha_composite(im0)
    if subject in AUTHOR:
        im = AUTHOR[subject](bg)
    else:
        im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS - im.width) // 2, (CANVAS - im.height) // 2))
    px = c.load()
    W = H = CANVAS
    hist = [0]*256
    for y in range(H):
        for x in range(W):
            hist[px[x, y]] += 1
    total = W * H
    sum_all = sum(i * h for i, h in enumerate(hist))
    sum_b = w_b = best = 0.0
    t_best = 127
    for t in range(256):
        w_b += hist[t]
        if w_b == 0: continue
        w_f = total - w_b
        if w_f == 0: break
        sum_b += t * hist[t]
        m_b, m_f = sum_b / w_b, (sum_all - sum_b) / w_f
        between = w_b * w_f * (m_b - m_f) ** 2
        if between > best: best, t_best = between, t
    T = min(t_best, 160)
    ink = [[px[x, y] <= T for x in range(W)] for y in range(H)]
    ink_pixels = sum(1 for y in range(H) for x in range(W) if ink[y][x])
    bnd = set()
    for y in range(H):
        for x in range(W):
            if not ink[y][x]: continue
            if x == 0 or y == 0 or x == W-1 or y == H-1 or not (ink[y][x-1] and ink[y][x+1] and ink[y-1][x] and ink[y+1][x]):
                bnd.add((x, y))
    comps = []
    seen = set()
    for start in sorted(bnd):
        if start in seen: continue
        comp = []
        stack = [start]; seen.add(start)
        while stack:
            (x, y) = stack.pop()
            comp.append((x, y))
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    n = (x+dx, y+dy)
                    if n in bnd and n not in seen:
                        seen.add(n); stack.append(n)
        comps.append(comp)
    def moore(comp_set, start):
        """Textbook Moore-neighbour contour trace on the INK mask (not the
        boundary set): follows connectivity, so tight coils can't make the
        walk jump the gap and strand — that is how a 3000px dragon became
        a 250px stub. Start is the topmost-leftmost ink pixel; the initial
        backtrack is its west neighbour, known background."""
        N8 = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)]
        def isink(p):
            return 0 <= p[0] < W and 0 <= p[1] < H and ink[p[1]][p[0]]
        b = start
        prev = (start[0] - 1, start[1])
        ring = [b]
        first_step = None
        for _ in range(8 * len(comp_set) + 64):
            d0 = N8.index((prev[0] - b[0], prev[1] - b[1])) if (prev[0]-b[0], prev[1]-b[1]) in N8 else 7
            nxt = None
            for k in range(1, 9):
                d = (d0 + k) % 8
                cand = (b[0] + N8[d][0], b[1] + N8[d][1])
                if isink(cand):
                    nxt = cand
                    prev = (b[0] + N8[(d - 1) % 8][0], b[1] + N8[(d - 1) % 8][1])
                    break
            if nxt is None:
                break
            b = nxt
            ring.append(b)
            if first_step is None:
                first_step = b
            elif b == start and ring[-2] == prev_start_guard if False else False:
                pass
            if len(ring) > 3 and b == start:
                break
        return ring

    ordered = []
    for comp in comps:
        if len(comp) < 8: continue
        rem = set(comp)
        cur = min(rem)
        ring = [cur]; rem.discard(cur)
        while rem:
            nxt = min(rem, key=lambda p: (p[0]-cur[0])**2 + (p[1]-cur[1])**2)
            d2 = (nxt[0]-cur[0])**2 + (nxt[1]-cur[1])**2
            if d2 > 36:  # ring walk jumped — separate strand
                break
            ring.append(nxt); rem.discard(nxt); cur = nxt
        # repair: a greedy walk that covered far less than the component
        # terminated early — redo it with the connectivity-following trace
        if len(ring) < 0.6 * len(comp):
            # start at the topmost-leftmost pixel of this component
            start = min(comp, key=lambda p: (p[1], p[0]))
            ring = moore(set(comp), start)
        if len(ring) >= 8 and len(comp) >= 34:
            ring.append(ring[0])  # close the loop
            pts = [(float(x), float(y)) for x, y in ring[::2]]
            # Contour smoothing: pixel staircases make EVERY step a 45-deg
            # turn, so corner marks explode (the dragon: 517 marks on
            # 5139px, chopping it into unhostable stubs). Chaikin twice
            # rounds the staircase; true limb corners survive.
            for _ in range(2):
                if len(pts) < 4:
                    break
                sm = [pts[0]]
                for a, b in zip(pts, pts[1:]):
                    sm.append((0.75*a[0]+0.25*b[0], 0.75*a[1]+0.25*b[1]))
                    sm.append((0.25*a[0]+0.75*b[0], 0.25*a[1]+0.75*b[1]))
                sm.append(pts[-1])
                pts = sm
            # decimate back to ~2px spacing so the point count stays sane
            out2 = [pts[0]]
            for p in pts[1:]:
                if math.hypot(p[0]-out2[-1][0], p[1]-out2[-1][1]) >= 2.0:
                    out2.append(p)
            if math.hypot(out2[-1][0]-out2[0][0], out2[-1][1]-out2[0][1]) < 2.0:
                out2[-1] = out2[0]
            ordered.append(out2)
    return ordered

def pgm(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255)); bg.alpha_composite(im)
    im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS - im.width) // 2, (CANVAS - im.height) // 2))
    return b"P5 %d %d 255\n" % c.size + c.tobytes()

def plen(p):
    return sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))

def turn(a, b, c):
    v1 = (b[0]-a[0], b[1]-a[1]); v2 = (c[0]-b[0], c[1]-b[1])
    m = (math.hypot(*v1) * math.hypot(*v2)) or 1e-9
    d = max(-1.0, min(1.0, (v1[0]*v2[0]+v1[1]*v2[1]) / m))
    return math.degrees(math.acos(d))

def fnv(paths):
    h = 0xcbf29ce484222325
    for p in paths:
        for x, y in p:
            for b in int(round(x*10)).to_bytes(4, "little", signed=True) + int(round(y*10)).to_bytes(4, "little", signed=True):
                h ^= b; h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h

def seg_marks(p):
    """Pre-marked boundaries: corners past CORNER_DEG plus interior marks
    keeping every segment <= SEG_MAX px. Fractions of total arc."""
    total = plen(p)
    if total <= 0: return []
    cums = [0.0]
    for a, b in zip(p, p[1:]):
        cums.append(cums[-1] + math.hypot(b[0]-a[0], b[1]-a[1]))
    marks = set()
    # A corner is a turn sustained across a WINDOW (~9px either side), not
    # a single-vertex wiggle: residual contour noise otherwise fires marks
    # every few px and chops the path into unhostable stubs.
    WIN = 9.0
    def idx_back(i):
        j = i
        while j > 0 and cums[i] - cums[j] < WIN:
            j -= 1
        return j
    def idx_fwd(i):
        j = i
        n = len(p) - 1
        while j < n and cums[j] - cums[i] < WIN:
            j += 1
        return j
    for i in range(1, len(p) - 1):
        a, b = idx_back(i), idx_fwd(i)
        if a == i or b == i:
            continue
        if turn(p[a], p[i], p[b]) > CORNER_DEG:
            marks.add(round(cums[i] / total, 4))
    # collapse mark clusters: keep one mark per corner
    if marks:
        keep, last = [], -1.0
        for t in sorted(marks):
            if last < 0 or (t - last) * total > 18.0:
                keep.append(t)
                last = t
        marks = set(keep)
    bounds = sorted({0.0, 1.0} | marks)
    final = set(marks)
    for a, b in zip(bounds, bounds[1:]):
        span = (b - a) * total
        n = int(span // SEG_MAX)
        for k in range(1, n + 1):
            final.add(round(a + (b - a) * k / (n + 1), 4))
    # Corner marks are kept UNCONDITIONALLY (v6 star law: text never
    # bends around a corner). Only interior subdivision marks respect the
    # minimum span; sub-hostable slivers between corner marks are skipped
    # by the typesetter and excluded from recall (D-A / D2 territory).
    return sorted(final)

def path_min_clearance(p, others):
    """Interior-to-interior clearance: junction contact (a leg meeting the
    body at shared endpoints) is structure, not a violation — F4's G3 is
    about PARALLEL strokes without word-height space between them."""
    def sd(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    J = 18.0
    def interior(path):
        e0, e1 = path[0], path[-1]
        return [pt for pt in path
                if math.hypot(pt[0]-e0[0], pt[1]-e0[1]) > J
                and math.hypot(pt[0]-e1[0], pt[1]-e1[1]) > J]
    pi = interior(p)
    if not pi: return 999.0
    m = 1e9
    for q in others:
        qi = interior(q)
        if len(qi) < 2: continue
        for pt in pi[::3]:
            for u, v in zip(qi, qi[1:]):
                d = sd(pt, u, v)
                if d < m: m = d
    return m

# CC-SCAN-STACK v1.2 module 12, the export-block: an unreviewed suggestion
# cannot be exported. If a subject has a suggestions sidecar with ANY
# candidate still pending, its scan does not build -- approval is the human
# half of the contract and the tool refuses to proceed without it.
SUGGESTIONS = ROOT / "content-pipeline/wordpic/suggestions"
SUGGESTED_SUBJECTS = set()
PROVENANCE = {r["subject"]: r for r in json.loads(
    (ROOT / "content-pipeline/wordpic/ref/provenance-f8.json").read_text())}
def _pending_suggestions(sub):
    f = SUGGESTIONS / f"{sub}.json"
    if not f.exists():
        return 0
    doc = json.loads(f.read_text())
    return sum(1 for c in doc.get("candidates", []) if c.get("status") == "pending")

def _approved_candidates(sub):
    """Module 12 consumption: once every candidate is reviewed and at least
    one is approved, the APPROVED polylines become the subject's paths --
    "accepted paths become ordinary v7.x traced paths, indistinguishable
    downstream". The suggester emits in the same 512-canvas frame this
    pipeline scales into, so they slot in where rings() output would."""
    f = SUGGESTIONS / f"{sub}.json"
    if not f.exists():
        return None
    doc = json.loads(f.read_text())
    ok = [c for c in doc.get("candidates", []) if c.get("status") == "approved"]
    return [[(float(x), float(y)) for x, y in c["points"]] for c in ok] or None

for sub, (ref, mode, tier) in SUBJ.items():
    npend = _pending_suggestions(sub)
    if npend:
        raise SystemExit(
            f"{sub}: {npend} suggestion candidate(s) still pending review — "
            f"approve or reject them in content-pipeline/wordpic/suggestions/{sub}.json "
            f"before this subject can build (module 12 export-block)")
    suggested = _approved_candidates(sub)
    if suggested is not None:
        paths = suggested
        SUGGESTED_SUBJECTS.add(sub)
    else:
        paths = rings(REF / ref, sub)
    paths.sort(key=plen, reverse=True)
    # v8.2.1 small-feature pass: a SHORT CLOSED contour is a solid source
    # feature (snowman eyes, nose; animal eyes). It is preserved intact —
    # exempt from decorative dedup, from the tight-run split, and from
    # sub-floor merging — and carries micro_feature so the renderer draws
    # it filled and the gate can require it.
    def is_small_closed(p):
        return (math.hypot(p[0][0]-p[-1][0], p[0][1]-p[-1][1]) < 3.0
                and plen(p) < SMALL_FEATURE_MAX)
    small_feats = [p for p in paths if is_small_closed(p)]
    paths = [p for p in paths if not is_small_closed(p)]
    entries = []
    lens = [plen(p) for p in paths]
    for i, p in enumerate(paths):
        L = lens[i]
        sub_floor = L < FLOOR * MIN_WORD_CHARS
        parent = None
        if sub_floor and len(paths) > 1:
            # D-A: merge tag -> nearest longer path
            best, bd = None, 1e9
            cx = sum(x for x, _ in p)/len(p); cy = sum(y for _, y in p)/len(p)
            for j, q in enumerate(paths):
                if j == i or lens[j] < FLOOR * MIN_WORD_CHARS: continue
                d = min(math.hypot(cx-x, cy-y) for x, y in q[::3])
                if d < bd: bd, best = d, j
            parent = best
        marks = seg_marks(p)
        cums = [0.0]
        for a2, b2 in zip(p, p[1:]):
            cums.append(cums[-1] + math.hypot(b2[0]-a2[0], b2[1]-a2[1]))
        totL = cums[-1] or 1.0
        bounds = [0.0] + [t for t in marks if 0 < t < 1] + [1.0]
        worst_turn = 0.0
        for k in range(1, len(p)-1):
            t_here = cums[k] / totL
            near_mark = any(abs(t_here - m) < 0.012 for m in bounds)
            if not near_mark:
                worst_turn = max(worst_turn, turn(p[k-1], p[k], p[k+1]))
        entries.append({
            "points": p, "arc": round(L, 2), "tier": tier,
            "sub_floor": sub_floor, "merged_into": parent,
            "decorative_thin": False,
            "segments": marks,
            "worst_turn_deg": round(worst_turn, 1),
            "min_clearance": None,  # filled below
        })
    # Stroke-side dedup: ink_boundary emits BOTH sides of a drawn stroke.
    # The far side (>=70% of points within 16px of a LONGER path) is
    # decorative_thin — one drawn line, one word baseline; covering one
    # side recovers the stroke. Recall excludes decorative_thin per F5.
    def near_frac(p, q, tol=22.0):
        def sd(pt, u, v):
            vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
            t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
            return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
        pts = p[::2] if len(p) > 4 else p
        hit = sum(1 for pt in pts if any(sd(pt, u, v) <= tol for u, v in zip(q, q[1:])))
        return hit / len(pts)
    order = sorted(range(len(entries)), key=lambda i: -lens[i])
    kept = []
    for i in order:
        e = entries[i]
        if e["sub_floor"]:
            continue
        if any(near_frac(e["points"], entries[j]["points"]) >= 0.7 for j in kept):
            e["decorative_thin"] = True
        else:
            kept.append(i)
    # Tight-run split (scan acceptance, geometry verbatim): runs of a word
    # path closer than one floor glyph to another word path are the far
    # sides of strokes (leg gaps, hat brims) — they become decorative
    # entries; the clear runs stay word paths. F4 then gates truthfully
    # and F3 never faces an impossible segment.
    def sd3(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    word_idx = [i for i, e in enumerate(entries) if not e["sub_floor"] and not e["decorative_thin"]]
    new_entries = []
    for i, e in enumerate(entries):
        if e["sub_floor"] or e["decorative_thin"]:
            new_entries.append(e)
            continue
        others = [entries[j]["points"] for j in word_idx if j != i]
        if not others:
            new_entries.append(e)
            continue
        pts = e["points"]
        tight = [any(sd3(pt, u, v) < FLOOR * 1.15 for q in others for u, v in zip(q, q[1:])) for pt in pts]
        # smooth runs: a run flips only if >= 3 consecutive agree
        runs = []
        cur = tight[0]; start = 0
        k = 0
        while k < len(pts):
            if tight[k] != cur:
                nxt = tight[k:k+2]
                if len(nxt) == 2 and all(v == tight[k] for v in nxt):
                    runs.append((start, k, cur)); start = k; cur = tight[k]
            k += 1
        runs.append((start, len(pts), cur))
        if all(not t for _, _, t in runs) or len(runs) == 1:
            e2 = dict(e)
            if runs[0][2]:
                e2["decorative_thin"] = True
            new_entries.append(e2)
            continue
        for (a, b, is_tight) in runs:
            seg = pts[a:b+1][:]
            if len(seg) < 2 or plen(seg) < 20:
                continue
            e2 = dict(e)
            e2["points"] = seg
            e2["arc"] = round(plen(seg), 2)
            e2["decorative_thin"] = bool(is_tight)
            e2["sub_floor"] = (not is_tight) and plen(seg) < FLOOR * MIN_WORD_CHARS
            e2["segments"] = seg_marks(seg)
            e2["worst_turn_deg"] = 0.0
            marks2 = e2["segments"]
            cums2 = [0.0]
            for a3, b3 in zip(seg, seg[1:]):
                cums2.append(cums2[-1] + math.hypot(b3[0]-a3[0], b3[1]-a3[1]))
            tot2 = cums2[-1] or 1.0
            bounds2 = [0.0] + [t for t in marks2 if 0 < t < 1] + [1.0]
            for k2 in range(1, len(seg)-1):
                th = cums2[k2] / tot2
                if not any(abs(th - m) < 0.012 for m in bounds2):
                    e2["worst_turn_deg"] = max(e2["worst_turn_deg"], round(turn(seg[k2-1], seg[k2], seg[k2+1]), 1))
            new_entries.append(e2)
    entries = new_entries
    lens = [plen(e["points"]) for e in entries]
    for i, e in enumerate(entries):
        others = [q["points"] for j, q in enumerate(entries)
                  if j != i and not q["sub_floor"] and not q["decorative_thin"]]
        if not others:
            e["min_clearance"] = 999.0
            e["tight_frac"] = 0.0
            continue
        e["min_clearance"] = round(path_min_clearance(e["points"], others), 2)
        # fraction of this path's arc closer than one floor glyph to any
        # other word path — local contact zones (stacked circles touching)
        # are F3's job; only broadly-parallel geometry blocks.
        def sd2(pt, u, v):
            vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
            t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
            return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
        pts = e["points"][::2]
        tight = sum(1 for pt in pts if any(sd2(pt, u, v) < FLOOR for q in others for u, v in zip(q, q[1:])))
        e["tight_frac"] = round(tight / max(1, len(pts)), 3)
    drops = DROP_FEATURES.get(sub, [])
    for p in small_feats:
        cx = sum(x for x, _ in p) / len(p)
        cy = sum(y for _, y in p) / len(p)
        if any(math.hypot(cx - d["near"][0], cy - d["near"][1]) <= d["tol"] for d in drops):
            continue
        entries.append({
            "points": p, "arc": round(plen(p), 2), "tier": tier,
            "sub_floor": False, "merged_into": None, "decorative_thin": False,
            "micro_feature": True, "segments": [], "worst_turn_deg": 0.0,
            "min_clearance": 999.0, "tight_frac": 0.0,
        })
    # v8.2.1 (2): required micro features — named, signed off in the scan
    # file. The render gate refuses to display a subject missing any.
    req = REQUIRED_MICRO.get(sub)
    doc = {"subject": sub, "tier": tier, "pin_hash": fnv(paths), "canvas": CANVAS,
           "required_micro": req or [], "paths": entries,
           # Module 12 provenance, stamped as a fact about THIS artifact at
           # the moment it was built -- never re-derived later by guessing.
           "authoring": "suggested-then-approved" if sub in SUGGESTED_SUBJECTS else "hand-traced",
           # Masterpiece attribution rides the scan into the app bundle so the
           # share card can print it (CC-FINALE feature 2: honest and classy).
           "attribution": PROVENANCE.get(sub, {}).get("attribution", "")}
    (OUT / f"{sub}.json").write_text(json.dumps(doc))
    print(f'{sub:10} paths={len(entries):5} sub_floor={sum(e["sub_floor"] for e in entries):4} hash={doc["pin_hash"]:#x}')


# ---- authored scans: geometric subjects (F8-exempt) + Mona's passed build ----
GEO = json.loads((pathlib.Path(__file__).parent.parent /
                  "content-pipeline/wordpic/geometric-scans.json").read_text())
for sub, spec in GEO.items():
    entries = []
    for pts, flags in spec["paths"]:
        entries.append({
            "points": pts, "arc": round(plen(pts), 2), "tier": spec["tier"],
            "sub_floor": bool(flags & 1), "merged_into": None,
            "decorative_thin": bool(flags & 2), "micro_feature": bool(flags & 4),
            "segments": seg_marks(pts), "worst_turn_deg": 0.0,
            "min_clearance": 999.0, "tight_frac": 0.0,
        })
    doc = {"subject": sub, "tier": spec["tier"], "pin_hash": fnv([e["points"] for e in entries]),
           "canvas": CANVAS, "required_micro": [], "paths": entries,
           "authoring": "hand-traced",
           "attribution": PROVENANCE.get(sub, {}).get("attribution", "")}
    (OUT / f"{sub}.json").write_text(json.dumps(doc))
    print(f'{sub:10} paths={len(entries):5} (authored geometry, F8-exempt)')

# Mona: the build Eric passed in v7.5.1 (frame + figure + face + hands).
mona = json.loads((pathlib.Path(__file__).parent.parent /
                   "content-pipeline/wordpic/mona-engraving-draft.json").read_text())
entries = []
for name, pts in mona["paths"].items():
    pts = [[float(x), float(y)] for x, y in pts]
    if plen(pts) < 11:
        continue
    closed = math.hypot(pts[0][0]-pts[-1][0], pts[0][1]-pts[-1][1]) < 3.0
    small = plen(pts) < SMALL_FEATURE_MAX and closed
    # her brows, eyes and nose are short OPEN strokes: features drawn as
    # pinned ink, never word paths (D-A) — words there would be squeezed.
    short_open = (not closed) and plen(pts) < FLOOR * MIN_WORD_CHARS
    # Eric: her eyes and arms need more detail. Short face strokes are
    # FEATURES drawn as pinned ink, never paths to drop.
    if short_open:
        small = True
    entries.append({
        "points": pts, "arc": round(plen(pts), 2), "tier": "expert",
        "sub_floor": short_open, "merged_into": None, "decorative_thin": False,
        "micro_feature": small, "segments": seg_marks(pts), "worst_turn_deg": 0.0,
        "min_clearance": 999.0, "tight_frac": 0.0, "feature": name,
    })
# Authored scans get the same clearance law as traced ones: where two
# word paths run closer than a glyph is tall, the SHORTER one becomes
# pinned ink (her hairline hugs the veil edge by construction).
def _mind(a, b):
    def sd(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    return min((sd(pt, u, v) for pt in a[::2] for u, v in zip(b, b[1:])), default=999.0)

order = sorted(range(len(entries)), key=lambda i: -entries[i]["arc"])
for k, i in enumerate(order):
    if entries[i]["sub_floor"] or entries[i]["decorative_thin"] or entries[i].get("micro_feature"):
        continue
    for j in order[:k]:
        if entries[j]["decorative_thin"] or entries[j]["sub_floor"] or entries[j].get("micro_feature"):
            continue
        if _mind(entries[i]["points"], entries[j]["points"]) < FLOOR:
            entries[i]["decorative_thin"] = True
            break

doc = {"subject": "mona", "tier": "expert", "pin_hash": fnv([e["points"] for e in entries]),
       "canvas": CANVAS, "required_micro": [], "paths": entries,
       "authoring": "hand-traced",
       "attribution": PROVENANCE.get("mona", {}).get("attribution", "")}
(OUT / "mona.json").write_text(json.dumps(doc))
print(f'{"mona":10} paths={len(entries):5} (Eric-passed v7.5.1 engraving build)')
