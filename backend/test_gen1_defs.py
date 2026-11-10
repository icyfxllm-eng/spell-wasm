"""CC-LAUNCH-EN rev 4 -- Spell's own English definitions are served only on src=gen1.

    ~/spellgame-server/venv/bin/python -m unittest backend/test_gen1_defs.py
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

import app as app_module  # noqa: E402


class Gen1(unittest.TestCase):
    def setUp(self):
        self.c = app_module.app.test_client()

    def test_meaning_gen1_is_reviewed_and_tagged(self):
        r = self.c.get("/api/meaning?word=moon&lang=en&src=gen1&mask=0")
        self.assertEqual(r.status_code, 200)
        j = r.get_json()
        self.assertTrue(j["reviewed_definition"] and j["reviewed_sentence"])
        self.assertEqual((j["source"], j["source_license"]), ("spell-gen-gen-v2", "original"))
        self.assertIn("sky", j["definition"])

    def test_meaning_gen1_masks_the_word(self):
        j = self.c.get("/api/meaning?word=moon&lang=en&src=gen1").get_json()
        self.assertNotIn("moon", j["example"].lower())

    def test_unknown_word_is_404_not_third_party_text(self):
        r = self.c.get("/api/meaning?word=zzqx&lang=en&src=gen1")
        self.assertEqual(r.status_code, 404)

    def test_unreviewed_field_is_blank(self):
        j = self.c.get("/api/meaning?word=artillery&lang=en&src=gen1&mask=0").get_json()
        self.assertFalse(j["reviewed_definition"])
        self.assertEqual(j["definition"], "")

    def test_defpool_gen1_rows_are_all_reviewed_originals(self):
        j = self.c.get("/api/defpool?lang=en&tier=easy&src=gen1").get_json()
        self.assertGreater(len(j["rows"]), 40)
        for row in j["rows"]:
            self.assertTrue(row["reviewed_definition"])
            self.assertEqual(row["source_license"], "original")

    def test_no_src_keeps_todays_pool(self):
        j = self.c.get("/api/defpool?lang=en&tier=easy").get_json()
        self.assertFalse(any("reviewed_definition" in r for r in j["rows"]))

    def test_other_languages_ignore_src(self):
        a = self.c.get("/api/defpool?lang=es&tier=easy&src=gen1").get_json()
        b = self.c.get("/api/defpool?lang=es&tier=easy").get_json()
        self.assertEqual(a, b)

    def test_files_never_carry_unreviewed_text(self):
        m = json.load(open(app_module.GEN_MEANINGS_PATH, encoding="utf-8"))
        for w, row in m.items():
            if not row["reviewed_definition"]:
                self.assertEqual(row["definition"], "", w)
            if not row["reviewed_sentence"]:
                self.assertEqual(row["example"], "", w)


if __name__ == "__main__":
    unittest.main()
