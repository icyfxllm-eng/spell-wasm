"""Regional entitlement grants — CC-ENTITLEMENTS Feature 4 (the web half).

A child in a language's home country never pays for that language. On the web we
learn the country from Cloudflare's `CF-IPCountry` request header — spellgame.net
already sits behind Cloudflare, the header is free, and it asks the user for
NOTHING (no location permission, no prompt).

The country -> language map is the SINGLE SOURCE OF TRUTH shared with the Rust
core (which bundles the same file at build time). It is never copied: the file is
bind-mounted read-only into this container by docker-compose, so a map edit needs
only a restart and can never diverge from what the app ships.

Deliberately NOT done (Decision 4 / non-goals): no IP-reputation service, no
fingerprinting, no tracking, no logging of who asked from where. VPN leakage is
accepted, not "fixed" — a handful of free grants is a far smaller cost than
surveilling children.

Route:
  GET /api/entitlements/regional -> {"country": "KR", "grants": ["ko"]}
"""

import json
import os
from pathlib import Path

from flask import Blueprint, jsonify, request

bp = Blueprint("entitlements", __name__)

# Cloudflare sends these when it can't attribute a country ("XX") or the request
# came over Tor ("T1"). Either way: no grant. There is no locale fallback — a
# device locale is trivially spoofed and would hand out the paid catalogue.
_NO_COUNTRY = {"", "XX", "T1"}

# Path resolution only (never a DATA fallback): explicit env override, else the
# container mount, else the repo checkout for local dev/tests.
_CANDIDATES = [
    os.environ.get("COUNTRY_LANGUAGE_MAP"),
    "/app/config/country-language-map.json",
    str(Path(__file__).resolve().parent.parent / "config" / "country-language-map.json"),
]

_MAP = None


def country_map():
    """The country->languages map, loaded once.

    A missing map is a HARD ERROR, never an empty map: silently granting nothing
    would quietly break the home-country promise for every player, and we'd only
    hear about it as refund requests.
    """
    global _MAP
    if _MAP is None:
        for candidate in _CANDIDATES:
            if candidate and Path(candidate).exists():
                _MAP = json.loads(Path(candidate).read_text(encoding="utf-8"))
                return _MAP
        raise RuntimeError(
            "country-language map not found; looked in: "
            + ", ".join(c for c in _CANDIDATES if c)
        )
    return _MAP


def grants_for_country(country):
    """Languages granted free in `country` (alpha-2). Unknown/absent -> []."""
    code = (country or "").strip().upper()
    if code in _NO_COUNTRY:
        return []
    return list(country_map().get(code, []))


@bp.route("/api/entitlements/regional")
def regional():
    """Report which languages are free here, per Cloudflare's country header.

    Unauthenticated and uncacheable-by-country on purpose: the answer depends
    entirely on the caller's edge country, and it reveals nothing about them.
    """
    country = (request.headers.get("CF-IPCountry") or "").strip().upper()
    grants = grants_for_country(country)
    return jsonify({"country": country if grants or country not in _NO_COUNTRY else None,
                    "grants": grants})
