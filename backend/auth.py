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


# ---------- password policy (CC-ONBOARD-JR F9 / D3) ----------

PASSWORD_MIN = 8
PASSWORD_MAX = 200            # D3 asks for a max of at least 64; passphrases welcome
_BLOCKLIST_PATH = os.path.join(os.path.dirname(__file__), "password-blocklist.txt")
_pw_blocklist = None


def _password_blocklist() -> set:
    global _pw_blocklist
    if _pw_blocklist is None:
        try:
            with open(_BLOCKLIST_PATH, encoding="utf-8") as f:
                _pw_blocklist = {
                    ln.strip().lower() for ln in f if ln.strip() and not ln.lstrip().startswith("#")
                }
        except OSError:
            _pw_blocklist = set()
    return _pw_blocklist


def password_problem(pw: str):
    """None if the password is acceptable, else the reason in a player's words.

    Eric's rule (D3): eight or more characters, at least one digit, at least one
    symbol. Around it: a generous maximum, spaces allowed, and a common-password
    blocklist -- because composition rules alone wave through exactly what is
    tried first ("Password1!" satisfies all three). A space does not count as the
    symbol, or "password 1" would pass on a space.
    """
    pw = pw or ""
    if len(pw) < PASSWORD_MIN:
        return "Password must be at least 8 characters."
    if len(pw) > PASSWORD_MAX:
        return f"Password must be {PASSWORD_MAX} characters or fewer."
    if not any(ch.isdigit() for ch in pw):
        return "Password needs at least one number."
    if not any((not ch.isalnum()) and not ch.isspace() for ch in pw):
        return "Password needs at least one symbol."
    lowered = pw.lower()
    letters = "".join(ch for ch in lowered if ch.isalpha())
    blocked = _password_blocklist()
    if lowered in blocked or (letters and letters in blocked):
        return "That password is too common. Please pick another."
    return None


# ---------- one-time email codes (CC-ONBOARD-JR F8/F10 / D4) ----------

CODE_TTL = 10 * 60            # ten minutes
CODE_MAX_ATTEMPTS = 5         # then the code is dead, not merely wrong
CODE_RESEND_COOLDOWN = 60     # seconds between sends to one address
CODE_SENDS_PER_HOUR = 5       # per address, per hour


def new_code() -> str:
    return f"{secrets.randbelow(1000000):06d}"


def code_send_problem(email_lc: str, purpose: str):
    """None if another code may go to this address now, else the reason.

    Counted in the DATABASE, not in memory. The backend runs two gunicorn
    workers, so an in-process counter is two counters and every limit is quietly
    doubled -- which is what the Step 0 inventory found for the per-IP limiter.
    """
    t = now()
    row = db.conn().execute(
        "SELECT MAX(sent_at) AS last, COUNT(*) AS n FROM email_sends "
        "WHERE email_lc=? AND purpose=? AND sent_at > ?",
        (email_lc, purpose, t - 3600),
    ).fetchone()
    if row and row["last"] and row["last"] > t - CODE_RESEND_COOLDOWN:
        return "Please wait a minute before asking for another code."
    if row and row["n"] >= CODE_SENDS_PER_HOUR:
        return "Too many codes requested for that address. Please try again later."
    return None


def note_code_send(email_lc: str, purpose: str) -> None:
    c = db.conn()
    c.execute("INSERT INTO email_sends(email_lc,purpose,sent_at) VALUES(?,?,?)", (email_lc, purpose, now()))
    c.commit()


def issue_code(email_lc: str, purpose: str) -> str:
    """Replace any live code for (address, purpose) with a fresh one."""
    c = db.conn()
    code = new_code()
    t = now()
    c.execute("DELETE FROM email_codes WHERE email_lc=? AND purpose=?", (email_lc, purpose))
    c.execute(
        "INSERT INTO email_codes(email_lc,purpose,code_hash,expires_at,attempts,used,created_at) "
        "VALUES(?,?,?,?,0,0,?)",
        (email_lc, purpose, hash_password(code), t + CODE_TTL, t),
    )
    c.commit()
    return code


def check_code(email_lc: str, purpose: str, code: str, consume: bool):
    """None when the code is right, else the reason. Attempts are counted and the
    code dies after CODE_MAX_ATTEMPTS, so guessing six digits is not a matter of
    patience. Codes are bcrypt-hashed at rest, like passwords."""
    c = db.conn()
    row = c.execute(
        "SELECT * FROM email_codes WHERE email_lc=? AND purpose=?", (email_lc, purpose)
    ).fetchone()
    if not row or row["used"] or row["expires_at"] < now():
        return "That code is invalid or has expired."
    if row["attempts"] >= CODE_MAX_ATTEMPTS:
        c.execute("UPDATE email_codes SET used=1 WHERE email_lc=? AND purpose=?", (email_lc, purpose))
        c.commit()
        return "Too many incorrect codes. Please ask for a new one."
    if not verify_password(code or "", row["code_hash"]):
        c.execute("UPDATE email_codes SET attempts=attempts+1 WHERE email_lc=? AND purpose=?", (email_lc, purpose))
        c.commit()
        return "That code isn't right."
    if consume:
        c.execute("UPDATE email_codes SET used=1 WHERE email_lc=? AND purpose=?", (email_lc, purpose))
        c.commit()
    return None


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
                headers={
                    "Authorization": f"Bearer {api_key}",
                    "Content-Type": "application/json",
                    # Cloudflare fronts the Resend API and returns 403/1010 to
                    # clients with no User-Agent (e.g. bare urllib), so set one.
                    "User-Agent": "SpellRPS-backend/1.0",
                },
            )
            urllib.request.urlopen(req, timeout=10).read()
            return True
        except Exception as e:  # pragma: no cover
            print(f"[climb] email send failed: {e}", flush=True)
            return False
    print(f"[climb] (stub email) to={to} subject={subject!r}\n{html}", flush=True)
    return True


# CC-ONBOARD-JR F12 / I12: send_sms is DELETED, not stubbed. Phone signup and
# SMS recovery are gone from the product, and a dormant sender is how they
# would quietly come back.
