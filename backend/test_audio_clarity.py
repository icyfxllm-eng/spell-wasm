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


class PronunciationOverrides(unittest.TestCase):
    """CC-AUDIO-CLARITY F3. The mechanism, not the entries: a row is a native
    speaker's judgement and none ship."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        app.LEXICON_DIR = self.dir
        app._LEXICON.clear()

    def write(self, lang, rows):
        with open(os.path.join(self.dir, f"{lang}.tsv"), "w", encoding="utf-8") as f:
            f.write("word\tipa\tprovider\tauditor\tsigned_at\n")
            for r in rows:
                f.write("\t".join(r) + "\n")

    def test_a_signed_row_loads(self):
        self.write("en", [("half", "hæf", "google", "A. Speaker", "2026-09-23")])
        e = app.lexicon_entry("en", "half")
        self.assertIsNotNone(e)
        self.assertEqual(e[0], "hæf")

    def test_an_unsigned_row_does_not_load(self):
        """F3 step 2: no auditor, no entry. A guess about how a language sounds
        is what this file exists to prevent."""
        self.write("en", [("half", "hæf", "google", "", "")])
        self.assertIsNone(app.lexicon_entry("en", "half"))

    def test_no_rows_ship(self):
        """The real lexicon carries a header and nothing else until an auditor
        signs something."""
        app.LEXICON_DIR = os.path.join(os.path.dirname(os.path.abspath(app.__file__)), "..", "audio", "lexicon")
        app._LEXICON.clear()
        self.assertEqual(app._load_lexicon("en"), {})

    def test_an_override_changes_only_its_own_clip(self):
        """F3 step 3 / I8: editing one row regenerates exactly that clip."""
        self.write("en", [("half", "hæf", "google", "A. Speaker", "2026-09-23")])
        app._LEXICON.clear()
        with_override = app.cache_path_for("half", "normal", "en")
        other = app.cache_path_for("thief", "normal", "en")
        self.write("en", [("half", "hɑːf", "google", "A. Speaker", "2026-09-23")])
        app._LEXICON.clear()
        self.assertNotEqual(with_override, app.cache_path_for("half", "normal", "en"), "the edited word regenerates")
        self.assertEqual(other, app.cache_path_for("thief", "normal", "en"), "every other word stays cached")

    def test_the_override_reaches_the_provider_as_ipa(self):
        self.write("en", [("half", "hæf", "google", "A. Speaker", "2026-09-23")])
        app._LEXICON.clear()
        sent = {}

        class Client:
            def synthesize_speech(self, input=None, voice=None, audio_config=None):
                sent["ssml"] = getattr(input, "ssml", "") or ""
                return FakeResponse()

        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("half", "normal", os.path.join(self.dir, "o.mp3"), "en")
        self.assertIn('alphabet="ipa"', sent["ssml"])
        self.assertIn("hæf", sent["ssml"])
        # And it is still padded: an override must not cost the clip its edges.
        lead, trail = app._breaks()
        self.assertIn(lead, sent["ssml"])
        self.assertIn(trail, sent["ssml"])

    def test_a_word_without_a_row_is_untouched(self):
        self.write("en", [("half", "hæf", "google", "A. Speaker", "2026-09-23")])
        app._LEXICON.clear()
        sent = {}

        class Client:
            def synthesize_speech(self, input=None, voice=None, audio_config=None):
                sent["ssml"] = getattr(input, "ssml", "") or ""
                return FakeResponse()

        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("thief", "normal", os.path.join(self.dir, "t.mp3"), "en")
        self.assertNotIn("phoneme", sent["ssml"])


class BakeOffIsolation(unittest.TestCase):
    """CC-AUDIO-CLARITY F4. A bake-off must never reach a player: its clips are
    a different voice, and if they landed in the ordinary cache every listener
    would quietly get the candidate."""

    def test_a_candidate_clip_is_cached_somewhere_else(self):
        normal = app.cache_path_for("half", "normal", "en")
        cand = app.cache_path_for("half", "normal", "en", "bakeoff:en-US-Neural2-F:en:half")
        self.assertNotEqual(normal, cand)

    def test_two_candidates_do_not_share_a_clip(self):
        a = app.cache_path_for("half", "normal", "en", "bakeoff:en-US-Neural2-F:en:half")
        b = app.cache_path_for("half", "normal", "en", "bakeoff:en-US-Studio-O:en:half")
        self.assertNotEqual(a, b, "each candidate is judged on its own audio")

    def test_the_override_names_its_own_language(self):
        """en-US-Neural2-F implies en-US: a candidate cannot be handed to the
        wrong language's synthesis by accident."""
        sent = {}

        class Client:
            def synthesize_speech(self, input=None, voice=None, audio_config=None):
                sent["name"] = voice.name
                sent["code"] = voice.language_code
                return FakeResponse()

        with mock.patch.object(app, "tts_client", Client()):
            app.synthesize_to_cache("half", "normal", os.path.join(_TMP, "b.mp3"), "en",
                                    voice_override="en-US-Neural2-F")
        self.assertEqual(sent["name"], "en-US-Neural2-F")
        self.assertEqual(sent["code"], "en-US")

    def test_production_ignores_the_parameter(self):
        """The guard is an env flag, off by default, so a stray ?voice= on the
        live server changes nothing."""
        src = Path(app.__file__).with_name("app.py").read_text(encoding="utf-8")
        self.assertIn('os.environ.get("BAKEOFF") == "1"', src)
        i = src.index('os.environ.get("BAKEOFF")')
        j = src.index('request.args.get("voice"')
        self.assertLess(i, j, "the voice parameter is only read inside the guard")
