// accounts.spec — CC-ACCOUNTS Phase 2 on the real screen: the lazy gate (F1), the neutral age
// screen (A12), local data untouched by sign-in (A7/I5), tokens kept out of web storage (A11/I6),
// and solo play with no prompt and no account traffic (A6/I1/I3).
//
// The identity service is a route stub: nothing here reaches a server. The browser build has no
// native plugin, so tokens live in memory here; the Keychain half is the Swift side's.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const ON = { online_enabled: 'on', accounts_enabled: 'on', accounts_online_gate: 'on' };
const TOKEN = 'eyJhbGciOiJFUzI1NiJ9.eyJleHAiOjk5OTk5OTk5OTl9.sigsigsig';
const REFRESH = 'rt_refresh-token-value-for-the-test';

async function app(browser, base, flags, { stub = true, offline = false } = {}) {
  const init = (f) => { for (const [k, v] of Object.entries(f)) localStorage.setItem(`spell_flag_${k}`, v); };
  const { ctx, page } = await openApp(browser, base, { lang: 'en', init: `(${init})(${JSON.stringify(flags)})` });
  const calls = [];
  if (stub) {
    await ctx.route('**/api/accounts/**', (r) => {
      const req = r.request();
      const path = new URL(req.url()).pathname;
      calls.push({ path, body: req.postData() ? JSON.parse(req.postData()) : null });
      if (path.endsWith('/email/request')) return r.fulfill({ status: 200, contentType: 'application/json', body: '{"ok":true}' });
      if (path.endsWith('/email/verify')) {
        return r.fulfill({ status: 200, contentType: 'application/json',
          body: JSON.stringify({ access_token: TOKEN, refresh_token: REFRESH, user_id: 7, expires_in: 900 }) });
      }
      return r.fulfill({ status: 200, contentType: 'application/json', body: '{"ok":true}' });
    });
  }
  if (offline) await ctx.setOffline(true);
  return { ctx, page, calls };
}

const shown = (page, sel) => page.$eval(sel, (e) => !e.hidden && e.offsetParent !== null).catch(() => false);
const scrimUp = (page) => page.$eval('#acnScrim', (e) => e.classList.contains('show')).catch(() => false);

async function tapPlayOnline(page) {
  await page.evaluate(() => document.getElementById('vsBtn').click());
  await domSettled(page);
  await page.click('#vsOnline');
}

async function answerAge(page, year) {
  await page.selectOption('#acnYear', String(year));
  await page.click('#acnAgeGo');
}

const storageDump = (page) => page.evaluate(async () => {
  const out = { local: {}, session: {}, cookie: document.cookie, idb: [] };
  for (let i = 0; i < localStorage.length; i++) out.local[localStorage.key(i)] = localStorage.getItem(localStorage.key(i));
  for (let i = 0; i < sessionStorage.length; i++) out.session[sessionStorage.key(i)] = sessionStorage.getItem(sessionStorage.key(i));
  try { out.idb = (await indexedDB.databases()).map((d) => d.name); } catch (_) { /* not available */ }
  return out;
});

