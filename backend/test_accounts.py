"""CC-ACCOUNTS Phase 1 acceptance: A1-A5, A9, A10, A12, plus the ticket and kill-flag rules.

    ~/spellgame-server/venv/bin/python -m unittest test_accounts    # from backend/

No network: Apple's keys and revoke endpoint are replaced with local stand-ins, and mail is
captured instead of sent. Room-server enforcement (A1/A2 as a 401/403 on a room join) is Phase 4;
here A1 is checked at the token layer a room server will call.
"""
import base64
import json
import os
import tempfile
import time
import unittest

_TMP = tempfile.mkdtemp()
os.environ["ACCOUNTS_DB_PATH"] = os.path.join(_TMP, "accounts.db")
os.environ["ACCOUNTS_JWT_KEY_PATH"] = os.path.join(_TMP, "jwt.pem")
os.environ["ACCOUNTS_TOKEN_ENC_KEY"] = base64.b64encode(os.urandom(32)).decode()
os.environ["ACCOUNTS_INTERNAL_KEY"] = "internal-test-key"
os.environ["KILL_FLAGS_FILE"] = os.path.join(_TMP, "kill.json")

from cryptography.hazmat.primitives import hashes, serialization  # noqa: E402
from cryptography.hazmat.primitives.asymmetric import ec, padding, rsa  # noqa: E402
from flask import Flask  # noqa: E402

_k = ec.generate_private_key(ec.SECP256R1())
with open(os.environ["ACCOUNTS_JWT_KEY_PATH"], "wb") as fh:
    fh.write(_k.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
                              serialization.NoEncryption()))

import accounts  # noqa: E402
import auth  # noqa: E402

# A stand-in Apple: one RSA key, a JWKS for it, and a signer for identity tokens.
_apple = rsa.generate_private_key(public_exponent=65537, key_size=2048)
_nums = _apple.public_key().public_numbers()


