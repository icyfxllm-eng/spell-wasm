// cant-tell.spec -- CLIP-REPORT: the "Can't tell" step in the rescue panel.
//
// A player who says a clip was unintelligible must keep their streak, record
// no miss, earn nothing, and (as a standard player, with telemetry on) leave
// one report: lang, a hash of the word, the clip variant and the audio source.
// The flag is OFF by default, and off is the existing rescue, exactly.
import { gunzipSync } from 'node:zlib';
import { openApp, typeOnKeyboard, assert, assertEq, clickSettled } from '../harness.mjs';
import { validate } from '../../../workers/telemetry/src/index.js';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const flagOn = () => localStorage.setItem('spell_flag_cant_tell', 'on');

function endpoint() {
  const posts = [];
  const handler = (route) => {
    const req = route.request();
    const url = req.url();
    if (url.endsWith('/v1/flags')) {
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ telemetry_enabled: true }) });
    }
    let raw = req.postDataBuffer() || Buffer.alloc(0);
    if ((req.headers()['content-encoding'] || '') === 'gzip') raw = gunzipSync(raw);
    posts.push({ path: new URL(url).pathname, text: raw.toString('utf8') });
    return route.fulfill({ status: 204, body: '' });
  };
  return { handler, posts };
}

const store = (page, key) => page.evaluate((k) => localStorage.getItem(k), key);
const streak = (page) => page.$eval('#streakNum', (e) => e.textContent.trim());
const hide = (page) => page.evaluate(() => {
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' });
  document.dispatchEvent(new Event('visibilitychange'));
});
async function until(fn, ms = 5000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (await fn()) return true;
    await new Promise((r) => setTimeout(r, 100));
  }
  return false;
}

async function serve(page) {
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 }).catch(() => {});
  return page.evaluate(() => window.__spelltest.currentWord());
}

/// Spell the current word right, so there is a streak to protect.
async function spellRight(page) {
  const w = await serve(page);
  await typeOnKeyboard(page, w);
  await page.click('#checkBtn');
  await page.waitForFunction(() => document.getElementById('streakNum').textContent.trim() === '1', null, { timeout: 5000 });
  return w;
}

