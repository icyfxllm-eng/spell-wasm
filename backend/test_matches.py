"""Async "Spell Off" match regression tests (stdlib unittest -> no pytest):

    python3 -m unittest backend.test_matches    # from repo root
    python3 -m unittest test_matches             # from backend/

Drives the whole 1v1 flow against a throwaway sqlite DB and a Flask test client:
create -> join -> both submit -> winner is correct; you can't join your own
match; unauthenticated calls are rejected. Auth is exercised via the real
session table (a Bearer token per user), so the account gate is covered too.
"""
import os
import tempfile
import unittest

# Point the data layer at a temp DB BEFORE importing db/app (db reads the env at
# import time via DB_PATH). Each test run gets a fresh file.
_TMP = tempfile.mkdtemp(prefix="matches-test-")
os.environ["CLIMB_DB_PATH"] = os.path.join(_TMP, "test.db")
os.environ.setdefault("SESSION_COOKIE_SECURE", "0")

import auth  # noqa: E402
import db  # noqa: E402
import matches  # noqa: E402

from flask import Flask  # noqa: E402


def _app():
    app = Flask(__name__)
    app.register_blueprint(matches.bp)
    return app


class MatchFlow(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        db.init()
        cls.app = _app()

    def setUp(self):
        # Fresh users + client per test (unique emails so signup never collides).
        self.client = self.app.test_client()
        self.uid_a, self.tok_a = self._make_user("alice")
        self.uid_b, self.tok_b = self._make_user("bob")

    def _make_user(self, name):
        import time
        c = db.conn()
        email = f"{name}{time.time_ns()}@example.test"
        cur = c.execute(
            "INSERT INTO users(username,username_lc,email,email_lc,pw_hash,created_at) "
            "VALUES(?,?,?,?,?,?)",
            (name, f"{name}{time.time_ns()}", email, email, auth.hash_password("pw"), time.time()),
        )
        c.commit()
        uid = cur.lastrowid
        tok = auth.create_session(uid)
        return uid, tok

    def _hdr(self, tok):
        return {"Authorization": f"Bearer {tok}"}

    # ---- happy path -------------------------------------------------------

    def test_create_join_submit_declares_winner(self):
        # A creates.
        r = self.client.post("/api/match", json={"lang": "en", "tier": "medium"}, headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 200, r.get_data(as_text=True))
        m = r.get_json()
        code, seed = m["code"], m["seed"]
        self.assertEqual(m["status"], "open")
        self.assertEqual(m["you"], "a")
        self.assertTrue(seed and len(seed) == 16)      # 64-bit hex seed
        self.assertEqual(m["wordCount"], matches.WORD_COUNT)

        # B joins — same seed comes back, status flips to active.
        r = self.client.post(f"/api/match/{code}/join", headers=self._hdr(self.tok_b))
        self.assertEqual(r.status_code, 200, r.get_data(as_text=True))
        jm = r.get_json()
        self.assertEqual(jm["status"], "active")
        self.assertEqual(jm["you"], "b")
        self.assertEqual(jm["seed"], seed)             # both play the SAME words

        # A submits the weaker result.
        r = self.client.post(f"/api/match/{code}/result",
                             json={"score": 100, "correct": 6, "total": 10, "elapsed_ms": 30000},
                             headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 200, r.get_data(as_text=True))
        self.assertEqual(r.get_json()["status"], "active")   # waiting for B
        self.assertIsNone(r.get_json()["winner"])

        # B submits the stronger result -> match completes, B wins (more correct).
        r = self.client.post(f"/api/match/{code}/result",
                             json={"score": 150, "correct": 9, "total": 10, "elapsed_ms": 40000},
                             headers=self._hdr(self.tok_b))
        self.assertEqual(r.status_code, 200, r.get_data(as_text=True))
        done = r.get_json()
        self.assertEqual(done["status"], "complete")
        self.assertEqual(done["winner"], "b")

        # GET reflects the final state for either player.
        r = self.client.get(f"/api/match/{code}", headers=self._hdr(self.tok_a))
        self.assertEqual(r.get_json()["winner"], "b")

    def test_tiebreak_by_elapsed(self):
        code = self._open_active()
        # Equal correct — faster (lower elapsed) wins.
        self.client.post(f"/api/match/{code}/result",
                         json={"score": 100, "correct": 7, "total": 10, "elapsed_ms": 50000},
                         headers=self._hdr(self.tok_a))
        r = self.client.post(f"/api/match/{code}/result",
                            json={"score": 100, "correct": 7, "total": 10, "elapsed_ms": 20000},
                            headers=self._hdr(self.tok_b))
        self.assertEqual(r.get_json()["winner"], "b")   # B faster

    # ---- guards -----------------------------------------------------------

    def test_cannot_join_own_match(self):
        code = self._create(self.tok_a)
        r = self.client.post(f"/api/match/{code}/join", headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 400)

    def test_cannot_join_full_match(self):
        code = self._open_active()
        # A third user tries to join a full match.
        _, tok_c = self._make_user("carol")
        r = self.client.post(f"/api/match/{code}/join", headers=self._hdr(tok_c))
        self.assertEqual(r.status_code, 409)

    def test_double_submit_rejected(self):
        code = self._open_active()
        self.client.post(f"/api/match/{code}/result",
                         json={"score": 1, "correct": 1, "total": 10, "elapsed_ms": 1000},
                         headers=self._hdr(self.tok_a))
        r = self.client.post(f"/api/match/{code}/result",
                            json={"score": 1, "correct": 1, "total": 10, "elapsed_ms": 1000},
                            headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 409)

    def test_unauth_rejected_on_every_route(self):
        self.assertEqual(self.client.post("/api/match", json={"lang": "en", "tier": "medium"}).status_code, 401)
        code = self._create(self.tok_a)
        self.assertEqual(self.client.post(f"/api/match/{code}/join").status_code, 401)
        self.assertEqual(self.client.post(f"/api/match/{code}/result", json={}).status_code, 401)
        self.assertEqual(self.client.get(f"/api/match/{code}").status_code, 401)

    def test_bad_lang_or_tier_rejected(self):
        r = self.client.post("/api/match", json={"lang": "zz", "tier": "medium"}, headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 400)
        r = self.client.post("/api/match", json={"lang": "en", "tier": "insane"}, headers=self._hdr(self.tok_a))
        self.assertEqual(r.status_code, 400)

    def test_non_player_cannot_read(self):
        code = self._open_active()
        _, tok_c = self._make_user("dave")
        r = self.client.get(f"/api/match/{code}", headers=self._hdr(tok_c))
        self.assertEqual(r.status_code, 403)

    # ---- helpers ----------------------------------------------------------

    def _create(self, tok):
        r = self.client.post("/api/match", json={"lang": "en", "tier": "medium"}, headers=self._hdr(tok))
        return r.get_json()["code"]

    def _open_active(self):
        code = self._create(self.tok_a)
        self.client.post(f"/api/match/{code}/join", headers=self._hdr(self.tok_b))
        return code


if __name__ == "__main__":
    unittest.main()
