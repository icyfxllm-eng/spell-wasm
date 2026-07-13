"""Username filter regression tests (stdlib unittest -> no pytest needed):

    python3 -m unittest backend.test_usernames    # from repo root
    python3 -m unittest test_usernames             # from backend/

Covers the decision addendum (2026-07): 'negro' must stay blocked in free-text
usernames via the owner blocklist (the global/English seed) — independent of the
optional better_profanity library and of the client's locale — while remaining a
valid Spanish puzzle word elsewhere. 'leche' needs no free-text block.
"""
import unittest

import usernames as u


class NegroBlocked(unittest.TestCase):
    def test_negro_is_in_owner_blocklist(self):
        # Guaranteed by our own file, not merely by the third-party wordlist.
        self.assertIn("negro", u._BLOCKLIST)

    def test_negro_rejected_as_username(self):
        self.assertTrue(u.is_offensive("negro"))
        self.assertFalse(u.is_acceptable("Negro"))
        self.assertFalse(u.is_acceptable("negro123"))

    def test_leet_and_substring_dodges(self):
        self.assertTrue(u.is_offensive("n3gr0"))   # leetspeak fold
        self.assertTrue(u.is_offensive("elnegro"))  # >=4-char substring

    def test_locale_independent(self):
        # The filter normalizes server-side and takes no locale, so the verdict
        # is identical whatever the device locale — es-locale and en-locale users
        # are both blocked. Same input -> same result, deterministically.
        self.assertEqual(u.is_offensive("negro"), u.is_offensive("negro"))
        self.assertTrue(u.is_offensive("negro"))


class LegitimateWordsAllowed(unittest.TestCase):
    def test_leche_not_blocked_in_freetext(self):
        # Decision: 'leche' needs no free-text block.
        self.assertFalse(u.is_offensive("leche"))

    def test_spanish_words_ok_as_usernames(self):
        for name in ("casa", "manzana", "coolspeller", "elgato"):
            self.assertTrue(u.is_acceptable(name), name)


if __name__ == "__main__":
    unittest.main()
