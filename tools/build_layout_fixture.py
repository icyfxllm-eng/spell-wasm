#!/usr/bin/env python3
"""Build the CC-SNAP-LAYOUT (Level 2) fixture from a folder of phone photos.

Level 2 is done when "a 15-page fixture set (5 two-column, 5 bold-word
sentence sheets, 5 book pages) imports with <=1 manual correction per page
averaged". That is a count, and a count is not a shot list: fifteen pages
that all happen to be clean two-column sheets would pass a counter and tell
you nothing about the cases 2.1 actually has to get right.

So the plan below is the four layout shapes 2.1 names -- numbered list, two
column, emphasised token, fill-in-the-blank -- plus the ones that decide
whether the detector is honest rather than lucky:

  * a NARROW gutter (row 5), which should fall back to plain rather than
    guess. 2.1's own rule is "confidence below threshold -> plain list with
    every line present", and nothing currently proves the fallback fires.
  * a two-column page of PROSE (row 14). This is the hard negative. A novel
    printed in two columns has a consistent gutter and is not a word list,
    and a detector that imports its left column has learned the wrong thing.
  * a trailing blank, `Name: ______` (row 10), which belongs to CC-SNAP-CLEAN
    F3 and must NOT be taken as 2.1's fill-in-the-blank case -- that line has
    a perfectly good word on it.
  * a skewed shot (row 15), because every x-coordinate in the detector comes
    from a photo somebody took by hand.

    tools/build_layout_fixture.py ~/Desktop/layout-shots          # dry run
    tools/build_layout_fixture.py ~/Desktop/layout-shots --write

Then fill in `expected_words` in each sidecar: the words that SHOULD import
from that page, in order. That list is the ground truth the "<=1 correction
per page" number is computed against -- a correction is a word you would
have to add or uncheck -- and nothing can derive it from the pixels.
"""

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import fixture_shoot as fs  # noqa: E402

DEST = pathlib.Path("tests/fixtures/snap-layout")

PLAN = [
    ("01-two-col-dash", 1, "two columns, word then an em dash then its definition"),
    ("02-two-col-sentence", 2, "two columns, word then a sentence using it"),
    ("03-two-col-wrapping", 3, "two columns where some definitions WRAP to a second line"),
    ("04-two-col-numbered", 4, "two columns with a numbered left column (1. cat  2. dog)"),
    ("05-two-col-narrow", 5, "two columns with a NARROW gutter -- should fall back to plain"),
    ("06-bold-target", 6, "sentence list, the target word in BOLD"),
    ("07-underlined-target", 7, "sentence list, the target word UNDERLINED"),
    ("08-boxed-target", 8, "sentence list, the target word in a BOX"),
    ("09-blank-midline", 9, "fill-in-the-blank: 'The ___ sat on the mat.' -- import nothing"),
    ("10-blank-trailing", 10, "'Name: ______' -- CLEAN's case, the word still imports"),
    ("11-mixed-blanks", 11, "a page mixing fill-in-the-blank lines with ordinary ones"),
    ("12-numbered-list", 12, "a plain numbered spelling list -- the control, stays plain"),
    ("13-book-prose", 13, "an ordinary single-column book page -- the control"),
    ("14-book-two-col-prose", 14, "a book page genuinely set in two columns of PROSE (hard negative)"),
    ("15-two-col-skewed", 15, "a two-column sheet shot at an angle, not square to the camera"),
]


def sidecar(row, needs):
    return {
        "_row": row,
        "_needs": needs,
        "_fill_this_in": "expected_words = what SHOULD import, in order. Then delete this key.",
        "layout": "two_column | plain",
        "expected_words": [],
        "must_not_import": [],
        "notes": "",
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
    print("  now fill in `expected_words` in each .expected.json — nothing can infer it.")


if __name__ == "__main__":
    main()