async function nextWord(page, previous) {
  await page.click('#orbWrap').catch(() => {});
  await page.waitForFunction((p) => { const w = window.__spelltest.currentWord(); return !!w && w !== p; }, previous, { timeout: 8000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}

const visibleSteps = (page) => page.$$eval('#rescuePanel .rescue-steps button:not([hidden])', (b) => b.map((x) => x.id));

export async function run(browser, base, suite) {
  await suite.test('cant-tell: the flag is off by default and the rescue is unchanged', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#orbWrap');
      await serve(page);
      await page.click('#cantHearBtn');
      assertEq(JSON.stringify(await visibleSteps(page)), JSON.stringify(['rescueSlow', 'rescueShow']), 'flag off: the two existing steps');
      assert(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden')), 'the new step is hidden');
    } finally { await ctx.close(); }
  });

  // The one that matters: the streak SURVIVES the skip. A mutation that makes
  // the skip a give-up (a miss) must turn this red.
  await suite.test('cant-tell: the streak survives, nothing is scored, one report is queued', async () => {
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', telemetry: ep.handler, init: flagOn });
    try {
      assert(await until(async () => (await store(page, 'spell_flag_telemetry_enabled')) === 'on'), 'kill switch never cached');
      await page.click('#orbWrap');
      const first = await spellRight(page);
      const second = await nextWord(page, first);
      const before = {
        streak: await streak(page),
        misses: await store(page, 'byear_misses_v1'),
        stats: await store(page, 'byear_stats_v1'),
      };
      assertEq(before.streak, '1', 'there is a streak to protect');
      await page.click('#cantHearBtn');
      assert(!(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden'))), 'flag on: the step is offered');
      await clickSettled(page, '#rescueCantTell');
      assertEq(await streak(page), '1', 'the streak is untouched by the skip');
      assertEq(await store(page, 'byear_misses_v1'), before.misses, 'no miss recorded');
      assertEq(await store(page, 'byear_stats_v1'), before.stats, 'no credit and no wrong answer');
      assert(await page.$eval('#checkBtn', (e) => e.disabled), 'the voided word cannot be submitted');
      const shown = await page.$eval('#feedback', (e) => e.textContent);
      assert(!shown.includes(second), 'Can\'t tell does not show the word');
      await nextWord(page, second);
      assertEq(await streak(page), '1', 'and the streak is still there on the next word');

      const queue = JSON.parse((await store(page, 'spell_tel_clips_v1')) || '[]');
      assertEq(queue.length, 1, 'one report queued');
      assert(!queue[0].includes(second), 'the queue holds a hash, never the word');
      await hide(page);
      assert(await until(() => ep.posts.some((p) => p.path.endsWith('/v1/clips'))), 'the report was never sent');
      const post = ep.posts.find((p) => p.path.endsWith('/v1/clips'));
      const body = JSON.parse(post.text);
      assertEq(validate('clip_batch', body), null, 'schema');
      assert(!post.text.includes(second), 'the wire carries no word');
      assert(!/session_id/.test(post.text), 'the report is not linked to a launch');
      assertEq(body.reports.length, 1, 'one report');
      assertEq(body.reports[0].lang, 'en', 'lang');
      assertEq(body.reports[0].variant, 'normal', 'variant');
      assert(['server_cache', 'native_tts', 'human', 'pack', 'other'].includes(body.reports[0].source), 'source');
    } finally { await ctx.close(); }
  });

  // The Climb is the default level, and its own void cap (1) sits under the session cap.
  await suite.test('cant-tell: the Climb allows one, then only Give up', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: flagOn });
    try {
      await page.click('#orbWrap');
      const w = await serve(page);
      await page.click('#cantHearBtn');
      assert(!(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden'))), 'the first is offered');
      await clickSettled(page, '#rescueCantTell');
      await nextWord(page, w);
      await page.click('#cantHearBtn');
      assert(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden')), 'the second is not offered in the Climb');
    } finally { await ctx.close(); }
  });

  // Practice has no run cap, so here the session cap (5) is the only limit.
  await suite.test('cant-tell: the session cap hides the step after five', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: flagOn });
    try {
      await page.evaluate(() => {
        const sel = document.getElementById('levelSel');
        sel.value = 'easy';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
      });
      await page.click('#orbWrap');
      let w = await serve(page);
      for (let i = 0; i < 5; i++) {
        await page.click('#cantHearBtn');
        assert(!(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden'))), `skip ${i + 1} is offered`);
        await clickSettled(page, '#rescueCantTell');
        w = await nextWord(page, w);
      }
      await page.click('#cantHearBtn');
      assert(await page.$eval('#rescueCantTell', (e) => e.hasAttribute('hidden')), 'the sixth is not offered');
    } finally { await ctx.close(); }
  });

  // Jr / unknown age: the skip still works, but nothing is ever queued.
  await suite.test('cant-tell: a Jr device queues no report', async () => {
    const ep = endpoint();
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, telemetry: ep.handler, init: flagOn });
    try {
      await page.click('#orbWrap');
      await serve(page);
      await page.click('#cantHearBtn');
      await clickSettled(page, '#rescueCantTell');
      assertEq(await store(page, 'spell_tel_clips_v1'), null, 'no clip queue on a Jr device');
      await hide(page);
      // sleep-ok: the claim is that NOTHING is posted; an absent post has no signal to wait for.
      await page.waitForTimeout(800);
      assertEq(ep.posts.filter((p) => p.path.endsWith('/v1/clips')).length, 0, 'nothing posted');
    } finally { await ctx.close(); }
  });
}
