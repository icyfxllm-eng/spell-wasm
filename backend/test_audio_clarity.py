"""CC-AUDIO-CLARITY v1.1 Phase A — F1's acceptance, at the level this pipeline
actually operates. Run with the server venv:

    ~/spellgame-server/venv/bin/python -m unittest backend/test_audio_clarity.py

A-1 in the spec is written against a decoder ("feed a clip with -60 dBFS
content in its first and last 80 ms"). The census (C3) found that nothing here
decodes, trims, resamples or re-encodes: the provider's bytes are written to
the cache and served unchanged. So the honest form of A-1 is byte identity --
if the bytes that reach disk are the bytes the provider sent, nothing was
removed, and no decoder is needed to prove it.
"""
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_TMP = tempfile.mkdtemp()
os.environ["AUDIO_CACHE_DIR"] = os.path.join(_TMP, "audio")
os.makedirs(os.environ["AUDIO_CACHE_DIR"], exist_ok=True)
os.environ.setdefault("CLIMB_DB_PATH", os.path.join(_TMP, "climb.db"))
# Never the real key: synthesis is faked in every test below.
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")

import app  # noqa: E402  (env first, as the other backend tests do)

PAYLOAD = bytes([0xFF, 0xFB]) + bytes(range(256)) * 8


class FakeResponse:
    def __init__(self, payload=PAYLOAD):
        self.audio_content = payload


class Padding(unittest.TestCase):
    def test_defined_once(self):
        lead, trail = app._breaks()
        self.assertEqual(lead, '<break time="%dms"/>' % app.PAD_LEAD_MS)
        self.assertEqual(trail, '<break time="%dms"/>' % app.PAD_TRAIL_MS)
        self.assertEqual((app.PAD_LEAD_MS, app.PAD_TRAIL_MS), (200, 150))
        body = app._padded_ssml("half")
        self.assertTrue(body.startswith("<speak>" + lead))
        self.assertTrue(body.endswith(trail + "</speak>"))

    def test_google_text_path_pads(self):
        sent = {}

        class Client:
            def synthesize_speech(self, input=None, voice=None, audio_config=None):
                sent["ssml"] = getattr(input, "ssml", "") or ""
                return FakeResponse()

        lead, trail = app._breaks()
        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("half", "normal", os.path.join(_TMP, "en.mp3"), "en")
        self.assertIn(lead, sent["ssml"])
        self.assertIn(trail, sent["ssml"])
        self.assertIn("half", sent["ssml"])

    def test_mandarin_pads_and_keeps_its_reading(self):
        sent = {}

        class Client:
            def synthesize_speech(self, input=None, voice=None, audio_config=None):
                sent["ssml"] = getattr(input, "ssml", "") or ""
                return FakeResponse()

        lead, trail = app._breaks()
        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("爱", "normal", os.path.join(_TMP, "zh.mp3"), "zh", py="ai4")
        self.assertIn(lead, sent["ssml"])
        self.assertIn(trail, sent["ssml"])
        # CC-ZH-TONE F6: a padded clip must still carry the forced reading.
        self.assertIn("pinyin", sent["ssml"])
        self.assertIn("ai4", sent["ssml"])

    def test_azure_pads(self):
        posted = {}

        class R:
            def read(self):
                return PAYLOAD

            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False

        def fake_urlopen(req, timeout=None):
            posted["body"] = req.data.decode("utf-8")
            return R()

        lead, trail = app._breaks()
        with mock.patch.object(app, "AZURE_SPEECH_KEY", "test-key"), \
             mock.patch.object(app, "AZURE_SPEECH_REGION", "testregion"), \
             mock.patch.object(app.urllib.request, "urlopen", fake_urlopen):
            app.synthesize_to_cache("moja", "normal", os.path.join(_TMP, "sw.mp3"), "sw")
        self.assertIn(lead, posted["body"])
        self.assertIn(trail, posted["body"])


class NothingIsRemoved(unittest.TestCase):
    def test_cached_bytes_are_the_providers_bytes(self):
        class Client:
            def synthesize_speech(self, **kwargs):
                return FakeResponse()

        out = os.path.join(_TMP, "clip.mp3")
        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("half", "normal", out, "en")
        self.assertEqual(Path(out).read_bytes(), PAYLOAD, "the pipeline altered the provider's audio")


class Formats(unittest.TestCase):
    def test_azure_clears_the_bitrate_floor(self):
        src = Path(app.__file__).with_name("app.py").read_text(encoding="utf-8")
        line = [l for l in src.splitlines() if "X-Microsoft-OutputFormat" in l]
        self.assertTrue(line, "the Azure output format line moved")
        self.assertIn("96kbitrate", line[0], "D3 sets a 64 kbps floor; Swahili shipped at 48")
        self.assertIn("24khz", line[0])

    def test_google_holds_the_sample_rate_floor(self):
        cfg = app._audio_config(app.SPEAKING_RATE_NORMAL)
        self.assertGreaterEqual(cfg.sample_rate_hertz, 24000)


class CacheVersion(unittest.TestCase):
    def test_a_padded_clip_cannot_be_served_from_the_old_cache(self):
        self.assertNotEqual(app.CACHE_VERSION, "v3", "bump the version or v3 clips keep serving")
        name = Path(app.cache_path_for("half", "normal", "en")).name
        self.assertTrue(name.startswith(app.CACHE_VERSION + "_"), name)


if __name__ == "__main__":
    unittest.main()
