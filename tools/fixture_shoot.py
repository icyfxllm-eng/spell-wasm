#!/usr/bin/env python3
"""Shared machinery for turning a folder of phone photos into a test fixture.

Two shoots use this: CC-SNAP-HIGHLIGHT's C2 set and CC-SNAP-LAYOUT's Level 2
set. Both do the same three things -- pair each photo with the acceptance row
it is meant to satisfy, downscale it, and write a sidecar for the ground
truth only a person can supply -- so both do them the same way.

Why 1600px: OCR needs resolution, git does not need 100 MB of phone JPEGs
forever. A 12-megapixel photo is 2-4 MB; at 1600px it is 200-400 KB and still
far above what Vision needs to read body text. Eric's call, 2026-10-03.

PAIRING, which is the part that can quietly ruin a shoot. The first version
of this paired photos to the shot list by sort order alone. That is fine when
twenty photos arrive in exactly the right order and nothing was skipped, and
it is silently wrong otherwise: skip one scene and every later photo is filed
under the wrong row, the sidecars describe the wrong thing, and the first
sign of trouble is an acceptance table that fails for reasons nobody can
reproduce. So a filename may NAME its row -- any photo whose name begins with
the row number (3.jpg, 03-lamp.heic, 3 warm lamp.png) is filed there
regardless of order -- and order is only the fallback for the ones that do
not. Mixing the two is fine; numbered photos are placed first, and the rest
fill the gaps in order.
"""

import json
import pathlib
import re
import shutil
import subprocess
import sys

LONG_EDGE = 1600
EXTS = (".jpg", ".jpeg", ".png", ".heic")


def photos_in(folder):
    src = pathlib.Path(folder).expanduser()
    if not src.is_dir():
        sys.exit(f"FATAL: {src} is not a folder")
    found = sorted(p for p in src.iterdir() if p.suffix.lower() in EXTS and not p.name.startswith("."))
    if not found:
        sys.exit(f"FATAL: no photos in {src}")
    return found


def leading_number(name):
    m = re.match(r"0*(\d+)", name)
    return int(m.group(1)) if m else None


def pair(photos, plan):
    """[(photo|None, slot)] in plan order, plus the photos that found no slot.

    A photo whose filename starts with a number claims that slot (1-based,
    against the plan). Everything else fills the remaining slots in order.
    """
    slots = [None] * len(plan)
    rest = []
    for p in photos:
        n = leading_number(p.name)
        if n is not None and 1 <= n <= len(plan) and slots[n - 1] is None:
            slots[n - 1] = p
        else:
            rest.append(p)
    spare = iter(rest)
    leftover = []
    for i, s in enumerate(slots):
        if s is None:
            nxt = next(spare, None)
            if nxt is not None:
                slots[i] = nxt
    leftover = list(spare)
    return list(zip(slots, plan)), leftover


def long_edge_px(path):
    out = subprocess.run(
        ["sips", "-g", "pixelWidth", "-g", "pixelHeight", str(path)],
        capture_output=True, text=True,
    ).stdout
    dims = [int(l.split(":")[1]) for l in out.splitlines()
            if ":" in l and l.split(":")[1].strip().isdigit()]
    return max(dims) if dims else 0


def report(paired, leftover, dest):
    """What is in the shoot, what is already filed, and what is still missing.

    Filed shots are read from `dest`, not from the source folder. A shoot
    happens over several sittings -- two screenshots on a phone today, the
    book pages next week -- and a tool that only looked at today's folder
    would keep telling you to re-take what is already in the repo.
    """
    # Any filed extension counts, not just .jpg. Row 20 was filed as a .png
    # and a jpg-only test silently called it missing, so the coverage line --
    # the one thing this function exists to print -- was under by one from the
    # day that row landed.
    filed = {s[0] for s in (sl for _, sl in paired)
             if any((dest / f"{s[0]}{e}").exists() for e in (".jpg", ".png"))} \
        if dest.exists() else set()
    have = sum(1 for p, sl in paired if p or sl[0] in filed)
    print(f"  {have} of {len(paired)} shots present -> {dest}")
    if filed:
        print(f"  ({len(filed)} already filed from an earlier sitting)")
    print()
    for photo, slot in paired:
        name, row, needs = slot
        if name in filed and not photo:
            print(f"  {'(already filed)':24} -> {name}.jpg")
        elif photo:
            print(f"  {photo.name:24} -> {name}.jpg   "
                  f"{long_edge_px(photo)}px {photo.stat().st_size/1e6:.1f}MB"
                  + ("   REPLACES a filed shot" if name in filed else ""))
        else:
            print(f"  {'(missing)':24} -> {name}.jpg")
        print(f"      {'row ' + str(row) if row else 'filler':9} {needs}")
    missing = [sl for p, sl in paired if not p and sl[0] not in filed]
    if missing:
        print(f"\n  NOT COVERED — {len(missing)} shot(s) still needed:")
        for name, row, needs in missing:
            print(f"    {('row ' + str(row)) if row else 'filler':9} {needs}")
    if leftover:
        print(f"\n  {len(leftover)} photo(s) beyond the shot list, ignored: "
              + ", ".join(p.name for p in leftover))


