"""F4 -- /api/kill-flags hands over the signed file verbatim, or 204 when there is none. Run with the
server venv:

    ~/spellgame-server/venv/bin/python -m unittest backend/test_kill_flags.py
"""
import json
import os
import tempfile
import unittest

_TMP = tempfile.mkdtemp()
os.environ["AUDIO_CACHE_DIR"] = os.path.join(_TMP, "audio")
os.environ["SPEAK_METRICS_DIR"] = os.path.join(_TMP, "speak")
os.environ.setdefault("CLIMB_DB_PATH", os.path.join(_TMP, "climb.db"))
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")
os.environ["KILL_FLAGS_FILE"] = os.path.join(_TMP, "kill-flags.json")

import app as app_module  # noqa: E402


class KillFlagsRoute(unittest.TestCase):
    def setUp(self):
        self.c = app_module.app.test_client()
        if os.path.exists(os.environ["KILL_FLAGS_FILE"]):
            os.unlink(os.environ["KILL_FLAGS_FILE"])

    def test_no_file_is_204(self):
        r = self.c.get("/api/kill-flags")
        self.assertEqual(r.status_code, 204)
        self.assertEqual(r.data, b"")

    def test_the_file_is_served_verbatim_and_uncached(self):
        doc = {"v": 1, "issued": "2026-10-19T12:00:00Z", "kill": ["impossible_fr"], "sig": "00" * 64}
        with open(os.environ["KILL_FLAGS_FILE"], "w") as f:
            json.dump(doc, f)
        r = self.c.get("/api/kill-flags")
        self.assertEqual(r.status_code, 200)
        self.assertEqual(json.loads(r.data), doc)
        self.assertIn("no-cache", r.headers["Cache-Control"])

    def test_the_server_holds_no_key(self):
        src = open(os.path.join(os.path.dirname(app_module.__file__), "app.py")).read()
        self.assertNotIn("BEGIN PRIVATE KEY", src)


if __name__ == "__main__":
    unittest.main()
