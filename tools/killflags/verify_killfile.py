#!/usr/bin/env python3
"""CC-IMPOSSIBLE-LAUNCH Feature 1 -- does this private key sign kill files the app will accept?

    python3 tools/killflags/verify_killfile.py --key /path/to/backup-copy.pem
    python3 tools/killflags/verify_killfile.py --doc ~/spellgame-server/backend/kill-flags.json

Run it against your BACKUP copy of the signing key. It signs a throwaway probe document with that key
and checks the signature against the public key compiled into the app (PUBLIC_KEY_HEX in
src/killflags.rs), using the same message format the app verifies. It exits 0 and says OK only when
the app would accept a document signed by this key; otherwise it exits 1 and says FAIL.

Safe by construction: the private key is only ever handed to openssl by path. This tool never reads
its bytes, never prints, logs or copies it, and prints nothing derived from it. It writes only to a
temporary file holding the probe message and its signature, deleted on exit. Needs openssl >= 3.

--doc checks an already signed document against the app's key instead (no private key involved).
--pubkey-hex overrides the embedded key; it exists for the tests and should not be used otherwise.
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

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
import sign as signer  # noqa: E402  (the one signer: same message bytes the app checks)

# DER prefix of an Ed25519 SubjectPublicKeyInfo: the raw 32-byte key follows.
SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")


def embedded_public_key_hex():
    src = (ROOT / "src" / "killflags.rs").read_text(encoding="utf-8")
    m = re.search(r'pub const PUBLIC_KEY_HEX: &str = "([0-9a-f]{64})";', src)
    if not m:
        sys.exit("FAIL: could not find PUBLIC_KEY_HEX in src/killflags.rs")
    return m.group(1)


def verify(pubkey_hex, message, sig_hex):
    """True iff sig_hex is a valid Ed25519 signature of message under pubkey_hex."""
    try:
        pub = bytes.fromhex(pubkey_hex)
        sig = bytes.fromhex(sig_hex)
    except ValueError:
        return False
    if len(pub) != 32 or len(sig) != 64:
        return False
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        (d / "pub.der").write_bytes(SPKI_PREFIX + pub)
        (d / "msg").write_bytes(message.encode("utf-8"))
        (d / "sig").write_bytes(sig)
        r = subprocess.run(
            ["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(d / "pub.der"), "-keyform", "DER",
             "-rawin", "-in", str(d / "msg"), "-sigfile", str(d / "sig")],
            capture_output=True,
        )
    return r.returncode == 0


def check_key(key_path, pubkey_hex):
    if not pathlib.Path(key_path).is_file():
        return False, f"no such file: {key_path}"
    issued = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    kill = ["verify_probe"]
    try:
        sig = signer.sign(key_path, issued, kill)
    except subprocess.CalledProcessError:
        return False, "openssl could not sign with that file (not an Ed25519 private key?)"
    if verify(pubkey_hex, signer.message(issued, kill), sig):
        return True, "a document signed with this key is accepted by the app's public key"
    return False, "this key signs fine, but the app's public key REJECTS its signatures (a different key)"


def check_doc(doc_path, pubkey_hex):
    try:
        doc = json.loads(pathlib.Path(doc_path).read_text(encoding="utf-8"))
        issued, kill, sig = doc["issued"], doc["kill"], doc["sig"]
    except (OSError, ValueError, KeyError, TypeError):
        return False, "not a readable kill document"
    if doc.get("v") != 1 or not isinstance(kill, list) or not all(isinstance(k, str) for k in kill):
        return False, "not a v1 kill document"
    if verify(pubkey_hex, signer.message(issued, kill), sig):
        return True, "the document's signature verifies against the app's public key"
    return False, "the document's signature does NOT verify against the app's public key"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--key", help="path to an Ed25519 private key PEM (your backup copy)")
    g.add_argument("--doc", help="path to a signed kill document")
    ap.add_argument("--pubkey-hex", help="override the embedded public key (tests only)")
    a = ap.parse_args(argv)
    pub = a.pubkey_hex or embedded_public_key_hex()
    ok, why = check_key(a.key, pub) if a.key else check_doc(a.doc, pub)
    print(("OK: " if ok else "FAIL: ") + why)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
