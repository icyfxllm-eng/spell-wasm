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

EVERY SHOT MUST BE CAMERA-ORIGINAL (or a real screen capture). This fixture
exists to calibrate thresholds against what a phone sensor actually records,
so a rendered or AI-generated picture of a page is not a weaker version of a
photo -- it is a different measurement with no sensor noise, no demosaic, no
real optics, and shadows that were painted rather than cast. Its numbers look
exactly like the real ones and would silently move a threshold.

This is not hypothetical. On 2026-10-06 a generated image of a worksheet on a
wooden desk was measured and reported before anyone said where it came from;
its "sunlit oak" read S 0.757, more saturated than every real highlighter in
the set, and that number was briefly carried into an argument about F1
alongside genuine camera measurements of rows 22 and 23. Nothing reached the
repo, and `_provenance` exists so the next one is declared before it is
measured rather than after.

Nothing here can detect a generated image, and nothing tries to. The field is
a place to state what a human knows and the tool cannot.
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
    ("03-worksheet-warm-lamp", 3, "a worksheet under a tungsten lamp, warm cast, NO highlighter "
                                  "(FILED 2026-10-06 -- the cast moves HUE onto the highlighters, "
                                  "not saturation)"),
    ("04-page-aged", 4, "aged or cream paper, NO highlighter (FILED 2026-10-06)"),
    ("05-word-60pct", 5, "one word about 60% covered by the stroke (FILED 2026-10-06)"),
    ("06-word-30pct", 6, "one word about 30% covered -- a clipped edge (FILED 2026-10-06, same frame as row 5)"),
    ("07-highlight-wraps", 7, "a highlight running across a line break, 4 words"),
    ("08-hyphen-both", 8, "a word hyphenated at a line break, BOTH halves highlighted"),
    ("09-hyphen-first-only", 9, "the same shape, only the FIRST half highlighted"),
    ("10-two-colours", 10, "one page with yellow AND a second colour (FILED 2026-10-06 with FOUR)"),
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
    # A no-highlighter control, and the only one shot in poor light. Pen on
    # ruled paper is the shape most likely to be mistaken for a mark.
    ("19-handwritten-ruled", 19, "a HANDWRITTEN word list in pen on RULED paper, "
                                 "no highlighter (FILED 2026-10-06)"),
    # Added 2026-10-06. A pale callout box is a bright saturated fill with dark
    # text on it -- the same shape as a highlight -- so only saturation tells
    # them apart, and this page is what bounds delta_s from below.
    ("20-pale-callout-boxes", 20, "an infographic with PALE COLOURED CALLOUT BOXES behind "
                                  "dark text, no highlighter (FILED 2026-10-06)"),
    # 21 and 22 are a pair: the same colour at the two ends of its range, which
    # is what bounds the dynamic range a detector has to cover. 21 is the one
    # that shows saturation cannot work -- it is a REAL mark that is less
    # saturated than row 20, which is not a mark at all.
    ("21-palest-mark-pink", 21, "the PALEST real highlight available, pink on white "
                                "(FILED 2026-10-06)"),
    ("22-vividest-mark-pink", 22, "the most VIVID real highlight, same colour family, on paper "
                                  "(FILED 2026-10-06)"),
    # The top of the pink range, and the only page with handwriting UNDER a
    # mark. Also the caution against row 10's size proposal: this stroke is
    # 1.43x the height of the word it covers.
    ("23-neon-over-handwriting", 23, "a NEON highlighter over HANDWRITING on ruled paper "
                                     "(FILED 2026-10-06)"),
    # The A/B control for row 20, and the only matched pair in the set: the
    # SAME card, screenshotted once and photographed off a monitor once. It is
    # what measures the camera path's cost in saturation, which nothing else
    # can, because every other row differs in content as well as in source.
    ("24-screen-photo-of-row-20", 24, "a PHOTOGRAPH of a monitor showing row 20's card, "
                                      "same content, camera instead of screen capture "
                                      "(FILED 2026-10-06)"),
    # The first COOL-coloured mark on paper, which row 24 made urgent: its
    # photographed blue callout arrived at delta_s 0.016 and raised the
    # question whether a cool marker survives a camera at all.
    ("25-cyan-on-aged-paper", 25, "a COOL-coloured highlighter (cyan, blue or green) on AGED "
                                  "CREAM book paper, photographed (FILED 2026-10-06)"),
]
# C2 also asks for bulk: 10 plain worksheets and 5 yellow book pages in total.
# The named rows cover some of those; these are the filler.
FILLER = [(f"26-worksheet-plain-{i}", None, "another plain worksheet") for i in range(2, 8)]
PLAN = ROWS + FILLER



def sidecar(row, needs):
    return {
        "_row": row,
        "_needs": needs,
        # Provenance first, because it decides whether anything below is worth
        # reading. See EVERY SHOT MUST BE CAMERA-ORIGINAL in the docstring.
        "_provenance": "camera | screenshot | GENERATED -- say which, and say what device",
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
