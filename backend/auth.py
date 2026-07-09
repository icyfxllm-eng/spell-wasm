"""Auth helpers for The Climb: password hashing, sessions, provider stubs.

Sessions are opaque random tokens stored in the `sessions` table, delivered two
ways so both clients work: an httpOnly Secure cookie for the web, and the same
token in the JSON body for the Capacitor app to keep in Preferences and send as
`Authorization: Bearer`. Either is accepted on requests. 90-day rolling expiry.

Passwords are bcrypt-hashed; plaintext is never stored or logged.

Email/SMS/Turnstile are wired to real providers when their env vars are set, and
otherwise STUBBED (logged) so the whole flow works in dev without credentials.
"""

import json
import os
import re
import secrets
import threading
import time
import urllib.parse
import urllib.request

import bcrypt
from flask import request

import db

SESSION_TTL = 90 * 24 * 3600          # 90 days, rolling
EMAIL_VERIFY_TTL = 24 * 3600
COOKIE_NAME = "climb_session"
FRONTEND_BASE = os.environ.get("FRONTEND_BASE", "https://spellgame.net")
DEV = os.environ.get("CLIMB_DEV") == "1"

_EMAIL_RE = re.compile(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")


def now() -> float:
    return time.time()


def valid_email(e: str) -> bool:
    return bool(_EMAIL_RE.match(e or ""))


# ---------- passwords ----------

def hash_password(pw: str) -> str:
    return bcrypt.hashpw(pw.encode("utf-8"), bcrypt.gensalt()).decode("utf-8")


def verify_password(pw: str, hashed: str) -> bool:
    try:
        return bcrypt.checkpw(pw.encode("utf-8"), hashed.encode("utf-8"))
    except Exception:
        return False


# ---------- sessions ----------

def _new_token() -> str:
    return secrets.token_urlsafe(32)


def create_session(user_id: int) -> str:
    tok = _new_token()
    t = now()
    c = db.conn()
    c.execute(
        "INSERT INTO sessions(token,user_id,created_at,expires_at) VALUES(?,?,?,?)",
        (tok, user_id, t, t + SESSION_TTL),
    )
    c.commit()
    return tok


def session_user(tok):
    """The user row for a live session token, or None (expired sessions are
    reaped)."""
    if not tok:
        return None
    c = db.conn()
    row = c.execute(
        "SELECT u.*, s.expires_at AS _sess_exp FROM sessions s "
        "JOIN users u ON u.id = s.user_id WHERE s.token = ?",
        (tok,),
    ).fetchone()
    if not row:
        return None
    if row["_sess_exp"] < now():
        c.execute("DELETE FROM sessions WHERE token=?", (tok,))
        c.commit()
        return None
    return row


def refresh_session(tok: str) -> None:
    c = db.conn()
    c.execute("UPDATE sessions SET expires_at=? WHERE token=?", (now() + SESSION_TTL, tok))
    c.commit()


def revoke_session(tok: str) -> None:
    c = db.conn()
    c.execute("DELETE FROM sessions WHERE token=?", (tok,))
    c.commit()


def revoke_all_for_user(user_id: int) -> None:
    """Invalidate every session for a user (used after a password reset)."""
    c = db.conn()
    c.execute("DELETE FROM sessions WHERE user_id=?", (user_id,))
    c.commit()


def request_token():
    auth_header = request.headers.get("Authorization", "")
    if auth_header.startswith("Bearer "):
        return auth_header[7:].strip()
    return request.cookies.get(COOKIE_NAME)


def current_user():
    return session_user(request_token())


def set_session_cookie(resp, tok: str):
    secure = os.environ.get("SESSION_COOKIE_SECURE", "1") != "0"
    resp.set_cookie(
        COOKIE_NAME, tok, max_age=SESSION_TTL, httponly=True, secure=secure, samesite="Lax", path="/"
    )
    return resp


def clear_session_cookie(resp):
    resp.delete_cookie(COOKIE_NAME, path="/")
    return resp


# ---------- rate limiting (in-memory, per-process) ----------

_rl_lock = threading.Lock()
_rl: dict = {}


def rate_limit(key: str, limit: int, window: float) -> bool:
    """True if this call is within `limit` per `window` seconds for `key`.
    In-memory/per-process — fine for a single backend; move to the DB/KV if the
    backend is ever scaled out."""
    t = now()
    with _rl_lock:
        hits = [h for h in _rl.get(key, []) if h > t - window]
        if len(hits) >= limit:
            _rl[key] = hits
            return False
        hits.append(t)
        _rl[key] = hits
        return True


def client_ip() -> str:
    xff = request.headers.get("X-Forwarded-For", "")
    return xff.split(",")[0].strip() if xff else (request.remote_addr or "?")


# ---------- Turnstile (optional) ----------

def verify_turnstile(token) -> bool:
    """Verify a Cloudflare Turnstile token. Skipped (returns True) when
    TURNSTILE_SECRET isn't configured, so dev/testing isn't blocked."""
    secret = os.environ.get("TURNSTILE_SECRET")
    if not secret:
        return True
    try:
        data = urllib.parse.urlencode(
            {"secret": secret, "response": token or "", "remoteip": client_ip()}
        ).encode()
        req = urllib.request.Request(
            "https://challenges.cloudflare.com/turnstile/v0/siteverify", data=data
        )
        with urllib.request.urlopen(req, timeout=8) as r:
            return bool(json.loads(r.read()).get("success", False))
    except Exception:
        return False


# ---------- providers (STUBBED until credentials are configured) ----------

def send_email(to: str, subject: str, html: str) -> bool:
    """Send via Resend if RESEND_API_KEY is set; otherwise log (stub) so the
    signup/reset flows work end-to-end in dev. MailChannels/other providers can
    slot in here behind the same signature."""
    api_key = os.environ.get("RESEND_API_KEY")
    if api_key:
        try:
            payload = json.dumps(
                {
                    "from": os.environ.get("EMAIL_FROM", "Spell <noreply@spellgame.net>"),
                    "to": [to],
                    "subject": subject,
                    "html": html,
                }
            ).encode()
            req = urllib.request.Request(
                "https://api.resend.com/emails",
                data=payload,
                headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
            )
            urllib.request.urlopen(req, timeout=10).read()
            return True
        except Exception as e:  # pragma: no cover
            print(f"[climb] email send failed: {e}", flush=True)
            return False
    print(f"[climb] (stub email) to={to} subject={subject!r}\n{html}", flush=True)
    return True


def send_sms(to: str, body: str) -> bool:
    """SMS delivery. TODO: wire Twilio Verify (TWILIO_* env) when configured.
    Stubbed (logged) for now — see README."""
    print(f"[climb] (stub sms) to={to} body={body!r}", flush=True)
    return True
