#!/usr/bin/env python3
"""Build the CC-SNAP-HIGHLIGHT C2 fixture from a folder of phone photos.

Downscales to 1600px on the long edge, files each photo under the acceptance
row it is meant to satisfy, and writes a sidecar for the ground truth.

Rows 16 and 17 were added after the first two screenshots were filed and
measured, and both are false-positive hunts rather than detection cases.

Row 16 exists because measuring row 11 found TWO saturated clusters on it,
not one: the yellow highlights at hue ~53deg, and the blue section heading at
hue ~200deg, 15k pixels against the highlights' 28k. F2 masks text by
"luminance below an Otsu threshold", that cut fell at L=143, and mid-tone
coloured type is brighter than that -- so a heading survives the mask as
"background" and reads as strongly saturated. On a technical book every
section title would import. Row 16 is that page on its own, with no
highlighter anywhere, so the answer has to be zero.

Row 17 is a printed form marked in cyan. The list had quietly assumed
worksheets are unmarked and marked pages are books, and neither is true of a
form somebody highlighted at work. It also carries a page full of ruled
fill-in lines, which is the environment any future underline detection would
have to survive.

Pairing, downscaling and sidecars live in tools/fixture_shoot.py, shared
with the Level 2 layout shoot. Note the pairing rule it documents: a photo
whose filename starts with a row number claims that row, so a skipped scene
no longer silently misfiles every photo after it.

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
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import fixture_shoot as fs  # noqa: E402

DEST = pathlib.Path("tests/fixtures/snap-highlight")

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
    ("11-ereader-pale", 11, "e-reader SCREENSHOT, PALE highlight -- Kindle (FILED 2026-10-06)"),
    ("12-ereader-saturated", 12, "e-reader SCREENSHOT, SATURATED highlight -- Apple Books (FILED 2026-10-06)"),
    ("13-phrase", 13, "a highlighted phrase of 3+ words"),
    ("14-low-confidence", 14, "a highlighted word OCR will read poorly -- blurred or skewed"),
    ("15-pen-underline", 15, "a word underlined in PEN, no highlighter"),
    # Added 2026-10-06 after measuring the two filed screenshots. Both scenes
    # are false-positive hunts: the first is one the fixture caught before any
    # code ran, the second is a shape the list had assumed away.
    ("16-coloured-headings", 16, "a page with COLOURED HEADING TEXT and NO highlighter "
                                 "(a technical book, blue or red section titles)"),
    ("17-form-other-colour", 17, "a printed FORM marked with a NON-YELLOW highlighter "
                                 "(cyan, pink or green), photographed not screenshotted"),
    # Added 2026-10-06 from a photo Eric sent. The facing page is the point:
    # it is in frame and readable, so OCR returns words from a page nobody
    # meant to import, and no other row has a second page in it at all.
    ("18-open-book-facing-page", 18, "an OPEN BOOK with the FACING PAGE in frame, "
                                     "highlights on one side only (FILED 2026-10-06)"),
]
# C2 also asks for bulk: 10 plain worksheets and 5 yellow book pages in total.
# The named rows cover some of those; these are the filler.
FILLER = [(f"19-worksheet-plain-{i}", None, "another plain worksheet") for i in range(2, 8)]
PLAN = ROWS + FILLER



def sidecar(row, needs):
    return {
        "_row": row,
        "_needs": needs,
        "_fill_this_in": "list the words you actually marked, then delete this key",
        "highlighted": [],
        "colours": [],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("source", help="folder of photos; a name starting with a row number claims that row")
    ap.add_argument("--write", action="store_true", help="actually convert and write")
    args = ap.parse_args()

    paired, leftover = fs.pair(fs.photos_in(args.source), PLAN)
    fs.report(paired, leftover, DEST)
    if not args.write:
        print("\n  (dry run. add --write to convert)")
        return
    fs.write(paired, DEST, sidecar)
    print("  now fill in `highlighted` in each .expected.json — nothing can infer it.")


if __name__ == "__main__":
    main()
