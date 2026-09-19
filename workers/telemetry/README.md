# spell-telemetry — the CC-TELEMETRY-FOUNDATION v1.1 endpoint

A Cloudflare Worker on `spellgame.net/telemetry/*` (D5). It accepts only what
`schema.json` declares. That file is GENERATED from `src/telemetry/schema.rs`,
so never edit it by hand. Run `TELEMETRY_BLESS=1 cargo test --lib telemetry`
from the repo root after a schema change.

| Route | Purpose |
|---|---|
| `GET /telemetry/v1/flags` | `{"telemetry_enabled": bool}`: the kill switch (I7) |
| `POST /telemetry/v1/events` | a standard player's error batch (`event_batch`) |
| `POST /telemetry/v1/aggregate` | a Jr / unknown / Education daily count (`agg_batch`) |

Tests: `npm test` (Node 20+, no dependencies). One test proves the Worker
never reads an IP, the User-Agent, or `request.cf`.

## Before the first deploy: check the IP rule

The spec says the endpoint must not store client IPs, and says to **stop and
ask** if the host keeps them in logs that can't be disabled. The Worker itself
never reads or stores an IP. Check the zone as well:

1. **Logpush** (zone → Analytics & Logs → Logpush): there must be no HTTP
   requests job, or one that excludes `ClientIP` or filters out `/telemetry/`.
2. **Workers Logs / Tail**: off (`[observability] enabled = false` here, and
   no tail consumers).
3. **Security Analytics / Security Events**: Cloudflare's dashboard shows
   sampled requests, with client IPs, for all proxied traffic on the zone. As
   far as I know this can't be switched off per path. It already applies to
   every spellgame.net request through the tunnel today. **This is Eric's call
   under the stop-and-ask rule.**

## Deployed 2026-09-18

- Account "Icyfxllm@gmail.com's Account"; D1 `spell-telemetry` (`88eeaf34-…`, ENAM), migrations 0001 and 0002 applied.
- Route `spellgame.net/telemetry/*` only. `workers_dev = false`, `preview_urls = false`.
- **Kill switch OFF** (`TELEMETRY_ENABLED = "false"` in wrangler.toml) until the privacy policy is live. To turn it on, set it to `"true"` in wrangler.toml and `npx wrangler deploy`, so the file stays the record.
- IP rule: Eric accepted Cloudflare's standing Security Analytics sampling (it can't be switched off per path).
- **Open: the daily purge cron is NOT registered.** Cloudflare refuses cron triggers (code 10063) until the account has a workers.dev subdomain, which is created the first time the Workers page is opened in the dashboard. After that, run `npx wrangler deploy` again. Nothing is stored while the kill switch is off, so nothing ages past 90 days meanwhile.

## First deploy (reference)

```bash
cd workers/telemetry
npx wrangler login
npx wrangler d1 create spell-telemetry        # paste database_id into wrangler.toml
npx wrangler d1 migrations apply spell-telemetry --remote
npx wrangler deploy
curl -s https://spellgame.net/telemetry/v1/flags   # {"telemetry_enabled":true}
```

Until the flags route answers, apps hold errors on the device and send nothing.

## Kill switch

```bash
npx wrangler deploy --var TELEMETRY_ENABLED:false
```

Clients learn it on their next flags fetch (every launch). They clear their
queues at once and send nothing afterwards. Posts that are still in flight
are accepted and discarded.

## Reading the data

```bash
npx wrangler d1 execute spell-telemetry --remote --command \
  "SELECT build, error_code, lang, SUM(n) AS n FROM daily_counts WHERE day >= date('now','-14 day') GROUP BY 1,2,3 ORDER BY n DESC"
```

Raw `events` rows are deleted after 90 days by the daily cron (D6). The
`daily_counts` table is kept.
