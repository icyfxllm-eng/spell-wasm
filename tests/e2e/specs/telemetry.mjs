// telemetry.spec — CC-TELEMETRY-FOUNDATION v1.1 acceptance 2, 3, 5, 6.
//
// Every telemetry request is answered by this spec (flags) or captured (posts),
// and every captured payload is checked with the WORKER's own validator over
// the GENERATED schema: what the client sends is exactly what the server would
// accept, and nothing else.
//
// Errors are raised the way players raise them -- an uncaught throw in the
// page -- and flushed the way the app flushes on a phone: the page going
// hidden. No seam, no private entry point.
import { gunzipSync } from 'node:zlib';
import { openApp, assert, assertEq } from '../harness.mjs';
import { validate } from '../../../workers/telemetry/src/index.js';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

function endpoint({ on = true, down = false } = {}) {
  const posts = [];
  const flagGets = [];
  const handler = (route) => {
    const req = route.request();
    const url = req.url();
    if (url.endsWith('/v1/flags')) {
      flagGets.push(url);
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ telemetry_enabled: on }) });
    }
    if (down) return route.abort('connectionrefused');
    let raw = req.postDataBuffer() || Buffer.alloc(0);
    if ((req.headers()['content-encoding'] || '') === 'gzip') raw = gunzipSync(raw);
    posts.push({ path: new URL(url).pathname, text: raw.toString('utf8'), headers: req.headers() });
    return route.fulfill({ status: 204, body: '' });
  };
  return { handler, posts, flagGets };
}

// An uncaught error carrying text that must never leave the device.
const raise = (page, text = 'boom secret-word-kitten') =>
  page.evaluate((t) => setTimeout(() => { throw new TypeError(t); }, 0), text);

// What iOS does when the app backgrounds: the page goes hidden.
const hide = (page) => page.evaluate(() => {
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' });
  document.dispatchEvent(new Event('visibilitychange'));
});

const store = (page, key) => page.evaluate((k) => localStorage.getItem(k), key);

async function until(fn, ms = 4000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (await fn()) return true;
    await new Promise((r) => setTimeout(r, 100));
  }
  return false;
}

