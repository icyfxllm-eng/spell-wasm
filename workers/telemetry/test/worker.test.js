// node --test — the Worker against an in-memory D1 stand-in.
import { test } from "node:test";
import assert from "node:assert/strict";
import { handle, purge, validate } from "../src/index.js";

const IP_HEADERS = ["cf-connecting-ip", "x-forwarded-for", "x-real-ip", "true-client-ip", "user-agent"];

// A Request whose IP/UA headers and `cf` object THROW if touched: storing an IP
// is impossible if reading one fails the test.
function req(method, path, { body, gzip = false, origin = "capacitor://localhost" } = {}) {
  const headers = new Headers({ Origin: origin, "CF-Connecting-IP": "203.0.113.9", "User-Agent": "iPhone", "X-Forwarded-For": "203.0.113.9" });
  let payload;
  if (body !== undefined) {
    headers.set("Content-Type", "application/json");
    payload = new TextEncoder().encode(typeof body === "string" ? body : JSON.stringify(body));
    if (gzip) headers.set("Content-Encoding", "gzip");
  }
  const r = new Request(`https://spellgame.net/telemetry${path}`, { method, headers, body: payload && gzip ? gzipSync(payload) : payload });
  const realGet = r.headers.get.bind(r.headers);
  Object.defineProperty(r, "headers", {
    value: new Proxy(r.headers, {
      get(t, k) {
        if (k === "get") return (name) => {
          if (IP_HEADERS.includes(String(name).toLowerCase())) throw new Error(`Worker read ${name}`);
          return realGet(name);
        };
        const v = Reflect.get(t, k);
        return typeof v === "function" ? v.bind(t) : v;
      },
    }),
  });
  Object.defineProperty(r, "cf", { get() { throw new Error("Worker read request.cf"); } });
  return r;
}

import { gzipSync as zlibGzip } from "node:zlib";
const gzipSync = (u8) => new Uint8Array(zlibGzip(u8));

function fakeDb() {
  const rows = { events: [], daily_counts: new Map(), perf_counts: new Map() };
  const run = (sql, args) => {
    if (sql.startsWith("INSERT INTO events")) {
      const [day, build, platform, session_id, error_code, lang, mode, stack_hash] = args;
      rows.events.push({ day, build, platform, session_id, error_code, lang, mode, stack_hash });
    } else if (sql.startsWith("INSERT INTO daily_counts")) {
      const agg = sql.includes("'aggregate'");
      const [day, build, platform, error_code, a4, a5] = args;
      const lang = agg ? "" : a4;
      const n = agg ? a4 : 1;
      const key = [day, build, platform, agg ? "aggregate" : "events", error_code, lang].join("|");
      rows.daily_counts.set(key, (rows.daily_counts.get(key) || 0) + n);
    } else if (sql.startsWith("INSERT INTO perf_counts")) {
      const [day, build, platform, source, metric, bucket, lang, n] = args;
      const key = [day, build, platform, source, metric, bucket, lang].join("|");
      rows.perf_counts.set(key, (rows.perf_counts.get(key) || 0) + n);
    } else if (sql.startsWith("DELETE FROM events")) {
      rows.events = rows.events.filter((e) => e.day >= args[0]);
    } else throw new Error(`unexpected SQL ${sql}`);
  };
  const prepare = (sql) => ({ bind: (...args) => ({ sql, args, run: async () => run(sql, args) }) });
  return { rows, prepare, batch: async (stmts) => stmts.forEach((s) => run(s.sql, s.args)) };
}

const env = (on = true) => ({ TELEMETRY_ENABLED: on ? "true" : "false", DB: fakeDb() });

const batch = () => ({
  v: 1,
  build: "0a1b2c3d4e5f",
  platform: "ios",
  session_id: "00000000000000ff",
  events: [{ error_code: "wasm_panic", lang: "ru", mode: "daily", stack_hash: "0123456789abcdef" }],
  perf: [{ metric: "tap_to_audio_ms", bucket: "lt250", lang: "ru", count: 3 }],
});
const agg = () => ({
  v: 1,
  build: "dev",
  platform: "web",
  rows: [{ error_code: "js_uncaught", count: 4 }],
  perf: [{ metric: "audio_resolution", bucket: "unavailable", count: 2 }],
});

test("flags answer the kill switch", async () => {
  for (const on of [true, false]) {
    const r = await handle(req("GET", "/v1/flags"), env(on));
    assert.equal(r.status, 200);
    assert.deepEqual(await r.json(), { telemetry_enabled: on });
    assert.equal(r.headers.get("Cache-Control"), "no-store");
  }
});

