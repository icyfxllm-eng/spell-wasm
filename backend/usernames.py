"""Username validation for The Climb — enforced SERVER-SIDE.

Rules: 3–16 chars, letters/numbers/underscores only, unique case-insensitively
(uniqueness is enforced at the DB via users.username_lc). Offensiveness is
checked AFTER normalization — lowercase, fold common leetspeak, strip anything
non-alphabetic — so "b4dw0rd" / "b_a_d" style variants are caught.

Two layers of screening:
  1. A maintained library (better_profanity) for general profanity.
  2. A local, owner-appendable blocklist file (blocklist.txt) for slurs, sexual
     and hate terms, plus impersonation terms (admin/moderator/spellgame/...).

Rejections are surfaced to the user with a single generic message ("That
username isn't available") — callers must NOT reveal which rule failed.
"""

import os
import re

try:
    from better_profanity import profanity as _bp
    _bp.load_censor_words()
    _HAVE_BP = True
except Exception:  # library missing / load failure — fall back to blocklist only
    _HAVE_BP = False

_VALID = re.compile(r"^[A-Za-z0-9_]{3,16}$")

# Fold digits/symbols commonly substituted for letters before matching.
_LEET = str.maketrans({
    "0": "o", "1": "i", "!": "i", "|": "i", "3": "e",
    "4": "a", "@": "a", "5": "s", "$": "s", "7": "t",
})

# Impersonation / authority terms that must never be usernames.
_IMPERSONATION = {
    "admin", "administrator", "moderator", "mod", "staff", "official",
    "support", "root", "system", "spellgame", "spell", "theclimb",
}


def _load_blocklist() -> set:
    path = os.path.join(os.path.dirname(__file__), "blocklist.txt")
    words = set()
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                w = line.strip().lower()
                if w and not w.startswith("#"):
                    words.add(w)
    except FileNotFoundError:
        pass
    return words


# Slurs / profanity from the file are substring-matched (so "shitlord" is
# caught); impersonation terms are matched EXACTLY after normalization (so
# "admin123" -> "admin" is caught, but a legit "coolspeller" isn't blocked
# merely for containing "spell").
_BLOCKLIST = _load_blocklist()


def normalize(name: str) -> str:
    return re.sub(r"[^a-z]", "", name.lower().translate(_LEET))


def is_offensive(name: str) -> bool:
    norm = normalize(name)
    if not norm:
        return False
    # Impersonation: exact match against BOTH the leet-folded form ("adm1n" ->
    # "admin") and a plain digits/symbols-stripped form ("admin123" -> "admin"),
    # so trailing-digit and letter-substitution dodges are both caught without
    # substring-blocking innocent names like "coolspeller" or "modern".
    plain = re.sub(r"[^a-z]", "", name.lower())
    if norm in _IMPERSONATION or plain in _IMPERSONATION:
        return True
    if norm in _BLOCKLIST or any(bad in norm for bad in _BLOCKLIST if len(bad) >= 4):
        return True
    if _HAVE_BP and _bp.contains_profanity(norm):
        return True
    return False


def structurally_valid(name: str) -> bool:
    return bool(_VALID.match(name))


def is_acceptable(name: str) -> bool:
    """Structurally valid AND not offensive. (DB enforces uniqueness.)"""
    return structurally_valid(name) and not is_offensive(name)
