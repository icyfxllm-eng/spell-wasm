# Deploy runbook — Online Spell Off (async matches)

Online 1v1 "Spell Off" is **async**: it adds Flask endpoints + one SQLite table to the existing backend. No new services, no WebSockets, no new ports/secrets/DNS/CORS. The Pi already serves `spellgame.net` via docker-compose (Caddy static frontend + Flask backend + cloudflared) behind a Cloudflare Tunnel; the DB is SQLite at `/data/climb/climb.db` on a persistent volume, and `db.init()` (backend/app.py) runs `CREATE TABLE IF NOT EXISTS` on every startup.

## What this change adds
- `backend/matches.py` — new Blueprint (create / join / result / get), registered in `backend/app.py`.
- `matches` table in `backend/db.py`'s schema — auto-created on restart (idempotent).
- No new secrets, ports, tunnel/DNS/CORS changes.

## Deploy (on the Pi — requires SSH access to the Pi)
1. Get the reviewed/merged code onto the Pi (however code reaches it):
   ```bash
   cd <repo-on-pi>
   git pull
   ```
2. Rebuild + restart:
   ```bash
   docker compose up -d --build
   ```
   The backend container rebuilds with `matches.py` + the registered blueprint. On startup `db.init()` creates the `matches` table (`CREATE TABLE IF NOT EXISTS`) — **no manual migration, no data loss**; the `/data/climb` volume persists accounts, leaderboard, and matches.
3. If the web frontend UI also changed, the same command rebuilds Caddy's static bundle. (iOS clients get the UI via a separate TestFlight build, not the Pi.)

## Verify (live)
```bash
curl https://spellgame.net/api/health
# Log in via the app, copy the session token, then:
TOKEN=...
curl -s -X POST https://spellgame.net/api/match \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"lang":"en","tier":"medium"}'
# expect: { code, seed, lang, tier, wordCount }
curl -s https://spellgame.net/api/match/<code> -H "Authorization: Bearer $TOKEN"
# expect: match state (status, players, winner once both submit)
```

## Rollback
```bash
git checkout <previous-commit> && docker compose up -d --build
```
The `matches` table is additive; rolling back the code leaves the (empty) table harmless.

## Operational hardening (recommended)
- **Back up `/data/climb/climb.db` nightly, off-device** — it now holds accounts AND matches.
- **Match-expiry sweep** — matches carry `expires_at`; add a cron or delete-on-access to purge abandoned matches.
- **Rate-limit match creation** — a new spam surface; extend the existing submit throttle.
- SQLite is fine at this scale; revisit Postgres only if concurrent match writes get heavy.

## Not in this change (follow-ups)
- **Server-side anti-cheat**: the server owns the seed but currently trusts submitted scores. Full validation = re-derive the word list from the seed server-side and check each answer + timing.
- Real-time (live) head-to-head, stranger matchmaking, chat, and push "you've been challenged" notifications.
