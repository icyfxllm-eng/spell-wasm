#!/usr/bin/env python3
"""Wave 10 sourced pair (Eric's "strongest five", 2026-08-01).

redfuji: Hokusai, "South Wind, Clear Sky" (Red Fuji) — PD-Art via
         Wikimedia Commons. The mountain is everything that is neither
         sky-blue nor cloud-white; the snow dendrites arrive as holes of
         the print's own making, and the forest band solidifies into the
         mountain's base (close + largest component).
scream:  Munch, "The Scream" (1893, National Gallery of Norway) — PD-Art
         via Wikimedia Commons. The dark masses ARE the composition:
         figure with its pale face holes, the bridge rails, the fjord.
         Margins cropped, sky streak noise dropped by component floor.

Run: python3 tools/prep_wave10.py <src-dir>
"""
import pathlib
import sys
from collections import deque

from PIL import Image, ImageDraw, ImageFilter

S = pathlib.Path(sys.argv[1])
REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"


def components(img, keep):
    W, H = img.size
    px = img.load()
    seen = [[False] * W for _ in range(H)]
    out = Image.new("L", (W, H), 0)
    op = out.load()
    for y0 in range(H):
        for x0 in range(W):
            if px[x0, y0] == 255 and not seen[y0][x0]:
                q = deque([(x0, y0)])
                seen[y0][x0] = True
                cells = [(x0, y0)]
                while q:
                    x, y = q.popleft()
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < W and 0 <= ny < H and not seen[ny][nx] and px[nx, ny] == 255:
                            seen[ny][nx] = True
                            q.append((nx, ny))
                            cells.append((nx, ny))
                if keep(cells):
                    for x, y in cells:
                        op[x, y] = 255
    return out


def pad_invert_save(img, name, pad=70):
    W, H = img.size
    c = Image.new("L", (W + 2 * pad, H + 2 * pad), 0)
    c.paste(img, (pad, pad))
    from PIL import ImageOps
    ImageOps.invert(c).save(REF / f"{name}.png")
    print(name, "->", c.size)


# ---- redfuji --------------------------------------------------------------
im = Image.open(S / "redfuji.jpg").convert("RGB")
im.thumbnail((1400, 1400))
W, H = im.size
src = im.load()
m = Image.new("L", (W, H), 0)
mp = m.load()

# The mountain is the region BELOW its own two slopes — geometry, not
# color, excludes the clouds (three global-threshold attempts welded
# cloud lace into the summit).
def slope_top(x):
    sx, sy = 0.712 * W, 0.175 * H          # summit
    if x <= sx:
        lx, ly = 0.0, 0.72 * H             # left slope foot at the frame
        t = (x - lx) / (sx - lx)
        return ly + (sy - ly) * t
    rx, ry = W, 0.50 * H                   # right slope exit
    t = (x - sx) / (rx - sx)
    return sy + (ry - sy) * t

for y in range(H):
    for x in range(W):
        if y < slope_top(x) - 10:
            continue
        r, g, b = src[x, y]
        sky = b > r + 20 and b > 110
        white = r + g + b > 620
        if not sky and not white:
            mp[x, y] = 255
d = ImageDraw.Draw(m)
d.rectangle([0, 0, int(W * 0.12), int(H * 0.55)], fill=0)   # cartouche + signature
m = m.filter(ImageFilter.MaxFilter(11)).filter(ImageFilter.MinFilter(7))
m = components(m, keep=lambda c: len(c) >= W * H * 0.02)
m = m.filter(ImageFilter.MedianFilter(7))
pad_invert_save(m, "redfuji")

# ---- scream ---------------------------------------------------------------
# Fjord extracted; figure and rails AUTHORED over the source (the
# cypress precedent): Munch's deck shadow is as dark as the coat, so no
# threshold can carve the figure free — and the figure is the one shape
# that must be right. Rails stop short of both the figure and the
# fjord so nothing welds.
im2 = Image.open(S / "scream.jpg").convert("RGB")
im2.thumbnail((1200, 1600))
W2, H2 = im2.size
crop = im2.crop((int(W2 * 0.07), int(H2 * 0.05), int(W2 * 0.93), int(H2 * 0.97)))
W2, H2 = crop.size
src2 = crop.load()
C = Image.new("L", (W2, H2), 0)

