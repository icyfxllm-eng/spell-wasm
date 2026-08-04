#!/usr/bin/env python3
"""Words-as-shading preview render (tool-side, step 9 material).

NOT an export — export stays blocked on pending candidates + the D4
focal touch. This renders the candidate set typographically so Eric's
review judges the real thing: darker band = larger/heavier words,
lighter = smaller/lighter; features carry tiny words; the focal stroke
renders last. Squint eval re-runs against this render.

Usage: tonal_render.py <tonal.json> <out.png> [--wordlist path]
"""
import json
import math
import pathlib
import sys

from PIL import Image, ImageDraw, ImageFont

CANVAS = 512
SS = 4  # supersample: preview crisp under zoom (the app itself is vector text)
# words-as-shading: (font_px, brightness, weight_sim) per band 0..4
BAND_TYPE = [(13, 235, 2), (11, 215, 1), (9, 190, 1), (8, 165, 0), (7, 145, 0)]
FEATURE_TYPE = (7, 255, 1)   # tiny words, brightest — the features pop
FOCAL_TYPE = (7, 255, 2)

tonal_path, out_path = sys.argv[1], sys.argv[2]
# F2 (POLISH, signed): feathered band boundaries — inside a feather zone
# (15% of the local band's thickness, D4) a placement deterministically
# inherits the NEIGHBOR band's params from a position hash. The melt is
# seeded: same seed, same interleave.
FEATHER_FRAC = 0.15

def build_feather(ref_gray, quantiles):
    import cv2 as _c
    import numpy as _n
    band_map = _n.digitize(ref_gray.astype(float), quantiles)
    feather = _n.zeros_like(band_map, dtype=bool)
    neighbor = _n.copy(band_map)
    for b in range(len(quantiles) + 1):
        m = (band_map == b).astype(_n.uint8)
        if m.sum() == 0:
            continue
        dist = _c.distanceTransform(m, _c.DIST_L2, 3)
        thickness = float(dist.max()) * 2 or 1.0
        zone = (dist > 0) & (dist <= max(1.5, FEATHER_FRAC * thickness))
        feather |= zone
        # neighbor band at the closest edge: dilate the complement's ids
        inv = (band_map != b).astype(_n.uint8)
        k = _n.ones((5, 5), _n.uint8)
        near = _c.dilate((band_map * inv).astype(_n.float32), k)
        neighbor[zone] = near[zone].astype(band_map.dtype)
    return band_map, feather, neighbor

def feather_pick(band, x, y, feather, neighbor):
    xi, yi = int(x), int(y)
    if 0 <= yi < feather.shape[0] and 0 <= xi < feather.shape[1] and feather[yi, xi]:
        h = (xi * 73856093 ^ yi * 19349663) & 0xffff
        if h % 100 < 50:
            return int(neighbor[yi, xi])
    return band

# F1+F3 (POLISH, signed): continuous weight + tracking from LOCAL
# LUMINANCE at each word's centroid, sampled from the reference canvas
# (tool-side; the reference never ships — resolved values would be
# stored in piece data). Deterministic: position -> luminance -> params.
TRACK_RANGE = 0.08  # D3 signed: ±8%, report-only this render
TRACK_REPORT = []
# 0a — Mona color side-by-side (POLISH F0 + PICTURE-COLOR D2 tripwire):
# --palette faithful|floored applies her SAMPLED palette per band/region.
# Faithful = raw sampled hexes; floored = legibility-lifted. His verdict
# picks (or escalates) — the tool never picks for him.
PALETTES = {
    "faithful": {0: (58, 46, 38), 1: (96, 74, 52), 2: (128, 96, 70),
                 3: (196, 152, 88), 4: (255, 223, 103),
                 "vocab": (214, 178, 128)},
    "floored":  {0: (110, 96, 84), 1: (140, 116, 90), 2: (170, 136, 104),
                 3: (222, 182, 116), 4: (255, 234, 140),
                 "vocab": (226, 196, 148)},
    # cascade replaces the hybrid: uniform lift preserving band RATIOS
    # until band 0 clears 3.0:1 — Lint B killed the single-band lift
    # (it inverted band order; the Step-0 dump has the numbers).
    "cascade":  {0: (78, 62, 51), 1: (129, 99, 70), 2: (172, 129, 94),
                 3: (255, 204, 118), 4: (255, 251, 139),
                 "vocab": (222, 186, 134)},
}

