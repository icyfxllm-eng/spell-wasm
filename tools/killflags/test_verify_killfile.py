"""AT-1.1 .. AT-1.3 for tools/killflags/verify_killfile.py. Run: python3 -m unittest tools/killflags/test_verify_killfile.py

The real signing key is never used here (CI does not have it). The tests make throwaway keys, which
exercises the same code path: sign with the key at some path, check against a public key."""
import contextlib
import io
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import verify_killfile as vk  # noqa: E402

# A document signed with the REAL key (tools/killflags/sign.py, 2026-10-19), the same fixture the Rust
# tests use: proves the verifier agrees with the app's embedded public key.
REAL_DOC = """{
 "v": 1,
 "issued": "2026-10-19T12:00:00Z",
 "kill": ["impossible_fr", "impossible_mix"],
 "sig": "bd2e970d4e27620794019a061fe849b2c3a9f7704f98811d4a6f3af78aa23a823657dd22db41ad6ac079cd4ff1bb3162fbf6fb739a77e09b3d59f9408199180e"
}"""


def make_key(path):
    subprocess.run(["openssl", "genpkey", "-algorithm", "ed25519", "-out", str(path)], check=True, capture_output=True)
    os.chmod(path, 0o600)
    der = subprocess.run(["openssl", "pkey", "-in", str(path), "-pubout", "-outform", "DER"], check=True, capture_output=True).stdout
    return der[-32:].hex()


def run(argv):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = vk.main(argv)
    return code, out.getvalue()


class T(unittest.TestCase):
    def setUp(self):
        self.d = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.d, ignore_errors=True)

    def test_a_copy_of_the_key_at_another_path_verifies(self):  # AT-1.1
        orig = self.d / "orig.pem"
        pub = make_key(orig)
        backup = self.d / "elsewhere" / "backup-copy.pem"
        backup.parent.mkdir()
        shutil.copy(orig, backup)
        code, out = run(["--key", str(backup), "--pubkey-hex", pub])
        self.assertEqual(code, 0, out)
        self.assertTrue(out.startswith("OK"))

    def test_an_unrelated_key_fails(self):  # AT-1.2
        _, pub = self.d / "a.pem", make_key(self.d / "a.pem")
        make_key(self.d / "b.pem")
        code, out = run(["--key", str(self.d / "b.pem"), "--pubkey-hex", pub])
        self.assertEqual(code, 1, out)
        self.assertTrue(out.startswith("FAIL"))

    def test_nothing_secret_is_printed(self):  # AT-1.3
        pem = self.d / "k.pem"
        pub = make_key(pem)
        secret = pem.read_text()
        body = "".join(l for l in secret.splitlines() if "-----" not in l)
        for argv in (["--key", str(pem), "--pubkey-hex", pub], ["--key", str(self.d / "missing.pem"), "--pubkey-hex", pub]):
            _, out = run(argv)
            self.assertNotIn("PRIVATE", out)
            self.assertNotIn(body, out)
            self.assertNotIn(body[:20], out)
        # The tool never even opens the key file itself; it hands the path to openssl.
        src = (HERE / "verify_killfile.py").read_text()
        self.assertNotIn("read_text(", src.split("def check_key")[1].split("def check_doc")[0])

    def test_not_a_key_fails_cleanly(self):
        junk = self.d / "junk.pem"
        junk.write_text("not a key")
        code, out = run(["--key", str(junk)])
        self.assertEqual(code, 1)
        self.assertTrue(out.startswith("FAIL"))

    def test_the_embedded_key_accepts_the_real_signed_document(self):
        doc = self.d / "kill-flags.json"
        doc.write_text(REAL_DOC)
        code, out = run(["--doc", str(doc)])
        self.assertEqual(code, 0, out)
        doc.write_text(REAL_DOC.replace("impossible_fr", "impossible_de"))
        code, out = run(["--doc", str(doc)])
        self.assertEqual(code, 1, out)

    def test_the_embedded_key_is_read_from_the_apps_source(self):
        self.assertEqual(len(vk.embedded_public_key_hex()), 64)


if __name__ == "__main__":
    unittest.main()
