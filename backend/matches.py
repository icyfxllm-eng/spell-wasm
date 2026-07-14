"""Async 1v1 "Spell Off" — HTTP routes (Flask blueprint).

A head-to-head between two friends across the world, played ASYNC (like the
Daily Challenge, but 1v1 with a SHARED SEED — no WebSockets, no real-time). One
player creates a match; the server generates a crypto-random seed and hands back
a short share code. Both players spell the SAME words on their own time — the
words are derived deterministically from that one seed by the client's engine
(see `build_words_from_seed` in src/daily.rs) — then submit their result. When
both sides are in, the server compares them and declares the winner.

ADULT-GATED BY DESIGN (COPPA): every route requires a logged-in account
(`auth.current_user()`). Accounts are the app's adult surface — there is NO
anonymous or Kid-Mode online play, NO stranger matchmaking, and NO chat. Play is
friend-code only: you can only face someone you've shared an 8-char code with.

The server OWNS the seed — that's the integrity foundation. For THIS SCAFFOLD the
winner is decided from the client-submitted score/correct/elapsed, which are
trusted as-is.

  TODO(anti-cheat, follow-up — NOT built here): make submissions authoritative
  by having the server RE-DERIVE the word list from `seed` (port the deterministic
  engine to Python, or verify against a signed client attestation), then validate
  each submitted answer AND its per-word timing server-side before scoring. Until
  that lands, `/result` trusts the client's numbers. The stored seed + per-side
  fields are the hook for it.
"""

import secrets
import time

from flask import Blueprint, jsonify, request

import auth
import db

bp = Blueprint("matches", __name__)

# Word languages a match may use. Mirrors the client's active locales; unknown
# codes are rejected rather than silently coerced (both players must agree).
SUPPORTED_LANGS = {
    "en", "es", "fr", "de", "pt", "it", "nl", "pl", "sv", "nb",
    "tr", "vi", "ko", "ja", "th", "fil", "zh",
}
VALID_TIERS = ("easy", "medium", "hard", "expert")

WORD_COUNT = 10                      # words per match (matches the Daily arc length)
MATCH_TTL = 7 * 24 * 3600            # a match code is joinable/playable for 7 days
CODE_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"  # no I/O/0/1 — unambiguous to type
CREATE_LIMIT_PER_HOUR = 30          # per-account match-creation rate limit


def _code() -> str:
    return "".join(secrets.choice(CODE_ALPHABET) for _ in range(8))


def _public_match(m, viewer_id=None):
    """Serialize a match row for a client. Includes the seed (both players need
    it to derive the same words) but never any user PII — only opaque ids.
    `you` tells the caller which side they are ('a'|'b'|None)."""
    you = None
    if viewer_id is not None:
        if m["player_a"] == viewer_id:
            you = "a"
        elif m["player_b"] == viewer_id:
            you = "b"
    return {
        "code": m["code"],
        "seed": m["seed"],
        "lang": m["lang"],
        "tier": m["tier"],
        "wordCount": m["word_count"],
        "status": m["status"],
        "you": you,
        "playerA": m["player_a"],
        "playerB": m["player_b"],
        "submittedA": m["correct_a"] is not None,
        "submittedB": m["correct_b"] is not None,
        "scoreA": m["score_a"], "correctA": m["correct_a"], "elapsedA": m["elapsed_a"],
        "scoreB": m["score_b"], "correctB": m["correct_b"], "elapsedB": m["elapsed_b"],
        "winner": m["winner"],
        "expiresAt": m["expires_at"],
    }


def _get_match(c, code):
    return c.execute("SELECT * FROM matches WHERE code=?", (code or "",)).fetchone()


def _require_user():
    """Return the logged-in user or None. All match routes are account-gated."""
    return auth.current_user()


@bp.route("/api/match", methods=["POST"])
def create_match():
    u = _require_user()
    if not u:
        return jsonify(error="Log in to challenge a friend."), 401
    if not auth.rate_limit(f"match-create:{u['id']}", CREATE_LIMIT_PER_HOUR, 3600):
        return jsonify(error="Too many matches. Please try again later."), 429

    data = request.get_json(silent=True) or {}
    lang = (data.get("lang") or "").strip().lower()
    tier = (data.get("tier") or "").strip().lower()
    if lang not in SUPPORTED_LANGS:
        return jsonify(error="Unsupported language."), 400
    if tier not in VALID_TIERS:
        return jsonify(error="Unsupported difficulty."), 400

    c = db.conn()
    t = time.time()
    # Crypto-random 64-bit seed as 16 hex chars; the client parses it to a u64
    # and feeds it to the deterministic word engine (build_words_from_seed).
    seed = secrets.token_hex(8)
    # Retry on the (astronomically unlikely) code collision.
    for _ in range(5):
        code = _code()
        try:
            c.execute(
                "INSERT INTO matches(code, seed, lang, tier, word_count, player_a, status, created_at, expires_at) "
                "VALUES(?,?,?,?,?,?,?,?,?)",
                (code, seed, lang, tier, WORD_COUNT, u["id"], "open", t, t + MATCH_TTL),
            )
            c.commit()
            break
        except Exception:
            continue
    else:
        return jsonify(error="Couldn't create a match. Please try again."), 500

    return jsonify(_public_match(_get_match(c, code), u["id"]))


