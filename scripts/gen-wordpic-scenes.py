#!/usr/bin/env python3
"""CC-WORD-PICTURE v3 D2a — vector picture generator (fill-canvas model).

v3: pictures are FILL TEMPLATES. Each <g data-piece="N"> is one fill piece;
N is the fill order (D6: broad regions first, focal details last). There is
NO semantic word→piece relationship — words come from the tier pools.

The shared element library lives here as parameterized part functions reused
across pictures at one consistent style (D7 carry-over). Rerunning
regenerates every picture deterministically.

Template spec (docs/wordpic-template-spec.md, enforced by wordpic-check.mjs):
  * viewBox 0 0 512 512; groups <g data-piece="1..N"> in ascending order,
    piece counts inside the tier band (easy 6-10, medium 15-25, hard 30-50).
  * Paths: class="fl" fills, class="st" ink strokes (any number of each —
    v3 fills fade whole pieces in; no stroke-draw choreography).
  * Ink var(--wp-ink,#2b2f3a) width 6 round; fills via var(--wp-*) tokens so
    palette variants stay palette-map-only.
"""
import json
import math
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "assets", "scenes", "starter")
INK = "var(--wp-ink,#2b2f3a)"
ST = f'fill="none" stroke="{INK}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"'


def P(d, fill):
    return f'<path class="fl" fill="{fill}" d="{d}"/>'


def S(d):
    return f'<path class="st" {ST} d="{d}"/>'


class Pic:
    def __init__(self, sid):
        self.sid = sid
        self.groups = []

    def piece(self, parts):
        self.groups.append(parts)
        return self

    def svg(self):
        body = "".join(
            f'<g data-piece="{i + 1}">' + "".join(parts) + "</g>"
            for i, parts in enumerate(self.groups))
        return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" '
                f'class="wp-scene" data-scene="{self.sid}" data-pieces="{len(self.groups)}">'
                f'{body}</svg>')


# ---------- shared element library ----------

def star5(cx, cy, r):
    pts = []
    for i in range(10):
        rr = r if i % 2 == 0 else r * 0.45
        a = -math.pi / 2 + i * math.pi / 5
        pts.append(f"{cx + rr * math.cos(a):.0f} {cy + rr * math.sin(a):.0f}")
    return "M" + "L".join(pts) + "Z"


def cloud(cx, cy, s):
    return (f"M{cx - 2 * s} {cy} a{s} {s} 0 0 1 {s * 1.1:.0f} -{s * 1.1:.0f} "
            f"a{s * 1.2:.0f} {s * 1.2:.0f} 0 0 1 {s * 2:.0f} 0 "
            f"a{s} {s} 0 0 1 {s * 0.9:.0f} {s * 1.1:.0f} Z")


def circle(cx, cy, r):
    return f"M{cx - r} {cy} a{r} {r} 0 1 0 {2 * r} 0 a{r} {r} 0 1 0 -{2 * r} 0"


def sunburst(cx, cy, r):
    d = circle(cx, cy, r)
    for i in range(8):
        a = i * math.pi / 4
        x1, y1 = cx + (r + 12) * math.cos(a), cy + (r + 12) * math.sin(a)
        x2, y2 = cx + (r + 30) * math.cos(a), cy + (r + 30) * math.sin(a)
        d += f" M{x1:.0f} {y1:.0f} L{x2:.0f} {y2:.0f}"
    return d


def crescent(cx, cy, r):
    return (f"M{cx} {cy - r} a{r} {r} 0 1 0 0 {2 * r} "
            f"a{r * 0.78:.0f} {r * 0.78:.0f} 0 1 1 0 -{2 * r} Z")


def flame(cx, cy, s):
    return (f"M{cx} {cy} c{s * 0.9:.0f} -{s * 0.7:.0f} {s * 0.3:.0f} -{s * 1.6:.0f} 0 -{s * 2.2:.0f} "
            f"c-{s * 0.3:.0f} {s * 0.6:.0f} -{s * 0.9:.0f} {s * 1.5:.0f} 0 {s * 2.2:.0f} Z")


def bird(cx, cy, s):
    return f"M{cx - 2 * s} {cy} q{s} -{s} {2 * s} 0 q{s} -{s} {2 * s} 0"


