"""Regional entitlement grant tests (stdlib unittest -> no pytest):

    python3 -m unittest backend.test_entitlements   # from repo root
    python3 -m unittest test_entitlements            # from backend/

Covers CC-ENTITLEMENTS Feature 4 / acceptance 3: a spoofed `CF-IPCountry`
yields exactly the mapped grants, a missing header yields none, and Cloudflare's
unknown/Tor codes yield none. Reads the REAL shipped map (the same file the Rust
core bundles) so a map edit that breaks the home-country promise fails here.
"""
import os
import unittest
from pathlib import Path

# Point the module at the repo's real map before import (path is read lazily,
# but be explicit so the test never depends on cwd).
_MAP = Path(__file__).resolve().parent.parent / "config" / "country-language-map.json"
os.environ["COUNTRY_LANGUAGE_MAP"] = str(_MAP)

import entitlements  # noqa: E402

from flask import Flask  # noqa: E402


def _app():
    app = Flask(__name__)
    app.register_blueprint(entitlements.bp)
    return app.test_client()


class RegionalGrantTests(unittest.TestCase):
    def setUp(self):
        entitlements._MAP = None  # force a fresh load per test
        self.c = _app()

    def get(self, country=None):
        headers = {"CF-IPCountry": country} if country is not None else {}
        return self.c.get("/api/entitlements/regional", headers=headers).get_json()

    def test_single_language_country(self):
        self.assertEqual(self.get("KR")["grants"], ["ko"])
        self.assertEqual(self.get("JP")["grants"], ["ja"])
        self.assertEqual(self.get("VN")["grants"], ["vi"])

    def test_multi_language_country_grants_all_shipped_official(self):
        # Switzerland ships three of its official languages.
        self.assertEqual(sorted(self.get("CH")["grants"]), ["de", "fr", "it"])
        self.assertEqual(sorted(self.get("BE")["grants"]), ["fr", "nl"])

    def test_spanish_across_latin_america(self):
        for country in ("ES", "MX", "AR", "CO", "PE"):
            self.assertEqual(self.get(country)["grants"], ["es"], country)

    def test_lowercase_header_is_normalized(self):
        self.assertEqual(self.get("kr")["grants"], ["ko"])

    def test_missing_header_grants_nothing(self):
        r = self.get(None)
        self.assertEqual(r["grants"], [])
        self.assertIsNone(r["country"])

    def test_unknown_and_tor_grant_nothing(self):
        # Cloudflare sends XX when it can't attribute, T1 for Tor. No locale
        # fallback anywhere — an unattributed request simply gets no grant.
        for code in ("XX", "T1", ""):
            self.assertEqual(self.get(code)["grants"], [], code)

    def test_country_not_in_map_grants_nothing(self):
        self.assertEqual(self.get("ZZ")["grants"], [])

    def test_cut_languages_are_not_granted_anywhere(self):
        # Thai + Turkish were cut from the game; no country may grant them.
        for langs in entitlements.country_map().values():
            self.assertNotIn("th", langs)
            self.assertNotIn("tr", langs)

    def test_missing_map_is_a_hard_error_not_empty_grants(self):
        entitlements._MAP = None
        saved = entitlements._CANDIDATES[:]
        try:
            entitlements._CANDIDATES[:] = ["/nonexistent/country-language-map.json"]
            with self.assertRaises(RuntimeError):
                entitlements.country_map()
        finally:
            entitlements._CANDIDATES[:] = saved
            entitlements._MAP = None


if __name__ == "__main__":
    unittest.main()
