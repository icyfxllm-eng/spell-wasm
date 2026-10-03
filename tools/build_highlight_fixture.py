#!/usr/bin/env python3
"""Build the CC-SNAP-HIGHLIGHT C2 fixture from a folder of phone photos.

Downscales to 1600px on the long edge, files each photo under the acceptance
row it is meant to satisfy, and writes a sidecar for the ground truth.

Why 1600px: OCR needs resolution, git does not need 100 MB of phone JPEGs
forever. tests/fixtures is 36 KB today. A 12-megapixel photo is 2-4 MB; at
1600px it is 200-400 KB and still far above what Vision needs to read body
text. Eric's call, 2026-10-03.

Why it nags about rows: C2 asks for "20 photos" in four buckets, but the
acceptance table asks for specific SCENES -- a warm lamp, an aged page, a
30%-covered word, a hyphen split across a line break. Twenty photos that miss
those leave rows untestable, and you only find out after the shoot. So this
reports coverage against the table rather than against the count.

    tools/build_highlight_fixture.py ~/Desktop/shots          # dry run
    tools/build_highlight_fixture.py ~/Desktop/shots --write

Then fill in the `highlighted` list in each sidecar: the words you actually
marked. That list is the ground truth every acceptance row scores against,
and nothing can derive it from the pixels -- that is the whole point of it.
"""

import argparse
import json
import pathlib
import shutil
import subprocess
import sys

DEST = pathlib.Path("tests/fixtures/snap-highlight")
LONG_EDGE = 1600

# The acceptance table, as a shot list. `needs` is what must be in the frame.
ROWS = [
    ("01-book-5-yellow", 1, "a book page of ~300 words with EXACTLY 5 highlighted in yellow"),
    ("02-worksheet-plain", 2, "an ordinary worksheet, no highlighter anywhere"),
    ("03-worksheet-warm-lamp", 3, "a worksheet under a tungsten lamp, warm cast, NO highlighter"),
    ("04-page-aged", 4, "aged or cream paper, NO highlighter"),
    ("05-word-60pct", 5, "one word about 60% covered by the stroke"),
    ("06-word-30pct", 6, "one word about 30% covered -- a clipped edge"),
    ("07-highlight-wraps", 7, "a highlight running across a line break, 4 words"),
    ("08-hyphen-both", 8, "a word hyphenated at a line break, BOTH halves highlighted"),
    ("09-hyphen-first-only", 9, "the same shape, only the FIRST half highlighted"),
    ("10-two-colours", 10, "one page with yellow AND a second colour"),
    ("11-apple-books", 11, "Apple Books SCREENSHOT with 3 highlights"),
    ("12-kindle", 12, "Kindle SCREENSHOT with 2 highlights"),
    ("13-phrase", 13, "a highlighted phrase of 3+ words"),
    ("14-low-confidence", 14, "a highlighted word OCR will read poorly -- blurred or skewed"),
    ("15-pen-underline", 15, "a word underlined in PEN, no highlighter"),
]
# C2 also asks for bulk: 10 plain worksheets and 5 yellow book pages in total.
# The named rows cover some of those; these are the filler.
FILLER = [(f"16-worksheet-plain-{i}", None, "another plain worksheet") for i in range(2, 8)]
PLAN = ROWS + FILLER


def long_edge(path):
    out = subprocess.run(
        ["sips", "-g", "pixelWidth", "-g", "pixelHeight", str(path)],
        capture_output=True, text=True,
    ).stdout
    dims = [int(l.split(":")[1]) for l in out.splitlines() if ":" in l and l.split(":")[1].strip().isdigit()]
    return max(dims) if dims else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("source", help="folder of photos, in the order of the shot list")
    ap.add_argument("--write", action="store_true", help="actually convert and write")
    args = ap.parse_args()

    if not shutil.which("sips"):
        sys.exit("FATAL: sips not found. It ships with macOS; this script assumes it.")

    src = pathlib.Path(args.source).expanduser()
    photos = sorted(
        p for p in src.iterdir()
        if p.suffix.lower() in (".jpg", ".jpeg", ".png", ".heic") and not p.name.startswith(".")
    )
    if not photos:
        sys.exit(f"FATAL: no photos in {src}")

    print(f"  {len(photos)} photo(s) in {src}")
    print(f"  the shot list wants {len(PLAN)}\n")

    paired = list(zip(photos, PLAN))
    for photo, (name, row, needs) in paired:
        size_mb = photo.stat().st_size / 1e6
        edge = long_edge(photo)
        print(f"  {photo.name:22} -> {name}.jpg   {edge}px {size_mb:.1f}MB"
              + (f"   [row {row}]" if row else ""))
        print(f"      needs: {needs}")

    missing = PLAN[len(photos):]
    if missing:
        print(f"\n  NOT COVERED — {len(missing)} shot(s) still needed:")
        for name, row, needs in missing:
            print(f"    {('row ' + str(row)) if row else 'filler':9} {needs}")
    extra = photos[len(PLAN):]
    if extra:
        print(f"\n  {len(extra)} photo(s) beyond the shot list, ignored.")

    if not args.write:
        print("\n  (dry run. add --write to convert)")
        return

    DEST.mkdir(parents=True, exist_ok=True)
    total = 0
    for photo, (name, row, needs) in paired:
        out = DEST / f"{name}.jpg"
        subprocess.run(
            ["sips", "-s", "format", "jpeg", "-Z", str(LONG_EDGE), str(photo), "--out", str(out)],
            capture_output=True, check=True,
        )
        total += out.stat().st_size
        side = DEST / f"{name}.expected.json"
        if side.exists():
            print(f"  kept existing {side.name} (not overwriting your ground truth)")
            continue
        side.write_text(json.dumps({
            "_row": row,
            "_needs": needs,
            "_fill_this_in": "list the words you actually marked, then delete this key",
            "highlighted": [],
            "colours": [],
        }, indent=2) + "\n")
    print(f"\n  wrote {len(paired)} photo(s), {total/1e6:.1f} MB total, to {DEST}")
    print("  now fill in `highlighted` in each .expected.json — nothing can infer it.")


if __name__ == "__main__":
    main()