def flowerhead(cx, cy, s):
    d = ""
    for i in range(5):
        a = -math.pi / 2 + i * 2 * math.pi / 5
        px, py = cx + s * math.cos(a), cy + s * math.sin(a)
        d += f"M{px:.0f} {py:.0f} a{s * 0.55:.0f} {s * 0.55:.0f} 0 1 1 0.1 0 "
    return d + circle(cx, cy, s * 0.45)


def pine(cx, gy, h):
    w = h * 0.55
    return (f"M{cx} {gy - h} L{cx + w / 2:.0f} {gy - h * 0.45:.0f} L{cx + w * 0.3:.0f} {gy - h * 0.45:.0f} "
            f"L{cx + w * 0.75:.0f} {gy} L{cx - w * 0.75:.0f} {gy} L{cx - w * 0.3:.0f} {gy - h * 0.45:.0f} "
            f"L{cx - w / 2:.0f} {gy - h * 0.45:.0f} Z M{cx} {gy} L{cx} {gy + h * 0.12:.0f}")


GROUND = "M20 400 Q256 386 492 400"
SKY_BLUE = "var(--wp-sky,#8ec9e8)"
SUN_Y = "var(--wp-sun,#ffd166)"
LEAF_G = "var(--wp-leaf,#7bc47f)"
SNOW_W = "var(--wp-snow,#f2f6fb)"
WOOD = "var(--wp-wood,#a9825e)"
FUR = "var(--wp-fur,#c9a26b)"
FUR2 = "var(--wp-fur2,#a9825e)"
BODY = "var(--wp-body,#7fb069)"
ACC = "var(--wp-accent,#e07a7a)"
ROCK = "var(--wp-rock,#9aa3b2)"
INKF = "var(--wp-ink,#2b2f3a)"
STONE = "var(--wp-stone,#d9c48f)"
STONE2 = "var(--wp-stone2,#c4ae79)"
IRON = "var(--wp-iron,#8a7f74)"


# ---------- easy (6-10 pieces) ----------

def pic_cat():
    body = "M256 380 q-90 0 -90 -80 q0 -60 40 -84 q-8 -40 50 -40 q58 0 50 40 q40 24 40 84 q0 80 -90 80 Z"
    return (Pic("cat")
        .piece([P("M156 396 a100 14 0 1 0 200 0 a100 14 0 1 0 -200 0", "rgba(0,0,0,.12)")])
        .piece([P(body, FUR), S(body)])
        .piece([P("M212 186 l-26 -46 l50 22 Z M300 186 l26 -46 l-50 22 Z", FUR2),
                S("M212 186 l-26 -46 l50 22 M300 186 l26 -46 l-50 22")])
        .piece([S("M346 360 q60 -10 52 -70 q-6 -40 -40 -36")])
        .piece([P(circle(226, 224, 10) + " " + circle(286, 224, 10), INKF),
                S(circle(226, 224, 14) + " " + circle(286, 224, 14))])
        .piece([P("M248 252 l16 0 l-8 12 Z", ACC),
                S("M248 252 l16 0 l-8 12 Z M256 264 l0 14 M256 278 q-12 10 -24 2 M256 278 q12 10 24 2")])
        .piece([S("M226 356 q30 12 60 0")])
        .piece([P(circle(120, 372, 26), ACC),
                S(circle(120, 372, 26) + " M104 366 q16 -10 32 0 M106 382 q14 -8 28 0")]))


def pic_star():
    return (Pic("star")
        .piece([S(GROUND)])
        .piece([P("M60 400 L170 250 L280 400 Z", ROCK), S("M60 400 L170 250 L280 400")])
        .piece([P("M170 250 L200 291 L184 296 L160 282 L140 291 Z", SNOW_W),
                S("M140 291 L160 282 L184 296 L200 291")])
        .piece([P(pine(360, 400, 150).split(" M")[0], LEAF_G), S(pine(360, 400, 150))])
        .piece([P(crescent(120, 120, 46), SUN_Y), S(crescent(120, 120, 46))])
        .piece([P(star5(300, 100, 34), SUN_Y), S(star5(300, 100, 34))])
        .piece([P(star5(400, 170, 16) + " " + star5(230, 180, 12), SUN_Y),
                S(star5(400, 170, 16) + " " + star5(230, 180, 12))])
        .piece([P(cloud(420, 90, 26), SNOW_W), S(cloud(420, 90, 26))])
        .piece([S("M20 60 Q256 20 492 60 M60 200 l0.1 0 M470 240 l0.1 0 M256 250 l0.1 0 M100 320 l0.1 0 M430 330 l0.1 0")]))


