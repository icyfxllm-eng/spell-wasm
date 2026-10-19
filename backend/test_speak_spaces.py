"""CC-IMPOSSIBLE-LAUNCH Feature 8 -- /api/speak accepts a single space inside a Vietnamese word, and
nothing else about validation moves. Run with the server venv from backend/:

    ~/spellgame-server/venv/bin/python -m unittest test_speak_spaces.py
"""
import json
import os
import re
import tempfile
import unicodedata
import unittest
import urllib.parse

_TMP = tempfile.mkdtemp()
os.environ["AUDIO_CACHE_DIR"] = os.path.join(_TMP, "audio")
os.environ["SPEAK_METRICS_DIR"] = os.path.join(_TMP, "speak")
os.environ.setdefault("CLIMB_DB_PATH", os.path.join(_TMP, "climb.db"))
os.environ.setdefault("GOOGLE_TTS_API_KEY", "test-not-a-key")

import app as app_module  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def bank(lang):
    txt = open(os.path.join(ROOT, "src", "word_data.rs"), encoding="utf-8").read()
    out = {}
    for tier in ("EASY", "MEDIUM", "HARD", "EXPERT"):
        m = re.search(rf"pub const {lang.upper()}_{tier}: &\[&str\] = &\[(.*?)\];", txt, re.S)
        out[tier.lower()] = json.loads("[" + m.group(1) + "]")
    return out


class Validate(unittest.TestCase):
    def test_vi_inner_single_space_is_kept(self):
        self.assertEqual(app_module.validate_word("rượt đuổi", allow_inner_space=True), "rượt đuổi")

    def test_double_space_is_refused(self):
        self.assertIsNone(app_module.validate_word("rượt  đuổi", allow_inner_space=True))

    def test_leading_and_trailing_spaces_are_trimmed_as_always(self):
        # Existing behaviour for every language, unchanged: trimmed, not refused.
        self.assertEqual(app_module.validate_word("  rượt đuổi  ", allow_inner_space=True), "rượt đuổi")
        self.assertEqual(app_module.validate_word(" cat "), "cat")

    def test_other_whitespace_is_still_refused(self):
        for bad in ("rượt\tđuổi", "rượt đuổi", "rượt\nđuổi", "rượt　đuổi"):
            self.assertIsNone(app_module.validate_word(bad, allow_inner_space=True), repr(bad))

    def test_default_is_unchanged_for_every_other_language(self):
        self.assertIsNone(app_module.validate_word("two words"))
        self.assertEqual(app_module.validate_word("abc-d'e"), "abc-d'e")
        self.assertIsNone(app_module.validate_word("a1"))

    def test_nfc_for_the_spaced_path(self):
        decomposed = unicodedata.normalize("NFD", "rượt đuổi")
        self.assertNotEqual(decomposed, "rượt đuổi")
        self.assertEqual(app_module.validate_word(decomposed, allow_inner_space=True), "rượt đuổi")

    def test_only_vietnamese_may_have_spaces(self):
        self.assertEqual(app_module.SPACED_LANGS, frozenset({"vi"}))

    def test_every_vietnamese_bank_row_validates(self):
        rows = [w for ws in bank("vi").values() for w in ws]
        self.assertGreater(len(rows), 4000)
        bad = [w for w in rows if app_module.validate_word(w, allow_inner_space=True) is None]
        self.assertEqual(bad, [])
        # AT-8.1: all 2,500 Expert rows.
        self.assertEqual(len(bank("vi")["expert"]), 2500)

    def test_no_other_language_needs_a_space(self):
        # zh lives outside word_data.rs: its entries are pinyin readings and hanzi, with no spaces.
        for lang in ("en", "es", "fr", "de", "pt", "pl", "ko", "ja", "fil", "ru", "ar", "sw", "hi"):
            for tier, ws in bank(lang).items():
                self.assertEqual([w for w in ws if " " in w], [], f"{lang}/{tier}")


class Route(unittest.TestCase):
    def setUp(self):
        self.client = app_module.app.test_client()
        self.words = []

        def fake_synth(word, variant, path, lang="en", py=None, **_kw):
            self.words.append((lang, word))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as fh:
                fh.write(b"ID3")

        self._orig = app_module.synthesize_to_cache
        app_module.synthesize_to_cache = fake_synth

    def tearDown(self):
        app_module.synthesize_to_cache = self._orig

    def get(self, word, lang):
        r = self.client.get(f"/api/speak?word={urllib.parse.quote(word)}&lang={lang}")
        code = r.status_code
        r.close()
        return code

    def test_vi_spaced_word_gets_audio_and_the_voice_hears_the_space(self):
        self.assertEqual(self.get("rượt đuổi", "vi"), 200)
        self.assertEqual(self.words, [("vi", "rượt đuổi")])

    def test_double_space_and_other_languages_stay_refused(self):
        self.assertEqual(self.get("rượt  đuổi", "vi"), 400)
        self.assertEqual(self.get("buenos dias", "es"), 400)
        self.assertEqual(self.get("hello world", "en"), 400)
        self.assertEqual(self.get("hello world", "xx"), 400)  # an unknown lang falls back to en
        self.assertEqual(self.words, [])

    def test_a_b_and_ab_are_different_cache_entries(self):
        spaced = app_module.cache_path_for("rượt đuổi", "normal", "vi")
        joined = app_module.cache_path_for("rượtđuổi", "normal", "vi")
        self.assertNotEqual(spaced, joined)
        self.assertNotEqual(app_module.cache_path_for("a b", "normal", "vi"), app_module.cache_path_for("ab", "normal", "vi"))

    def test_existing_vietnamese_single_syllable_words_are_unaffected(self):
        self.assertEqual(self.get("nhà", "vi"), 200)


if __name__ == "__main__":
    unittest.main()
