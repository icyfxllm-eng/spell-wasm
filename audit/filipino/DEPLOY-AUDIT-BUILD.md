# Deploy runbook — Filipino audit build (audit.spellgame.net)

**Audience:** Eric (has Pi + Cloudflare access). The agent that prepared this
branch does **not** have Pi/Cloudflare access, so nothing below was executed —
these are exact, copy-pasteable steps plus how to verify each of the work order's
nine "Done when" checkpoints.

**Guardrails baked into the design**
- The audit build is a **separate artifact** (`dist-audit/`) served on a
  **separate host/port** with its **own backend + its own SQLite DB**. Production
  (`spellgame.net`, its webroot, its Climb DB, its DNS) is never touched.
- Filipino is unlocked ONLY in this build, via the `--features audit` flag +
  `AUDIT_LANGS=fil` (see Feature 1). A normal build has zero audit code (proven
  by `scripts/seam-absence-check.mjs`).
- The audit backend **shares the production TTS audio cache** (so you don't
  re-pay Google TTS for words already generated) but writes accounts/leaderboard
  to a **sandbox DB** (so audit play never pollutes the real leaderboard).

---

## (a) Build `dist-audit` and serve it on a new port

### a.1 — Build the audit frontend
Two ways; pick one.

**Option A — on the Pi host directly (quickest):**
```bash
cd /path/to/spell-wasm
git fetch origin && git checkout feature/filipino-audit-build
AUDIT_LANGS=fil bash scripts/build-web.sh      # -> ./dist-audit/
```
`dist-audit/` now contains `index.html`, `pkg/`, icons, etc., with Filipino
Active + the Feature-7 preselect/pin/banner.

**Option B — as a Docker image (matches the prod pipeline):**
Create `Dockerfile.audit` (identical to `Dockerfile` except it passes the flag
into the cargo step and serves the audit Caddy config):
```dockerfile
FROM rust:1-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.126
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
# The one line that differs from the prod Dockerfile:
ENV AUDIT_LANGS=fil
RUN cargo build --release --target wasm32-unknown-unknown --features audit \
    && wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
       --out-dir /out/pkg --target web --no-typescript

FROM caddy:2-alpine
COPY audit/filipino/Caddyfile.audit /etc/caddy/Caddyfile   # see step a.2
COPY index.html privacy.html audio-native.js manifest.json sw.js /srv/
COPY icons /srv/icons
COPY audit/filipino/robots.audit.txt /srv/robots.txt        # see step (d)
COPY --from=build /out/pkg /srv/pkg
RUN VER=$(sha256sum /srv/pkg/spell_wasm_bg.wasm | cut -c1-12) \
    && sed -i "s/?v=DEV/?v=$VER/g" /srv/index.html /srv/sw.js
```

### a.2 — Caddy site for the audit host (`Caddyfile.audit`)
Serve the audit frontend on **:8081** and proxy `/api/*` to the **audit
backend** (`backend-audit:8000`). This mirrors the prod `:8080` block but adds a
`noindex` header (step d) and points at the sandbox backend.
```caddy
:8081 {
	header {
		Strict-Transport-Security "max-age=31536000; includeSubDomains"
		X-Content-Type-Options "nosniff"
		X-Frame-Options "SAMEORIGIN"
		Referrer-Policy "strict-origin-when-cross-origin"
		# (d) keep the audit host out of search indexes
		X-Robots-Tag "noindex, nofollow"
	}
	handle /api/* {
		reverse_proxy backend-audit:8000
	}
	handle /robots.txt {
		root * /srv
		file_server
	}
	handle {
		root * /srv
		file_server
		header /pkg/*.wasm Content-Type application/wasm
		header /sw.js Cache-Control "no-cache"
	}
	encode gzip
}
```