def pic_fish():
    fishbody = "M150 300 q60 -60 150 -20 q30 12 30 20 q0 8 -30 20 q-90 40 -150 -20 Z"
    return (Pic("fish")
        .piece([S("M40 460 q20 -16 40 0 M120 470 q20 -16 40 0 M420 462 q20 -16 40 0")])
        .piece([P("M20 150 q40 -18 80 0 t80 0 t80 0 t80 0 t80 0 t72 0 L492 130 L20 130 Z", "var(--wp-water,#79b8d9)"),
                S("M20 150 q40 -18 80 0 t80 0 t80 0 t80 0 t80 0 t72 0")])
        .piece([P(fishbody, ACC), S(fishbody + " M262 268 q10 30 0 62")])
        .piece([P("M330 300 l52 -34 q-14 34 0 68 Z", ACC), S("M330 300 l52 -34 q-14 34 0 68 Z")])
        .piece([P(circle(196, 292, 7), INKF), S(circle(196, 292, 10))])
        .piece([P("M180 128 L332 128 L302 156 L210 156 Z", WOOD), S("M180 128 L332 128 L302 156 L210 156 Z")])
        .piece([S("M256 128 L256 60 L306 96 L256 96")])
        .piece([S("M60 220 q20 -14 40 0 t40 0 M330 236 q20 -14 40 0 t40 0 M160 400 q20 -14 40 0 t40 0")]))


def pic_house():
    return (Pic("house")
        .piece([S(GROUND)])
        .piece([P("M156 400 L156 260 L256 190 L356 260 L356 400 Z", "var(--wp-wall,#e8d9b0)"),
                S("M156 400 L156 260 M356 260 L356 400")])
        .piece([P("M136 266 L256 180 L376 266 L356 240 L256 170 L156 240 Z", ACC),
                S("M136 266 L256 180 L376 266 M296 214 L296 186 L322 186 L322 232")])
        .piece([P("M236 400 L236 316 q20 -14 40 0 L276 400 Z", WOOD),
                S("M236 400 L236 316 q20 -14 40 0 L276 400 M264 356 l0.1 0")])
        .piece([P("M186 300 h44 v40 h-44 Z", SKY_BLUE),
                S("M186 300 h44 v40 h-44 Z M208 300 v40 M186 320 h44")])
        .piece([S("M310 186 q-14 -18 4 -30 q18 -12 8 -28 q-8 -14 6 -24")])
        .piece([P(circle(96, 96, 34), SUN_Y), S(sunburst(96, 96, 34))])
        .piece([P(flowerhead(120, 372, 16), ACC), S(flowerhead(120, 372, 16) + " M120 388 L120 400")])
        .piece([P(flowerhead(410, 378, 12), ACC), S(flowerhead(410, 378, 12) + " M410 390 L410 400")]))


# ---------- medium (15-25 pieces) ----------

def pic_rocket():
    p = Pic("rocket")
    p.piece([S("M120 430 L392 430")])
    p.piece([S("M20 60 Q256 16 492 60")])
    p.piece([P("M236 410 L236 250 L276 250 L276 410 Z", SNOW_W),
             S("M236 410 L236 250 M276 250 L276 410 M236 410 L276 410")])
    p.piece([P("M236 250 q0 -70 20 -90 q20 20 20 90 Z", ACC),
             S("M236 250 q0 -70 20 -90 q20 20 20 90")])
    p.piece([P("M236 340 l-36 44 l36 8 Z", ACC), S("M236 340 l-36 44 l36 8")])
    p.piece([P("M276 340 l36 44 l-36 8 Z", ACC), S("M276 340 l36 44 l-36 8")])
    p.piece([P(circle(256, 264, 28), SNOW_W), S(circle(256, 264, 28))])
    p.piece([P(circle(256, 264, 20), SKY_BLUE), S(circle(256, 264, 20))])
    p.piece([S("M244 300 L268 300 M244 316 L268 316")])
    p.piece([P(flame(256, 480, 24), SUN_Y), S(flame(256, 480, 24))])
    p.piece([P(flame(236, 470, 13), ACC), S(flame(236, 470, 13))])
    p.piece([P(flame(276, 470, 13), ACC), S(flame(276, 470, 13))])
    p.piece([S(cloud(180, 466, 18))])
    p.piece([S(cloud(338, 470, 15))])
    p.piece([P(crescent(90, 140, 38), SUN_Y), S(crescent(90, 140, 38))])
    p.piece([P(star5(410, 120, 26), SUN_Y), S(star5(410, 120, 26))])
    p.piece([P(star5(340, 60, 12), SUN_Y), S(star5(340, 60, 12))])
    p.piece([P(star5(120, 260, 12), SUN_Y), S(star5(120, 260, 12))])
    return p


