#!/usr/bin/env python3
"""Wave-1 batch 3, sourced references (Eric approved the downloads).

wolf: Wikimedia Commons File:Wolf.svg, CC0 -- the classic standing howl.
      Composed over a drawn perch with an ALIGNED SEAM: the paw line sits
      flush along the rock's top edge with a 36px white gap, so wolf and
      rock trace as two contours (fused, the paws would vanish into the
      rock) while the eye closes the aligned gap and reads "standing on".
koi:  FreeSVG/OpenClipart Koi-Fish, public domain -- a coloured top view.
      The SILHOUETTE is extracted (everything that is not the background
      colour, largest component, holes filled) and placed twice, rotated
      180, for the circling pair. Composing two copies of PD art is PD.
"""
import pathlib
import sys

from PIL import Image

S = pathlib.Path(sys.argv[1])
REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"

# ---- wolf ----------------------------------------------------------------
w = Image.open(S / "wolf-cc0.svg.png").convert("RGBA")
bg = Image.new("RGBA", w.size, (255, 255, 255, 255))
bg.alpha_composite(w)
g = bg.convert("L")
px = g.load()
W, H = g.size
# paw baseline: lowest dark row
lowest = max(y for y in range(H) for x in range(W) if px[x, y] < 128)
canvas = Image.new("L", (W + 160, lowest + 260), 255)
canvas.paste(g, (80, 0))
from PIL import ImageDraw
d = ImageDraw.Draw(canvas)
seam = 36
top = lowest + seam
d.polygon([(60, top), (W + 100, top), (W + 130, top + 90), (110, top + 110), (30, top + 55)], fill=0)
canvas.save(REF / "wolf.png")
print("wolf.png", canvas.size, "paw baseline", lowest, "ridge top", top)

# ---- koi ------------------------------------------------------------------
k = Image.open(S / "koi-pd.png").convert("RGBA")
pw, ph = k.size
# The source has a TRANSPARENT border and a tan card behind the fish; the
# fish is whatever is opaque and not tan (corner-sampling found the
# transparent border and masked the whole card -- the two black squares).
TAN = (255, 225, 161)
def is_fish(c):
    r, g, b, a = c
    if a < 64:
        return False
    return not all(abs(v - t) < 40 for v, t in zip((r, g, b), TAN))
mask = [[is_fish(k.getpixel((x, y))) for x in range(pw)] for y in range(ph)]
# largest component, 4-connected
seen = [[False] * pw for _ in range(ph)]
best = []
for sy in range(ph):
    for sx in range(pw):
        if mask[sy][sx] and not seen[sy][sx]:
            stack = [(sx, sy)]; comp = []
            seen[sy][sx] = True
            while stack:
                x, y = stack.pop(); comp.append((x, y))
                for nx, ny in ((x+1,y),(x-1,y),(x,y+1),(x,y-1)):
                    if 0 <= nx < pw and 0 <= ny < ph and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = True; stack.append((nx, ny))
            if len(comp) > 800:      # keep body AND fins; drop speckle
                best.extend(comp)
solid = Image.new("L", (pw, ph), 255)
sp = solid.load()
for x, y in best:
    sp[x, y] = 0
# fill holes: flood white from border; anything white not reached is interior
reach = [[False] * pw for _ in range(ph)]
stack = [(x, y) for x in range(pw) for y in (0, ph - 1)] + [(x, y) for y in range(ph) for x in (0, pw - 1)]
for x, y in stack:
    reach[y][x] = True
stack = [p for p in stack if sp[p[0], p[1]] == 255]
while stack:
    x, y = stack.pop()
    for nx, ny in ((x+1,y),(x-1,y),(x,y+1),(x,y-1)):
        if 0 <= nx < pw and 0 <= ny < ph and not reach[ny][nx] and sp[nx, ny] == 255:
            reach[ny][nx] = True; stack.append((nx, ny))
for y in range(ph):
    for x in range(pw):
        if sp[x, y] == 255 and not reach[y][x]:
            sp[x, y] = 0
# CLOSE the mask (dilate+erode): the brush art paints fins a hairline off
# the body, and 14 of 16 raw contours flagged at 0.3-4.7px. Fused, each
# fish is ONE silhouette with its fins in the outline -- the dragon recipe.
from PIL import ImageFilter
solid = solid.filter(ImageFilter.MinFilter(13)).filter(ImageFilter.MaxFilter(13))

# compose the circling pair: two copies rotated 180, offset diagonally
fishA = solid
fishB = solid.rotate(180)
CW = int(pw * 1.62)
pair = Image.new("L", (CW, CW), 255)
ax, ay = 30, int(CW * 0.06)
# Eric: the right fish's tail clipped the frame -- pulled 65px left, and
# the corridor between the fish is re-checked by the suggester after.
bx, by = CW - pw - 95, CW - ph - int(CW * 0.06)
pair.paste(fishA, (ax, ay), Image.eval(fishA, lambda v: 255 - v))
pair.paste(fishB, (bx, by), Image.eval(fishB, lambda v: 255 - v))
pair.save(REF / "koi.png")
print("koi.png", pair.size)
