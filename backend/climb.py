"""The Climb — HTTP routes (Flask blueprint).

Phase 2: account auth — signup, login, logout, refresh, me, verify-email.
Leaderboard submit/read, password recovery, change-username and account
deletion come in later phases. All input is validated; signup/login are
rate-limited and Turnstile-gated (skipped when unconfigured); errors are generic
to avoid revealing which rule failed or whether an account exists.
"""

import secrets
import time

from flask import Blueprint, jsonify, make_response, request

import auth
import db
import usernames

bp = Blueprint("climb", __name__)


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