def pic_dragon():
    body = ("M150 388 Q116 320 172 290 Q168 220 262 216 Q330 214 352 250 "
            "Q368 224 406 230 Q446 238 440 272 Q436 300 402 302 Q404 336 372 348 "
            "Q300 392 150 388 Z")
    p = Pic("dragon")
    p.piece([S(GROUND)])
    p.piece([P("M14 396 L60 316 L106 396 Z", ROCK), S("M14 396 L60 316 L106 396")])
    p.piece([P("M84 396 L118 342 L152 396 Z", ROCK), S("M84 396 L118 342 L152 396")])
    p.piece([P(body, BODY), S(body)])
    p.piece([S("M330 366 q20 10 40 2")])
    p.piece([P("M260 246 q-4 -78 60 -104 q-10 38 8 52 q-28 12 -16 52 Z", "var(--wp-body2,#5f8f52)"),
             S("M260 246 q-4 -78 60 -104 q-10 38 8 52 q-28 12 -16 52 Z")])
    p.piece([S("M154 356 q-76 -10 -84 -76 l0 -12")])
    p.piece([P("M70 268 l-26 -14 l10 28 Z", BODY), S("M70 268 l-26 -14 l10 28 Z")])
    p.piece([P(circle(408, 258, 6), INKF), S(circle(408, 258, 9))])
    p.piece([S("M438 284 q26 8 34 24")])
    p.piece([P(flame(472, 320, 20), ACC), S(flame(472, 320, 20))])
    p.piece([P(flame(492, 316, 12), ACC), S(flame(492, 316, 12))])
    p.piece([S("M214 250 l-6 20 M250 240 l-6 20 M286 238 l-6 20")])
    p.piece([P("M310 400 l0 -34 q12 -10 24 0 l0 34 Z", ACC),
             S("M310 400 l0 -34 q12 -10 24 0 l0 34")])
    p.piece([S("M308 356 l0 -12 l7 7 l7 -9 l7 9 l7 -7 l0 12 Z " + circle(322, 340, 3))])
    p.piece([P(cloud(120, 120, 22), SNOW_W), S(cloud(120, 120, 22))])
    p.piece([P(star5(420, 90, 16), SUN_Y), S(star5(420, 90, 16))])
    return p


def pic_snowman():
    p = Pic("snowman")
    p.piece([S("M40 440 Q256 424 472 440")])
    p.piece([P(circle(256, 350, 74), SNOW_W), S(circle(256, 350, 74))])
    p.piece([P(circle(256, 226, 52), SNOW_W), S(circle(256, 226, 52))])
    p.piece([P(circle(238, 212, 6), INKF), S(circle(238, 212, 6))])
    p.piece([P(circle(274, 212, 6), INKF), S(circle(274, 212, 6))])
    p.piece([P("M256 228 l34 8 l-34 10 Z", "var(--wp-carrot,#e8944a)"), S("M256 228 l34 8 l-34 10 Z")])
    p.piece([S("M234 252 q22 14 44 0")])
    p.piece([P(circle(256, 322, 7) + " " + circle(256, 352, 7) + " " + circle(256, 382, 7), INKF),
             S(circle(256, 322, 7) + " " + circle(256, 352, 7) + " " + circle(256, 382, 7))])
    p.piece([P("M216 176 h80 v-14 h-80 Z", INKF), S("M216 176 h80 M216 162 h80")])
    p.piece([P("M234 162 v-40 h44 v40 Z", INKF), S("M234 162 v-40 h44 v40")])
    p.piece([P("M208 268 q48 22 96 0 l0 18 q-48 20 -96 0 Z", ACC),
             S("M208 268 q48 22 96 0 M208 286 q48 20 96 0")])
    p.piece([P("M296 282 l14 40 l-22 -6 Z", ACC), S("M296 282 l14 40 l-22 -6 Z")])
    p.piece([S("M180 300 l-44 -14 M180 310 l-40 12")])
    p.piece([S("M332 300 l44 -14 M332 310 l40 12")])
    p.piece([S(bird(380, 120, 14))])
    p.piece([S(bird(120, 150, 11))])
    p.piece([S("M120 120 l0.1 0 M420 160 l0.1 0 M90 260 l0.1 0 M430 300 l0.1 0 M160 90 l0.1 0 M350 80 l0.1 0")])
    return p