def _i2b(n):
    return accounts._b64(n.to_bytes((n.bit_length() + 7) // 8, "big"))


JWKS = {"keys": [{"kid": "applekid", "kty": "RSA", "n": _i2b(_nums.n), "e": _i2b(_nums.e)}]}


def apple_jwt(claims, kid="applekid"):
    head = accounts._b64(json.dumps({"alg": "RS256", "kid": kid}).encode())
    body = accounts._b64(json.dumps(claims).encode())
    sig = _apple.sign(f"{head}.{body}".encode(), padding.PKCS1v15(), hashes.SHA256())
    return f"{head}.{body}.{accounts._b64(sig)}"


def id_token(sub="001.abc", nonce="n-1", **over):
    c = {"iss": accounts.APPLE_ISS, "aud": accounts.APPLE_AUD, "sub": sub, "nonce": nonce,
         "exp": int(time.time()) + 600, "email": "relay@privaterelay.appleid.com", "email_verified": "true"}
    c.update(over)
    return apple_jwt(c)


class Base(unittest.TestCase):
    def setUp(self):
        accounts.conn().executescript(
            "DELETE FROM users; DELETE FROM sessions; DELETE FROM email_codes; DELETE FROM rate_events;"
            "DELETE FROM tickets; DELETE FROM apple_events; DELETE FROM sync_records; DELETE FROM child_profiles;")
        accounts.conn().commit()
        app = Flask(__name__)
        app.register_blueprint(accounts.bp)
        self.c = app.test_client()
        self.mail, self.revoked = [], []
        self._orig = (auth.send_email, accounts.fetch_apple_jwks, accounts.apple_revoke)
        auth.send_email = lambda to, subj, html: self.mail.append((to, subj, html)) or True
        accounts.fetch_apple_jwks = lambda: JWKS
        accounts.apple_revoke = lambda tok: self.revoked.append(tok) or True
        if os.path.exists(os.environ["KILL_FLAGS_FILE"]):
            os.unlink(os.environ["KILL_FLAGS_FILE"])

    def tearDown(self):
        auth.send_email, accounts.fetch_apple_jwks, accounts.apple_revoke = self._orig

    def post(self, path, body=None, token=None, headers=None):
        h = dict(headers or {})
        if token:
            h["Authorization"] = f"Bearer {token}"
        return self.c.post(path, json=body or {}, headers=h)

    def code_from_mail(self):
        import re
        return re.search(r"<strong>(\d{6})</strong>", self.mail[-1][2]).group(1)

    def sign_in_email(self, email="a@example.com"):
        self.post("/api/accounts/email/request", {"email": email})
        r = self.post("/api/accounts/email/verify", {"email": email, "code": self.code_from_mail(), "adult": True})
        self.assertEqual(r.status_code, 200)
        return r.get_json()


class TokenLayer(Base):
    def test_a1_no_token_and_expired_token_are_refused(self):
        self.assertEqual(self.post("/api/accounts/ws-ticket").status_code, 401)
        s = self.sign_in_email()
        old = accounts.ACCESS_TTL
        accounts.ACCESS_TTL = -5
        try:
            expired = accounts.mint_access(s["user_id"], "fam")
        finally:
            accounts.ACCESS_TTL = old
        self.assertIsNone(accounts.verify_access(expired))
        self.assertEqual(self.post("/api/accounts/ws-ticket", token=expired).status_code, 401)

    def test_room_server_verifies_with_public_key_alone(self):
        s = self.sign_in_email()
        pem = self.c.get("/api/accounts/public-key").get_json()["pem"]
        self.assertNotIn("PRIVATE", pem)
        self.assertEqual(accounts.verify_access(s["access_token"], pem)["sub"], str(s["user_id"]))
        h, b, sig = s["access_token"].split(".")
        forged = f"{h}.{b}.{accounts._b64(bytes(64))}"
        self.assertIsNone(accounts.verify_access(forged, pem))
        none_alg = accounts._b64(json.dumps({"alg": "none"}).encode()) + f".{b}."
        self.assertIsNone(accounts.verify_access(none_alg, pem))

    def test_a3_replayed_refresh_revokes_the_family(self):
        s = self.sign_in_email()
        r1 = self.post("/api/accounts/refresh", {"refresh_token": s["refresh_token"]})
        self.assertEqual(r1.status_code, 200)
        new = r1.get_json()
        self.assertEqual(self.post("/api/accounts/refresh", {"refresh_token": s["refresh_token"]}).status_code, 401)
        # the replay burned the family: even the newest link is dead, and so is its access token here
        self.assertEqual(self.post("/api/accounts/refresh", {"refresh_token": new["refresh_token"]}).status_code, 401)
        self.assertEqual(self.c.get("/api/accounts/me", headers={"Authorization": f"Bearer {new['access_token']}"}).status_code, 401)

    def test_ticket_is_single_use_and_expires(self):
        s = self.sign_in_email()
        t = self.post("/api/accounts/ws-ticket", token=s["access_token"]).get_json()["ticket"]
        key = {"X-Internal-Key": "internal-test-key"}
        self.assertEqual(self.post("/api/accounts/internal/redeem-ticket", {"ticket": t}).status_code, 403)
        ok = self.post("/api/accounts/internal/redeem-ticket", {"ticket": t}, headers=key)
        self.assertEqual(ok.get_json()["user_id"], s["user_id"])
        self.assertEqual(self.post("/api/accounts/internal/redeem-ticket", {"ticket": t}, headers=key).status_code, 404)
        t2 = self.post("/api/accounts/ws-ticket", token=s["access_token"]).get_json()["ticket"]
        accounts.conn().execute("UPDATE tickets SET expires_at=?", (time.time() - 1,))
        accounts.conn().commit()
        self.assertEqual(self.post("/api/accounts/internal/redeem-ticket", {"ticket": t2}, headers=key).status_code, 404)


class EmailCode(Base):
    def test_a4_five_wrong_codes_burn_it(self):
        self.post("/api/accounts/email/request", {"email": "b@example.com"})
        good = self.code_from_mail()
        wrong = "000000" if good != "000000" else "111111"
        for _ in range(5):
            r = self.post("/api/accounts/email/verify", {"email": "b@example.com", "code": wrong, "adult": True})
            self.assertEqual(r.status_code, 400)
        burned = self.post("/api/accounts/email/verify", {"email": "b@example.com", "code": good, "adult": True})
        self.assertEqual(burned.status_code, 400)

    def test_a4_identical_response_known_and_unknown(self):
        self.sign_in_email("known@example.com")
        a = self.post("/api/accounts/email/request", {"email": "known@example.com"})
        b = self.post("/api/accounts/email/request", {"email": "unknown@example.com"})
        self.assertEqual((a.status_code, a.get_json()), (b.status_code, b.get_json()))
        w1 = self.post("/api/accounts/email/verify", {"email": "known@example.com", "code": "123456", "adult": True})
        w2 = self.post("/api/accounts/email/verify", {"email": "nobody@example.com", "code": "123456", "adult": True})
        self.assertEqual((w1.status_code, w1.get_json()), (w2.status_code, w2.get_json()))

    def test_a4_reused_code_fails(self):
        self.post("/api/accounts/email/request", {"email": "c@example.com"})
        code = self.code_from_mail()
        body = {"email": "c@example.com", "code": code, "adult": True}
        self.assertEqual(self.post("/api/accounts/email/verify", body).status_code, 200)
        self.assertEqual(self.post("/api/accounts/email/verify", body).status_code, 400)

    def test_expired_code_fails(self):
        self.post("/api/accounts/email/request", {"email": "d@example.com"})
        code = self.code_from_mail()
        accounts.conn().execute("UPDATE email_codes SET expires_at=?", (time.time() - 1,))
        accounts.conn().commit()
        r = self.post("/api/accounts/email/verify", {"email": "d@example.com", "code": code, "adult": True})
        self.assertEqual(r.status_code, 400)

    def test_resend_cooldown_sends_once(self):
        for _ in range(3):
            self.post("/api/accounts/email/request", {"email": "e@example.com"})
        self.assertEqual(len(self.mail), 1)

    def test_a12_no_attestation_no_row(self):
        self.post("/api/accounts/email/request", {"email": "kid@example.com"})
        r = self.post("/api/accounts/email/verify", {"email": "kid@example.com", "code": self.code_from_mail()})
        self.assertEqual(r.status_code, 403)
        r = self.post("/api/accounts/siwa", {"identity_token": id_token(), "nonce": "n-1"})
        self.assertEqual(r.status_code, 403)
        self.assertEqual(accounts.conn().execute("SELECT COUNT(*) FROM users").fetchone()[0], 0)

    def test_code_is_not_stored_in_clear(self):
        self.post("/api/accounts/email/request", {"email": "f@example.com"})
        code = self.code_from_mail()
        row = accounts.conn().execute("SELECT * FROM email_codes").fetchone()
        self.assertNotIn(code, tuple(str(v) for v in row))


class Siwa(Base):
    def go(self, tok, nonce="n-1", **extra):
        return self.post("/api/accounts/siwa", {"identity_token": tok, "nonce": nonce, "adult": True, **extra})

    def test_a5_rejections(self):
        self.assertEqual(self.go(id_token()).status_code, 200)
        self.assertEqual(self.go(id_token(aud="com.someone.else")).status_code, 401)
        self.assertEqual(self.go(id_token(exp=int(time.time()) - 5)).status_code, 401)
        self.assertEqual(self.go(id_token(), nonce="other").status_code, 401)
        self.assertEqual(self.go(id_token(iss="https://evil.example")).status_code, 401)
        self.assertEqual(self.go(id_token(), nonce="").status_code, 401)
        self.assertEqual(self.go(id_token(), nonce=None).status_code, 401)
        self.assertEqual(self.go(apple_jwt({"sub": "x"}, kid="unknown")).status_code, 401)
        tampered = id_token().rsplit(".", 1)[0] + "." + accounts._b64(os.urandom(256))
        self.assertEqual(self.go(tampered).status_code, 401)

    def test_hashed_nonce_accepted_and_same_sub_same_user(self):
        a = self.go(id_token(nonce=accounts._sha("n-1"))).get_json()["user_id"]
        b = self.go(id_token()).get_json()["user_id"]
        self.assertEqual(a, b)

    def test_relay_email_is_an_ordinary_email(self):
        self.go(id_token())
        row = accounts.conn().execute("SELECT * FROM users").fetchone()
        self.assertEqual(row["email"], "relay@privaterelay.appleid.com")


class Deletion(Base):
    def seeded_apple_user(self):
        accounts.apple_exchange_code = lambda code: "apple-refresh-token"
        s = self.post("/api/accounts/siwa", {"identity_token": id_token(), "nonce": "n-1", "adult": True,
                                             "authorization_code": "authcode"}).get_json()
        uid = s["user_id"]
        c = accounts.conn()
        c.execute("INSERT INTO child_profiles(parent_id,generated_name,created_at) VALUES(?,?,?)", (uid, "Brave Bee 42", 1))
        c.execute("INSERT INTO sync_records(user_id,kind,record_id,payload,updated_at) VALUES(?,?,?,?,?)", (uid, "word", "w1", "{}", 1))
        c.execute("INSERT INTO entitlements(user_id,product_id,transaction_id,verified_at) VALUES(?,?,?,?)", (uid, "p", "t", 1))
        c.commit()
        return s

    def count_rows(self, uid):
        c = accounts.conn()
        return sum(c.execute(q, (uid,)).fetchone()[0] for q in (
            "SELECT COUNT(*) FROM users WHERE id=?", "SELECT COUNT(*) FROM sessions WHERE user_id=?",
            "SELECT COUNT(*) FROM child_profiles WHERE parent_id=?", "SELECT COUNT(*) FROM sync_records WHERE user_id=?",
            "SELECT COUNT(*) FROM entitlements WHERE user_id=?", "SELECT COUNT(*) FROM tickets WHERE user_id=?"))

    def test_a9_delete_account(self):
        s = self.seeded_apple_user()
        self.assertGreater(self.count_rows(s["user_id"]), 0)
        r = self.post("/api/accounts/delete", token=s["access_token"])
        self.assertEqual(r.status_code, 200)
        self.assertEqual(self.revoked, ["apple-refresh-token"])
        self.assertEqual(self.count_rows(s["user_id"]), 0)
        self.assertEqual(self.post("/api/accounts/refresh", {"refresh_token": s["refresh_token"]}).status_code, 401)
        self.assertEqual(self.c.get("/api/accounts/me", headers={"Authorization": f"Bearer {s['access_token']}"}).status_code, 401)

    def test_apple_token_not_stored_in_clear(self):
        self.seeded_apple_user()
        blob = accounts.conn().execute("SELECT apple_revoke FROM users").fetchone()[0]
        self.assertNotIn(b"apple-refresh-token", blob)

    def test_a10_consent_revoked_runs_the_same_deletion(self):
        s = self.seeded_apple_user()
        ev = json.dumps({"type": "consent-revoked", "sub": "001.abc", "event_time": 1})
        payload = apple_jwt({"iss": accounts.APPLE_ISS, "aud": accounts.APPLE_AUD, "jti": "j1", "events": ev})
        r = self.post("/api/accounts/apple/notify", {"payload": payload})
        self.assertEqual(r.status_code, 200)
        self.assertEqual(self.count_rows(s["user_id"]), 0)
        self.assertEqual(self.post("/api/accounts/apple/notify", {"payload": payload}).status_code, 200)  # replay is harmless

    def test_notify_rejects_unsigned_and_wrong_audience(self):
        s = self.seeded_apple_user()
        ev = json.dumps({"type": "account-delete", "sub": "001.abc"})
        bad_aud = apple_jwt({"iss": accounts.APPLE_ISS, "aud": "other", "jti": "j2", "events": ev})
        self.assertEqual(self.post("/api/accounts/apple/notify", {"payload": bad_aud}).status_code, 400)
        forged = bad_aud.rsplit(".", 1)[0] + "." + accounts._b64(os.urandom(256))
        self.assertEqual(self.post("/api/accounts/apple/notify", {"payload": forged}).status_code, 400)
        self.assertEqual(self.post("/api/accounts/apple/notify", {"payload": "junk"}).status_code, 400)
        self.assertGreater(self.count_rows(s["user_id"]), 0)


class Flags(Base):
    def test_server_flag_can_only_kill(self):
        self.assertEqual(self.post("/api/accounts/email/request", {"email": "g@example.com"}).status_code, 200)
        with open(os.environ["KILL_FLAGS_FILE"], "w") as fh:
            json.dump({"v": 1, "kill": ["accounts_enabled"]}, fh)
        self.assertEqual(self.post("/api/accounts/email/request", {"email": "g@example.com"}).status_code, 503)
        with open(os.environ["KILL_FLAGS_FILE"], "w") as fh:
            json.dump({"v": 1, "kill": [], "on": ["accounts_enabled"]}, fh)     # the file can name nothing "on"
        self.assertEqual(self.post("/api/accounts/email/request", {"email": "g@example.com"}).status_code, 200)

    def test_missing_key_is_503_not_autogenerated(self):
        old = accounts.KEY_PATH
        accounts.KEY_PATH = os.path.join(_TMP, "absent.pem")
        try:
            self.assertEqual(self.post("/api/accounts/email/request", {"email": "h@example.com"}).status_code, 503)
        finally:
            accounts.KEY_PATH = old
        self.assertFalse(os.path.exists(os.path.join(_TMP, "absent.pem")))


class Sync(Base):
    def rec(self, rid, ts, payload='{"n":1}', kind="list", deleted=False, profile=""):
        return {"kind": kind, "record_id": rid, "profile_id": profile, "payload": payload,
                "updated_at": ts, "deleted": deleted}

    def push(self, tok, records=None, since=0):
        return self.post("/api/accounts/sync", {"records": records or [], "since": since}, token=tok)

    def two_devices(self):
        a = self.sign_in_email("same@example.com")
        self.mail.clear()
        accounts.conn().execute("DELETE FROM rate_events")
        accounts.conn().commit()
        b = self.sign_in_email("same@example.com")
        self.assertEqual(a["user_id"], b["user_id"])
        return a["access_token"], b["access_token"]

    def test_a8_added_on_one_device_appears_on_the_other_and_delete_propagates(self):
        da, db = self.two_devices()
        r = self.push(da, [self.rec("g1", 1000.0, '{"name":"Animals"}')]).get_json()
        self.assertEqual(r["accepted"], 1)
        pulled = self.push(db, since=0).get_json()
        self.assertEqual([x["record_id"] for x in pulled["records"]], ["g1"])
        self.assertEqual(json.loads(pulled["records"][0]["payload"])["name"], "Animals")
        cur = pulled["cursor"]
        # device B deletes it; device A, pulling from its own cursor, sees the tombstone
        self.push(db, [self.rec("g1", 2000.0, deleted=True)], since=cur)
        seen = self.push(da, since=cur).get_json()["records"]
        self.assertEqual([(x["record_id"], x["deleted"], x["payload"]) for x in seen], [("g1", True, None)])

    def test_last_write_wins_ties_and_resurrection(self):
        t = self.sign_in_email("lww@example.com")["access_token"]
        self.push(t, [self.rec("a", 100.0, '{"v":"old"}')])
        self.push(t, [self.rec("a", 200.0, '{"v":"new"}')])
        self.push(t, [self.rec("a", 150.0, '{"v":"stale"}')])           # older: ignored
        self.push(t, [self.rec("a", 200.0, '{"v":"tie"}')])             # tie: the server's copy stands
        got = {x["record_id"]: x for x in self.push(t).get_json()["records"]}
        self.assertEqual(json.loads(got["a"]["payload"])["v"], "new")
        self.push(t, [self.rec("a", 300.0, deleted=True)])
        self.push(t, [self.rec("a", 250.0, '{"v":"late"}')])            # older than the tombstone: stays deleted
        self.assertTrue(self.push(t).get_json()["records"][0]["deleted"])
        self.push(t, [self.rec("a", 400.0, '{"v":"back"}')])            # newer than the tombstone: resurrects
        row = self.push(t).get_json()["records"][0]
        self.assertFalse(row["deleted"])
        self.assertEqual(json.loads(row["payload"])["v"], "back")

    def test_replayed_push_changes_nothing(self):
        t = self.sign_in_email("replay@example.com")["access_token"]
        batch = [self.rec("x", 10.0), self.rec("y", 11.0)]
        first = self.push(t, batch).get_json()
        again = self.push(t, batch, since=first["cursor"]).get_json()
        self.assertEqual((again["accepted"], again["records"]), (0, []))

    def test_a_clock_in_the_future_cannot_win_forever(self):
        t = self.sign_in_email("future@example.com")["access_token"]
        self.push(t, [self.rec("f", 9e15, '{"v":"far"}')])
        stored = accounts.conn().execute("SELECT updated_at FROM sync_records").fetchone()[0]
        self.assertLess(stored, (time.time() + 600) * 1000)

    def test_cursor_pages_through_everything(self):
        t = self.sign_in_email("pages@example.com")["access_token"]
        old = accounts.PULL_PAGE
        accounts.PULL_PAGE = 3
        try:
            self.push(t, [self.rec(f"r{i}", 1.0 + i) for i in range(8)])
            seen, cur = [], 0
            for _ in range(5):
                r = self.push(t, since=cur).get_json()
                seen += [x["record_id"] for x in r["records"]]
                cur = r["cursor"]
                if not r["more"]:
                    break
        finally:
            accounts.PULL_PAGE = old
        self.assertEqual(sorted(seen), sorted(f"r{i}" for i in range(8)))

    def test_requires_a_session_and_validates_records(self):
        self.assertEqual(self.post("/api/accounts/sync", {}).status_code, 401)
        t = self.sign_in_email("val@example.com")["access_token"]
        bad = [self.rec("k", 1.0, kind="nonsense"), self.rec("", 1.0), self.rec("p", 1.0, payload="not json"),
               self.rec("q", -5.0), self.rec("big", 1.0, payload='"' + "x" * 70000 + '"'),
               self.rec("pp", 1.0, profile="3"), {"junk": True}, "string"]
        r = self.push(t, bad).get_json()
        self.assertEqual((r["accepted"], r["rejected"]), (0, len(bad)))
        self.assertEqual(self.post("/api/accounts/sync", {"records": [0] * 501}, token=t).status_code, 400)
        self.assertEqual(self.post("/api/accounts/sync", {"since": "x"}, token=t).status_code, 400)

    def test_users_cannot_see_each_others_records(self):
        a = self.sign_in_email("one@example.com")["access_token"]
        accounts.conn().execute("DELETE FROM rate_events")
        accounts.conn().commit()
        b = self.sign_in_email("two@example.com")["access_token"]
        self.push(a, [self.rec("mine", 1.0)])
        self.assertEqual(self.push(b).get_json()["records"], [])

    def test_a13_child_sync_off_leaves_no_child_rows(self):
        s = self.sign_in_email("parent@example.com")
        t, uid = s["access_token"], s["user_id"]
        c = accounts.conn()
        c.execute("INSERT INTO child_profiles(id,parent_id,generated_name,created_at) VALUES(5,?,?,1)", (uid, "Brave Bee 42"))
        c.commit()
        kid = self.rec("w1", 1.0, kind="child_progress", profile="5")
        self.assertEqual(self.push(t, [kid]).get_json()["accepted"], 1)          # default on (D11)
        self.assertEqual(self.post("/api/accounts/child-sync", {"enabled": False}, token=t).status_code, 200)
        self.assertEqual(c.execute("SELECT COUNT(*) FROM sync_records WHERE profile_id!=''").fetchone()[0], 0)
        r = self.push(t, [self.rec("w2", 2.0, kind="child_progress", profile="5")]).get_json()
        self.assertEqual((r["accepted"], r["rejected"]), (0, 1))
        self.assertEqual(c.execute("SELECT COUNT(*) FROM sync_records WHERE profile_id!=''").fetchone()[0], 0)
        # a profile that is not this parent's is refused even with sync on
        self.post("/api/accounts/child-sync", {"enabled": True}, token=t)
        r = self.push(t, [self.rec("w3", 3.0, kind="child_progress", profile="99")]).get_json()
        self.assertEqual(r["accepted"], 0)

    def test_export_returns_the_records_as_json(self):
        s = self.sign_in_email("export@example.com")
        self.push(s["access_token"], [self.rec("e1", 5.0, '{"name":"Kept"}')])
        r = self.c.get("/api/accounts/export", headers={"Authorization": f"Bearer {s['access_token']}"})
        body = r.get_json()
        self.assertEqual(body["records"][0]["payload"], {"name": "Kept"})
        self.assertEqual(body["account"]["user_id"], s["user_id"])
        self.assertEqual(self.c.get("/api/accounts/export").status_code, 401)

    def test_delete_account_takes_the_synced_rows_with_it(self):
        s = self.sign_in_email("gone@example.com")
        self.push(s["access_token"], [self.rec("z", 1.0)])
        self.post("/api/accounts/delete", token=s["access_token"])
        self.assertEqual(accounts.conn().execute("SELECT COUNT(*) FROM sync_records").fetchone()[0], 0)

    def test_sync_kill_flag_only_kills_sync(self):
        t = self.sign_in_email("kill@example.com")["access_token"]
        with open(os.environ["KILL_FLAGS_FILE"], "w") as fh:
            json.dump({"v": 1, "kill": ["accounts_sync"]}, fh)
        self.assertEqual(self.push(t).status_code, 503)
        self.assertEqual(self.c.get("/api/accounts/me", headers={"Authorization": f"Bearer {t}"}).status_code, 200)


class Migration(Base):
    def test_climb_ids_kept_and_rerun_is_safe(self):
        import sqlite3
        import migrate_climb_accounts as m
        path = os.path.join(_TMP, "climb.db")
        src = sqlite3.connect(path)
        src.executescript("DROP TABLE IF EXISTS users; CREATE TABLE users(id INTEGER, email TEXT, email_lc TEXT, "
                          "email_verified INTEGER, created_at REAL, pw_hash TEXT);")
        src.executemany("INSERT INTO users VALUES(?,?,?,?,?,?)", [
            (7, "A@x.com", "a@x.com", 1, 1.0, "hash"), (9, "b@x.com", "b@x.com", 1, 2.0, "hash")])
        src.commit()
        self.assertEqual(m.migrate(path, False)["copied"], 2)
        self.assertEqual(accounts.conn().execute("SELECT COUNT(*) FROM users").fetchone()[0], 0)
        self.assertEqual(m.migrate(path, True)["copied"], 2)
        self.assertEqual([r[0] for r in accounts.conn().execute("SELECT id FROM users ORDER BY id")], [7, 9])
        self.assertEqual(m.migrate(path, True), {"copied": 0, "skipped": 2, "applied": True})
        cols = [r[1] for r in accounts.conn().execute("PRAGMA table_info(users)")]
        self.assertNotIn("pw_hash", cols)
        # a migrated player signs in by emailed code and lands on the same id
        self.post("/api/accounts/email/request", {"email": "a@x.com"})
        r = self.post("/api/accounts/email/verify", {"email": "a@x.com", "code": self.code_from_mail(), "adult": True})
        self.assertEqual(r.get_json()["user_id"], 7)


class Logs(Base):
    def test_logs_carry_no_email_code_or_token(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            s = self.sign_in_email("secret.person@example.com")
        out = buf.getvalue()
        self.assertNotIn("secret.person", out)
        self.assertNotIn(s["access_token"], out)
        self.assertNotIn(s["refresh_token"], out)
        self.assertNotIn(self.code_from_mail(), out)


if __name__ == "__main__":
    unittest.main()
