"""Persian (fa) reaches /api/speak: Azure-routed, voice in the cache key, and the
ZWNJ its bank stores is accepted between letters. Run with the server venv from backend/:

    ~/spellgame-server/venv/bin/python -m unittest test_speak_fa.py
"""
import os
import re
import tempfile
import unittest

_TMP = tempfile.mkdtemp()
os.environ["AUDIO_CACHE_DIR"] = os.path.join(_TMP, "audio")
os.environ["SPEAK_METRICS_DIR"] = os.path.join(_TMP, "speak")
os.environ.setdefault("CLIMB_DB_PATH", os.path.join(_TMP, "climb.db"))
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")

import app as app_module  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
ZWNJ = "\u200c"


def fa_bank():
    words = []
    for tier in ("easy", "medium", "hard", "expert"):
        with open(os.path.join(ROOT, "assets", "words", "fa", f"{tier}.txt"), encoding="utf-8") as f:
            words += [w.strip() for w in f if w.strip() and not w.startswith("#")]
    return words


class PersianSpeak(unittest.TestCase):
    def test_fa_is_azure_routed_and_voice_keyed(self):
        self.assertEqual(app_module.AZURE_VOICES["fa"][0], "fa-IR")
        self.assertNotIn("fa", app_module.LANG_VOICES)
        self.assertIn("fa", app_module.VOICE_IN_KEY)

    def test_every_bank_word_validates_unchanged(self):
        words = fa_bank()
        self.assertGreater(len(words), 8000)
        self.assertGreater(sum(ZWNJ in w for w in words), 500, "the bank does carry ZWNJ words")
        for w in words:
            self.assertEqual(app_module.validate_word(w, allow_zwnj=True), w, repr(w))

    def test_zwnj_only_between_letters_and_only_for_fa(self):
        word = "\u06a9\u062a\u0627\u0628" + ZWNJ + "\u0647\u0627"
        self.assertIsNone(app_module.validate_word(word), "other languages still refuse it")
        for bad in (ZWNJ + "\u0622\u0628", "\u0622\u0628" + ZWNJ, "\u0622" + ZWNJ * 2 + "\u0628"):
            self.assertIsNone(app_module.validate_word(bad, allow_zwnj=True), repr(bad))

    def test_speak_routes_fa_with_its_zwnj(self):
        calls = []
        orig = app_module.synthesize_to_cache

        def fake(word, variant, path, lang="en", *args, **kw):
            calls.append((word, lang))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as f:
                f.write(b"ID3" + b"\0" * 64)

        app_module.synthesize_to_cache = fake
        try:
            word = "\u0645\u06cc" + ZWNJ + "\u0634\u0648\u062f"
            r = app_module.app.test_client().get("/api/speak", query_string={"word": word, "lang": "fa"})
            self.assertEqual(r.status_code, 200, r.data[:200])
            self.assertEqual(calls, [(word, "fa")])
        finally:
            app_module.synthesize_to_cache = orig


if __name__ == "__main__":
    unittest.main()
