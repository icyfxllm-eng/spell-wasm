"""R8 -- the /api/speak metrics line: closed values, no word, no IP, and never
in the way of the audio. Run with the server venv:

    ~/spellgame-server/venv/bin/python -m unittest backend/test_speak_metrics.py
"""
import glob
import json
import os
import tempfile
import unittest

_TMP = tempfile.mkdtemp()
os.environ["SPEAK_METRICS_DIR"] = os.path.join(_TMP, "speak")
os.environ["AUDIO_CACHE_DIR"] = os.path.join(_TMP, "audio")
os.environ.setdefault("CLIMB_DB_PATH", os.path.join(_TMP, "climb.db"))
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")  # synthesis is faked below

import app as app_module  # noqa: E402
import speak_metrics  # noqa: E402


def lines():
    out = []
    for f in sorted(glob.glob(os.path.join(os.environ["SPEAK_METRICS_DIR"], "speak-*.jsonl"))):
        with open(f) as fh:
            out += [json.loads(x) for x in fh if x.strip()]
    return out


class LineTest(unittest.TestCase):
    def test_closed_values(self):
        self.assertEqual(
            json.loads(speak_metrics.line("ru", "normal", "hit", 200, 3.14159)),
            {"lang": "ru", "variant": "normal", "cache": "hit", "status": 200, "ms": 3.1},
        )
        self.assertIsNone(speak_metrics.line("en", "whisper", "hit", 200, 1))
        self.assertIsNone(speak_metrics.line("en", "normal", "maybe", 200, 1))
        # A lang that isn't a short code can't smuggle text in.
        self.assertEqual(json.loads(speak_metrics.line("the word cat", "slow", "miss", 400, 1))["lang"], "other")

    def test_day_file_and_no_timestamp(self):
        speak_metrics.record("fr", "slow", "miss", 502, 12, now=0)
        with open(os.path.join(os.environ["SPEAK_METRICS_DIR"], "speak-1970-01-01.jsonl")) as fh:
            rec = json.loads(fh.read().splitlines()[-1])
        self.assertEqual(set(rec), {"lang", "variant", "cache", "status", "ms"})

    def test_unwritable_dir_is_silent(self):
        old = os.environ["SPEAK_METRICS_DIR"]
        os.environ["SPEAK_METRICS_DIR"] = "/dev/null/nope"
        try:
            speak_metrics.record("en", "normal", "hit", 200, 1)  # must not raise
        finally:
            os.environ["SPEAK_METRICS_DIR"] = old


class RouteTest(unittest.TestCase):
    def setUp(self):
        for f in glob.glob(os.path.join(os.environ["SPEAK_METRICS_DIR"], "*")):
            os.remove(f)
        self.client = app_module.app.test_client()
        self.calls = 0

        def fake_synth(word, variant, path, lang="en", py=None):
            self.calls += 1
            if word == "broken":
                raise RuntimeError("tts down")
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as fh:
                fh.write(b"ID3")

        self._orig = app_module.synthesize_to_cache
        app_module.synthesize_to_cache = fake_synth

    def tearDown(self):
        app_module.synthesize_to_cache = self._orig

    def test_miss_then_hit_then_errors(self):
        r1 = self.client.get("/api/speak?word=kitten&lang=en", environ_base={"REMOTE_ADDR": "203.0.113.9"})
        r2 = self.client.get("/api/speak?word=kitten&lang=en&variant=slow")
        r3 = self.client.get("/api/speak?word=kitten&lang=en")
        r4 = self.client.get("/api/speak?word=&lang=en")
        r5 = self.client.get("/api/speak?word=broken&lang=en")
        self.assertEqual([r.status_code for r in (r1, r2, r3, r4, r5)], [200, 200, 200, 400, 502])
        self.assertEqual(r3.data, b"ID3")
        for r in (r1, r2, r3, r4, r5):
            r.close()
        got = [(x["lang"], x["variant"], x["cache"], x["status"]) for x in lines()]
        self.assertEqual(got, [
            ("en", "normal", "miss", 200),
            ("en", "slow", "miss", 200),
            ("en", "normal", "hit", 200),
            ("other", "normal", "miss", 400),
            ("en", "normal", "miss", 502),
        ])
        raw = "".join(open(f).read() for f in glob.glob(os.path.join(os.environ["SPEAK_METRICS_DIR"], "*")))
        for secret in ("kitten", "broken", "203.0.113.9"):
            self.assertNotIn(secret, raw)


if __name__ == "__main__":
    unittest.main()
