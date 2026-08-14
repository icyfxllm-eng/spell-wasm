#!/usr/bin/env python3
"""The alphabet program: 213 glyphs across 7 scripts, optically centered,
per-glyph auto-tuned dilation. Emits PNGs named <setid><nn>.png."""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import json, os, subprocess, sys

S = os.path.dirname(os.path.abspath(__file__))
SUGGEST = "/Users/eric/repos/spell-wasm/target/release/suggest"

SETS = [
    ("lat",  "DejaVuSans.ttf",         [chr(c) for c in range(65, 91)],            [13, 11, 9]),
    ("cyr",  "DejaVuSans.ttf",         [chr(c) for c in [0x0401] + list(range(0x0410, 0x0430))], [13, 11, 9]),
    ("arb",  "NotoSansArabic.ttf",     list("ابتثجحخدذرزسشصضطظعغفقكلمنهوي"),        [15, 13, 11, 9]),
    ("dev",  "NotoSansDevanagari.ttf", list("अआइईउऊऋएऐओऔकखगघङचछजझञटठडढणतथदधनपफबभमयरलवशषसह"), [11, 9, 7]),
    ("kor",  "NotoSansKR.ttf",         list("ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎㅏㅑㅓㅕㅗㅛㅜㅠㅡㅣ"), [13, 11, 9]),
    ("hir",  "NotoSansJP.ttf",         list("あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん"), [11, 9, 7]),
    ("hnz",  "NotoSansSC.ttf",         list("一二三四五六七八九十"),                  [13, 11, 9]),
]

def render(ch, fontfile, k):
    f = ImageFont.truetype(f"{S}/{fontfile}", 700)
    x0, y0, x1, y1 = f.getbbox(ch)
    if x1 <= x0:
        return None
    big = Image.new("L", (x1 - x0 + 100, y1 - y0 + 100), 255)
    ImageDraw.Draw(big).text((50 - x0, 50 - y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(k))
    bb = big.point(lambda v: 255 - v).getbbox()
    if bb is None:
        return None
    big = big.crop(bb)
    s = 430 / max(big.size)
    big = big.resize((max(1, round(big.size[0] * s)), max(1, round(big.size[1] * s))), Image.LANCZOS)
    big = big.point(lambda v: 0 if v < 128 else 255)
    out = Image.new("L", (512, 512), 255)
    px = big.load()
    sx = sy = m = 0
    for yy in range(big.size[1]):
        for xx in range(big.size[0]):
            if px[xx, yy] < 128:
                sx += xx; sy += yy; m += 1
    if m == 0:
        return None
    ox = min(max(round(256 - sx / m), 26), 512 - big.size[0] - 26)
    oy = min(max(round(256 - sy / m), 26), 512 - big.size[1] - 26)
    out.paste(big, (ox, oy))
    return out

def audit(sub, outdir):
    d = json.load(open(f"{outdir}/{sub}.json"))
    problems = []
    for c in d["candidates"]:
        if c["flagged"]:
            problems.append(f"flag@{c['id']}")
        if not c["micro"] and c["id"] > 0 and 130 < c["arc"] < 250:
            problems.append(f"danger{round(c['arc'])}@{c['id']}")
        if c["clearance"] < 20:
            problems.append(f"tight{c['clearance']:.0f}@{c['id']}")
    return problems

manifest = {}
for setid, fontfile, chars, ks in SETS:
    outdir = f"{S}/sugg_{setid}"
    os.makedirs(outdir, exist_ok=True)
    for i, ch in enumerate(chars, 1):
        sub = f"{setid}{i:02d}"
        best = None
        for k in ks:
            im = render(ch, fontfile, k)
            if im is None:
                continue
            im.save(f"{S}/{sub}.png")
            if os.path.exists(f"{outdir}/{sub}.json"):
                os.remove(f"{outdir}/{sub}.json")
            r = subprocess.run([SUGGEST, f"{S}/{sub}.png", outdir], capture_output=True, text=True)
            probs = audit(sub, outdir) if os.path.exists(f"{outdir}/{sub}.json") else ["nojson"]
            if not probs:
                best = (k, [])
                break
            if best is None or len(probs) < len(best[1]):
                best = (k, probs)
                im.save(f"{S}/{sub}.best.png")
        if best and best[1]:
            # restore the least-bad take
            if os.path.exists(f"{S}/{sub}.best.png"):
                os.replace(f"{S}/{sub}.best.png", f"{S}/{sub}.png")
                if os.path.exists(f"{outdir}/{sub}.json"):
                    os.remove(f"{outdir}/{sub}.json")
                subprocess.run([SUGGEST, f"{S}/{sub}.png", outdir], capture_output=True, text=True)
        elif os.path.exists(f"{S}/{sub}.best.png"):
            os.remove(f"{S}/{sub}.best.png")
        manifest[sub] = {"set": setid, "char": ch, "k": best[0] if best else None,
                         "problems": best[1] if best else ["unrenderable"]}
    clean = sum(1 for s2, m in manifest.items() if m["set"] == setid and not m["problems"])
    print(f"{setid}: {clean}/{len(chars)} clean", flush=True)

json.dump(manifest, open(f"{S}/alpha_manifest.json", "w"), ensure_ascii=False, indent=1)
dirty = {k: v for k, v in manifest.items() if v["problems"]}
print("NEEDS ATTENTION:", json.dumps(dirty, ensure_ascii=False) if dirty else "none")
