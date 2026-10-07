#!/usr/bin/env python3
"""Put every CC-SNAP fixture shot in Display P3, so saturations compare.

WHY. On 2026-10-06 the highlight fixture was found to hold four different
colour spaces: 13 shots tagged Display P3, 3 sRGB, 2 untagged, and one
carrying a MONITOR profile (PHL 241V8LB). Every measurement in the review
record read raw pixels, which ignores the tag, so rows in different spaces
were compared in the same tables. That is not a rounding error -- the same
file reads delta_s 0.129 as P3 and 0.204 converted to sRGB, which is larger
than the gaps the thresholds are argued over.

WHY P3 AND NOT sRGB. Most shots already carry it, so normalising to P3 moves
the fewest bytes. More importantly, converting the vivid markers to sRGB
CLIPS them: rows 10's orange and cyan both hit S = 1.000, which destroys the
measurement. P3 is wide enough to hold every real highlighter measured.
Eric's call, 2026-10-06.

WHAT IT DOES NOT DO. It does not re-encode a shot that is already P3. A JPEG
generation costs a little chroma, and spending one on 13 correct files to
make the pipeline uniform would trade real fidelity for tidiness. So the
converted shots carry one extra JPEG generation and the others do not; that
asymmetry is deliberate and is recorded here rather than hidden.

    tools/normalise_fixture_colour.py            # report only
    tools/normalise_fixture_colour.py --write
"""

import argparse
import io
import pathlib
import sys

from PIL import Image, ImageCms

DIRS = [pathlib.Path("tests/fixtures/snap-highlight"),
        pathlib.Path("tests/fixtures/snap-layout")]
TARGET = "Display P3"
SYSTEM_P3 = pathlib.Path("/System/Library/ColorSync/Profiles/Display P3.icc")
# Quality high enough that the conversion itself is not what moves a number.
JPEG_OPTS = dict(quality=95, subsampling=0)


def describe(im):
    icc = im.info.get("icc_profile")
    if not icc:
        return None, "(none — treated as sRGB)"
    try:
        return icc, ImageCms.getProfileDescription(
            ImageCms.ImageCmsProfile(io.BytesIO(icc))).strip()
    except Exception as e:                                  # noqa: BLE001
        return icc, f"(unreadable: {e})"


def target_profile():
    """Prefer a P3 already in the tree, so this does not depend on macOS."""
    for d in DIRS:
        for p in sorted(d.glob("*.jpg")) + sorted(d.glob("*.png")):
            icc, name = describe(Image.open(p))
            if name == TARGET:
                return ImageCms.ImageCmsProfile(io.BytesIO(icc)), f"from {p.name}"
    if SYSTEM_P3.exists():
        return ImageCms.ImageCmsProfile(str(SYSTEM_P3)), f"from {SYSTEM_P3}"
    sys.exit("FATAL: no Display P3 profile found, in the fixture or on this Mac.")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--write", action="store_true", help="actually convert")
    args = ap.parse_args()

    p3, where = target_profile()
    print(f"  target: {TARGET} ({where})")
    srgb = ImageCms.createProfile("sRGB")
    todo = []
    for d in DIRS:
        if not d.exists():
            continue
        for p in sorted(list(d.glob("*.jpg")) + list(d.glob("*.png"))):
            icc, name = describe(Image.open(p))
            mark = "ok" if name == TARGET else "CONVERT"
            print(f"  {mark:8} {p.name:36} {name}")
            if name != TARGET:
                todo.append((p, icc))

    if not todo:
        print("\n  every shot is already Display P3.")
        return
    if not args.write:
        print(f"\n  {len(todo)} to convert. add --write")
        return

    for p, icc in todo:
        im = Image.open(p)
        src = ImageCms.ImageCmsProfile(io.BytesIO(icc)) if icc else srgb
        out = ImageCms.profileToProfile(im.convert("RGB"), src, p3, outputMode="RGB")
        if p.suffix.lower() == ".png":
            out.save(p, icc_profile=p3.tobytes())
        else:
            out.save(p, "JPEG", icc_profile=p3.tobytes(), **JPEG_OPTS)
        print(f"  wrote {p.name}")
    print(f"\n  {len(todo)} converted. Every `measured` block on a converted "
          "shot is now stale — re-measure before trusting it.")


if __name__ == "__main__":
    main()
