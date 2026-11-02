"""CC-ACCOUNTS Phase 1 -- the identity service.

Small and boring on purpose. One adult account, proven by Sign in with Apple or by
a six-digit email code typed in the app. Short-lived ES256 access tokens that a
room server can verify with the PUBLIC key alone; opaque, rotating refresh tokens
whose reuse kills the whole session family.

    ACCOUNTS_DB_PATH=<tmp> python3 -m unittest test_accounts    # from backend/

Keys (I11): the private key lives only on this host, outside the repo.
    openssl ecparam -name prime256v1 -genkey -noout -out accounts-jwt.pem
Missing key => every token endpoint answers 503. It is never auto-generated.

Logging (section 5): user id only. Emails are hashed. Never a token, a code, or a
child's anything. Nothing here prints a request body.

Deliberately NOT here: Climb leaderboard cutover, sync, StoreKit entitlement
binding, child profiles' endpoints (Phases 3-5; the tables exist for Phase 5).
"""

import base64
import hashlib
import hmac
import json
import os
import secrets
import sqlite3
import threading
import time
import urllib.parse
import urllib.request

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, padding, rsa, utils
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from flask import Blueprint, jsonify, request

import auth  # send_email, valid_email, client_ip: reused, not duplicated

bp = Blueprint("accounts", __name__)

DB_PATH = os.environ.get("ACCOUNTS_DB_PATH", "/data/accounts/accounts.db")
KEY_PATH = os.environ.get("ACCOUNTS_JWT_KEY_PATH", os.path.expanduser("~/spellgame-server/keys/accounts-jwt.pem"))
TOKEN_ENC_KEY = os.environ.get("ACCOUNTS_TOKEN_ENC_KEY", "")   # 32 bytes, base64; encrypts the SIWA revoke token
INTERNAL_KEY = os.environ.get("ACCOUNTS_INTERNAL_KEY", "")     # room servers redeem tickets with this
KILL_FLAGS_FILE = os.environ.get("KILL_FLAGS_FILE", "")

ISSUER = "https://spellgame.net/accounts"
AUDIENCE = "spell-rooms"
APPLE_ISS = "https://appleid.apple.com"
APPLE_AUD = os.environ.get("APPLE_BUNDLE_ID", "net.spellgame.app")
APPLE_TEAM_ID = os.environ.get("APPLE_TEAM_ID", "")
APPLE_KEY_ID = os.environ.get("APPLE_KEY_ID", "")
APPLE_P8_PATH = os.environ.get("APPLE_P8_PATH", "")

ACCESS_TTL = 15 * 60
REFRESH_TTL = 30 * 24 * 3600
CODE_TTL = 10 * 60
CODE_MAX_ATTEMPTS = 5
TICKET_TTL = 60
CODE_COOLDOWN = 60
CODE_PER_HOUR = 5

_local = threading.local()

SCHEMA = """
CREATE TABLE IF NOT EXISTS users (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  apple_sub      TEXT UNIQUE,
  email          TEXT,
  email_lc       TEXT UNIQUE,
  email_verified INTEGER NOT NULL DEFAULT 0,
  apple_revoke   BLOB,                 -- encrypted Apple refresh token, for revocation on delete
  created_at     REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS child_profiles (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  generated_name TEXT NOT NULL,        -- generated, never typed (I4)
  created_at     REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
  id           TEXT PRIMARY KEY,
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_id    TEXT,                   -- app-generated random, adults only
  family_id    TEXT NOT NULL,
  refresh_hash TEXT NOT NULL UNIQUE,
  created_at   REAL NOT NULL,
  last_used_at REAL NOT NULL,
  rotated_at   REAL,                   -- set when exchanged for the next link
  revoked_at   REAL
);
CREATE INDEX IF NOT EXISTS idx_sessions_family ON sessions(family_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE TABLE IF NOT EXISTS entitlements (
  user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  product_id     TEXT NOT NULL,
  transaction_id TEXT NOT NULL,
  verified_at    REAL NOT NULL,
  PRIMARY KEY (user_id, product_id)
);                                     -- Phase 4, once StoreKit exists
CREATE TABLE IF NOT EXISTS sync_records (
  user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  profile_id TEXT NOT NULL DEFAULT '',
  kind       TEXT NOT NULL,
  record_id  TEXT NOT NULL,
  payload    TEXT,
  updated_at REAL NOT NULL,
  deleted    INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, profile_id, kind, record_id)
);                                     -- Phase 3
CREATE TABLE IF NOT EXISTS email_codes (
  email_lc   TEXT PRIMARY KEY,
  salt       TEXT NOT NULL,
  code_hash  TEXT NOT NULL,
  expires_at REAL NOT NULL,
  attempts   INTEGER NOT NULL DEFAULT 0,
  created_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS rate_events (
  key TEXT NOT NULL,
  at  REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rate ON rate_events(key, at);
CREATE TABLE IF NOT EXISTS tickets (
  hash       TEXT PRIMARY KEY,
  user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS apple_events (
  jti TEXT PRIMARY KEY,
  at  REAL NOT NULL
);
"""


