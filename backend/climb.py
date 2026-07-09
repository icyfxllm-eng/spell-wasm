"""The Climb — HTTP routes (Flask blueprint).

Phase 2: account auth — signup, login, logout, refresh, me, verify-email.
Leaderboard submit/read, password recovery, change-username and account
deletion come in later phases. All input is validated; signup/login are
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


@bp.route("/api/auth/signup", methods=["POST"])
def signup():
    if not auth.rate_limit(f"signup:{auth.client_ip()}", 5, 3600):
        return jsonify(error="Too many attempts. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    username = (data.get("username") or "").strip()
    email = (data.get("email") or "").strip()
    password = data.get("password") or ""
    display_name = (data.get("displayName") or "").strip() or None
    phone = (data.get("phone") or "").strip() or None

    if not auth.verify_turnstile(data.get("turnstile")):
        return jsonify(error="Verification failed. Please try again."), 400
    if not usernames.is_acceptable(username):
        return jsonify(error="That username isn't available.", field="username"), 400
    if not auth.valid_email(email):
        return jsonify(error="Enter a valid email address.", field="email"), 400
    if len(password) < 8:
        return jsonify(error="Password must be at least 8 characters.", field="password"), 400

    c = db.conn()
    ulc, elc = username.lower(), email.lower()
    taken = c.execute("SELECT 1 FROM users WHERE username_lc=?", (ulc,)).fetchone() or c.execute(
        "SELECT 1 FROM reserved_usernames WHERE username_lc=? AND reserved_until>?", (ulc, time.time())
    ).fetchone()
    if taken:
        return jsonify(error="That username isn't available.", field="username"), 400
    if c.execute("SELECT 1 FROM users WHERE email_lc=?", (elc,)).fetchone():
        # Generic — don't confirm which email is registered.
        return jsonify(error="That email can't be used.", field="email"), 400

    t = time.time()
    cur = c.execute(
        "INSERT INTO users(username,username_lc,display_name,email,email_lc,phone,pw_hash,created_at) "
        "VALUES(?,?,?,?,?,?,?,?)",
        (username, ulc, display_name, email, elc, phone, auth.hash_password(password), t),
    )
    c.commit()
    uid = cur.lastrowid

    # Email verification (stubbed delivery until a provider is configured).
    vtok = secrets.token_urlsafe(32)
    c.execute(
        "INSERT INTO email_verifications(token,user_id,expires_at) VALUES(?,?,?)",
        (vtok, uid, t + auth.EMAIL_VERIFY_TTL),
    )
    c.commit()
    link = f"{auth.FRONTEND_BASE}/api/auth/verify-email?token={vtok}"
    auth.send_email(
        email,
        "Verify your Spell account",
        f'<p>Welcome to Spell! Confirm your email to finish setting up The Climb:</p>'
        f'<p><a href="{link}">Verify email</a></p>',
    )

    token = auth.create_session(uid)
    user_row = auth.session_user(token)
    extra = {"devVerifyLink": link} if auth.DEV else None
    return _auth_response(user_row, token, extra)


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


@bp.route("/api/auth/verify-email")
def verify_email():
    tok = request.args.get("token", "")
    c = db.conn()
    row = c.execute("SELECT * FROM email_verifications WHERE token=?", (tok,)).fetchone()
    if not row or row["expires_at"] < time.time():
        return "This verification link is invalid or has expired.", 400
    c.execute("UPDATE users SET email_verified=1 WHERE id=?", (row["user_id"],))
    c.execute("DELETE FROM email_verifications WHERE token=?", (tok,))
    c.commit()
    resp = make_response("", 302)
    resp.headers["Location"] = f"{auth.FRONTEND_BASE}/?verified=1"
    return resp


# ---------- leaderboard ("The Climb") ----------

def _rank_for(c, difficulty, user_id):
    """1-based rank of a player in a category (ties broken by earliest
    achievement), or None if they have no entry there."""
    row = c.execute(
        "SELECT best_chain, achieved_at FROM leaderboard_entries WHERE user_id=? AND difficulty=?",
        (user_id, difficulty),
    ).fetchone()
    if not row:
        return None
    better = c.execute(
        "SELECT COUNT(*) AS n FROM leaderboard_entries WHERE difficulty=? "
        "AND (best_chain > ? OR (best_chain = ? AND achieved_at < ?))",
        (difficulty, row["best_chain"], row["best_chain"], row["achieved_at"]),
    ).fetchone()["n"]
    return better + 1


@bp.route("/api/climb/submit-chain", methods=["POST"])
def submit_chain():
    u = auth.current_user()
    if not u:
        return jsonify(error="Log in to post your chain to The Climb."), 401
    data = request.get_json(silent=True) or {}
    difficulty = (data.get("difficulty") or "").strip().lower()
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
        "SELECT best_chain FROM leaderboard_entries WHERE user_id=? AND difficulty=?",
        (u["id"], difficulty),
    ).fetchone()
    is_record = prev is None or chain > prev["best_chain"]
    if is_record:
        # Server-side timestamp; run_meta kept for future verification/audit.
        run_meta = json.dumps({"wordCount": word_count, "durationMs": duration_ms})
        c.execute(
            "INSERT INTO leaderboard_entries(user_id,difficulty,best_chain,achieved_at,run_meta) "
            "VALUES(?,?,?,?,?) ON CONFLICT(user_id,difficulty) DO UPDATE SET "
            "best_chain=excluded.best_chain, achieved_at=excluded.achieved_at, run_meta=excluded.run_meta",
            (u["id"], difficulty, chain, t, run_meta),
        )
        c.commit()

    best = chain if is_record else prev["best_chain"]
    return jsonify(record=is_record, best=best, rank=_rank_for(c, difficulty, u["id"]))


@bp.route("/api/climb/leaderboard")
def leaderboard():
    difficulty = (request.args.get("difficulty") or "").strip().lower()
    if difficulty not in VALID_DIFFICULTIES:
        return jsonify(error="That difficulty isn't ranked."), 400
    c = db.conn()
    rows = c.execute(
        "SELECT u.id, u.username, e.best_chain, e.achieved_at "
        "FROM leaderboard_entries e JOIN users u ON u.id=e.user_id "
        "WHERE e.difficulty=? ORDER BY e.best_chain DESC, e.achieved_at ASC LIMIT 50",
        (difficulty,),
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
        rank = _rank_for(c, difficulty, u["id"])
        if rank and rank > 50:  # pin the player's own row when outside the top 50
            row = c.execute(
                "SELECT best_chain, achieved_at FROM leaderboard_entries WHERE user_id=? AND difficulty=?",
                (u["id"], difficulty),
            ).fetchone()
            me = {"rank": rank, "userId": u["id"], "username": u["username"],
                  "chain": row["best_chain"], "achievedAt": row["achieved_at"]}
    return jsonify(difficulty=difficulty, top=top, me=me)


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

RESET_EMAIL_TTL = 30 * 60   # 30 minutes
RESET_SMS_TTL = 10 * 60     # 10 minutes
RESET_SMS_MAX_ATTEMPTS = 3


@bp.route("/api/auth/request-reset-email", methods=["POST"])
def request_reset_email():
    if not auth.rate_limit(f"reset-email:{auth.client_ip()}", 5, 3600):
        return jsonify(error="Too many requests. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    email = (data.get("email") or "").strip().lower()
    c = db.conn()
    row = c.execute("SELECT id FROM users WHERE email_lc=?", (email,)).fetchone()
    token = None
    if row:
        token = secrets.token_urlsafe(32)
        c.execute(
            "INSERT INTO password_resets(token,user_id,kind,expires_at) VALUES(?,?,?,?)",
            (token, row["id"], "email", time.time() + RESET_EMAIL_TTL),
        )
        c.commit()
        link = f"{auth.FRONTEND_BASE}/?reset={token}"
        auth.send_email(
            email,
            "Reset your Spell password",
            f'<p>Reset your Spell password (link valid 30 minutes):</p>'
            f'<p><a href="{link}">Reset password</a></p>',
        )
    # Never reveal whether the account exists.
    body = {"ok": True, "message": "If that account exists, we've sent a reset link."}
    if auth.DEV and token:
        body["devResetToken"] = token
    return jsonify(body)


@bp.route("/api/auth/request-reset-sms", methods=["POST"])
def request_reset_sms():
    if not auth.rate_limit(f"reset-sms:{auth.client_ip()}", 5, 3600):
        return jsonify(error="Too many requests. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    identifier = (data.get("identifier") or "").strip().lower()
    c = db.conn()
    row = c.execute(
        "SELECT id, phone, phone_verified FROM users WHERE username_lc=? OR email_lc=?",
        (identifier, identifier),
    ).fetchone()
    # Always return a token so absence can't be probed; it maps to a real reset
    # row only when the account exists AND has a verified phone.
    token = secrets.token_urlsafe(24)
    dev_code = None
    if row and row["phone"] and row["phone_verified"]:
        code = f"{secrets.randbelow(1000000):06d}"
        c.execute(
            "INSERT INTO password_resets(token,user_id,kind,code_hash,expires_at) VALUES(?,?,?,?,?)",
            (token, row["id"], "sms", auth.hash_password(code), time.time() + RESET_SMS_TTL),
        )
        c.commit()
        auth.send_sms(row["phone"], f"Your Spell reset code is {code}")
        dev_code = code
    body = {"ok": True, "message": "If that account has a verified phone, we've sent a code.", "token": token}
    if auth.DEV and dev_code:
        body["devSmsCode"] = dev_code
    return jsonify(body)


@bp.route("/api/auth/confirm-reset", methods=["POST"])
def confirm_reset():
    if not auth.rate_limit(f"confirm-reset:{auth.client_ip()}", 10, 900):
        return jsonify(error="Too many attempts. Please try again later."), 429
    data = request.get_json(silent=True) or {}
    token = data.get("token") or ""
    new_password = data.get("newPassword") or ""
    code = (data.get("code") or "").strip()
    if len(new_password) < 8:
        return jsonify(error="Password must be at least 8 characters.", field="password"), 400

    c = db.conn()
    row = c.execute("SELECT * FROM password_resets WHERE token=?", (token,)).fetchone()
    if not row or row["used"] or row["expires_at"] < time.time():
        return jsonify(error="This reset link or code is invalid or has expired."), 400

    if row["kind"] == "sms":
        if row["attempts"] >= RESET_SMS_MAX_ATTEMPTS:
            c.execute("UPDATE password_resets SET used=1 WHERE token=?", (token,))
            c.commit()
            return jsonify(error="Too many incorrect codes. Request a new one."), 400
        if not auth.verify_password(code, row["code_hash"] or ""):
            c.execute("UPDATE password_resets SET attempts=attempts+1 WHERE token=?", (token,))
            c.commit()
            return jsonify(error="Incorrect code."), 400

    # Success: set the new password, consume the token, and invalidate ALL
    # existing sessions (both reset flows).
    c.execute("UPDATE users SET pw_hash=? WHERE id=?", (auth.hash_password(new_password), row["user_id"]))
    c.execute("UPDATE password_resets SET used=1 WHERE token=?", (token,))
    c.commit()
    auth.revoke_all_for_user(row["user_id"])
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