export async function run(browser, base, suite) {
  await suite.test('flags off: Play online opens the online screen at once and nothing talks to the identity service', async () => {
    const { ctx, page, calls } = await app(browser, base, { online_enabled: 'on' });
    try {
      await tapPlayOnline(page);
      await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
      assert(!(await scrimUp(page)), 'the account sheet opened with accounts off');
      assert(!(await shown(page, '#acnSetRow')), 'Settings shows an account row with accounts off');
      // sleep-ok: an absence of requests needs a window to watch.
      await page.waitForTimeout(400);
      assertEq(calls.length, 0, `identity-service requests with the flags off: ${JSON.stringify(calls)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('F1: signed out, Play online opens the sheet at the age screen and not the online screen', async () => {
    const { ctx, page, calls } = await app(browser, base, ON);
    try {
      await tapPlayOnline(page);
      await page.waitForSelector('#acnScrim.show #acnAge:not([hidden])', { timeout: 10000 });
      assert(!(await shown(page, '#olEntry')), 'the online screen opened behind the gate');
      assertEq(calls.length, 0, 'the sheet called the service before the player answered anything');
      // "Not now" returns without opening anything and without nagging.
      await page.click('#acnNotNow');
      assert(!(await scrimUp(page)), 'Not now left the sheet up');
      assert(!(await shown(page, '#olEntry')), 'Not now opened the online screen');
    } finally { await ctx.close(); }
  });

  await suite.test('A12: an under-18 answer ends at a parent message: no request, nothing stored, and it holds on retry', async () => {
    const { ctx, page, calls } = await app(browser, base, ON);
    try {
      const year = new Date().getFullYear() - 10;
      await tapPlayOnline(page);
      await page.waitForSelector('#acnAge:not([hidden])');
      await answerAge(page, year);
      await page.waitForSelector('#acnUnder:not([hidden])');
      assert(!(await shown(page, '#acnMethod')), 'an under-18 answer reached the sign-in doors');
      await page.click('#acnUnderClose');
      // Change the answer to an adult one: the same launch must not open the doors.
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      await page.click('#vsOnline');
      await page.waitForSelector('#acnUnder:not([hidden])');
      assert(!(await shown(page, '#acnMethod')), 'asking again got past the under-18 answer');
      assertEq(calls.length, 0, `requests after an under-18 answer: ${JSON.stringify(calls)}`);
      const s = await storageDump(page);
      assert(!Object.keys(s.local).some((k) => k.startsWith('spell_acct_')), 'an under-18 answer left account keys behind');
    } finally { await ctx.close(); }
  });

  await suite.test('email road: adult answer, code, sign-in lands on the online screen; A11 no token in web storage; A7 local data untouched', async () => {
    const { ctx, page, calls } = await app(browser, base, ON);
    try {
      await page.evaluate(() => {
        localStorage.setItem('spell_test_words_v1', JSON.stringify(Array.from({ length: 200 }, (_, i) => `word${i}`)));
        localStorage.setItem('spell_test_misses_v1', '{"cat":3}');
      });
      const before = await storageDump(page);
      await tapPlayOnline(page);
      await page.waitForSelector('#acnAge:not([hidden])');
      await answerAge(page, 1985);
      await page.waitForSelector('#acnMethod:not([hidden])');
      await page.click('#acnEmailBtn');
      await page.fill('#acnEmailIn', 'grownup@example.com');
      await page.click('#acnSend');
      await page.waitForSelector('#acnCode:not([hidden])');
      await page.fill('#acnCodeIn', '123 456');
      await page.click('#acnVerify');
      await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
      assert(!(await scrimUp(page)), 'the sheet stayed up after sign-in');

      const verify = calls.find((c) => c.path.endsWith('/email/verify'));
      assert(verify && verify.body.adult === true && verify.body.code === '123456', `verify body: ${JSON.stringify(verify)}`);
      assert(verify.body.device_id && verify.body.device_id.length >= 8, 'no device id sent');

      const after = await storageDump(page);
      const blob = JSON.stringify(after);
      assert(!blob.includes(TOKEN) && !blob.includes(REFRESH) && !blob.includes('rt_'), 'a token reached web storage or cookies');
      assertEq(after.idb.length, before.idb.length, 'sign-in created an IndexedDB database');
      // I5: every key that held progress before is byte-identical after.
      for (const [k, v] of Object.entries(before.local)) {
        if (k.startsWith('spell_acct_')) continue;
        assertEq(after.local[k], v, `sign-in changed ${k}`);
      }
      assertEq(JSON.parse(after.local.spell_test_words_v1).length, 200, 'words changed');
      assertEq(after.local.spell_acct_adopt_v1, '7', 'the device was not marked for adoption into account 7');
      const added = Object.keys(after.local).filter((k) => !(k in before.local));
      assert(added.every((k) => k.startsWith('spell_acct_') || k.startsWith('spell_flag_') || k.startsWith('byear_')),
        `unexpected new keys: ${added}`);
    } finally { await ctx.close(); }
  });

  await suite.test('a wrong code and an unreachable service are said plainly and the sheet can be left', async () => {
    const { ctx, page } = await app(browser, base, ON, { stub: false });
    try {
      await ctx.route('**/api/accounts/email/request', (r) => r.fulfill({ status: 200, body: '{"ok":true}', contentType: 'application/json' }));
      await ctx.route('**/api/accounts/email/verify', (r) => r.fulfill({ status: 400, body: '{"error":"invalid_code"}', contentType: 'application/json' }));
      await tapPlayOnline(page);
      await page.waitForSelector('#acnAge:not([hidden])');
      await answerAge(page, 1980);
      await page.click('#acnEmailBtn');
      await page.fill('#acnEmailIn', 'a@example.com');
      await page.click('#acnSend');
      await page.fill('#acnCodeIn', '000000');
      await page.click('#acnVerify');
      await page.waitForSelector('#acnErr:not([hidden])');
      assert((await page.textContent('#acnErrText')).includes("didn't work"), 'no wrong-code message');
      await page.click('#acnErrClose');
      assert(!(await scrimUp(page)), 'could not leave after an error');

      await ctx.unroute('**/api/accounts/email/request');
      await ctx.route('**/api/accounts/email/request', (r) => r.abort());
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      await page.click('#vsOnline');
      await page.waitForSelector('#acnMethod:not([hidden]), #acnAge:not([hidden])');
      if (await shown(page, '#acnAge')) await answerAge(page, 1980);
      await page.click('#acnEmailBtn');
      await page.fill('#acnEmailIn', 'a@example.com');
      await page.click('#acnSend');
      await page.waitForSelector('#acnErr:not([hidden])');
      assert((await page.textContent('#acnErrText')).includes('Solo play still works'), 'no offline message');
    } finally { await ctx.close(); }
  });

  await suite.test('A6/I1/I3: offline with every account flag on, launch and Head-to-Head setup show no prompt and make no account request', async () => {
    const { ctx, page, calls } = await app(browser, base, { ...ON, accounts_sync: 'on', accounts_family: 'on' }, { offline: true });
    try {
      await page.reload({ waitUntil: 'load' });
      await domSettled(page);
      assert(!(await scrimUp(page)), 'the account sheet appeared at launch');
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      assert(!(await scrimUp(page)), 'the account sheet appeared in local Head-to-Head setup');
      // sleep-ok: an absence of requests needs a window to watch.
      await page.waitForTimeout(400);
      assertEq(calls.length, 0, `account requests during solo use: ${JSON.stringify(calls)}`);
    } finally { await ctx.close(); }
  });
}