def conn() -> sqlite3.Connection:
    c = getattr(_local, "conn", None)
    if c is None or getattr(_local, "path", None) != DB_PATH:
        parent = os.path.dirname(DB_PATH)
        if parent:
            os.makedirs(parent, exist_ok=True)
        c = sqlite3.connect(DB_PATH)
        c.row_factory = sqlite3.Row
        c.execute("PRAGMA foreign_keys = ON")
        c.execute("PRAGMA journal_mode = WAL")
        c.executescript(SCHEMA)
        _local.conn, _local.path = c, DB_PATH
    return c


def now() -> float:
    return time.time()


def _b64(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()


def _unb64(s: str) -> bytes:
    return base64.urlsafe_b64decode(s + "=" * (-len(s) % 4))


def _sha(s: str) -> str:
    return hashlib.sha256(s.encode()).hexdigest()


def _log(event: str, user_id=None, **kw) -> None:
    """User id only. Anything that is an email arrives already hashed."""
    extra = " ".join(f"{k}={v}" for k, v in kw.items())
    print(f"[accounts] {event} user={user_id} {extra}".rstrip(), flush=True)


def email_tag(email_lc: str) -> str:
    return _sha(email_lc)[:12]


def killed(flag: str) -> bool:
    """I12: the server flag file can only kill. This is the same signed document the app reads
    ({"v":1,"issued":...,"kill":["accounts_enabled"],"sig":...}); the host trusts its own copy of
    the file, so the signature is the app's concern. Absent file or name => not killed."""
    if not KILL_FLAGS_FILE:
        return False
    try:
        with open(KILL_FLAGS_FILE, "rb") as fh:
            data = json.loads(fh.read(65536))
    except (OSError, ValueError):
        return False
    kill = data.get("kill") if isinstance(data, dict) else None
    return isinstance(kill, list) and flag in kill


# ---------- signing keys ----------

_key_cache = {}


def _private_key():
    path = KEY_PATH
    if path not in _key_cache:
        try:
            with open(path, "rb") as fh:
                _key_cache[path] = serialization.load_pem_private_key(fh.read(), password=None)
        except OSError:
            return None
    return _key_cache[path]


def public_key_pem() -> str:
    k = _private_key()
    return k.public_key().public_bytes(
        serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo
    ).decode()


def _kid() -> str:
    return _sha(public_key_pem())[:16]


def _es256_sign(key, signing_input: bytes) -> str:
    der = key.sign(signing_input, ec.ECDSA(hashes.SHA256()))
    r, s = utils.decode_dss_signature(der)
    return _b64(r.to_bytes(32, "big") + s.to_bytes(32, "big"))


def _jwt(claims: dict, key, header_extra: dict) -> str:
    header = {"alg": "ES256", "typ": "JWT", **header_extra}
    head = _b64(json.dumps(header, separators=(",", ":")).encode())
    body = _b64(json.dumps(claims, separators=(",", ":")).encode())
    return f"{head}.{body}.{_es256_sign(key, f'{head}.{body}'.encode())}"


def mint_access(user_id: int, family_id: str) -> str:
    t = int(now())
    return _jwt(
        {"iss": ISSUER, "aud": AUDIENCE, "sub": str(user_id), "sid": family_id, "iat": t, "exp": t + ACCESS_TTL},
        _private_key(), {"kid": _kid()},
    )


def verify_access(token: str, public_pem=None):
    """Claims for a good access token, else None. Needs only the PUBLIC key, which is
    exactly what a room server holds. Checks signature, algorithm, issuer, audience, expiry."""
    try:
        head_b, body_b, sig_b = token.split(".")
        header = json.loads(_unb64(head_b))
        if header.get("alg") != "ES256":
            return None
        pub = serialization.load_pem_public_key((public_pem or public_key_pem()).encode())
        sig = _unb64(sig_b)
        if len(sig) != 64:
            return None
        der = utils.encode_dss_signature(int.from_bytes(sig[:32], "big"), int.from_bytes(sig[32:], "big"))
        pub.verify(der, f"{head_b}.{body_b}".encode(), ec.ECDSA(hashes.SHA256()))
        claims = json.loads(_unb64(body_b))
    except Exception:
        return None
    if claims.get("iss") != ISSUER or claims.get("aud") != AUDIENCE or claims.get("exp", 0) <= now():
        return None
    return claims


# ---------- sessions ----------

def _issue_session(user_id: int, device_id, family_id=None) -> dict:
    c = conn()
    family_id = family_id or secrets.token_hex(16)
    refresh = "rt_" + secrets.token_urlsafe(32)
    t = now()
    c.execute(
        "INSERT INTO sessions(id,user_id,device_id,family_id,refresh_hash,created_at,last_used_at) VALUES(?,?,?,?,?,?,?)",
        (secrets.token_hex(16), user_id, device_id, family_id, _sha(refresh), t, t),
    )
    c.commit()
    return {"access_token": mint_access(user_id, family_id), "expires_in": ACCESS_TTL,
            "refresh_token": refresh, "user_id": user_id}


def revoke_family(family_id: str) -> None:
    c = conn()
    c.execute("UPDATE sessions SET revoked_at=? WHERE family_id=? AND revoked_at IS NULL", (now(), family_id))
    c.commit()


def current_user():
    """The account behind a Bearer access token, checked against the sessions table
    (so sign-out and delete bite at once on this service). Room servers verify locally
    and therefore see a revocation within ACCESS_TTL."""
    h = request.headers.get("Authorization", "")
    if not h.startswith("Bearer "):
        return None
    claims = verify_access(h[7:])
    if not claims:
        return None
    c = conn()
    live = c.execute(
        "SELECT 1 FROM sessions WHERE family_id=? AND user_id=? AND revoked_at IS NULL LIMIT 1",
        (claims["sid"], int(claims["sub"])),
    ).fetchone()
    if not live:
        return None
    return c.execute("SELECT * FROM users WHERE id=?", (int(claims["sub"]),)).fetchone()


# ---------- rate limits (in the DB: two gunicorn workers share it) ----------

def _rate_ok(key: str, limit: int, window: float) -> bool:
    c = conn()
    t = now()
    c.execute("DELETE FROM rate_events WHERE at < ?", (t - 86400,))
    n = c.execute("SELECT COUNT(*) FROM rate_events WHERE key=? AND at>?", (key, t - window)).fetchone()[0]
    if n >= limit:
        return False
    c.execute("INSERT INTO rate_events(key,at) VALUES(?,?)", (key, t))
    c.commit()
    return True


def _guard():
    """503 when the kill flag is thrown or the signing key is missing."""
    if killed("accounts_enabled"):
        return jsonify(error="unavailable"), 503
    if _private_key() is None:
        return jsonify(error="unavailable"), 503
    return None


def _device(data):
    d = data.get("device_id")
    return d if isinstance(d, str) and 8 <= len(d) <= 64 else None


# ---------- email code ----------

@bp.route("/api/accounts/email/request", methods=["POST"])
def email_request():
    """One flow for sign-in AND sign-up, so the response and the effect cannot reveal
    whether the address has an account (A4). Identical answer for every input."""
    g = _guard()
    if g:
        return g
    generic = jsonify(ok=True, message="If that address can receive it, we have sent a code.")
    data = request.get_json(silent=True) or {}
    email = (data.get("email") or "").strip()
    if not _rate_ok(f"ip:{auth.client_ip()}", 20, 3600):
        return generic
    if not auth.valid_email(email) or len(email) > 254:
        return generic
    email_lc = email.lower()
    if not _rate_ok(f"em:{email_lc}", CODE_PER_HOUR, 3600) or not _rate_ok(f"emc:{email_lc}", 1, CODE_COOLDOWN):
        return generic
    code = f"{secrets.randbelow(1000000):06d}"
    salt = secrets.token_hex(8)
    c = conn()
    c.execute(
        "INSERT OR REPLACE INTO email_codes(email_lc,salt,code_hash,expires_at,attempts,created_at) VALUES(?,?,?,?,0,?)",
        (email_lc, salt, _sha(salt + code), now() + CODE_TTL, now()),
    )
    c.commit()
    auth.send_email(
        email, "Your Spell sign-in code",
        f"<p>Your code is <strong>{code}</strong>. It works for 10 minutes. "
        f"If you did not ask for it, ignore this message.</p>",
    )
    _log("code_sent", None, email=email_tag(email_lc))
    return generic


@bp.route("/api/accounts/email/verify", methods=["POST"])
def email_verify():
    g = _guard()
    if g:
        return g
    data = request.get_json(silent=True) or {}
    bad = (jsonify(error="invalid_code"), 400)
    email_lc = (data.get("email") or "").strip().lower()
    code = str(data.get("code") or "")
    if not _rate_ok(f"vip:{auth.client_ip()}", 60, 3600):
        return jsonify(error="rate_limited"), 429
    if data.get("adult") is not True:
        return jsonify(error="adult_required"), 403          # A12: no row for an under-18 attempt
    if not auth.valid_email(email_lc) or not (code.isdigit() and len(code) == 6):
        return bad
    c = conn()
    row = c.execute("SELECT * FROM email_codes WHERE email_lc=?", (email_lc,)).fetchone()
    if not row or row["expires_at"] < now() or row["attempts"] >= CODE_MAX_ATTEMPTS:
        return bad
    if not hmac.compare_digest(row["code_hash"], _sha(row["salt"] + code)):
        c.execute("UPDATE email_codes SET attempts=attempts+1 WHERE email_lc=?", (email_lc,))
        c.commit()
        return bad
    c.execute("DELETE FROM email_codes WHERE email_lc=?", (email_lc,))   # single use
    u = c.execute("SELECT * FROM users WHERE email_lc=?", (email_lc,)).fetchone()
    if not u:
        cur = c.execute(
            "INSERT INTO users(email,email_lc,email_verified,created_at) VALUES(?,?,1,?)",
            (data["email"].strip(), email_lc, now()),
        )
        uid = cur.lastrowid
    else:
        uid = u["id"]
        c.execute("UPDATE users SET email_verified=1 WHERE id=?", (uid,))
    c.commit()
    _log("signin_email", uid)
    return jsonify(_issue_session(uid, _device(data)))


# ---------- Sign in with Apple ----------

_jwks_cache = {"at": 0, "keys": None}


def fetch_apple_jwks() -> dict:
    """Overridable in tests. Cached an hour."""
    if _jwks_cache["keys"] is None or now() - _jwks_cache["at"] > 3600:
        with urllib.request.urlopen(APPLE_ISS + "/auth/keys", timeout=8) as r:
            _jwks_cache.update(at=now(), keys=json.loads(r.read()))
    return _jwks_cache["keys"]


def verify_apple_token(token: str, nonce, jwks=None):
    """Claims for a valid Apple identity token, else None. Checks RS256 signature,
    issuer, audience, expiry and nonce. Apple puts SHA-256(nonce) in the token when the
    client sent the hash, so either the raw or the hashed form is accepted."""
    try:
        head_b, body_b, sig_b = token.split(".")
        header = json.loads(_unb64(head_b))
        if header.get("alg") != "RS256":
            return None
        keys = (jwks or fetch_apple_jwks())["keys"]
        jwk = next((k for k in keys if k.get("kid") == header.get("kid")), None)
        if not jwk:
            return None
        pub = rsa.RSAPublicNumbers(
            int.from_bytes(_unb64(jwk["e"]), "big"), int.from_bytes(_unb64(jwk["n"]), "big")
        ).public_key()
        pub.verify(_unb64(sig_b), f"{head_b}.{body_b}".encode(), padding.PKCS1v15(), hashes.SHA256())
        claims = json.loads(_unb64(body_b))
    except Exception:
        return None
    if claims.get("iss") != APPLE_ISS or claims.get("aud") != APPLE_AUD:
        return None
    if claims.get("exp", 0) <= now() or not claims.get("sub"):
        return None
    if not nonce or claims.get("nonce") not in (nonce, _sha(nonce)):
        return None
    return claims


def _enc_key():
    return base64.b64decode(TOKEN_ENC_KEY) if TOKEN_ENC_KEY else None


def _seal(plain: str):
    k = _enc_key()
    if not k or not plain:
        return None
    n = secrets.token_bytes(12)
    return n + AESGCM(k).encrypt(n, plain.encode(), b"apple-revoke")


def _unseal(blob):
    k = _enc_key()
    if not k or not blob:
        return None
    return AESGCM(k).decrypt(blob[:12], blob[12:], b"apple-revoke").decode()


def _apple_client_secret() -> str:
    with open(APPLE_P8_PATH, "rb") as fh:
        key = serialization.load_pem_private_key(fh.read(), password=None)
    t = int(now())
    return _jwt({"iss": APPLE_TEAM_ID, "iat": t, "exp": t + 600, "aud": APPLE_ISS, "sub": APPLE_AUD},
                key, {"kid": APPLE_KEY_ID})


def _apple_post(path: str, form: dict) -> dict:
    req = urllib.request.Request(APPLE_ISS + path, data=urllib.parse.urlencode(form).encode())
    with urllib.request.urlopen(req, timeout=8) as r:
        raw = r.read()
    return json.loads(raw) if raw else {}


def apple_exchange_code(auth_code: str):
    """Authorization code -> Apple refresh token, kept so deletion can revoke it."""
    try:
        out = _apple_post("/auth/token", {
            "client_id": APPLE_AUD, "client_secret": _apple_client_secret(),
            "code": auth_code, "grant_type": "authorization_code"})
        return out.get("refresh_token")
    except Exception:
        return None


def apple_revoke(refresh_token: str) -> bool:
    """Overridable in tests. True when Apple accepted the revocation."""
    try:
        _apple_post("/auth/revoke", {
            "client_id": APPLE_AUD, "client_secret": _apple_client_secret(),
            "token": refresh_token, "token_type_hint": "refresh_token"})
        return True
    except Exception:
        return False


@bp.route("/api/accounts/siwa", methods=["POST"])
def siwa():
    g = _guard()
    if g:
        return g
    data = request.get_json(silent=True) or {}
    if not _rate_ok(f"sip:{auth.client_ip()}", 60, 3600):
        return jsonify(error="rate_limited"), 429
    if data.get("adult") is not True:
        return jsonify(error="adult_required"), 403
    claims = verify_apple_token(data.get("identity_token") or "", data.get("nonce"))
    if not claims:
        return jsonify(error="invalid_token"), 401
    sub = claims["sub"]
    c = conn()
    u = c.execute("SELECT * FROM users WHERE apple_sub=?", (sub,)).fetchone()
    email = claims.get("email")           # private-relay addresses are ordinary emails
    if not u:
        email_lc = email.lower() if email else None
        owner = c.execute("SELECT * FROM users WHERE email_lc=?", (email_lc,)).fetchone() if email_lc else None
        if owner and claims.get("email_verified") in (True, "true"):
            c.execute("UPDATE users SET apple_sub=? WHERE id=?", (sub, owner["id"]))
            uid = owner["id"]
        else:
            cur = c.execute(
                "INSERT INTO users(apple_sub,email,email_lc,email_verified,created_at) VALUES(?,?,?,?,?)",
                (sub, email if not owner else None, email_lc if not owner else None,
                 1 if claims.get("email_verified") in (True, "true") and not owner else 0, now()),
            )
            uid = cur.lastrowid
    else:
        uid = u["id"]
    if data.get("authorization_code"):
        rt = apple_exchange_code(data["authorization_code"])
        if rt and _enc_key():
            c.execute("UPDATE users SET apple_revoke=? WHERE id=?", (_seal(rt), uid))
    c.commit()
    _log("signin_apple", uid)
    return jsonify(_issue_session(uid, _device(data)))


# ---------- refresh, logout ----------

@bp.route("/api/accounts/refresh", methods=["POST"])
def refresh():
    g = _guard()
    if g:
        return g
    tok = (request.get_json(silent=True) or {}).get("refresh_token") or ""
    c = conn()
    row = c.execute("SELECT * FROM sessions WHERE refresh_hash=?", (_sha(tok),)).fetchone()
    if not row:
        return jsonify(error="invalid_refresh"), 401
    if row["rotated_at"] is not None:
        revoke_family(row["family_id"])                  # A3: a replayed link burns the family
        _log("refresh_reuse", row["user_id"], family=row["family_id"][:8])
        return jsonify(error="invalid_refresh"), 401
    if row["revoked_at"] is not None or row["created_at"] + REFRESH_TTL < now():
        return jsonify(error="invalid_refresh"), 401
    c.execute("UPDATE sessions SET rotated_at=?, last_used_at=? WHERE id=?", (now(), now(), row["id"]))
    c.commit()
    return jsonify(_issue_session(row["user_id"], row["device_id"], row["family_id"]))


@bp.route("/api/accounts/logout", methods=["POST"])
def logout():
    h = request.headers.get("Authorization", "")
    claims = verify_access(h[7:]) if h.startswith("Bearer ") else None
    if claims:
        revoke_family(claims["sid"])
    return jsonify(ok=True)


@bp.route("/api/accounts/me")
def me():
    u = current_user()
    if not u:
        return jsonify(error="unauthorized"), 401
    return jsonify(user_id=u["id"], email_verified=bool(u["email_verified"]), has_apple=bool(u["apple_sub"]))


@bp.route("/api/accounts/public-key")
def public_key():
    g = _guard()
    if g:
        return g
    return jsonify(alg="ES256", kid=_kid(), pem=public_key_pem(), iss=ISSUER, aud=AUDIENCE)


# ---------- websocket tickets ----------

@bp.route("/api/accounts/ws-ticket", methods=["POST"])
def ws_ticket():
    """Single use, 60 seconds, minted over HTTPS (never a long-lived token in a URL)."""
    u = current_user()
    if not u:
        return jsonify(error="unauthorized"), 401
    t = secrets.token_urlsafe(24)
    c = conn()
    c.execute("DELETE FROM tickets WHERE expires_at < ?", (now(),))
    c.execute("INSERT INTO tickets(hash,user_id,expires_at) VALUES(?,?,?)", (_sha(t), u["id"], now() + TICKET_TTL))
    c.commit()
    return jsonify(ticket=t, expires_in=TICKET_TTL)


@bp.route("/api/accounts/internal/redeem-ticket", methods=["POST"])
def redeem_ticket():
    """Called by a room server. Burns the ticket; a second redeem finds nothing."""
    k = request.headers.get("X-Internal-Key", "")
    if not INTERNAL_KEY or not hmac.compare_digest(k, INTERNAL_KEY):
        return jsonify(error="forbidden"), 403
    t = (request.get_json(silent=True) or {}).get("ticket") or ""
    c = conn()
    row = c.execute("SELECT * FROM tickets WHERE hash=?", (_sha(t),)).fetchone()
    if not row:
        return jsonify(error="invalid_ticket"), 404
    c.execute("DELETE FROM tickets WHERE hash=?", (_sha(t),))
    c.commit()
    if row["expires_at"] < now():
        return jsonify(error="invalid_ticket"), 404
    return jsonify(user_id=row["user_id"])


# ---------- deletion ----------

def delete_user(user_id: int) -> dict:
    """I8: deletion deletes. Sessions die first, then Apple's token is revoked, then
    every row for the user goes (children, sync, entitlement binding, tickets)."""
    c = conn()
    u = c.execute("SELECT * FROM users WHERE id=?", (user_id,)).fetchone()
    if not u:
        return {"deleted": False}
    c.execute("UPDATE sessions SET revoked_at=? WHERE user_id=? AND revoked_at IS NULL", (now(), user_id))
    c.commit()
    revoked = None
    blob = u["apple_revoke"]
    if u["apple_sub"] and blob:
        tok = _unseal(blob)
        revoked = apple_revoke(tok) if tok else False
        _log("apple_revoke", user_id, ok=revoked)
    c.execute("DELETE FROM users WHERE id=?", (user_id,))    # CASCADE clears every dependent table
    c.commit()
    _log("deleted", user_id)
    return {"deleted": True, "apple_revoked": revoked}


@bp.route("/api/accounts/delete", methods=["POST"])
def delete_account():
    u = current_user()
    if not u:
        return jsonify(error="unauthorized"), 401
    out = delete_user(u["id"])
    return jsonify(ok=True, apple_revoked=out.get("apple_revoked"))


@bp.route("/api/accounts/apple/notify", methods=["POST"])
def apple_notify():
    """Apple's server-to-server notifications. The payload is a JWT signed by Apple;
    consent-revoked and account-delete run the same deletion as the in-app button."""
    body = request.get_json(silent=True) or {}
    payload = body.get("payload") or ""
    try:
        head_b, body_b, sig_b = payload.split(".")
        header = json.loads(_unb64(head_b))
        jwk = next((k for k in fetch_apple_jwks()["keys"] if k.get("kid") == header.get("kid")), None)
        pub = rsa.RSAPublicNumbers(
            int.from_bytes(_unb64(jwk["e"]), "big"), int.from_bytes(_unb64(jwk["n"]), "big")).public_key()
        pub.verify(_unb64(sig_b), f"{head_b}.{body_b}".encode(), padding.PKCS1v15(), hashes.SHA256())
        claims = json.loads(_unb64(body_b))
        if claims.get("iss") != APPLE_ISS or claims.get("aud") != APPLE_AUD:
            raise ValueError("iss/aud")
        ev = claims["events"]
        ev = json.loads(ev) if isinstance(ev, str) else ev
    except Exception:
        return jsonify(error="invalid_payload"), 400
    jti = claims.get("jti") or _sha(payload)
    c = conn()
    if c.execute("SELECT 1 FROM apple_events WHERE jti=?", (jti,)).fetchone():
        return jsonify(ok=True)                              # idempotent replay
    c.execute("INSERT INTO apple_events(jti,at) VALUES(?,?)", (jti, now()))
    c.commit()
    if ev.get("type") in ("consent-revoked", "account-delete"):
        u = c.execute("SELECT id FROM users WHERE apple_sub=?", (ev.get("sub"),)).fetchone()
        if u:
            # Apple has already revoked on its side; clear our copy so we do not call revoke again.
            c.execute("UPDATE users SET apple_revoke=NULL WHERE id=?", (u["id"],))
            c.commit()
            delete_user(u["id"])
    return jsonify(ok=True)