# fjord + far shore: the dark blue mass, upper band only
bx0, by0, bx1, by1 = int(W2 * 0.05), int(H2 * 0.14), int(W2 * 0.97), int(H2 * 0.40)
fj = Image.new("L", (W2, H2), 0)
jp = fj.load()
for y in range(by0, by1):
    for x in range(bx0, bx1):
        r, g, b = src2[x, y]
        if r + g + b < 300 and b >= r:
            jp[x, y] = 255
fj = fj.filter(ImageFilter.MaxFilter(13)).filter(ImageFilter.MinFilter(9))
fj = components(fj, keep=lambda c: len(c) >= W2 * H2 * 0.015)
fj = fj.filter(ImageFilter.MedianFilter(9))
C.paste(Image.new("L", (W2, H2), 255), (0, 0), fj)

d2 = ImageDraw.Draw(C)
def P(fx, fy):
    return (int(fx * W2), int(fy * H2))

def cr_loop(knots, per=8):
    def cr(p0, p1, p2, p3, t):
        t2, t3 = t * t, t * t * t
        return (0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
                0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3))
    pts = []
    n = len(knots)
    for i in range(n):
        p0, p1, p2, p3 = knots[(i - 1) % n], knots[i], knots[(i + 1) % n], knots[(i + 2) % n]
        for k in range(per):
            pts.append(cr(p0, p1, p2, p3, k / per))
    return pts

# the figure: swaying coat, skull head, hands pressed to the cheeks —
# one wavy Catmull-Rom mass, after Munch
coat = [P(0.50, 0.635), P(0.545, 0.66), P(0.565, 0.71), P(0.55, 0.77),
        P(0.565, 0.84), P(0.60, 0.91), P(0.615, 0.97), P(0.60, 0.985),
        P(0.44, 0.985), P(0.435, 0.93), P(0.46, 0.86), P(0.445, 0.78),
        P(0.46, 0.71), P(0.475, 0.66)]
d2.polygon(cr_loop(coat), fill=255)
d2.ellipse([P(0.455, 0.545)[0], P(0.455, 0.545)[1], P(0.575, 0.655)[0], P(0.575, 0.655)[1]], fill=255)  # head
# hands: two mitts rising to the cheeks
d2.polygon(cr_loop([P(0.435, 0.575), P(0.455, 0.60), P(0.462, 0.65), P(0.44, 0.685), P(0.418, 0.66), P(0.415, 0.61)]), fill=255)
d2.polygon(cr_loop([P(0.595, 0.575), P(0.615, 0.61), P(0.618, 0.66), P(0.594, 0.685), P(0.572, 0.65), P(0.578, 0.60)]), fill=255)
# no face hole: its ring pinched the head at every legal size. The
# eyes and mouth are micro holes straight into the head mass — the
# renderer fills micro light, and three light dots on the dark head
# ARE the screaming face.
d2.ellipse([P(0.486, 0.574)[0], P(0.486, 0.574)[1], P(0.510, 0.602)[0], P(0.510, 0.602)[1]], fill=0)
d2.ellipse([P(0.520, 0.574)[0], P(0.520, 0.574)[1], P(0.544, 0.602)[0], P(0.544, 0.602)[1]], fill=0)
d2.ellipse([P(0.502, 0.608)[0], P(0.502, 0.608)[1], P(0.528, 0.636)[0], P(0.528, 0.636)[1]], fill=0)

# rails to the vanishing point, stopped clear of figure and fjord
van = (int(W2 * 0.585), int(H2 * 0.345))
fig_left = int(W2 * 0.40)
rail_gap = 64   # 34 left the rail 12px off the figure at scan scale
for (foot, w, stop_x) in [((int(W2 * 0.02), int(H2 * 0.66)), 34, int(W2 * 0.52)),
                          ((int(W2 * 0.10), int(H2 * 0.99)), 30, fig_left - rail_gap),
                          ((int(W2 * 0.30), int(H2 * 0.99)), 26, fig_left - rail_gap)]:
    dx, dy = van[0] - foot[0], van[1] - foot[1]
    t1 = min(0.90, (stop_x - foot[0]) / dx if dx else 1.0)
    end = (foot[0] + dx * t1, foot[1] + dy * t1)
    # the handrail also stops 40px under the fjord's lowest reach
    if end[1] < int(H2 * 0.435):
        t1 *= (int(H2 * 0.435) - foot[1]) / (end[1] - foot[1])
        end = (foot[0] + dx * t1, foot[1] + dy * t1)
    d2.line([foot, end], fill=255, width=w)

pad_invert_save(C, "scream")
