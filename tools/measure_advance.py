#!/usr/bin/env python3
"""v8.1 F1 — AVG_ADVANCE measured from the actual font, never eyeballed:
mean advance width in em across the tier word banks."""
import json, pathlib
from PIL import ImageFont
ROOT = pathlib.Path(__file__).resolve().parents[1]
POOLS = ROOT / "content-pipeline/wordpic/pools"
SIZE = 100
font = None
for cand in ["/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/SFNS.ttf",
             "/System/Library/Fonts/Supplemental/Arial.ttf"]:
    try:
        font = ImageFont.truetype(cand, SIZE)
        src = cand
        break
    except Exception:
        continue
total_adv = total_chars = 0
for f in sorted(POOLS.glob("*.json")):
    for w in json.loads(f.read_text()):
        w = w.split("|")[0]
        if " " in w:
            continue
        try:
            adv = font.getlength(w)
        except Exception:
            continue
        total_adv += adv
        total_chars += len(w)
em = total_adv / total_chars / SIZE
print(f"font: {src}")
print(f"chars measured: {total_chars}")
print(f"AVG_ADVANCE = {em:.4f} em")
json.dump({"font": src, "avg_advance_em": round(em, 4), "chars": total_chars},
          open(ROOT / "out/avg_advance.json", "w"))
