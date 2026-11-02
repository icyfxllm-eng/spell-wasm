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

// ---- Phase 3: a shared stand-in for the service, with the same rules as backend/accounts.py's /sync ----
const LISTS_KEY = 'byear_word_lists_v1';
const list = (id, name, words, updatedAt = 10) => ({
  id, profile: '', name, createdAt: 1, updatedAt, source: 'Manual', order: 'Mixed', deletedAt: null,
  entries: words.map((text) => ({ text, lang: 'en', addedAt: 1, starred: false })),
});
const listsDoc = (lists) => JSON.stringify({ v: 1, lists, nextId: lists.length, migrated: true, selection: [] });

function service() {
  const rows = new Map();
  let seq = 0;
  const st = { rows, syncCalls: 0, failSync: false };
  st.attach = async (ctx) => {
    await ctx.route('**/api/accounts/email/request', (r) => r.fulfill({ status: 200, contentType: 'application/json', body: '{"ok":true}' }));
    await ctx.route('**/api/accounts/email/verify', (r) => r.fulfill({ status: 200, contentType: 'application/json',
      body: JSON.stringify({ access_token: TOKEN, refresh_token: REFRESH, user_id: 7, expires_in: 900 }) }));
    await ctx.route('**/api/accounts/sync', (r) => {
      st.syncCalls++;
      if (st.failSync) return r.abort();
      const body = JSON.parse(r.request().postData());
      for (const rec of body.records) {
        const k = `${rec.kind}|${rec.record_id}`;
        const cur = rows.get(k);
        if (cur && cur.rec.updated_at >= rec.updated_at) continue;
        rows.set(k, { rec, seq: ++seq });
      }
      const out = [...rows.values()].filter((x) => x.seq > body.since).sort((a, b) => a.seq - b.seq);
      return r.fulfill({ status: 200, contentType: 'application/json',
        body: JSON.stringify({ records: out.map((x) => x.rec), cursor: out.length ? out[out.length - 1].seq : Math.max(body.since, seq), more: false }) });
    });
  };
  return st;
}

async function device(browser, base, svc, seedLists) {
  const flags = { ...ON, accounts_sync: 'on' };
  const init = (a) => {
    for (const [k, v] of Object.entries(a.flags)) localStorage.setItem(`spell_flag_${k}`, v);
    if (a.lists && !localStorage.getItem(a.key)) localStorage.setItem(a.key, a.lists);
  };
  const { ctx, page } = await openApp(browser, base, { lang: 'en',
    init: `(${init})(${JSON.stringify({ flags, key: LISTS_KEY, lists: seedLists ? listsDoc(seedLists) : null })})` });
  await svc.attach(ctx);
  return { ctx, page };
}

async function signIn(page) {
  await tapPlayOnline(page);
  await page.waitForSelector('#acnAge:not([hidden])');
  await answerAge(page, 1985);
  await page.click('#acnEmailBtn');
  await page.fill('#acnEmailIn', 'grownup@example.com');
  await page.click('#acnSend');
  await page.fill('#acnCodeIn', '123456');
  await page.click('#acnVerify');
  await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
}

const storedLists = (page) => page.evaluate((k) => {
  const raw = localStorage.getItem(k);
  return raw ? JSON.parse(raw).lists : [];
}, LISTS_KEY);

