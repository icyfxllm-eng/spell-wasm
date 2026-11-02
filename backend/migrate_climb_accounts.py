"""One-off: copy Climb players into the accounts DB, keeping each user id.

    ACCOUNTS_DB_PATH=... python3 migrate_climb_accounts.py /path/to/climb.db [--apply]

Dry run unless --apply. Reads climb.db read-only; never writes it. Passwords are NOT
copied (CC-ACCOUNTS retires them): a migrated player signs in with an emailed code, or
with Apple once the same verified email arrives. Ids are kept so leaderboard rows and
Spell Off matches, which key on users.id, keep pointing at the right person. Re-running
is safe: existing ids are skipped.

Not done here, and needed before Climb auth is retired: the Climb routes and the app's
Climb screens still use the old sessions and carry a username the accounts table does not
hold. That cutover is its own step.
"""
import sqlite3
import sys

import accounts


def migrate(climb_path: str, apply: bool) -> dict:
    src = sqlite3.connect(f"file:{climb_path}?mode=ro", uri=True)
    src.row_factory = sqlite3.Row
    dst = accounts.conn()
    copied = skipped = 0
    for u in src.execute("SELECT id, email, email_lc, email_verified, created_at FROM users ORDER BY id"):
        clash = dst.execute("SELECT 1 FROM users WHERE id=? OR email_lc=?", (u["id"], u["email_lc"])).fetchone()
        if clash:
            skipped += 1
            continue
        if apply:
            dst.execute(
                "INSERT INTO users(id,email,email_lc,email_verified,created_at) VALUES(?,?,?,?,?)",
                (u["id"], u["email"], u["email_lc"], u["email_verified"], u["created_at"]),
            )
        copied += 1
    if apply:
        dst.commit()
    return {"copied": copied, "skipped": skipped, "applied": apply}


if __name__ == "__main__":
    print(migrate(sys.argv[1], "--apply" in sys.argv))