# ---------- hard (30-50 pieces) ----------

def pic_eiffel():
    p = Pic("eiffel")
    p.piece([S(GROUND)])
    p.piece([P(cloud(100, 90, 24), SNOW_W), S(cloud(100, 90, 24))])
    p.piece([P(cloud(420, 120, 20), SNOW_W), S(cloud(420, 120, 20))])
    p.piece([P(circle(440, 60, 26), SUN_Y), S(sunburst(440, 60, 26))])
    p.piece([P("M150 400 Q170 330 200 300 L216 300 Q190 340 178 400 Z", IRON),
             S("M150 400 Q170 330 200 300 M216 300 Q190 340 178 400")])
    p.piece([P("M362 400 Q342 330 312 300 L296 300 Q322 340 334 400 Z", IRON),
             S("M362 400 Q342 330 312 300 M296 300 Q322 340 334 400")])
    p.piece([S("M164 372 L192 372 M158 386 L186 386")])
    p.piece([S("M348 372 L320 372 M354 386 L326 386")])
    p.piece([S("M178 400 Q256 320 334 400")])
    p.piece([S("M190 392 Q256 330 322 392")])
    p.piece([P("M186 292 h140 v16 h-140 Z", IRON), S("M186 292 h140 v16 h-140 Z")])
    p.piece([P("M204 292 L232 200 L280 200 L308 292 Z", IRON),
             S("M204 292 L232 200 M308 292 L280 200")])
    p.piece([S("M212 276 L300 276")])
    p.piece([S("M212 276 L300 244 M300 276 L212 244")])
    p.piece([S("M218 244 L294 244")])
    p.piece([S("M218 244 L294 216 M294 244 L218 216")])
    p.piece([P("M222 200 h68 v12 h-68 Z", IRON), S("M222 200 h68 v12 h-68 Z")])
    p.piece([P("M236 200 L248 120 L264 120 L276 200 Z", IRON),
             S("M236 200 L248 120 M276 200 L264 120")])
    p.piece([S("M240 184 L272 184 M242 168 L270 168")])
    p.piece([S("M242 168 L270 152 M270 168 L242 152")])
    p.piece([S("M245 152 L267 152 M247 136 L265 136")])
    p.piece([P("M246 120 h20 v-10 h-20 Z", IRON), S("M246 120 h20 v-10 h-20 Z")])
    p.piece([S("M256 110 L256 70")])
    p.piece([P(star5(256, 58, 10), SUN_Y), S(star5(256, 58, 10))])
    p.piece([P(pine(80, 400, 80).split(" M")[0], LEAF_G), S(pine(80, 400, 80))])
    p.piece([P(pine(440, 400, 70).split(" M")[0], LEAF_G), S(pine(440, 400, 70))])
    p.piece([P(flowerhead(120, 380, 12), ACC), S(flowerhead(120, 380, 12) + " M120 390 L120 400")])
    p.piece([P(flowerhead(392, 384, 10), ACC), S(flowerhead(392, 384, 10) + " M392 392 L392 400")])
    p.piece([S(bird(180, 120, 12))])
    p.piece([S(bird(340, 90, 10))])
    p.piece([S("M60 430 Q256 416 452 430")])
    p.piece([S("M60 356 l0 -24 l16 8 l-16 8")])
    p.piece([S("M452 360 l0 -24 l-16 8 l16 8")])
    p.piece([P(cloud(256, 40, 16), SNOW_W), S(cloud(256, 40, 16))])
    return p