### a.3 — docker-compose override (`docker-compose.audit.yml`)
Adds the audit frontend + a second backend with the **sandbox DB** and the
**shared prod TTS cache** (step e). Run it alongside prod.
```yaml
services:
  spell-audit:
    build:
      context: .
      dockerfile: Dockerfile.audit
    restart: unless-stopped
    depends_on:
      - backend-audit
    ports:
      - "8081:8081"

  backend-audit:
    build: ./backend
    restart: unless-stopped
    env_file:
      - .env
    environment:
      # (e) SANDBOX the accounts/leaderboard DB — separate file, separate volume.
      CLIMB_DB_PATH: /data/climb-audit/audit.db
      # (e) SHARE the prod TTS cache — same value as prod (backend/app.py default).
      AUDIO_CACHE_DIR: /data/audio_cache
    volumes:
      # Prod audio cache, shared read/write (reuse existing generated clips).
      - audio_cache:/data/audio_cache
      # NEW, isolated volume for the audit Climb DB — never the prod climb_data.
      - climb_audit_data:/data/climb-audit

volumes:
  climb_audit_data:
```
Bring it up (the `audio_cache` volume already exists from the prod stack, so it
is genuinely shared):
```bash
docker compose -f docker-compose.yml -f docker-compose.audit.yml up -d --build spell-audit backend-audit
```

---

## (b) Add `audit.spellgame.net` to the Cloudflare Tunnel ingress

Edit `cloudflared/config.yml` and add the audit hostname **above** the catch-all
`http_status:404` rule (order matters — first match wins):
```yaml
tunnel: 37bfdf16-54b6-4f07-bc93-218382064786
credentials-file: /etc/cloudflared/creds.json

ingress:
  - hostname: spellgame.net
    service: http://localhost:8080
  - hostname: audit.spellgame.net      # NEW
    service: http://localhost:8081      # -> Caddyfile.audit
  - service: http_status:404
```
Then, in the Cloudflare dashboard (or via API), add a **DNS CNAME**
`audit` → `37bfdf16-54b6-4f07-bc93-218382064786.cfargotunnel.com` (proxied,
orange cloud). Restart the tunnel:
```bash
docker compose up -d --force-recreate cloudflared
```
> This adds a **sub**domain only. The apex `spellgame.net` record and its webroot
> are untouched.

---

## (c) Cloudflare Access — allow ONLY Paul, one-time PIN, 30-day session

In the Cloudflare **Zero Trust** dashboard → **Access → Applications → Add an
application → Self-hosted**:
1. **Application domain:** `audit.spellgame.net` (path: leave blank = whole host).
2. **Session Duration:** **30 days**.
3. **Identity / login method:** enable **One-time PIN** (email code). Disable other
   IdPs so PIN is the only path.
4. **Policy** — name it `paul-only`:
   - Action: **Allow**
   - Include → **Emails** → `xavierguevarra06@gmail.com` (this exact address only).
5. Save. (Optional: a second **Block / everyone else** policy is implied by the
   single Allow, but you can add an explicit Block for clarity.)

Now visiting `https://audit.spellgame.net` shows Cloudflare's email box → Paul
enters his email → gets a PIN → is in for 30 days. Anyone else is denied at the
edge, before the app ever loads.

---

## (d) Keep the audit host out of search indexes

Two layers, both already wired into step (a):
1. **Header:** `X-Robots-Tag "noindex, nofollow"` on every response (in
   `Caddyfile.audit`).
2. **robots.txt:** create `audit/filipino/robots.audit.txt` (COPYied to
   `/srv/robots.txt` by `Dockerfile.audit`):
   ```
   User-agent: *
   Disallow: /
   ```
Cloudflare Access already blocks unauthenticated crawlers entirely, so this is
belt-and-suspenders. (Do **not** add these to the prod image.)

---

## (e) Data sandbox — separate DB, shared TTS cache

Already configured in `docker-compose.audit.yml` (step a.3). The seam exists in
code:
- `backend/db.py` reads `CLIMB_DB_PATH` (default `/data/climb/climb.db`). The
  audit backend overrides it to `/data/climb-audit/audit.db` on its **own**
  volume, so audit sign-ups/leaderboard never touch prod's `climb_data`.
- `backend/app.py` reads `AUDIO_CACHE_DIR` (default `audio_cache`). The audit
  backend leaves it at the prod value and mounts the **same** `audio_cache`
  volume, so Filipino clips already generated for prod are reused and new ones
  are shared back.

