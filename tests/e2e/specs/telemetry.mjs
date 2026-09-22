// telemetry.spec — CC-TELEMETRY-FOUNDATION v1.1 acceptance 2–7, and F7.
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
import { openApp, typeOnKeyboard, assert, assertEq } from '../harness.mjs';
import { validate } from '../../../workers/telemetry/src/index.js';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

function endpoint({ on = true, down = false } = {}) {
  const posts = [];
  const flagGets = [];
  // Switchable so one session can alternate healthy and refused rounds; see
  // endpoint_down_costs_no_round_latency.
  let refusing = down;
  const handler = (route) => {
    const req = route.request();
    const url = req.url();
    if (url.endsWith('/v1/flags')) {
      flagGets.push(url);
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ telemetry_enabled: on }) });
    }
    if (refusing) return route.abort('connectionrefused');
    let raw = req.postDataBuffer() || Buffer.alloc(0);
    if ((req.headers()['content-encoding'] || '') === 'gzip') raw = gunzipSync(raw);
    posts.push({ path: new URL(url).pathname, text: raw.toString('utf8'), headers: req.headers() });
    return route.fulfill({ status: 204, body: '' });
  };
  return { handler, posts, flagGets, setDown: (v) => { refusing = v; } };
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

// Math.random seeded (mulberry32) before the app loads, so word selection is
// a fixed sequence. Telemetry draws from crypto, never from this stream.
const SEEDED = () => {
  let a = 0x5eed;
  Math.random = () => {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

const feedbackPainted = (page) => page.waitForFunction(() => {
  const c = document.getElementById('feedback').className;
  return c.includes('good') || c.includes('bad');
}, null, { timeout: 5000 });

// Play `rounds` words: every third answer wrong, an error raised and a flush
// forced every round so telemetry is busy the whole time. Returns what the
// player saw, and how long each check took to paint its verdict.
async function playRounds(page, rounds, beforeRound = null) {
  const seen = [];
  const latencies = [];
  // Telemetry busy BEFORE the first word too: the deck is shuffled at the
  // first serve, so that is where a stolen random number would show.
  await raise(page, 'before the first word');
  await page.waitForTimeout(100);
  await hide(page);
  await page.waitForTimeout(250);
  for (let i = 0; i < rounds; i++) {
    if (beforeRound) await beforeRound(i);
    await page.click('#orbWrap');
    await page.waitForTimeout(350);
    const word = await page.evaluate(() => window.__spelltest.currentWord());
    const answer = i % 3 === 2 ? 'zzzz' : word.toLowerCase();
    await typeOnKeyboard(page, answer);
    // Telemetry is sending WHILE the round is timed below. It used to flush at
    // the end of the round, which left the measured window (check -> verdict)
    // idle: a client that blocked on a failed send was invisible to the timing
    // test. Proven by mutation -- a 40 ms block in the failure path changed
    // the old numbers by 0.3 ms.
    // Timed from before the flush: the send, its failure handling and the
    // verdict paint are all inside the window, so a client that blocks on a
    // dead endpoint shows up here. Both conditions pay the same fixed cost for
    // raise + hide, so the comparison is still only about the endpoint.
    const t0 = Date.now();
    await raise(page, `round ${i}`);
    await hide(page);
    await page.click('#checkBtn');
    await feedbackPainted(page);
    latencies.push(Date.now() - t0);
    const verdict = await page.$eval('#feedback', (e) => (e.className.includes('good') ? 'good' : 'bad'));
    const streak = await page.evaluate(() => window.__spelltest.streak());
    // A miss that breaks a chain opens "Chain broken!": part of what the
    // player saw, then dismissed the way a player would.
    // It opens after the answer reveal, so a miss waits for it.
    const chainBroken = verdict === 'bad'
      && (await page.waitForSelector('#scrim.show', { timeout: 4000 }).then(() => true, () => false));
    if (chainBroken) await page.click('#skipSave');
    seen.push({ word, verdict, streak, chainBroken });
    await page.waitForTimeout(250);
  }
  return { seen, latencies };
}

// Flip a settings switch the way settings-effects.spec does.
const flip = (page, id, on) => page.evaluate(([i, v]) => {
  const el = document.getElementById(i);
  el.checked = v;
  el.dispatchEvent(new Event('change', { bubbles: true }));
}, [id, on]);

const openSettingsSheet = async (page) => {
  await page.click('#setBtn');
  await page.waitForSelector('#setScrim.show', { timeout: 4000 });
};

const eventCount = (ep) => ep.posts.reduce((n, p) => n + (JSON.parse(p.text).events || []).length, 0);

const median = (xs) => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor(s.length / 2)]; };