/** Poll a condition, nudging a sync each round the way coming back to the foreground does. */
async function until(page, what, test, tries = 30) {
  for (let i = 0; i < tries; i++) {
    if (await test()) return;
    await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
    // sleep-ok: the cycle is a background task with no event to wait on; the poll is the wait.
    await page.waitForTimeout(500);
  }
  throw new Error(`timed out waiting for ${what}`);
}

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

  await suite.test('A8: a list made on one device appears on the other exactly once, with ids that never collide, and a delete travels', async () => {
    const svc = service();
    const a = await device(browser, base, svc, [list('l1', 'Animals', ['cat', 'dog', 'owl'])]);
    const b = await device(browser, base, svc, [list('l1', 'Food', ['pie'])]);
    try {
      await signIn(a.page);
      await until(a.page, 'device A pushed', async () => svc.rows.size >= 1);
      await signIn(b.page);
      const names = async (p) => (await storedLists(p)).map((l) => l.name).sort();
      await until(b.page, 'both lists on B', async () => (await names(b.page)).length === 2);
      assertEq((await names(b.page)).join(','), 'Animals,Food');
      await until(a.page, 'both lists on A', async () => (await names(a.page)).length === 2);
      const animals = (await storedLists(b.page)).find((l) => l.name === 'Animals');
      assertEq(animals.entries.map((e) => e.text).join(','), 'cat,dog,owl');
      assert(animals.id !== 'l1' || (await storedLists(b.page)).filter((l) => l.id === 'l1').length === 1, 'two lists share a local id');

      // A removes Animals for good (the 30-day purge, or Delete now): the other device loses it too.
      await a.page.evaluate((k) => {
        const d = JSON.parse(localStorage.getItem(k));
        d.lists = d.lists.filter((l) => l.name !== 'Animals');
        localStorage.setItem(k, JSON.stringify(d));
      }, LISTS_KEY);
      await until(a.page, 'tombstone reached the service', async () => [...svc.rows.values()].some((x) => x.rec.deleted));
      await until(b.page, 'the delete on B', async () => (await names(b.page)).join(',') === 'Food');
      // Nothing churns: more cycles change nothing.
      await until(a.page, 'a quiet round', async () => true, 1);
      await a.page.waitForTimeout(2500); // sleep-ok: proving nothing changes needs a window to watch.
      await a.page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
      await a.page.waitForTimeout(800); // sleep-ok: as above.
      assertEq((await names(a.page)).join(','), 'Food');
    } finally { await a.ctx.close(); await b.ctx.close(); }
  });

  await suite.test('A7: a sync that fails leaves local lists exactly as they were, and the retry brings them together once', async () => {
    const svc = service();
    const a = await device(browser, base, svc, [list('l1', 'Animals', ['cat'])]);
    const b = await device(browser, base, svc, [list('l1', 'Mine', ['x', 'y'])]);
    try {
      await signIn(a.page);
      await until(a.page, 'A pushed', async () => svc.rows.size >= 1);
      svc.failSync = true;
      const before = await b.page.evaluate((k) => localStorage.getItem(k), LISTS_KEY);
      await signIn(b.page);
      // sleep-ok: a failing background call has no success event; this is the window it has to fail in.
      await b.page.waitForTimeout(1500);
      assertEq(await b.page.evaluate((k) => localStorage.getItem(k), LISTS_KEY), before, 'a failed sync changed local lists');
      svc.failSync = false;
      // The failed cycle backs off for 15 s; the poll below outlasts it, as a player simply coming back later would.
      await until(b.page, 'both lists after the retry', async () => (await storedLists(b.page)).length === 2, 60);
      const got = (await storedLists(b.page)).map((l) => l.name).sort();
      assertEq(got.join(','), 'Animals,Mine');
      assertEq((await storedLists(b.page)).find((l) => l.name === 'Mine').entries.length, 2);
    } finally { await a.ctx.close(); await b.ctx.close(); }
  });

  await suite.test('sync flag off: signing in sends nothing to /sync', async () => {
    const svc = service();
    const { ctx, page } = await openApp(browser, base, { lang: 'en',
      init: `(() => { for (const [k, v] of Object.entries(${JSON.stringify(ON)})) localStorage.setItem('spell_flag_' + k, v); })` + '()' });
    await svc.attach(ctx);
    try {
      await signIn(page);
      // sleep-ok: an absence of requests needs a window to watch.
      await page.waitForTimeout(1500);
      assertEq(svc.syncCalls, 0, 'sync ran with accounts_sync off');
    } finally { await ctx.close(); }
  });
}
