"""CC-ONBOARD-JR Phase B — password rules (F9), 6-digit email codes (F8/F10),
and the removal of phone auth (F12/I12).

    CLIMB_DB_PATH=<tmp> python3 -m unittest test_auth_codes    # from backend/

These are the Done-criteria tests for the parts a player can be hurt by: a
password rule that admits "password1!", a code that can be guessed by patience,
a resend that never cools down, a reset response that reveals which emails have
accounts, and an SMS path that must no longer exist.
"""
import os
import tempfile
import time
import unittest

# A throwaway on-disk DB (WAL needs a real file) BEFORE importing db.
_TMP = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
_TMP.close()
os.environ["CLIMB_DB_PATH"] = _TMP.name

from flask import Flask  # noqa: E402

import auth  # noqa: E402
import climb  # noqa: E402
import db  # noqa: E402


def _client():
    app = Flask(__name__)
    app.register_blueprint(climb.bp)
    return app.test_client()


def _sent_codes(monkey):
    """Capture what send_email would have delivered, without sending."""
    box = []
    original = auth.send_email

    def fake(to, subject, html):
        box.append((to, subject, html))
        return True

    auth.send_email = fake
    monkey.addCleanup(lambda: setattr(auth, "send_email", original))
    return box


def _code_from(html):
    """The six digits out of the email body."""
    digits = "".join(ch for ch in html if ch.isdigit())
    return digits[-6:] if len(digits) >= 6 else digits


class PasswordRules(unittest.TestCase):
    """F9's table, verbatim from the spec, plus the rules it implies."""

    def test_spec_table(self):
        cases = [
            ("Example5%", None, "the spec's own passing example"),
            ("Example5", "symbol", "no symbol"),
            ("Exam5%", "8 characters", "six characters"),
            ("password1!", "too common", "on the blocklist"),
            ("abcdef-ghijk1-lmnop2", None, "an Apple-style suggested password"),
        ]
        for pw, expect, why in cases:
            problem = auth.password_problem(pw)
            if expect is None:
                self.assertIsNone(problem, f"{pw!r} should pass ({why}), got {problem!r}")
            else:
                self.assertIsNotNone(problem, f"{pw!r} should fail ({why})")
                self.assertIn(expect, problem, f"{pw!r}: unhelpful reason {problem!r}")

    def test_spaces_allowed_and_long_passphrases_fit(self):
        self.assertIsNone(auth.password_problem("correct horse battery 7!"))
        self.assertIsNone(auth.password_problem("x9!" + "a" * 61))  # 64 chars
        self.assertIsNotNone(auth.password_problem("x9!" + "a" * 500))

    def test_a_space_is_not_the_symbol(self):
        # Otherwise "password 1" would pass the symbol rule on a space.
        self.assertIn("symbol", auth.password_problem("abcdefg 1"))


