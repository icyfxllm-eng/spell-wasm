#!/usr/bin/env python3
"""CC-WORD-PICTURE D10 — posterize-then-trace authoring tool (BUILD-TIME ONLY).

Ingests a public-domain scan, posterizes it to tonal bands, and emits
candidate flow-line stroke maps as generator fragments for Eric to curate.
Runtime never generates art (permanent invariant) — this tool's output is
committed data, review-gated like all content.

Usage:
    pip install pillow          # one-time
    python3 trace.py ref/mona-lisa.jpg --bands 4 --out mona-draft.py

Output: a Python fragment of flow(...) calls in the gen-wordpic-strokemaps
vocabulary (paths carry band + tier budgets), plus a .svg preview of the
traced lines over nothing (ghost preview). Curate: delete/merge/move lines,
then paste into the generator and run the L9 sweep — the sweep is the law.
"""
import argparse
import sys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--bands", type=int, default=4)
    ap.add_argument("--out", default="draft.py")
    ap.add_argument("--min-run", type=int, default=46, help="min stroke px (v6 solver floor)")
    ap.add_argument("--row-step", type=int, default=30, help="scanline spacing at darkest band")
    args = ap.parse_args()
    try:
        from PIL import Image
    except ImportError:
        sys.exit("Pillow required: pip install pillow")

    img = Image.open(args.image).convert("L")
    w, h = img.size
    scale = 460.0 / max(w, h)
    img = img.resize((int(w * scale), int(h * scale)))
    w, h = img.size
    ox, oy = (512 - w) // 2, (512 - h) // 2
    px = img.load()
    # Posterize: band 1 = lightest (large sparse), band N = darkest (dense).
    levels = args.bands
    def band_of(v):
        return levels - min(levels - 1, v * levels // 256)

    lines = []
    # One pass of text-legal rows (>=26px apart, the v6 collision law); each
    # row splits into tonal RUNS — every run becomes a stroke carrying its
    # band (band -> size/weight is the shading, D7).
    step = max(26, args.row_step)
    for y in range(0, h, step):
        run_start = 0
        run_band = band_of(px[0, y])
        for x in range(1, w):
            b = band_of(px[x, y])
            if b != run_band or x == w - 1:
                length = x - run_start
                if length >= args.min_run:
                    lines.append((run_band, run_start + ox, y + oy, x + ox, y + oy))
                run_start = x
                run_band = b
    with open(args.out, "w") as f:
        f.write("# D10 draft — curate before committing; the L9 sweep is the law.\n")
        f.write(f"# source: {args.image}  bands: {levels}  lines: {len(lines)}\n")
        for band, x0, y0, x1, y1 in lines:
            budget = "B_EXP"
            f.write(f"_m.append(flow(line({x0}, {y0}, {x1}, {y1}), mn(), {budget}, {band}, \"line\"))\n")
    svg = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512">',
           '<rect width="512" height="512" fill="#101623"/>']
    for band, x0, y0, x1, y1 in lines:
        op = 0.25 + band * 0.15
        svg.append(f'<line x1="{x0}" y1="{y0}" x2="{x1}" y2="{y1}" stroke="#e8ecf5" stroke-opacity="{op:.2f}" stroke-width="{1 + band}"/>')
    svg.append("</svg>")
    prev = args.out.rsplit(".", 1)[0] + "-preview.svg"
    with open(prev, "w") as f:
        f.write("".join(svg))
    print(f"{len(lines)} candidate strokes → {args.out}; preview: {prev}")
    print("Curate (delete/merge/reflow), paste into the generator, run the sweep.")


if __name__ == "__main__":
    main()