// Mean without the fastest and slowest samples: one descheduled round (a GC
// pause, another suite's cargo build) moves a median of ten by more than the
// tolerance, and that is what made this test flaky.
const trimmedMean = (xs) => {
  const s = [...xs].sort((a, b) => a - b).slice(1, -1);
  return s.reduce((a, b) => a + b, 0) / s.length;
};

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
  // Acceptance 7 — observe, never act: a fixed-seed replay serves the same
  // words, verdicts and streaks with telemetry on as with it killed.
  await suite.test('observe_never_act_replay_is_identical_on_and_off', async () => {
    const runs = [];
    for (const on of [true, false]) {
      const ep = endpoint({ on });
      const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler, init: SEEDED });
      try {
        assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === (on ? 'on' : 'off')), 'kill switch never cached');
        runs.push({ on, ...(await playRounds(page, 8)), posts: ep.posts.length });
      } finally { await ctx.close(); }
    }
    const [withT, without] = runs;
    assert(withT.posts > 0, 'telemetry never sent during the "on" run: the comparison proves nothing');
    assertEq(without.posts, 0, 'posts during the killed run');
    assertEq(JSON.stringify(withT.seen), JSON.stringify(without.seen), 'what the player saw');
    assert(new Set(withT.seen.map((r) => r.word)).size > 1, 'the replay served one word: seed not applied');
  });

  // Acceptance 5, timing half: a refused telemetry endpoint must not slow a
  // round down (I1).
  //
  // Measured INTERLEAVED inside ONE session: the endpoint flips between
  // refusing and healthy on alternate rounds, so both conditions meet the same
  // machine, the same page and the same moment. The first version played ten
  // rounds in one session and ten in another, minutes apart, and compared
  // medians: it failed 4 of ~8 full-suite runs on 2026-09-21/22, with the
  // direction flipping between runs ("down 44 vs up 64", then "down 59 vs up
  // 41"), which is machine load between sessions, not a regression.
  //
  // The comparison is ONE-SIDED, because the claim is one-sided: "down" being
  // faster is not a failure. It compares trimmed means over 12 samples per
  // side, and re-measures once before failing, so a single stalled round
  // cannot fail the suite on its own.
  await suite.test('endpoint_down_costs_no_round_latency', async () => {
    const ROUNDS = 24; // 12 per side, alternating
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler, init: SEEDED });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      const measure = async () => {
        const { latencies } = await playRounds(page, ROUNDS, (i) => ep.setDown(i % 2 === 1));
        const up = latencies.filter((_, i) => i % 2 === 0);
        const down = latencies.filter((_, i) => i % 2 === 1);
        return { up: trimmedMean(up), down: trimmedMean(down), n: down.length };
      };
      const tolerance = (up) => Math.max(0.05 * up, 16.7); // 5%, or one 60 Hz frame
      let m = await measure();
      let cost = m.down - m.up;
      if (cost > tolerance(m.up)) {
        process.stdout.write(`    round latency: re-measuring (first pass cost ${cost.toFixed(1)} ms)\n`);
        m = await measure();
        cost = m.down - m.up;
      }
      const tol = tolerance(m.up);
      process.stdout.write(`    round latency trimmed mean over ${m.n} rounds each: up ${m.up.toFixed(1)} ms, down ${m.down.toFixed(1)} ms, cost ${cost.toFixed(1)} ms (tolerance ${tol.toFixed(1)} ms)\n`);
      assert(cost <= tol, `a refused endpoint cost ${cost.toFixed(1)} ms per round (up ${m.up.toFixed(1)}, down ${m.down.toFixed(1)}, tolerance ${tol.toFixed(1)})`);
      // The refused posts are still queued for a later flush, and still capped.
      ep.setDown(true);
      await raise(page, 'queue check');
      await hide(page);
      await page.waitForTimeout(500);
      const q = JSON.parse((await store(page, 'spell_tel_queue_v1')) || '[]');
      assert(q.length > 0 && q.length <= 200, `queue ${q.length}`);
    } finally { await ctx.close(); }
  });

  // F7 — "Help improve SpellGame". Off: nothing leaves and nothing held
  // survives; on: sending resumes.
  await suite.test('settings_effect_telemetry', async () => {
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      await openSettingsSheet(page);
      assert(await page.evaluate(() => document.getElementById('telemetryToggle').checked), 'on by default for a standard player (D3)');
      await page.evaluate(() => localStorage.setItem('spell_tel_queue_v1', JSON.stringify([{ c: 'wasm_panic', l: 'en', m: 'home', h: '0123456789abcdef' }])));
      await flip(page, 'telemetryToggle', false);
      assertEq(await store(page, 'spell_telemetry_opt_v1'), 'off', 'choice stored');
      assertEq(await store(page, 'spell_tel_queue_v1'), null, 'queue cleared when switched off');
      const before = eventCount(ep);
      await raise(page);
      await page.waitForTimeout(100);
      await hide(page);
      await page.waitForTimeout(800);
      assertEq(eventCount(ep), before, 'events sent while off');

      await flip(page, 'telemetryToggle', true);
      await raise(page, 'again');
      await page.waitForTimeout(100);
      await hide(page);
      assert(await until(() => eventCount(ep) > before), 'nothing sent after switching back on');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_telemetry_jr_needs_a_grown_up', async () => {
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, telemetry: ep.handler });
    try {
      await openSettingsSheet(page);
      await flip(page, 'telemetryToggle', false);
      await page.waitForSelector('#parentScrim.show', { timeout: 4000 });
      assert(await page.evaluate(() => document.getElementById('telemetryToggle').checked), 'the switch snapped back');
      assertEq(await store(page, 'spell_telemetry_opt_v1'), null, 'a child changed it without the gate');
      const q = (await page.textContent('#parentQ')) || '';
      const W = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
      const nums = (q.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => W[w]);
      assert(nums.length === 2, `could not read the parent question ${JSON.stringify(q)}`);
      await page.fill('#parentAnswer', String(nums[0] * nums[1]));
      await page.click('#parentSubmit');
      assert(await until(async () => (await store(page, 'spell_telemetry_opt_v1')) === 'off'), 'the grown-up could not turn it off');
      assert(!(await page.evaluate(() => document.getElementById('telemetryToggle').checked)), 'switch shows off');
      assert(!(await page.evaluate(() => document.getElementById('ageScrim').classList.contains('show'))), 'the gate reopened the birthday prompt instead');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_telemetry_kill_switch_overrides', async () => {
    const ep = endpoint({ on: false });
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'off'), 'kill switch never cached');
      await openSettingsSheet(page);
      const st = await page.evaluate(() => {
        const el = document.getElementById('telemetryToggle');
        return { disabled: el.disabled, checked: el.checked, row: el.closest('.set-row').classList.contains('suppressed') };
      });
      assert(st.disabled && st.row && !st.checked, `not rendered as overridden: ${JSON.stringify(st)}`);
    } finally { await ctx.close(); }
  });
  // Acceptance 4 — personal data never reaches a payload: My Words, sign-in,
  // and a Snap a List review, with errors raised and flushed at each stage.
  // Say It needs the iOS speech bridge and can't run here (sayit.spec proves
  // it stays hidden off-iOS); the schema has no field that could carry audio
  // either way (schema_lint, I2).
  await suite.test('personal_data_never_reaches_a_payload', async () => {
    const SECRETS = ['quokkazebra', 'marmotflute', 'snapwordalpha', 'secret.parent@example.com', 'Tr0ub4dorXyz9%', 'Pat Parent'];
    const ep = endpoint();
    const urls = [];
    const capture = (route) => { urls.push(route.request().url()); return ep.handler(route); };
    const custom = { words: ['quokkazebra', 'marmotflute'], speakLang: 'en-US', wordLang: {} };
    const { ctx, page } = await openApp(browser, base, { age: '', telemetry: capture });
    // Seeded for the reload below (addInitScript applies to later navigations).
    await ctx.addInitScript((c) => { if (!localStorage.getItem('byear_custom_v1')) localStorage.setItem('byear_custom_v1', JSON.stringify(c)); }, custom);
    await ctx.route('**/api/auth/**', (r) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ ok: true }) }));
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      const flushNow = async (label) => { await raise(page, `${label} ${SECRETS.join(' ')}`); await page.waitForTimeout(100); await hide(page); await page.waitForTimeout(400); };

      // Sign-in: an adult birthday opens the front door; type an address and a password.
      await page.waitForSelector('#ageScrim.show', { timeout: 4000 });
      await page.selectOption('#ageYear', '1990');
      await page.selectOption('#ageMonth', '1');
      await page.selectOption('#ageDay', '1');
      await page.click('#ageSubmit');
      await page.waitForSelector('#frontDoor.show', { timeout: 5000 });
      await page.fill('#fdEmail', 'secret.parent@example.com');
      await page.fill('#fdPassword', 'Tr0ub4dorXyz9%');
      await page.click('#fdLogin').catch(() => {});
      await page.waitForTimeout(400);
      await flushNow('login');
      await page.click('#fdGuest').catch(() => {});
      await page.waitForTimeout(300);

      // My Words: reload so the seeded words exist, play them.
      await page.reload({ waitUntil: 'load' });
      await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#langSel', '__mine').catch(() => {});
      await page.click('#setupDone').catch(() => {});
      await page.waitForTimeout(200);
      await page.click('#orbWrap');
      await page.waitForTimeout(400);
      const served = await page.evaluate(() => window.__spelltest.currentWord());
      assert(['quokkazebra', 'marmotflute'].includes(served), `My Words not served (got ${served})`);
      await flushNow('mywords');

      // Snap a List: the review sheet with a recognised word.
      await page.evaluate(() => window.__spelltest.photoReview(JSON.stringify(['snapwordalpha'])));
      await page.waitForTimeout(300);
      await flushNow('photo');

      assert(ep.posts.length > 0, 'nothing was sent: the test proves nothing');
      for (const p of ep.posts) {
        const b = JSON.parse(p.text);
        assertEq(validate(p.path.endsWith('/v1/events') ? 'event_batch' : 'agg_batch', b), null, 'schema');
        for (const secret of SECRETS) assert(!p.text.includes(secret), `payload carried "${secret}": ${p.text}`);
      }
      for (const u of urls) for (const secret of SECRETS) assert(!u.includes(encodeURIComponent(secret)) && !u.includes(secret), `URL carried "${secret}"`);
      const langs = new Set(ep.posts.flatMap((p) => JSON.parse(p.text).events || []).map((e) => e.lang));
      assert(langs.has('mine'), `a My Words error is recorded as lang "mine", got ${JSON.stringify([...langs])}`);
    } finally { await ctx.close(); }
  });
}
