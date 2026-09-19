// CC-TELEMETRY-FOUNDATION v1.1 — the telemetry endpoint (D5: Cloudflare Worker).
//
// Routes (mounted at spellgame.net/telemetry/*, ahead of the tunnel, so the
// Mac mini being down never costs a crash report):
//   GET  /telemetry/v1/flags      → {"telemetry_enabled": bool}   (I7, R4)
//   POST /telemetry/v1/events     → a standard player's error batch
//   POST /telemetry/v1/aggregate  → a Jr / unknown / Education daily count
//
// WHAT THIS NEVER DOES. It never reads the client IP (CF-Connecting-IP,
// X-Forwarded-For, X-Real-IP), request.cf (geo), or the User-Agent, and it
// never logs a request. Observability is off in wrangler.toml. The only thing
// stored is what `schema.json` (GENERATED from src/telemetry/schema.rs)
// declares, plus the UTC day it arrived.
//
// Retention (D6): raw events 90 days, deleted by the daily cron; the per-day
// counts in `daily_counts` are kept.

import SCHEMA from "../schema.json" with { type: "json" };

const MAX_BODY = 256 * 1024; // compressed or not; a full 200-event queue is far smaller
const RAW_DAYS = 90;
const ALLOWED_ORIGINS = new Set([
  "https://spellgame.net",
  "capacitor://localhost", // iOS app
  "https://localhost", // Android app
  "http://localhost",
  "tauri://localhost", // desktop
]);

// ---- Validation: the Rust `schema::validate`, over the generated schema ----

const HEX16 = /^[0-9a-f]{16}$/;
const BUILD = /^([0-9a-f]{12}|dev)$/;

export function validate(name, value, schema = SCHEMA) {
  const fields = schema.records[name];
  if (!fields) return `unknown record ${name}`;
  if (value === null || typeof value !== "object" || Array.isArray(value)) return `${name}: not an object`;
  for (const k of Object.keys(value)) {
    if (!fields.some((f) => f.name === k)) return `${name}: undeclared field ${k}`;
  }
  for (const f of fields) {
    if (!(f.name in value)) return `${name}.${f.name}: missing`;
    const x = value[f.name];
    const t = f.type;
    let ok;
    if (t === "bool") ok = typeof x === "boolean";
    else if (t === "hash64") ok = typeof x === "string" && HEX16.test(x);
    else if (t === "build") ok = typeof x === "string" && BUILD.test(x);
    else if (t === "version") ok = x === schema.version;
    else if (t.enum) ok = typeof x === "string" && t.enum.includes(x);
    else if ("uint" in t) ok = Number.isInteger(x) && x >= 0 && x <= t.uint;
    else if (t.list) {
      ok = Array.isArray(x) && x.length <= t.max;
      if (ok) {
        for (const item of x) {
          const err = validate(t.list, item, schema);
          if (err) return err;
        }
      }
    } else ok = false;
    if (!ok) return `${name}.${f.name}: bad value`;
  }
  return null;
}

// ---- HTTP helpers ------------------------------------------------------------

function cors(request) {
  const origin = request.headers.get("Origin");
  const h = { Vary: "Origin" };
  if (origin && ALLOWED_ORIGINS.has(origin)) {
    h["Access-Control-Allow-Origin"] = origin;
    h["Access-Control-Allow-Methods"] = "GET, POST, OPTIONS";
    h["Access-Control-Allow-Headers"] = "Content-Type, Content-Encoding";
    h["Access-Control-Max-Age"] = "86400";
  }
  return h;
}

function reply(request, status, body) {
  const headers = { ...cors(request), "Cache-Control": "no-store" };
  if (body === undefined) return new Response(null, { status, headers });
  headers["Content-Type"] = "application/json";
  return new Response(JSON.stringify(body), { status, headers });
}

async function readJson(request) {
  const declared = Number(request.headers.get("Content-Length") || 0);
  if (declared > MAX_BODY) return { error: 413 };
  let stream = request.body;
  if (!stream) return { error: 400 };
  if ((request.headers.get("Content-Encoding") || "").toLowerCase() === "gzip") {
    stream = stream.pipeThrough(new DecompressionStream("gzip"));
  }
  const reader = stream.getReader();
  const chunks = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > MAX_BODY * 4) return { error: 413 };
    chunks.push(value);
  }
  const bytes = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) {
    bytes.set(c, at);
    at += c.byteLength;
  }
  try {
    return { value: JSON.parse(new TextDecoder().decode(bytes)) };
  } catch {
    return { error: 400 };
  }
}

