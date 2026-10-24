#!/usr/bin/env python3
"""CC-ECLIPSE-HOTFIX F4 (D3, I7): cut the mic diag readout from a release dist.

Every release build runs this: the app (build-web.sh), the education build and
the auditor build. TestFlight and the App Store ship one binary, so there is no
store-only build to cut it from. Dev and test builds keep the readout, behind
its default-off dev-menu switch. The Dockerfile makes the same cut for the site.

    python3 scripts/cut-diag.py dist
"""
import pathlib
import sys

PAIRS = (("<!-- SPELL-DIAG:BEGIN", "<!-- SPELL-DIAG:END -->"),
         ("/* SPELL-DIAG:BEGIN", "/* SPELL-DIAG:END */"))


def cut(src):
    for begin, end in PAIRS:
        while begin in src:
            a = src.index(begin)
            b = src.index(end, a) + len(end)
            src = src[:a] + src[b:]
    return src


if __name__ == "__main__":
    dist = pathlib.Path(sys.argv[1])
    for name in ("index.html", "native-language-kit.js"):
        f = dist / name
        if f.exists():
            f.write_text(cut(f.read_text()))
    print(f"==> cut the mic diag readout from {dist}/")
