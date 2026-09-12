"""The Climb — HTTP routes (Flask blueprint).

Account auth — signup by emailed code, login, logout, refresh, me.
Leaderboard submit/read, password recovery by emailed code, change-username and
account deletion. CC-ONBOARD-JR F8/F10 replaced the emailed LINKS with six-digit
codes, and F12 deleted phone signup and SMS recovery outright. All input is validated; signup/login are
rate-limited and Turnstile-gated (skipped when unconfigured); errors are generic
to avoid revealing which rule failed or whether an account exists.
"""

import json
import secrets
import time

from flask import Blueprint, jsonify, make_response, request

import auth
import db
import usernames

bp = Blueprint("climb", __name__)

VALID_DIFFICULTIES = ("medium", "hard", "expert")  # no 'easy' board, ever

# Anti-cheat (minimum viable). Client-submitted scores are only trustable so
# far — these are sanity gates, not proof. FUTURE: server-side word
# verification (server picks/verifies the words for a ranked run) would make
# submissions authoritative; the run_meta stored per entry is the hook for it.
MAX_CHAIN = 500                 # implausibly long streak ceiling
MIN_MS_PER_WORD = 800           # hear+spell floor; a 40-chain in 20s is rejected
SUBMIT_LIMIT_PER_HOUR = 40      # per-account submission rate limit


def _public_user(u):
    """Only ever expose non-sensitive fields — never email/phone."""
    return {
        "id": u["id"],
        "username": u["username"],
        "displayName": u["display_name"],
        "emailVerified": bool(u["email_verified"]),
    }


def _auth_response(user_row, token, extra=None):
    body = {"user": _public_user(user_row), "token": token}
    if extra:
        body.update(extra)
    resp = make_response(jsonify(body))
    return auth.set_session_cookie(resp, token)


# ---------- signup and recovery by 6-digit code (CC-ONBOARD-JR F8/F10, D4) ----------

def _send_code(email: str, email_lc: str, purpose: str) -> None:
    code = auth.issue_code(email_lc, purpose)
    auth.note_code_send(email_lc, purpose)
    if purpose == "signup":
        subject = "Your Spell code"
        html = (f"<p>Your code to finish setting up Spell is <b>{code}</b>.</p>"
                f"<p>It expires in ten minutes. If you did not ask for it, ignore this email.</p>")
    else:
        subject = "Your Spell password reset code"
        html = (f"<p>Your Spell password reset code is <b>{code}</b>.</p>"
                f"<p>It expires in ten minutes. If you did not ask for it, ignore this email.</p>")
    auth.send_email(email, subject, html)