export function enabled(env) {
  return String(env.TELEMETRY_ENABLED ?? "false") === "true";
}

const utcDay = (now) => new Date(now).toISOString().slice(0, 10);

// ---- Storage -------------------------------------------------------------------

async function storeEvents(env, batch, day) {
  const stmts = [];
  for (const e of batch.events) {
    stmts.push(
      env.DB.prepare(
        "INSERT INTO events (day, build, platform, session_id, error_code, lang, mode, stack_hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
      ).bind(day, batch.build, batch.platform, batch.session_id, e.error_code, e.lang, e.mode, e.stack_hash),
      env.DB.prepare(
        "INSERT INTO daily_counts (day, build, platform, source, error_code, lang, n) VALUES (?, ?, ?, 'events', ?, ?, 1) " +
          "ON CONFLICT (day, build, platform, source, error_code, lang) DO UPDATE SET n = n + 1",
      ).bind(day, batch.build, batch.platform, e.error_code, e.lang),
    );
  }
  stmts.push(...perfStmts(env, batch, day, "events"));
  if (stmts.length) await env.DB.batch(stmts);
}

// F5 histogram cells. Aggregate rows have no lang; they are stored as ''.
function perfStmts(env, batch, day, source) {
  return batch.perf
    .filter((p) => p.count > 0)
    .map((p) =>
      env.DB.prepare(
        "INSERT INTO perf_counts (day, build, platform, source, metric, bucket, lang, n) VALUES (?, ?, ?, ?, ?, ?, ?, ?) " +
          "ON CONFLICT (day, build, platform, source, metric, bucket, lang) DO UPDATE SET n = n + excluded.n",
      ).bind(day, batch.build, batch.platform, source, p.metric, p.bucket, p.lang ?? "", p.count),
    );
}

async function storeAggregate(env, batch, day) {
  const stmts = batch.rows
    .filter((r) => r.count > 0)
    .map((r) =>
      env.DB.prepare(
        "INSERT INTO daily_counts (day, build, platform, source, error_code, lang, n) VALUES (?, ?, ?, 'aggregate', ?, '', ?) " +
          "ON CONFLICT (day, build, platform, source, error_code, lang) DO UPDATE SET n = n + excluded.n",
      ).bind(day, batch.build, batch.platform, r.error_code, r.count),
    );
  stmts.push(...perfStmts(env, batch, day, "aggregate"));
  if (stmts.length) await env.DB.batch(stmts);
}

// ---- Entry points --------------------------------------------------------------

export async function handle(request, env, now = Date.now()) {
  const path = new URL(request.url).pathname.replace(/^\/telemetry/, "");
  if (request.method === "OPTIONS") return reply(request, 204);

  if (path === "/v1/flags" && request.method === "GET") {
    return reply(request, 200, { telemetry_enabled: enabled(env) });
  }

  const record = path === "/v1/events" ? "event_batch" : path === "/v1/aggregate" ? "agg_batch" : null;
  if (!record || request.method !== "POST") return reply(request, 404);

  // Kill switch: accept and discard, so clients stop retrying a dead queue.
  if (!enabled(env)) return reply(request, 204);

  const { value, error } = await readJson(request);
  if (error) return reply(request, error);
  const invalid = validate(record, value);
  if (invalid) return reply(request, 400, { error: invalid });

  const day = utcDay(now);
  if (record === "event_batch") await storeEvents(env, value, day);
  else await storeAggregate(env, value, day);
  return reply(request, 204);
}

export async function purge(env, now = Date.now()) {
  const cutoff = utcDay(now - RAW_DAYS * 86_400_000);
  await env.DB.prepare("DELETE FROM events WHERE day < ?").bind(cutoff).run();
}

export default {
  async fetch(request, env) {
    try {
      return await handle(request, env);
    } catch {
      // No logging (I4): a failure is a 500 and nothing else.
      return reply(request, 500);
    }
  },
  async scheduled(_event, env, ctx) {
    ctx.waitUntil(purge(env));
  },
};