export async function run(browser, base, suite) {
  await suite.test('standard_player_sends_valid_batches_with_perf_and_no_text', async () => {
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      // Serve a word so the audio resolver runs (F5 tap_to_audio / resolution).
      await page.click('#orbWrap').catch(() => {});
      await page.waitForTimeout(1500);
      await raise(page);
      await page.waitForTimeout(100);
      await hide(page);
      assert(await until(() => ep.posts.some((p) => JSON.parse(p.text).events.length > 0)), 'no error batch sent');
      const bodies = ep.posts.map((p) => {
        assert(p.path.endsWith('/v1/events'), `posted to ${p.path}`);
        assert(!/secret|kitten|boom|TypeError|http/.test(p.text), `payload leaked text: ${p.text}`);
        assert(!('cookie' in p.headers), 'sent a cookie');
        const b = JSON.parse(p.text);
        assertEq(validate('event_batch', b), null, 'schema');
        return b;
      });
      const events = bodies.flatMap((b) => b.events);
      assertEq(events.length, 1, 'events across batches');
      assertEq(events[0].error_code, 'js_uncaught', 'code');
      const perf = bodies.flatMap((b) => b.perf);
      assert(perf.some((r) => r.metric === 'wasm_init_ms' && r.lang === 'en'), `no wasm_init_ms: ${JSON.stringify(perf)}`);
      assert(perf.some((r) => r.metric === 'audio_resolution'), `no audio_resolution: ${JSON.stringify(perf)}`);
      assertEq(new Set(bodies.map((b) => b.session_id)).size, 1, 'one session id per launch');
    } finally { await ctx.close(); }
  });

  // Acceptance 2 (Jr) and 3 (unknown age): no per-event request ever; at most
  // one aggregate per day; no identifier in it.
  for (const [title, age] of [['jr_sends_only_one_daily_aggregate', KID], ['unknown_age_sends_only_one_daily_aggregate', '']]) {
    await suite.test(title, async () => {
      const ep = endpoint();
      const { ctx, page } = await openApp(browser, base, { lang: 'en', age, telemetry: ep.handler });
      try {
        assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
        await raise(page);
        await page.waitForTimeout(100);
        await hide(page);
        await page.waitForTimeout(800);
        assertEq(ep.posts.length, 0, 'posts before the day is due');
        const agg = JSON.parse(await store(page, 'spell_tel_agg_v1'));
        assertEq(agg.counts.js_uncaught, 1, 'counted locally');
        assert(agg.due > Date.now() && agg.due <= Date.now() + 86_400_000, `due ${agg.due} not within a day`);

        // Make the day due, then background twice: exactly one aggregate.
        await page.evaluate(() => {
          const a = JSON.parse(localStorage.getItem('spell_tel_agg_v1'));
          a.due = 1;
          localStorage.setItem('spell_tel_agg_v1', JSON.stringify(a));
        });
        await raise(page);
        await page.waitForTimeout(100);
        await hide(page);
        assert(await until(() => ep.posts.length > 0), 'aggregate never sent');
        await page.waitForTimeout(300);
        await hide(page);
        await page.waitForTimeout(800);
        assertEq(ep.posts.length, 1, 'aggregate posts');
        const p = ep.posts[0];
        assert(p.path.endsWith('/v1/aggregate'), `posted to ${p.path}`);
        const body = JSON.parse(p.text);
        assertEq(validate('agg_batch', body), null, 'schema');
        assert(!/session_id|stack_hash|"lang"|"mode"/.test(p.text), `aggregate carried an identifier: ${p.text}`);
        assertEq(body.rows.find((r) => r.error_code === 'js_uncaught')?.count, 2, 'count');
        assert(body.perf.some((r) => r.metric === 'wasm_init_ms'), 'no wasm_init_ms in the aggregate');
        const next = JSON.parse(await store(page, 'spell_tel_agg_v1')).due;
        assert(next >= Date.now() + 86_400_000 - 5000, 'next send less than a day away');
      } finally { await ctx.close(); }
    });
  }

  // Acceptance 6 — the kill switch: nothing sent, the queue cleared, and the
  // same on the next launch. The flags GET itself continues (it carries no
  // data, and is how telemetry would ever come back on).
  await suite.test('kill_switch_sends_nothing_and_clears_the_queue', async () => {
    const ep = endpoint({ on: false });
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'off'), 'kill switch never cached');
      // A queue left over from before the switch flipped.
      await page.evaluate(() => localStorage.setItem('spell_tel_queue_v1', JSON.stringify([{ c: 'wasm_panic', l: 'en', m: 'home', h: '0123456789abcdef' }])));
      await raise(page);
      await page.waitForTimeout(100);
      await hide(page);
      await page.waitForTimeout(800);
      assertEq(ep.posts.length, 0, 'posts this launch');
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await raise(page);
      await page.waitForTimeout(100);
      await hide(page);
      await page.waitForTimeout(800);
      assertEq(ep.posts.length, 0, 'posts next launch');
      for (const k of ['spell_tel_queue_v1', 'spell_tel_agg_v1', 'spell_tel_jsbuf_v1']) {
        assertEq(await store(page, k), null, `${k} after kill`);
      }
    } finally { await ctx.close(); }
  });

  // Acceptance 5 (partial) — endpoint down: the player sees nothing, the game
  // keeps serving, the queue keeps the errors for later and stays capped.
  await suite.test('endpoint_down_is_invisible_and_keeps_the_queue', async () => {
    const ep = endpoint({ down: true });
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      for (let i = 0; i < 3; i++) await raise(page, `e${i}`);
      await page.waitForTimeout(100);
      await hide(page);
      await page.waitForTimeout(1000);
      const q = JSON.parse((await store(page, 'spell_tel_queue_v1')) || '[]');
      assertEq(q.length, 3, 'queued for retry');
      const toast = await page.evaluate(() => {
        const t = document.getElementById('toast');
        return t ? t.classList.contains('show') : false;
      });
      assert(!toast, 'a toast appeared');
      assert(await page.evaluate(() => window.__spelltest.build() === 'testseam'), 'app stopped responding');
    } finally { await ctx.close(); }
  });
}