@bp.route("/api/auth/request-code", methods=["POST"])
def request_code():
    """Send a six-digit code for signup or for a password reset.

    The response is IDENTICAL in every case: valid address or not, registered or
    not, first request or sixth. F10 requires that for reset, because a
    different answer is an account-enumeration oracle; signup gets the same
    treatment, or the oracle simply moves to the other endpoint.
    """
    if not auth.rate_limit(f"code:{auth.client_ip()}", 20, 3600):
        return jsonify(error="Too many attempts. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    email = (data.get("email") or "").strip()
    purpose = (data.get("purpose") or "").strip()
    generic = jsonify(ok=True, message="If that address can receive it, we have sent a code.")
    if purpose not in ("signup", "reset") or not auth.valid_email(email):
        return generic
    email_lc = email.lower()
    if auth.code_send_problem(email_lc, purpose):
        return generic
    c = db.conn()
    exists = c.execute("SELECT 1 FROM users WHERE email_lc=?", (email_lc,)).fetchone()
    if purpose == "signup" and exists:
        # Tell the OWNER of the address, not whoever typed it in.
        auth.note_code_send(email_lc, purpose)
        auth.send_email(
            email,
            "You already have a Spell account",
            "<p>Someone tried to create a Spell account with this address. You already "
            "have one — use Forgot password if you need to get back in.</p>",
        )
        return generic
    if purpose == "reset" and not exists:
        return generic
    _send_code(email, email_lc, purpose)
    return generic


@bp.route("/api/auth/verify-code", methods=["POST"])
def verify_code():
    """Check a code WITHOUT spending it, so the app can move on to the password
    step before an account exists. Completion is what consumes it."""
    data = request.get_json(silent=True) or {}
    email_lc = (data.get("email") or "").strip().lower()
    purpose = (data.get("purpose") or "").strip()
    if purpose not in ("signup", "reset"):
        return jsonify(error="That code is invalid or has expired."), 400
    problem = auth.check_code(email_lc, purpose, data.get("code") or "", consume=False)
    if problem:
        return jsonify(error=problem), 400
    return jsonify(ok=True)


@bp.route("/api/auth/complete-signup", methods=["POST"])
def complete_signup():
    """Create the account once the code is proved. The code IS the verification,
    so there is no link to click and no half-verified account to chase."""
    data = request.get_json(silent=True) or {}
    email = (data.get("email") or "").strip()
    email_lc = email.lower()
    username = (data.get("username") or "").strip()
    password = data.get("password") or ""
    display_name = (data.get("displayName") or "").strip() or None
    if not auth.verify_turnstile(data.get("turnstile")):
        return jsonify(error="Verification failed. Please try again."), 400
    if not auth.valid_email(email):
        return jsonify(error="Enter a valid email address.", field="email"), 400
    problem = auth.check_code(email_lc, "signup", data.get("code") or "", consume=False)
    if problem:
        return jsonify(error=problem, field="code"), 400
    if not usernames.is_acceptable(username):
        return jsonify(error="That username isn't available.", field="username"), 400
    pw_problem = auth.password_problem(password)
    if pw_problem:
        return jsonify(error=pw_problem, field="password"), 400

    c = db.conn()
    ulc = username.lower()
    taken = c.execute("SELECT 1 FROM users WHERE username_lc=?", (ulc,)).fetchone() or c.execute(
        "SELECT 1 FROM reserved_usernames WHERE username_lc=? AND reserved_until>?", (ulc, time.time())
    ).fetchone()
    if taken:
        return jsonify(error="That username isn't available.", field="username"), 400
    if c.execute("SELECT 1 FROM users WHERE email_lc=?", (email_lc,)).fetchone():
        return jsonify(error="That email can't be used.", field="email"), 400

    auth.check_code(email_lc, "signup", data.get("code") or "", consume=True)
    t = time.time()
    cur = c.execute(
        "INSERT INTO users(username,username_lc,display_name,email,email_lc,email_verified,pw_hash,created_at) "
        "VALUES(?,?,?,?,?,1,?,?)",
        (username, ulc, display_name, email, email_lc, auth.hash_password(password), t),
    )
    c.commit()
    token = auth.create_session(cur.lastrowid)
    return _auth_response(auth.session_user(token), token)


@bp.route("/api/auth/signup", methods=["POST"])
def signup_gone():
    """An older installed build still posts here. 410 with a plain instruction
    beats a 404: the player is told what to do instead of seeing a dead end."""
    return jsonify(error="Please update the app to create an account."), 410


@bp.route("/api/auth/login", methods=["POST"])
def login():
    if not auth.rate_limit(f"login:{auth.client_ip()}", 10, 900):
        return jsonify(error="Too many attempts. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    identifier = (data.get("identifier") or "").strip().lower()  # email OR username
    password = data.get("password") or ""
    if not auth.verify_turnstile(data.get("turnstile")):
        return jsonify(error="Verification failed. Please try again."), 400

    c = db.conn()
    row = c.execute(
        "SELECT * FROM users WHERE username_lc=? OR email_lc=?", (identifier, identifier)
    ).fetchone()
    if not row or not auth.verify_password(password, row["pw_hash"]):
        # Same message either way — no account enumeration.
        return jsonify(error="Incorrect login or password."), 401

    token = auth.create_session(row["id"])
    return _auth_response(auth.session_user(token), token)


@bp.route("/api/auth/logout", methods=["POST"])
def logout():
    tok = auth.request_token()
    if tok:
        auth.revoke_session(tok)
    resp = make_response(jsonify(ok=True))
    return auth.clear_session_cookie(resp)


@bp.route("/api/auth/refresh", methods=["POST"])
def refresh():
    u = auth.current_user()
    if not u:
        return jsonify(error="Not signed in."), 401
    tok = auth.request_token()
    auth.refresh_session(tok)  # roll the 90-day window
    return _auth_response(u, tok)


@bp.route("/api/auth/me")
def me():
    u = auth.current_user()
    return jsonify(user=_public_user(u) if u else None)


# ---------- leaderboard ("The Climb") ----------

# Leaderboards are segmented by the run's word language (§4.4). Unknown/absent
# codes fall back to English so pre-locale clients keep working.
SUPPORTED_LOCALES = {"en", "es", "fr", "de", "pt", "it", "nl", "pl", "sv", "nb", "tr"}


def _locale(raw) -> str:
    code = (raw or "en").strip().lower()
    return code if code in SUPPORTED_LOCALES else "en"


def _rank_for(c, difficulty, locale, user_id):
    """1-based rank of a player within a (difficulty, locale) board (ties broken
    by earliest achievement), or None if they have no entry there."""
    row = c.execute(
        "SELECT best_chain, achieved_at FROM leaderboard_entries WHERE user_id=? AND difficulty=? AND locale=?",
        (user_id, difficulty, locale),
    ).fetchone()
    if not row:
        return None
    better = c.execute(
        "SELECT COUNT(*) AS n FROM leaderboard_entries WHERE difficulty=? AND locale=? "
        "AND (best_chain > ? OR (best_chain = ? AND achieved_at < ?))",
        (difficulty, locale, row["best_chain"], row["best_chain"], row["achieved_at"]),
    ).fetchone()["n"]
    return better + 1


@bp.route("/api/climb/submit-chain", methods=["POST"])
def submit_chain():
    u = auth.current_user()
    if not u:
        return jsonify(error="Log in to post your chain to The Climb."), 401
    data = request.get_json(silent=True) or {}
    difficulty = (data.get("difficulty") or "").strip().lower()
    locale = _locale(data.get("locale"))
    chain = data.get("chain")
    meta = data.get("meta") or {}

    if difficulty not in VALID_DIFFICULTIES:
        return jsonify(error="That difficulty isn't ranked."), 400
    if not isinstance(chain, int) or isinstance(chain, bool) or chain < 1 or chain > MAX_CHAIN:
        return jsonify(error="Score rejected."), 400

    c = db.conn()
    t = time.time()

    # Per-account submission rate limit (server-side, DB-backed).
    recent = c.execute(
        "SELECT COUNT(*) AS n FROM submit_log WHERE user_id=? AND ts>?", (u["id"], t - 3600)
    ).fetchone()["n"]
    if recent >= SUBMIT_LIMIT_PER_HOUR:
        return jsonify(error="Too many submissions. Please try again later."), 429

    # Timing/shape sanity: must have spelled at least `chain` words, and a run
    # can't be implausibly fast (a 40-chain in 20s => 500ms/word => rejected).
    word_count = meta.get("wordCount")
    duration_ms = meta.get("durationMs")
    if isinstance(word_count, int) and word_count < chain:
        return jsonify(error="Score rejected."), 400
    if isinstance(duration_ms, (int, float)) and duration_ms < chain * MIN_MS_PER_WORD:
        return jsonify(error="Score rejected."), 400

    c.execute("INSERT INTO submit_log(user_id, ts) VALUES(?,?)", (u["id"], t))
    c.commit()

    prev = c.execute(
        "SELECT best_chain FROM leaderboard_entries WHERE user_id=? AND difficulty=? AND locale=?",
        (u["id"], difficulty, locale),
    ).fetchone()
    is_record = prev is None or chain > prev["best_chain"]
    if is_record:
        # Server-side timestamp; run_meta kept for future verification/audit.
        run_meta = json.dumps({"wordCount": word_count, "durationMs": duration_ms})
        c.execute(
            "INSERT INTO leaderboard_entries(user_id,difficulty,locale,best_chain,achieved_at,run_meta) "
            "VALUES(?,?,?,?,?,?) ON CONFLICT(user_id,difficulty,locale) DO UPDATE SET "
            "best_chain=excluded.best_chain, achieved_at=excluded.achieved_at, run_meta=excluded.run_meta",
            (u["id"], difficulty, locale, chain, t, run_meta),
        )
        c.commit()

    best = chain if is_record else prev["best_chain"]
    return jsonify(record=is_record, best=best, locale=locale, rank=_rank_for(c, difficulty, locale, u["id"]))


@bp.route("/api/climb/leaderboard")
def leaderboard():
    difficulty = (request.args.get("difficulty") or "").strip().lower()
    locale = _locale(request.args.get("locale"))
    if difficulty not in VALID_DIFFICULTIES:
        return jsonify(error="That difficulty isn't ranked."), 400
    c = db.conn()
    rows = c.execute(
        "SELECT u.id, u.username, e.best_chain, e.achieved_at "
        "FROM leaderboard_entries e JOIN users u ON u.id=e.user_id "
        "WHERE e.difficulty=? AND e.locale=? ORDER BY e.best_chain DESC, e.achieved_at ASC LIMIT 50",
        (difficulty, locale),
    ).fetchall()
    # Only rank, username, chain, date — never email/real name.
    top = [
        {"rank": i + 1, "userId": r["id"], "username": r["username"],
         "chain": r["best_chain"], "achievedAt": r["achieved_at"]}
        for i, r in enumerate(rows)
    ]

    me = None
    u = auth.current_user()
    if u:
        rank = _rank_for(c, difficulty, locale, u["id"])
        if rank and rank > 50:  # pin the player's own row when outside the top 50
            row = c.execute(
                "SELECT best_chain, achieved_at FROM leaderboard_entries WHERE user_id=? AND difficulty=? AND locale=?",
                (u["id"], difficulty, locale),
            ).fetchone()
            me = {"rank": rank, "userId": u["id"], "username": u["username"],
                  "chain": row["best_chain"], "achievedAt": row["achieved_at"]}
    return jsonify(difficulty=difficulty, locale=locale, top=top, me=me)


@bp.route("/api/climb/report-name", methods=["POST"])
def report_name():
    if not auth.rate_limit(f"report:{auth.client_ip()}", 20, 3600):
        return jsonify(error="Too many reports. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    reported = data.get("userId")
    if not isinstance(reported, int) or isinstance(reported, bool):
        return jsonify(error="Invalid report."), 400
    c = db.conn()
    if not c.execute("SELECT 1 FROM users WHERE id=?", (reported,)).fetchone():
        return jsonify(error="Invalid report."), 400
    reporter = auth.current_user()
    c.execute(
        "INSERT INTO name_reports(reported_user_id, reporter_user_id, created_at) VALUES(?,?,?)",
        (reported, reporter["id"] if reporter else None, time.time()),
    )
    c.commit()
    return jsonify(ok=True)


# ---------- password recovery ----------



@bp.route("/api/auth/reset-password", methods=["POST"])
def reset_password():
    """F10 — finish a reset with the emailed code, then revoke every session:
    if the reset was somebody else getting in, this is what pushes them out."""
    if not auth.rate_limit(f"reset:{auth.client_ip()}", 10, 900):
        return jsonify(error="Too many attempts. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    email_lc = (data.get("email") or "").strip().lower()
    new_password = data.get("newPassword") or ""
    problem = auth.check_code(email_lc, "reset", data.get("code") or "", consume=False)
    if problem:
        return jsonify(error=problem, field="code"), 400
    pw_problem = auth.password_problem(new_password)
    if pw_problem:
        return jsonify(error=pw_problem, field="password"), 400
    c = db.conn()
    row = c.execute("SELECT id FROM users WHERE email_lc=?", (email_lc,)).fetchone()
    if not row:
        return jsonify(error="That code is invalid or has expired."), 400
    auth.check_code(email_lc, "reset", data.get("code") or "", consume=True)
    c.execute("UPDATE users SET pw_hash=? WHERE id=?", (auth.hash_password(new_password), row["id"]))
    c.commit()
    auth.revoke_all_for_user(row["id"])
    return jsonify(ok=True)


# ---------- account management ----------

@bp.route("/api/auth/change-username", methods=["POST"])
def change_username():
    u = auth.current_user()
    if not u:
        return jsonify(error="Not signed in."), 401
    if not auth.rate_limit(f"rename:{u['id']}", 5, 86400):
        return jsonify(error="You can only change your username a few times a day."), 429
    data = request.get_json(silent=True) or {}
    new = (data.get("username") or "").strip()
    if not usernames.is_acceptable(new):
        return jsonify(error="That username isn't available.", field="username"), 400

    c = db.conn()
    nlc, old_lc = new.lower(), u["username_lc"]
    if nlc == old_lc:
        return jsonify(ok=True, username=new)  # case-only / no-op change
    taken = c.execute("SELECT 1 FROM users WHERE username_lc=? AND id<>?", (nlc, u["id"])).fetchone() or c.execute(
        "SELECT 1 FROM reserved_usernames WHERE username_lc=? AND reserved_until>?", (nlc, time.time())
    ).fetchone()
    if taken:
        return jsonify(error="That username isn't available.", field="username"), 400

    t = time.time()
    # Hold the old handle for 30 days so it can't be grabbed to impersonate.
    c.execute(
        "INSERT OR REPLACE INTO reserved_usernames(username_lc, reserved_until) VALUES(?,?)",
        (old_lc, t + 30 * 86400),
    )
    c.execute(
        "UPDATE users SET username=?, username_lc=?, username_changed_at=? WHERE id=?",
        (new, nlc, t, u["id"]),
    )
    c.commit()
    return jsonify(ok=True, username=new)


@bp.route("/api/auth/delete-account", methods=["POST"])
def delete_account():
    """In-app account deletion (Apple App Review 5.1.1(v)). Removes the user and
    ALL their data — sessions, leaderboard entries, resets, reports — via ON
    DELETE CASCADE, plus the un-cascaded submit_log."""
    u = auth.current_user()
    if not u:
        return jsonify(error="Not signed in."), 401
    data = request.get_json(silent=True) or {}
    if not auth.verify_password(data.get("password") or "", u["pw_hash"]):
        return jsonify(error="Incorrect password."), 401

    c = db.conn()
    c.execute("DELETE FROM submit_log WHERE user_id=?", (u["id"],))
    c.execute("DELETE FROM users WHERE id=?", (u["id"],))  # CASCADE clears the rest
    c.commit()
    resp = make_response(jsonify(ok=True))
    return auth.clear_session_cookie(resp)
