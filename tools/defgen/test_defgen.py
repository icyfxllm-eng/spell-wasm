"""CC-LAUNCH-EN D-SRC4 tests. Offline: no API call.

    tools/defgen/.venv/bin/python -m unittest tools/defgen/test_defgen.py
"""
import inspect
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import defgen  # noqa: E402

CANARY = "zxq-canary-bysa-7f3a: a domesticated feline kept as a household companion animal"


class CanaryA15_10(unittest.TestCase):
    """A15.10: a BY-SA row's text never reaches a prompt."""

    def test_canary_never_reaches_a_prompt(self):
        # A BY-SA-tagged row carrying the canary, as the server holds it.
        byasa_row = {"word": "cat", "pos": "noun", "definition": CANARY, "example": CANARY,
                     "source": "wiktionary", "source_license": "CC BY-SA 4.0"}
        pos, sense, _ = defgen.sense_for(byasa_row["word"])
        prompt = defgen.build_prompt(byasa_row["word"], pos, sense, "easy")
        self.assertNotIn("zxq-canary", prompt)
        self.assertEqual(defgen.canary_check([prompt], [CANARY]), [])
        # And the check bites when a prompt does carry it.
        self.assertEqual(defgen.canary_check([prompt + CANARY], [CANARY]), [CANARY])

    def test_the_prompt_builder_takes_only_the_four_inputs(self):
        self.assertEqual(list(inspect.signature(defgen.build_prompt).parameters),
                         ["headword", "pos", "sense", "tier"])


class Validator(unittest.TestCase):
    lists = (set(), {"slurword"})

    def check(self, field, text, word="garden", tier="easy"):
        return defgen.check_field(field, text, word, tier, self.lists)

    def test_good_fields_pass(self):
        self.assertEqual(self.check("definition", "A place where people grow flowers and vegetables."), [])
        self.assertEqual(self.check("sentence", "We planted beans in the garden today."), [])

    def test_definition_rules(self):
        self.assertIn("contains the headword or its stem", self.check("definition", "A small garden with flowers."))
        self.assertIn("contains the headword or its stem", self.check("definition", "Someone who is gardening outside.", "garden"))
        long = "A place " + "with many green plants " * 4 + "outside."
        self.assertTrue(any("max 15" in p for p in self.check("definition", long)))

    def test_sentence_rules(self):
        self.assertTrue(any("6-14" in p for p in self.check("sentence", "The garden grew.")))
        self.assertTrue(any("exactly 1" in p for p in self.check("sentence", "The garden next to my garden is big.")))
        self.assertTrue(any("exactly 1" in p for p in self.check("sentence", "My mum likes gardening in the warm sun.")))

    def test_reading_level_only_easy_and_medium(self):
        hard = "An extraordinarily sophisticated organizational methodology."
        self.assertTrue(any("grade 4" in p for p in self.check("definition", hard, "system", "easy")))
        self.assertFalse(any("grade 4" in p for p in self.check("definition", hard, "system", "expert")))

    def test_function_words_get_no_wordnet_anchor(self):
        for w in ("it", "us", "the", "of"):
            self.assertEqual(defgen.sense_for(w), ("function word", None, None), w)
        pos, sense, _ = defgen.sense_for("garden")
        self.assertEqual(pos, "noun")
        self.assertTrue(sense)

    def test_word_lists(self):
        self.assertTrue(any("blocklisted" in p for p in self.check("sentence", "They said a slurword in the garden today.")))
        # Prose matches whole words: a blocked term inside another word is fine.
        self.assertFalse(any("blocklisted" in p for p in defgen.check_field(
            "sentence", "We ate all the grapes in the garden bowl.", "garden", "easy", (set(), {"rape"}))))
        lists = ({"kill"}, set())
        self.assertTrue(any("kid-excluded" in p for p in defgen.check_field("sentence", "The cat will kill a mouse in the garden.", "garden", "easy", lists)))


class MeaningSwap(unittest.TestCase):
    """Eric 2026-11-09: Easy/Medium may swap to the child's common meaning; the swap is logged."""

    def st(self, tier, swapped):
        out = {"meaning_used": "a mix of things dissolved in water" if not swapped else "an answer to a problem",
               "swapped": swapped,
               "candidates": [{"definition": "The answer that fixes a problem or puzzle.",
                               "sentence": "We found a solution to the tricky maze puzzle."}] * 2}
        return {"rows": [{"word": "solution", "tier": tier}],
                "gen": {"g0": {"stop": "end_turn", "text": json.dumps(out)}}}

    def test_swap_kept_and_logged_on_easy(self):
        st = self.st("easy", True)
        self.assertEqual(len(defgen.validate(st)), 4)
        self.assertTrue(st["rows"][0]["swapped"])
        self.assertEqual(st["rows"][0]["meaning_used"], "an answer to a problem")

    def test_swap_outside_easy_medium_goes_dark(self):
        st = self.st("expert", True)
        self.assertEqual(defgen.validate(st), [])

    def test_schema_requires_the_swap_fields(self):
        self.assertEqual(set(defgen.GEN_SCHEMA["required"]), {"meaning_used", "swapped", "candidates"})
        self.assertIn("swapped", defgen.build_prompt("solution", "noun", "a mixture", "easy"))


if __name__ == "__main__":
    unittest.main()