test("a valid event batch is stored with the day and nothing else", async () => {
  const e = env();
  const r = await handle(req("POST", "/v1/events", { body: batch() }), e, Date.UTC(2026, 8, 18, 12));
  assert.equal(r.status, 204);
  assert.deepEqual(e.DB.rows.events, [{ day: "2026-09-18", build: "0a1b2c3d4e5f", platform: "ios", session_id: "00000000000000ff", error_code: "wasm_panic", lang: "ru", mode: "daily", stack_hash: "0123456789abcdef" }]);
  assert.equal(e.DB.rows.daily_counts.get("2026-09-18|0a1b2c3d4e5f|ios|events|wasm_panic|ru"), 1);
  assert.equal(e.DB.rows.perf_counts.get("2026-09-18|0a1b2c3d4e5f|ios|events|tap_to_audio_ms|lt250|ru"), 3);
});

test("gzip bodies are accepted", async () => {
  const e = env();
  const r = await handle(req("POST", "/v1/aggregate", { body: agg(), gzip: true }), e, Date.UTC(2026, 8, 18));
  assert.equal(r.status, 204);
  assert.equal(e.DB.rows.daily_counts.get("2026-09-18|dev|web|aggregate|js_uncaught|"), 4);
  assert.equal(e.DB.rows.perf_counts.get("2026-09-18|dev|web|aggregate|audio_resolution|unavailable|"), 2);
});

test("aggregates never carry an identifier or a language", async () => {
  const withId = { ...agg(), session_id: "00000000000000ff" };
  assert.equal((await handle(req("POST", "/v1/aggregate", { body: withId }), env())).status, 400);
  const withLang = agg();
  withLang.perf[0].lang = "ru";
  assert.equal((await handle(req("POST", "/v1/aggregate", { body: withLang }), env())).status, 400);
});

test("free text, unknown fields and oversized lists are rejected, nothing stored", async () => {
  const bad = [
    (b) => { b.events[0].lang = "the word was 'cat'"; },
    (b) => { b.events[0].answer = "kat"; },
    (b) => { b.events[0].stack_hash = "at foo (app.js:1:2)"; },
    (b) => { b.build = "1.1 (226)"; },
    (b) => { b.v = 2; },
    (b) => { b.events = Array(201).fill(b.events[0]); },
    (b) => { b.perf[0].bucket = "1234ms"; },
    (b) => { b.perf[0].ms = 231; },
  ];
  for (const mutate of bad) {
    const e = env();
    const b = batch();
    mutate(b);
    const r = await handle(req("POST", "/v1/events", { body: b }), e);
    assert.equal(r.status, 400);
    assert.equal(e.DB.rows.events.length, 0);
  }
  const e = env();
  assert.equal((await handle(req("POST", "/v1/events", { body: "not json" }), e)).status, 400);
});

test("kill switch: posts are accepted and discarded", async () => {
  const e = env(false);
  const r = await handle(req("POST", "/v1/events", { body: batch() }), e);
  assert.equal(r.status, 204);
  assert.equal(e.DB.rows.events.length, 0);
});

test("CORS only for the app and site origins", async () => {
  const ok = await handle(req("OPTIONS", "/v1/events"), env());
  assert.equal(ok.headers.get("Access-Control-Allow-Origin"), "capacitor://localhost");
  const evil = await handle(req("OPTIONS", "/v1/events", { origin: "https://evil.example" }), env());
  assert.equal(evil.headers.get("Access-Control-Allow-Origin"), null);
});

test("unknown routes 404", async () => {
  assert.equal((await handle(req("GET", "/v1/events"), env())).status, 404);
  assert.equal((await handle(req("POST", "/v2/events", { body: batch() }), env())).status, 404);
});

test("D6 — raw events older than 90 days are purged; counts stay", async () => {
  const e = env();
  const day = (d) => Date.UTC(2026, 0, 1) + d * 86_400_000;
  await handle(req("POST", "/v1/events", { body: batch() }), e, day(0));
  await handle(req("POST", "/v1/events", { body: batch() }), e, day(50));
  await purge(e, day(120));
  assert.deepEqual(e.DB.rows.events.map((x) => x.day), ["2026-02-20"]);
  assert.equal(e.DB.rows.daily_counts.size, 2);
});

test("the JS validator agrees with the generated schema's shape", () => {
  assert.equal(validate("flags", { telemetry_enabled: true }), null);
  assert.match(validate("flags", { telemetry_enabled: "yes" }), /bad value/);
  assert.match(validate("nope", {}), /unknown record/);
});

test("the IP trap itself works (instrument check)", () => {
  const r = req("GET", "/v1/flags");
  assert.throws(() => r.headers.get("CF-Connecting-IP"), /Worker read/);
  assert.throws(() => r.headers.get("user-agent"), /Worker read/);
  assert.throws(() => r.cf, /request.cf/);
  assert.equal(r.headers.get("Origin"), "capacitor://localhost");
});