**Verify the sandbox after boot:**
```bash
# audit DB is its own file, initially tiny / empty of prod users:
docker compose -f docker-compose.yml -f docker-compose.audit.yml exec backend-audit \
  sh -c 'ls -l /data/climb-audit/ ; echo "---prod climb untouched---"'
# audio cache is the SAME inode set as prod (shared):
docker compose exec backend            sh -c 'ls /data/audio_cache | wc -l'
docker compose -f docker-compose.yml -f docker-compose.audit.yml exec backend-audit \
  sh -c 'ls /data/audio_cache | wc -l'   # same count => shared
```

---

## "Done when" checkpoint → how to verify

> The work order lists nine acceptance checkpoints. Mapped below to a concrete
> check. Items marked **PENDING DEPLOY** need the live host (Pi/Cloudflare) and
> could not be run from the build branch; everything they depend on is built and
> proven locally.

| # | "Done when…" | How to verify | Status |
|---|--------------|---------------|--------|
| 1 | Filipino is unlocked in the audit build (alongside en/es) | On `audit.spellgame.net`, open the language menu → **Filipino · AUDIT** is playable (not "coming soon"). Locally proven: e2e `audit: fil is playable…` passes. | **PENDING DEPLOY** (host); mechanism verified locally |
| 2 | Production is untouched / zero audit code in normal builds | `bash scripts/build-web.sh && node scripts/seam-absence-check.mjs dist` → OK (no audit trace). Prod webroot/DNS: only a **sub**domain was added. | **VERIFIED locally** (seam-absence OK) |
| 3 | Access limited to Paul via one-time PIN, 30-day session | From a non-allowed email, `audit.spellgame.net` is denied; from Paul's, PIN login works and the session lasts 30 days. Confirm the Access policy = single Allow(email) + Session 30d. | **PENDING DEPLOY** (Cloudflare Access) |
| 4 | Filipino preselected + AUDIT badge + first-launch banner | Fresh visit lands on Filipino, picker pins **Filipino · AUDIT** first, banner shows once. Locally proven: e2e `audit: fil is PRESELECTED…`, `…pins fil FIRST with an AUDIT badge`, `…banner renders once…` all pass. | **VERIFIED locally**; confirm on host |
| 5 | All modes playable in Filipino | On the host, play Standard, Daily, Climb, Head-to-Head (solo), Kid Mode, Missed-words, My Words in fil. Locally: fil is Active so every mode's active-lang gate opens (same code path as en/th, covered by `modes`/`gameplay` e2e). | **PENDING DEPLOY** for full manual pass; gate verified locally |
| 6 | Audit host not indexed | `curl -sI https://audit.spellgame.net | grep -i x-robots-tag` → `noindex`; `curl -s https://audit.spellgame.net/robots.txt` → `Disallow: /`. | **PENDING DEPLOY** (config ready in step d) |
| 7 | Data sandboxed (separate DB, shared TTS cache) | Run the two commands in step (e): audit Climb DB is its own file; `audio_cache` file count matches prod (shared). | **PENDING DEPLOY** (compose ready in step e; seam confirmed in `db.py`/`app.py`) |
| 8 | Audit materials packet delivered | `ls audit/filipino/` shows the checklist (.md/.txt), word-list CSV, UI-string CSV, how-to-access, Apple primer, this runbook. | **VERIFIED locally** (files in repo) |
| 9 | Email draft to Paul prepared (not sent) | `audit/filipino/EMAIL-DRAFT-paul.md` exists and is marked DO NOT SEND; no mail was sent. | **VERIFIED locally** (draft only) |

---

## Teardown (after Paul signs off)
```bash
docker compose -f docker-compose.yml -f docker-compose.audit.yml down spell-audit backend-audit
# remove the audit ingress line from cloudflared/config.yml and the DNS CNAME
# optionally drop the sandbox DB volume:
docker volume rm spell-wasm_climb_audit_data
```
Then flip Filipino live for real by setting its registry status to `Active` in
`src/consts.rs` (the one-line change the audit gates) and shipping a normal build.