def lint_a(candidates):
    """Glow scope: PALETTE[4] legal on highlightBand bandFills only —
    ILLEGAL on vocabulary strokes and any eye-region path."""
    bad = []
    for c in candidates:
        if c["pathKind"] == "feature":
            if c.get("_palette_index") == 4:
                bad.append(c.get("feature"))
    return bad

def lint_b(pal, bright=(235, 215, 190, 165, 145), tol=0.005):
    """Band order must survive palette resolution (rendered luminance
    strictly ascending, dark->light)."""
    def rel_lum(rgb):
        def ch(c):
            c = c / 255
            return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
        return 0.2126 * ch(rgb[0]) + 0.7152 * ch(rgb[1]) + 0.0722 * ch(rgb[2])
    lums = [rel_lum(tuple(int(v * (bright[b] / 235.0) + 30) for v in pal[b])) for b in range(5)]
    fails = [(b, round(lums[b], 4), round(lums[b + 1], 4))
             for b in range(4) if lums[b] + tol >= lums[b + 1]]
    return fails
PALETTE = None
if "--palette" in sys.argv:
    PALETTE = PALETTES[sys.argv[sys.argv.index("--palette") + 1]]
words_file = pathlib.Path("../../assets/words/en/easy.txt")
WORDS = [w for w in words_file.read_text().split() if 3 <= len(w) <= 7]

data = json.loads(pathlib.Path(tonal_path).read_text())


def font_at(px, bold):
    for name in (["/System/Library/Fonts/Supplemental/Arial Bold.ttf"] if bold else
                 ["/System/Library/Fonts/Supplemental/Arial.ttf"]):
        try:
            return ImageFont.truetype(name, px)
        except OSError:
            continue
    return ImageFont.load_default()