class CodeFlow(unittest.TestCase):
    def setUp(self):
        db.init()
        c = db.conn()
        for t in ("email_codes", "email_sends", "sessions", "users"):
            c.execute(f"DELETE FROM {t}")
        c.commit()
        auth._rl.clear()  # the per-IP limiter is in-memory and shared across tests
        self.c = _client()
        self.box = _sent_codes(self)

    def request_code(self, email="a@b.co", purpose="signup"):
        return self.c.post("/api/auth/request-code", json={"email": email, "purpose": purpose})

    def test_signup_needs_a_code_then_a_password(self):
        r = self.request_code()
        self.assertEqual(r.status_code, 200)
        code = _code_from(self.box[-1][2])
        self.assertEqual(len(code), 6, f"D4 asks for six digits, got {code!r}")

        bad = self.c.post("/api/auth/complete-signup", json={
            "email": "a@b.co", "code": code, "username": "speller", "password": "Example5"})
        self.assertEqual(bad.status_code, 400, "the password rule is enforced on the server")

        ok = self.c.post("/api/auth/complete-signup", json={
            "email": "a@b.co", "code": code, "username": "speller", "password": "Example5%"})
        self.assertEqual(ok.status_code, 200, ok.get_json())
        body = ok.get_json()
        self.assertTrue(body["token"])
        self.assertEqual(body["user"]["username"], "speller")
        self.assertTrue(body["user"]["emailVerified"], "the code IS the verification")

        again = self.c.post("/api/auth/complete-signup", json={
            "email": "a@b.co", "code": code, "username": "other", "password": "Example5%"})
        self.assertEqual(again.status_code, 400, "a code is single-use")

    def test_code_expires(self):
        self.request_code()
        code = _code_from(self.box[-1][2])
        db.conn().execute("UPDATE email_codes SET expires_at=?", (time.time() - 1,))
        db.conn().commit()
        r = self.c.post("/api/auth/verify-code", json={"email": "a@b.co", "purpose": "signup", "code": code})
        self.assertEqual(r.status_code, 400, "an expired code is refused")

    def test_five_wrong_attempts_kill_the_code(self):
        self.request_code()
        code = _code_from(self.box[-1][2])
        wrong = "000000" if code != "000000" else "111111"
        for _ in range(auth.CODE_MAX_ATTEMPTS):
            self.c.post("/api/auth/verify-code", json={"email": "a@b.co", "purpose": "signup", "code": wrong})
        r = self.c.post("/api/auth/verify-code", json={"email": "a@b.co", "purpose": "signup", "code": code})
        self.assertEqual(r.status_code, 400, "the RIGHT code must not work after five wrong ones")

    def test_resend_cooldown_and_hourly_cap(self):
        self.assertEqual(self.request_code().status_code, 200)
        sent = len(self.box)
        self.assertEqual(self.request_code().status_code, 200, "the response stays generic")
        self.assertEqual(len(self.box), sent, "no second email inside the 60 s cooldown")

        # Walk past the cooldown repeatedly: the hourly cap still bites.
        for _ in range(auth.CODE_SENDS_PER_HOUR + 2):
            db.conn().execute("UPDATE email_sends SET sent_at = sent_at - ?", (auth.CODE_RESEND_COOLDOWN + 1,))
            db.conn().commit()
            self.request_code()
        self.assertLessEqual(len(self.box), auth.CODE_SENDS_PER_HOUR,
                             "D4 caps sends per email per hour")

    def test_reset_never_reveals_whether_an_account_exists(self):
        self.c.post("/api/auth/request-code", json={"email": "known@b.co", "purpose": "signup"})
        code = _code_from(self.box[-1][2])
        self.c.post("/api/auth/complete-signup", json={
            "email": "known@b.co", "code": code, "username": "known", "password": "Example5%"})

        known = self.c.post("/api/auth/request-code", json={"email": "known@b.co", "purpose": "reset"})
        unknown = self.c.post("/api/auth/request-code", json={"email": "nobody@b.co", "purpose": "reset"})
        self.assertEqual(known.status_code, unknown.status_code)
        self.assertEqual(known.get_json(), unknown.get_json(),
                         "identical body for known and unknown — no account enumeration")

    def test_reset_sets_the_password_and_drops_every_session(self):
        self.c.post("/api/auth/request-code", json={"email": "r@b.co", "purpose": "signup"})
        signup_code = _code_from(self.box[-1][2])
        signed = self.c.post("/api/auth/complete-signup", json={
            "email": "r@b.co", "code": signup_code, "username": "resetter", "password": "Example5%"})
        token = signed.get_json()["token"]
        self.assertIsNotNone(auth.session_user(token))

        db.conn().execute("DELETE FROM email_sends")  # past the cooldown
        db.conn().commit()
        self.c.post("/api/auth/request-code", json={"email": "r@b.co", "purpose": "reset"})
        reset_code = _code_from(self.box[-1][2])
        weak = self.c.post("/api/auth/reset-password", json={
            "email": "r@b.co", "code": reset_code, "newPassword": "letmein1!"})
        self.assertEqual(weak.status_code, 400, "the blocklist applies to resets too")

        ok = self.c.post("/api/auth/reset-password", json={
            "email": "r@b.co", "code": reset_code, "newPassword": "Newpass9%"})
        self.assertEqual(ok.status_code, 200, ok.get_json())
        self.assertIsNone(auth.session_user(token), "a reset revokes existing sessions")

        good = self.c.post("/api/auth/login", json={"identifier": "r@b.co", "password": "Newpass9%"})
        self.assertEqual(good.status_code, 200)

    def test_signup_on_a_taken_email_stays_generic(self):
        self.c.post("/api/auth/request-code", json={"email": "dup@b.co", "purpose": "signup"})
        code = _code_from(self.box[-1][2])
        self.c.post("/api/auth/complete-signup", json={
            "email": "dup@b.co", "code": code, "username": "dupe", "password": "Example5%"})
        db.conn().execute("DELETE FROM email_sends")
        db.conn().commit()
        again = self.request_code(email="dup@b.co")
        self.assertEqual(again.status_code, 200, "an existing email must not be revealed by status")
        self.assertNotIn("code", self.box[-1][2].lower().split("<b>")[0],
                         "no signup code is issued for an address that already has an account")


class PhoneAuthIsGone(unittest.TestCase):
    """F12 / I12 — zero phone fields, zero SMS paths."""

    def setUp(self):
        db.init()
        self.c = _client()

    def test_no_phone_columns(self):
        cols = [r["name"] for r in db.conn().execute("PRAGMA table_info(users)").fetchall()]
        self.assertNotIn("phone", cols)
        self.assertNotIn("phone_verified", cols)

    def test_no_sms_route_and_no_sender(self):
        self.assertEqual(self.c.post("/api/auth/request-reset-sms", json={}).status_code, 404)
        self.assertFalse(hasattr(auth, "send_sms"), "the SMS sender must not exist at all")

    def test_the_old_link_routes_are_gone(self):
        for path in ("/api/auth/verify-email", "/api/auth/request-reset-email"):
            r = self.c.get(path) if path.endswith("verify-email") else self.c.post(path, json={})
            self.assertEqual(r.status_code, 404, f"{path} should no longer exist")

    def test_the_old_signup_route_tells_an_old_build_to_update(self):
        r = self.c.post("/api/auth/signup", json={"username": "x", "email": "x@y.co", "password": "Example5%"})
        self.assertEqual(r.status_code, 410, "an installed older build gets a clear answer, not a 404")
        self.assertIn("update", (r.get_json() or {}).get("error", "").lower())


if __name__ == "__main__":
    unittest.main()
