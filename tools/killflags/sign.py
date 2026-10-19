#!/usr/bin/env python3
"""CC-IMPOSSIBLE-COMPLETE F4 -- sign the remote kill-flag document.

    python3 tools/killflags/sign.py --kill impossible_fr impossible_mix          # print to stdout
    python3 tools/killflags/sign.py --kill impossible_fr --out ~/spellgame-server/backend/kill-flags.json
    python3 tools/killflags/sign.py --clear --out ~/spellgame-server/backend/kill-flags.json   # all clear

The document can only name flags to switch OFF (src/killflags.rs). --clear signs an empty list, which
lifts every kill; each document carries its issue time and the app refuses an older one than it holds,
so always sign a fresh one to change the state. The private key stays on the server, outside git:
~/spellgame-server/keys/kill-flags.pem (override with --key). Needs openssl >= 3 (Ed25519 raw signing).

The served file is read by backend/app.py at /api/kill-flags; no restart is needed after writing it.
"""
import argparse
import datetime
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

NAME = re.compile(r"^[a-z0-9_]{1,64}$")


def message(issued, kill):
    return "spell-kill-v1\n" + issued + "\n" + "\n".join(sorted(kill))


def sign(key, issued, kill):
    with tempfile.NamedTemporaryFile("wb", delete=False) as f:
        f.write(message(issued, kill).encode("utf-8"))
        path = f.name
    try:
        out = subprocess.run(["openssl", "pkeyutl", "-sign", "-inkey", str(key), "-rawin", "-in", path], check=True, capture_output=True).stdout
    finally:
        os.unlink(path)
    if len(out) != 64:
        sys.exit(f"unexpected signature length {len(out)}")
    return out.hex()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--key", default=str(pathlib.Path.home() / "spellgame-server/keys/kill-flags.pem"))
    ap.add_argument("--kill", nargs="*", default=[])
    ap.add_argument("--clear", action="store_true")
    ap.add_argument("--issued", help="ISO time; default now (UTC)")
    ap.add_argument("--out")
    a = ap.parse_args()
    if bool(a.kill) == a.clear:
        sys.exit("give flag names with --kill, or --clear for an all-clear (not both, not neither)")
    bad = [n for n in a.kill if not NAME.match(n)]
    if bad:
        sys.exit(f"not a flag name: {bad}")
    issued = a.issued or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    kill = sorted(set(a.kill))
    doc = json.dumps({"v": 1, "issued": issued, "kill": kill, "sig": sign(a.key, issued, kill)}, indent=1) + "\n"
    if a.out:
        tmp = a.out + ".tmp"
        pathlib.Path(tmp).write_text(doc)
        os.replace(tmp, a.out)  # atomic: the server never serves a half-written file
        print(f"wrote {a.out}")
    else:
        sys.stdout.write(doc)


if __name__ == "__main__":
    main()