def place_words(dr, pts, font_px, fill, bold, wi):
    """Typeset words along a polyline: each word occupies its arc span,
    rotated to the local tangent. Returns the advanced word index."""
    f = font_at(font_px, bold)
    # cumulative arc
    if len(pts) < 2:
        return wi
    segs = []
    total = 0.0
    for a, b in zip(pts, pts[1:]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        segs.append((a, b, total, L))
        total += L
    pos = 2.0 * (font_px / 8.0)
    while pos < total - 8:
        word = WORDS[wi % len(WORDS)]
        wi += 1
        w_len = dr.textlength(word, font=f)
        # F1: local luminance modulates brightness within the band
        # (sfumato has no steps); F3: the same signal drives tracking.
        lum_local = None
        if REF_GRAY is not None:
            for a0, b0, start0, L0 in segs:
                if start0 <= pos <= start0 + L0 and L0 > 0:
                    t0 = (pos - start0) / L0
                    sx = int((a0[0] + (b0[0] - a0[0]) * t0) / SS)
                    sy = int((a0[1] + (b0[1] - a0[1]) * t0) / SS)
                    if 0 <= sx < 512 and 0 <= sy < 512:
                        lum_local = REF_GRAY[sy, sx] / 255.0
                    break
        if lum_local is not None:
            # POLARITY (the scorer caught the inversion): on a dark
            # canvas, light ink IS light — a bright locale renders
            # brighter ink, a shadowed locale dimmer.
            fill_mod = int(max(120, min(255, fill * (0.75 + 0.5 * lum_local))))
            track = 1.0 + TRACK_RANGE * (2 * lum_local - 1)  # dark=tight
            TRACK_REPORT.append(round(track, 3))
            w_len = w_len * track
        else:
            fill_mod = fill
        # find the segment containing pos
        for a, b, start, L in segs:
            if start <= pos <= start + L and L > 0:
                t = (pos - start) / L
                x = a[0] + (b[0] - a[0]) * t
                y = a[1] + (b[1] - a[1]) * t
                ang = math.degrees(math.atan2(b[1] - a[1], b[0] - a[0]))
                # render word onto a small sprite, rotate, paste
                sw, sh = int(w_len) + 4, font_px + 6
                sprite = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
                sd = ImageDraw.Draw(sprite)
                if PALETTE is not None and CURRENT_BAND[0] is not None:
                    key = "vocab" if CURRENT_BAND[0] == "vocab" else min(CURRENT_BAND[0], 4)
                    r, g, bch = PALETTE[key]
                    lum = fill_mod / 235.0
                    sd.text((2, 2), word, font=f,
                            fill=(int(r * lum + 30), int(g * lum + 30), int(bch * lum + 30), 255))
                else:
                    sd.text((2, 2), word, font=f, fill=(fill, fill, min(255, fill + 12), 255))
                sprite = sprite.rotate(-ang, expand=True, resample=Image.BICUBIC)
                dr._image.paste(sprite, (int(x - sprite.width / 2), int(y - sprite.height / 2)), sprite)
                break
        pos += w_len + font_px * 0.6
    return wi


CURRENT_BAND = [None]
REF_GRAY = None
FEATHER = None
if "--polish" in sys.argv:
    import cv2
    _refp = pathlib.Path("/private/tmp/claude-501/-Users-eric/d5d833c7-982c-4f16-9353-8fe37d983a28/scratchpad/mona-ref-canvas.png")
    REF_GRAY = cv2.imread(str(_refp), cv2.IMREAD_GRAYSCALE)
    _q = data.get("bandQuantiles")
    if _q:
        BAND_MAP, FEATHER_MASK, NEIGHBOR = build_feather(REF_GRAY, _q)
        FEATHER = True
im = Image.new("RGB", (CANVAS * SS, CANVAS * SS), (14, 19, 31))
dr = ImageDraw.Draw(im)
wi = 0
# darkest bands first (underlayer), features last, focal very last
flows = sorted([c for c in data["candidates"] if c["pathKind"] in ("flow", "boundary")],
               key=lambda c: c.get("bandIndex") or 0)
for c in flows:
    b = min(c.get("bandIndex") or 0, len(BAND_TYPE) - 1)
    if FEATHER and c["points"]:
        mx = sum(q[0] for q in c["points"]) / len(c["points"])
        my = sum(q[1] for q in c["points"]) / len(c["points"])
        b = min(feather_pick(b, mx, my, FEATHER_MASK, NEIGHBOR), len(BAND_TYPE) - 1)
    px, fill, bold = BAND_TYPE[b]
    CURRENT_BAND[0] = b
    wi = place_words(dr, [(q[0] * SS, q[1] * SS) for q in c["points"]], px * SS, fill, bold > 1, wi)
feats = [c for c in data["candidates"] if c["pathKind"] == "feature" and not c.get("focal")]
for c in feats:
    CURRENT_BAND[0] = "vocab"
    px, fill, bold = FEATURE_TYPE
    wi = place_words(dr, [(q[0] * SS, q[1] * SS) for q in c["points"]], px * SS, fill, bold > 0, wi)
for c in [c for c in data["candidates"] if c.get("focal")]:
    CURRENT_BAND[0] = "vocab"
    px, fill, bold = FOCAL_TYPE
    wi = place_words(dr, [(q[0] * SS, q[1] * SS) for q in c["points"]], px * SS, fill, True, wi)

if PALETTE is not None:
    fails_b = lint_b(PALETTE)
    assert not fails_b, f"LINT B: band order broken at {fails_b} — export blocked"
    # deliberate-failure self-tests (proven, not trusted):
    _bad = [{"pathKind": "feature", "feature": "leftEyeLid", "_palette_index": 4}]
    assert lint_a(_bad) == ["leftEyeLid"], "Lint A trap failed to catch"
    _badpal = dict(PALETTE); _badpal[0] = (240, 240, 240)
    assert lint_b(_badpal), "Lint B trap failed to catch"
im.save(out_path)
if TRACK_REPORT:
    import statistics
    print(f"F3 tracking report (D3, report-only): n={len(TRACK_REPORT)} "
          f"min={min(TRACK_REPORT)} max={max(TRACK_REPORT)} "
          f"mean={round(statistics.mean(TRACK_REPORT), 3)}")
print(f"words-on render: {wi} word placements -> {out_path}")