def to_display_p3(path):
    """Tag/convert a just-filed shot into Display P3.

    The fixture is normalised to one colour space because saturations from
    different spaces are different units -- one file read delta_s 0.129 as P3
    and 0.204 as sRGB, wider than the gaps the thresholds are argued over.
    Doing it here means `scripts/snap-fixture-colour-check.mjs` never has a
    reason to fail on a fresh shoot. Eric's call, 2026-10-06.
    """
    from PIL import Image, ImageCms  # noqa: PLC0415  (optional until needed)
    import io

    want = "Display P3"
    im = Image.open(path)
    icc = im.info.get("icc_profile")
    if icc:
        try:
            if ImageCms.getProfileDescription(
                    ImageCms.ImageCmsProfile(io.BytesIO(icc))).strip() == want:
                return False
        except Exception:                                  # noqa: BLE001
            pass
    p3_path = pathlib.Path("/System/Library/ColorSync/Profiles/Display P3.icc")
    if not p3_path.exists():
        print(f"  WARNING: no Display P3 profile on this Mac; {path.name} left as-is")
        return False
    p3 = ImageCms.ImageCmsProfile(str(p3_path))
    src = (ImageCms.ImageCmsProfile(io.BytesIO(icc)) if icc
           else ImageCms.createProfile("sRGB"))
    out = ImageCms.profileToProfile(im.convert("RGB"), src, p3, outputMode="RGB")
    if path.suffix.lower() == ".png":
        out.save(path, icc_profile=p3.tobytes())
    else:
        out.save(path, "JPEG", icc_profile=p3.tobytes(), quality=95, subsampling=0)
    return True


def write(paired, dest, sidecar_for):
    """Downscale and write sidecars. Never overwrites a sidecar you have filled in."""
    if not shutil.which("sips"):
        sys.exit("FATAL: sips not found. It ships with macOS; this script assumes it.")
    dest.mkdir(parents=True, exist_ok=True)
    total = n = 0
    for photo, slot in paired:
        if not photo:
            continue
        name, row, needs = slot
        out = dest / f"{name}.jpg"
        subprocess.run(["sips", "-s", "format", "jpeg", "-Z", str(LONG_EDGE),
                        str(photo), "--out", str(out)], capture_output=True, check=True)
        if to_display_p3(out):
            print(f"  {out.name}: converted to Display P3")
        total += out.stat().st_size
        n += 1
        side = dest / f"{name}.expected.json"
        if side.exists():
            print(f"  kept {side.name} (not overwriting your ground truth)")
            continue
        side.write_text(json.dumps(sidecar_for(row, needs), indent=2) + "\n")
    print(f"\n  wrote {n} photo(s), {total/1e6:.1f} MB, to {dest}")


def selftest():
    """The pairing rules, because a misfiled shoot is invisible until much later."""
    plan = [(f"{i:02d}-slot", i, f"scene {i}") for i in range(1, 6)]
    P = lambda s: pathlib.Path(s)
    cases = [
        ("order fills in order", ["a.jpg", "b.jpg"], ["a.jpg", "b.jpg", None, None, None]),
        ("a numbered name claims its slot", ["03-lamp.jpg"], [None, None, "03-lamp.jpg", None, None]),
        ("numbered and unnumbered mix", ["03-lamp.jpg", "x.jpg"], ["x.jpg", None, "03-lamp.jpg", None, None]),
        ("zero padding is the same number", ["3.jpg"], [None, None, "3.jpg", None, None]),
        ("a number past the plan falls back to order", ["99.jpg"], ["99.jpg", None, None, None, None]),
        ("two photos cannot claim one slot", ["03-a.jpg", "03-b.jpg"], ["03-b.jpg", None, "03-a.jpg", None, None]),
    ]
    bad = 0
    for name, names, want in cases:
        paired, _ = pair([P(n) for n in names], plan)
        got = [p.name if p else None for p, _ in paired]
        ok = got == want
        bad += not ok
        print(f"  {'ok    ' if ok else 'FAILED'} {name}")
        if not ok:
            print(f"         want {want}\n         got  {got}")
    extra_paired, extra = pair([P(f"{i}.jpg") for i in "abcdefg"], plan)
    ok = len(extra) == 2
    bad += not ok
    print(f"  {'ok    ' if ok else 'FAILED'} photos beyond the plan are reported, not dropped ({len(extra)})")
    if bad:
        sys.exit("fixture_shoot selftest: FAILED")
    print("fixture_shoot selftest: OK")


if __name__ == "__main__":
    selftest()