def pic_pyramids():
    p = Pic("pyramids")
    p.piece([S("M20 404 Q256 392 492 404")])
    p.piece([P(circle(430, 80, 32), SUN_Y), S(sunburst(430, 80, 32))])

    def courses(bx, by, bw, bh, rows, flip=False):
        out = []
        for i in range(rows):
            y0 = by - bh * i / rows
            y1 = by - bh * (i + 1) / rows
            w0 = bw * (1 - i / rows)
            w1 = bw * (1 - (i + 1) / rows)
            d = (f"M{bx + (bw - w0) / 2:.0f} {y0:.0f} L{bx + (bw + w0) / 2:.0f} {y0:.0f} "
                 f"L{bx + (bw + w1) / 2:.0f} {y1:.0f} L{bx + (bw - w1) / 2:.0f} {y1:.0f} Z")
            even = (i % 2 == 0) != flip
            out.append([P(d, STONE if even else STONE2), S(d)])
        return out

    for c in courses(170, 400, 240, 210, 8):
        p.piece(c)
    for c in courses(40, 400, 130, 110, 5, flip=True):
        p.piece(c)
    for c in courses(384, 400, 100, 80, 4):
        p.piece(c)
    p.piece([S("M120 400 Q116 350 128 320")])
    p.piece([P("M128 320 q-34 -18 -56 2 q28 4 34 14 Z M128 320 q34 -18 56 2 q-28 4 -34 14 Z", LEAF_G),
             S("M128 320 q-34 -18 -56 2 M128 320 q34 -18 56 2")])
    p.piece([S("M448 402 Q452 366 444 342")])
    p.piece([P("M444 342 q-28 -14 -46 2 q22 3 28 11 Z M444 342 q28 -14 46 2 q-22 3 -28 11 Z", LEAF_G),
             S("M444 342 q-28 -14 -46 2 M444 342 q28 -14 46 2")])
    p.piece([S("M60 440 q30 -10 60 0")])
    p.piece([S("M200 448 q30 -10 60 0")])
    p.piece([S("M360 442 q30 -10 60 0")])
    p.piece([S(bird(150, 100, 12))])
    p.piece([S(bird(210, 80, 9))])
    p.piece([P(cloud(90, 70, 18), SNOW_W), S(cloud(90, 70, 18))])
    p.piece([P(cloud(300, 56, 14), SNOW_W), S(cloud(300, 56, 14))])
    p.piece([P("M270 452 q6 -22 26 -22 q8 -12 18 -12 q10 0 16 10 q22 0 24 20 q-2 10 -14 10 l-56 0 q-12 0 -14 -6 Z", WOOD),
             S("M270 452 q6 -22 26 -22 q8 -12 18 -12 q10 0 16 10 q22 0 24 20")])
    p.piece([S("M354 448 q14 -4 16 -18 q0 -10 8 -12 M282 458 l0 14 M310 458 l0 14 M336 458 l0 14")])
    return p


PICS = {
    "cat": pic_cat, "star": pic_star, "fish": pic_fish, "house": pic_house,
    "rocket": pic_rocket, "dragon": pic_dragon, "snowman": pic_snowman,
    "eiffel": pic_eiffel, "pyramids": pic_pyramids,
}
BANDS = {"easy": (6, 10), "medium": (15, 25), "hard": (30, 50), "expert": (60, 100)}


def main():
    os.makedirs(OUT, exist_ok=True)
    manifest = json.load(open(os.path.join(ROOT, "config", "wordpic", "pictures.json")))
    tiers = {pic["id"]: pic["tier"] for pic in manifest["pictures"] if pic["class"] == "vector"}
    assert set(tiers) == set(PICS), set(tiers) ^ set(PICS)
    for sid, fn in PICS.items():
        pic = fn()
        n = len(pic.groups)
        lo, hi = BANDS[tiers[sid]]
        assert lo <= n <= hi, f"{sid}: {n} pieces outside {tiers[sid]} band {lo}-{hi}"
        with open(os.path.join(OUT, f"{sid}.svg"), "w", encoding="utf-8") as f:
            f.write(pic.svg() + "\n")
        print(f"  {sid}.svg — {n} pieces ({tiers[sid]})")
    print(f"gen-wordpic-scenes: {len(PICS)} vector pictures")


if __name__ == "__main__":
    main()
