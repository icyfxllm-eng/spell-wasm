"""A13 — The Climb season reset (PD1) is an ARCHIVE, not a deletion, and the
migration is idempotent.

    CLIMB_DB_PATH=:memory: python3 -m unittest test_climb_season   # from backend/

Proves: after the migration the ACTIVE leaderboard is empty, every prior score
still exists in the archive, a fresh run posts and ranks normally, and running
the migration a second time is a safe no-op.
"""
import os
import tempfile
import time
import unittest

# Use a throwaway on-disk DB (WAL needs a real file) before importing db.
_TMP = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
_TMP.close()
os.environ["CLIMB_DB_PATH"] = _TMP.name

import db  # noqa: E402


def _seed_user(c, uid, name):
    c.execute(
        "INSERT INTO users(id,username,username_lc,email,email_lc,pw_hash,created_at) "
        "VALUES(?,?,?,?,?,?,?)",
        (uid, name, name.lower(), f"{name}@e.x", f"{name}@e.x".lower(), "x", time.time()),
    )


def _seed_entry(c, uid, difficulty, chain, locale="en"):
    c.execute(
        "INSERT INTO leaderboard_entries(user_id,difficulty,locale,best_chain,achieved_at,run_meta) "
        "VALUES(?,?,?,?,?,?)",
        (uid, difficulty, locale, chain, time.time(), None),
    )


def _active_count(c):
    return c.execute("SELECT COUNT(*) AS n FROM leaderboard_entries").fetchone()["n"]


def _archive_count(c):
    return c.execute("SELECT COUNT(*) AS n FROM leaderboard_archive").fetchone()["n"]


class SeasonReset(unittest.TestCase):
    def setUp(self):
        db.init()
        self.c = db.conn()
        # Clean slate between tests (module-level shared DB).
        self.c.executescript(
            "DELETE FROM leaderboard_entries; DELETE FROM leaderboard_archive; DELETE FROM users;"
        )
        self.c.commit()
        _seed_user(self.c, 1, "alice")
        _seed_user(self.c, 2, "bob")
        _seed_entry(self.c, 1, "hard", 12)
        _seed_entry(self.c, 2, "hard", 7)
        _seed_entry(self.c, 1, "medium", 20)
        self.c.commit()

    def test_archive_empties_active_but_preserves_scores(self):
        self.assertEqual(_active_count(self.c), 3)
        moved = db.archive_active_season(self.c)
        self.assertEqual(moved, 3, "all live rows archived")
        self.assertEqual(_active_count(self.c), 0, "active leaderboard is now empty")
        self.assertEqual(_archive_count(self.c), 3, "prior scores preserved in the archive")
        # The archived scores are intact (not zeroed).
        row = self.c.execute(
            "SELECT best_chain FROM leaderboard_archive WHERE user_id=1 AND difficulty=? AND locale='en'",
            ("medium",),
        ).fetchone()
        self.assertEqual(row["best_chain"], 20)

    def test_new_run_posts_and_ranks_after_reset(self):
        db.archive_active_season(self.c)
        # A fresh run posts to the ACTIVE table normally.
        _seed_entry(self.c, 2, "hard", 5)
        self.c.commit()
        rows = self.c.execute(
            "SELECT user_id, best_chain FROM leaderboard_entries "
            "WHERE difficulty='hard' AND locale='en' ORDER BY best_chain DESC"
        ).fetchall()
        self.assertEqual(len(rows), 1, "only the post-reset run is on the active board")
        self.assertEqual(rows[0]["user_id"], 2)
        self.assertEqual(rows[0]["best_chain"], 5)

    def test_second_migration_is_a_noop(self):
        first = db.archive_active_season(self.c)
        self.assertEqual(first, 3)
        archived_after_first = _archive_count(self.c)
        second = db.archive_active_season(self.c)
        self.assertEqual(second, 0, "second run archives nothing")
        self.assertEqual(_archive_count(self.c), archived_after_first, "archive unchanged on re-run")
        self.assertEqual(_active_count(self.c), 0)

    def test_running_twice_across_two_real_seasons(self):
        # Two genuine resets (with a run in between) land in distinct seasons and
        # never collide on the archive primary key.
        db.archive_active_season(self.c)
        _seed_entry(self.c, 1, "hard", 30)
        self.c.commit()
        db.archive_active_season(self.c)
        seasons = [r["season"] for r in self.c.execute("SELECT DISTINCT season FROM leaderboard_archive ORDER BY season")]
        self.assertEqual(seasons, [1, 2])


if __name__ == "__main__":
    unittest.main()