@bp.route("/api/match/<code>/join", methods=["POST"])
def join_match(code):
    u = _require_user()
    if not u:
        return jsonify(error="Log in to join a match."), 401
    code = (code or "").strip().upper()
    c = db.conn()
    m = _get_match(c, code)
    if not m or m["expires_at"] < time.time():
        return jsonify(error="That match code isn't valid or has expired."), 404
    if m["player_a"] == u["id"]:
        return jsonify(error="You can't join your own match — share the code with a friend."), 400
    if m["player_b"] is not None:
        return jsonify(error="That match is already full."), 409
    if m["status"] != "open":
        return jsonify(error="That match isn't open to join."), 409

    c.execute(
        "UPDATE matches SET player_b=?, status='active' WHERE code=?",
        (u["id"], code),
    )
    c.commit()
    return jsonify(_public_match(_get_match(c, code), u["id"]))


def _side_for(m, user_id):
    if m["player_a"] == user_id:
        return "a"
    if m["player_b"] == user_id:
        return "b"
    return None


@bp.route("/api/match/<code>/result", methods=["POST"])
def submit_result(code):
    u = _require_user()
    if not u:
        return jsonify(error="Log in to submit your result."), 401
    code = (code or "").strip().upper()
    c = db.conn()
    m = _get_match(c, code)
    if not m or m["expires_at"] < time.time():
        return jsonify(error="That match code isn't valid or has expired."), 404
    side = _side_for(m, u["id"])
    if side is None:
        return jsonify(error="You're not a player in that match."), 403

    data = request.get_json(silent=True) or {}

    def _int(v):
        return v if isinstance(v, int) and not isinstance(v, bool) and v >= 0 else None

    score = _int(data.get("score"))
    correct = _int(data.get("correct"))
    elapsed = _int(data.get("elapsed_ms"))
    total = _int(data.get("total"))
    if correct is None or score is None or elapsed is None:
        return jsonify(error="Invalid result."), 400
    if total is not None and correct > total:
        return jsonify(error="Invalid result."), 400
    if correct > m["word_count"]:
        return jsonify(error="Invalid result."), 400
    # NOTE: these are the client's own numbers. See the anti-cheat TODO in the
    # module docstring — a follow-up re-derives the words from `seed` and
    # validates each answer + timing server-side before trusting them.

    if m[f"correct_{side}"] is not None:
        return jsonify(error="You've already submitted your result."), 409

    c.execute(
        f"UPDATE matches SET score_{side}=?, correct_{side}=?, elapsed_{side}=? WHERE code=?",
        (score, correct, elapsed, code),
    )
    c.commit()

    m = _get_match(c, code)
    # Both sides in? Decide the winner: more correct wins; tiebreak faster time.
    if m["correct_a"] is not None and m["correct_b"] is not None:
        winner = _decide_winner(m)
        c.execute("UPDATE matches SET status='complete', winner=? WHERE code=?", (winner, code))
        c.commit()
        m = _get_match(c, code)

    return jsonify(_public_match(m, u["id"]))


def _decide_winner(m):
    """More correct wins; tie on correct broken by lower elapsed; else 'tie'."""
    if m["correct_a"] > m["correct_b"]:
        return "a"
    if m["correct_b"] > m["correct_a"]:
        return "b"
    # Equal correct — faster (lower elapsed) player wins.
    if m["elapsed_a"] < m["elapsed_b"]:
        return "a"
    if m["elapsed_b"] < m["elapsed_a"]:
        return "b"
    return "tie"


@bp.route("/api/match/<code>", methods=["GET"])
def get_match(code):
    u = _require_user()
    if not u:
        return jsonify(error="Log in to view this match."), 401
    code = (code or "").strip().upper()
    c = db.conn()
    m = _get_match(c, code)
    if not m:
        return jsonify(error="That match code isn't valid."), 404
    # Only the two players may read a match's state (friend-code privacy).
    if _side_for(m, u["id"]) is None:
        return jsonify(error="You're not a player in that match."), 403
    return jsonify(_public_match(m, u["id"]))
