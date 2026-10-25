// online.spec — CC-GAME-SERVER Phase C: the Online Head-to-Head client against
// a REAL spell-server (server/spell-server, built with --features test-modes).
//
// Each test starts its own server, so one test's rate limits and rooms never
// leak into the next. The server's TTS is a local stub (a silent clip), and a
// tiny TCP proxy sits between the browser and the server so a test can cut
// the connection the way an elevator does (A13.2).
//
// Prerequisite: cargo build -p spell-server --features test-modes
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { createServer as createTcp, connect } from 'node:net';
import { mkdtempSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join as pathJoin } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const ROOT = pathJoin(fileURLToPath(import.meta.url), '..', '..', '..', '..');
const BIN = pathJoin(ROOT, 'target', 'debug', 'spell-server');
const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
// A13.3 needs a kill document signed with the real key. It is supplied by a
// person (tools/killflags/sign.py --kill online_enabled), never minted here.
const KILL_DOC = process.env.SPELL_KILL_ONLINE_DOC || '';

const listen = (srv) => new Promise((r) => srv.listen(0, '127.0.0.1', () => r(srv.address().port)));

/** A silent-clip TTS stand-in on its own port. */
async function ttsStub() {
  const srv = createServer((req, res) => {
    if (req.url.startsWith('/api/health')) return res.end('ok');
    if (req.url.startsWith('/api/speak')) {
      res.writeHead(200, { 'content-type': 'audio/mpeg' });
      return res.end(Buffer.from([0xff, 0xfb, 0x90, 0x00, 0, 0, 0, 0]));
    }
    res.writeHead(404).end();
  });
  return { port: await listen(srv), close: () => srv.close() };
}

/** Browser → proxy → server. `cut(ms)` drops every socket and refuses new ones for ms. */
async function proxy(target) {
  const socks = new Set();
  let refuseUntil = 0;
  const srv = createTcp((c) => {
    if (Date.now() < refuseUntil) return c.destroy();
    const u = connect(target, '127.0.0.1');
    socks.add(c); socks.add(u);
    c.pipe(u); u.pipe(c);
    const end = () => { c.destroy(); u.destroy(); socks.delete(c); socks.delete(u); };
    c.on('error', end); u.on('error', end); c.on('close', end); u.on('close', end);
  });
  const port = await listen(srv);
  return {
    port,
    cut(ms) { refuseUntil = Date.now() + ms; for (const s of socks) s.destroy(); },
    close() { for (const s of socks) s.destroy(); srv.close(); },
  };
}

/** One spell-server for one test, reachable through a proxy. */
async function roomServer(base, env = {}) {
  assert(existsSync(BIN), `build the server first: cargo build -p spell-server --features test-modes (${BIN})`);
  const tts = await ttsStub();
  const probe = createTcp();
  const port = await listen(probe);
  probe.close();
  const origin = base.replace(/\/$/, '');
  const child = spawn(BIN, [], {
    env: {
      PATH: process.env.PATH,
      SPELL_TTS_BASE: `http://127.0.0.1:${tts.port}`,
      SPELL_BIND_PUBLIC: `127.0.0.1:${port}`,
      SPELL_BIND_ADMIN: `127.0.0.1:${port + 1}`,
      SPELL_DB_PATH: pathJoin(mkdtempSync(pathJoin(tmpdir(), 'ol-e2e-')), 'spell.db'),
      SPELL_ALLOWED_ORIGINS: `${origin},capacitor://localhost`,
      SPELL_TRUST_PROXY: 'false',
      SPELL_LOG: 'warn',
      ...env,
    },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let err = '';
  child.stderr.on('data', (d) => { err += d; });
  for (let i = 0; i < 100; i++) {
    const ok = await fetch(`http://127.0.0.1:${port}/healthz`).then((r) => r.ok).catch(() => false);
    if (ok) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  const px = await proxy(port);
  return {
    play: `http://127.0.0.1:${px.port}`,
    admin: `http://127.0.0.1:${port + 1}`,
    px,
    close() { px.close(); child.kill('SIGKILL'); tts.close(); },
    stderr: () => err,
  };
}

/** Opens the app with online on, pointed at `srv`; logs every request to it. */
async function app(browser, base, srv, { kid = false, flags = { online_enabled: 'on' }, family = false } = {}) {
  const init = (args) => {
    for (const [k, v] of Object.entries(args.flags)) localStorage.setItem(`spell_flag_${k}`, v);
    if (args.family) localStorage.setItem('spell_online_family_v1', 'on');
  };
  const { ctx, page } = await openApp(browser, base, { lang: 'en', age: kid ? KID : undefined, init: `(${init})(${JSON.stringify({ flags, family })})` });
  const hits = [];
  if (srv) {
    page.on('request', (r) => { if (r.url().startsWith(srv.play)) hits.push(r.url()); });
    page.on('websocket', (w) => { if (w.url().includes(srv.play.replace('http://', ''))) hits.push(w.url()); });
    await page.evaluate((p) => { window.SPELL_PLAY_BASE = p; }, srv.play);
  }
  return { ctx, page, hits };
}

async function openOnline(page) {
  await page.evaluate(() => document.getElementById('vsBtn').click());
  await domSettled(page);
  await page.click('#vsOnline');
  await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
}

async function create(page, { timed = false, approve = false } = {}) {
  await page.fill('#olName', 'Mom');
  if (timed) await page.check('#olTimed');
  if (approve) await page.check('#olApprove');
  await page.click('#olCreate');
  await page.waitForSelector('#olLobby:not([hidden])', { timeout: 10000 });
  return (await page.textContent('#olRoomCode')).trim();
}

async function join(page, code, name = 'Dad') {
  await page.fill('#olName', name);
  await page.fill('#olCode', code);
  await page.click('#olJoin');
}

const visible = (page, sel) => page.$eval(sel, (e) => !e.hidden && e.offsetParent !== null).catch(() => false);

async function playMiss(page) {
  await page.waitForSelector('#olOrb.ready', { timeout: 15000 });
  await page.click('#olOrb');
  await page.waitForSelector('#olOrb.live', { timeout: 10000 });
  for (const k of ['z', 'q']) await page.click(`#olKeys [data-k="${k}"]`);
  await page.click('#olCheck');
}

export async function run(browser, base, suite) {
  await suite.test('A13.4: with the build flag off there is no Play online entry and no request to the room server', async () => {
    const srv = await roomServer(base);
    const { ctx, page, hits } = await app(browser, base, srv, { flags: {} });
    try {
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      assert(!(await visible(page, '#vsOnline')), 'the entry shows with online_enabled off');
      // sleep-ok: proving that NO request happens needs a window to watch; there is no event for an absence.
      await page.waitForTimeout(500);
      assertEq(hits.length, 0, `requests reached the room server: ${hits}`);
    } finally { await ctx.close(); srv.close(); }
  });

  await suite.test('A10.1: a kid without the parent toggle gets no entry and sends nothing', async () => {
    const srv = await roomServer(base);
    const { ctx, page, hits } = await app(browser, base, srv, { kid: true, flags: { online_enabled: 'on', online_kids: 'on' } });
    try {
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      assert(!(await visible(page, '#vsOnline')), 'a kid sees the entry without a grown-up saying yes');
      assertEq(hits.length, 0, `requests reached the room server: ${hits}`);
    } finally { await ctx.close(); srv.close(); }
  });

  await suite.test('A10.2: the parent toggle on but online.kids off still shows no entry', async () => {
    const srv = await roomServer(base);
    const { ctx, page, hits } = await app(browser, base, srv, { kid: true, flags: { online_enabled: 'on' }, family: true });
    try {
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      assert(!(await visible(page, '#vsOnline')), 'online.kids off must hide the entry');
      assertEq(hits.length, 0, `requests: ${hits}`);
    } finally { await ctx.close(); srv.close(); }
  });

  await suite.test('settings_effect_online_family: the family switch asks a grown-up, then a kid gets the entry', async () => {
    const { ctx, page } = await app(browser, base, null, { kid: true, flags: { online_enabled: 'on', online_kids: 'on' } });
    try {
      await page.evaluate(() => document.getElementById('setBtn').click());
      await page.waitForSelector('#olFamilyRow:not([hidden])', { timeout: 5000 });
      await page.evaluate(() => { const b = document.getElementById('olFamily'); b.checked = true; b.dispatchEvent(new Event('change')); });
      await page.waitForSelector('#parentScrim.show', { timeout: 4000 });
      assert(!(await page.evaluate(() => document.getElementById('olFamily').checked)), 'the switch moved before the gate');
      const q = (await page.textContent('#parentQ')) || '';
      const W = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
      const nums = (q.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => W[w]);
      assert(nums.length === 2, `could not read the parent question ${JSON.stringify(q)}`);
      await page.fill('#parentAnswer', String(nums[0] * nums[1]));
      await page.click('#parentSubmit');
      await page.waitForFunction(() => localStorage.getItem('spell_online_family_v1') === 'on', null, { timeout: 4000 });
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      assert(await visible(page, '#vsOnline'), 'the kid has the entry once a grown-up said yes');
    } finally { await ctx.close(); }
  });

  await suite.test('A12.1: create, share, join from a second device, start, play every turn, see results', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page);
      assert(/^[ACDEFGHJKMNPQRTUVWXY34679]{6}$/.test(code), `room code ${code}`);
      // The share sheet is native; its text (with the code) is pinned by
      // online_model's share_text test. Here: the button is there for an adult.
      assert(await visible(a.page, '#olShare'), 'no Share button for the host');
      await openOnline(b.page);
      await join(b.page, code.toLowerCase());
      await b.page.waitForSelector('#olLobby:not([hidden])', { timeout: 10000 });
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await a.page.click('#olStart');
      // 3 turns each, alternating; every answer a miss.
      for (let turn = 0; turn < 6; turn++) await playMiss(turn % 2 === 0 ? a.page : b.page);
      for (const p of [a.page, b.page]) await p.waitForSelector('#olResult:not([hidden])', { timeout: 15000 });
      assertEq((await a.page.textContent('#olResultTitle')).trim(), "It’s a tie!", 'both 0: a tie');
      assert(await visible(a.page, '#olRematch'), 'the host can rematch');
      assert(!(await visible(b.page, '#olRematch')), 'the guest cannot');
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('A12.2: a wrong code shows an inline error and nothing crashes', async () => {
    const srv = await roomServer(base);
    const { ctx, page } = await app(browser, base, srv);
    try {
      await openOnline(page);
      await join(page, 'ACDEFG');
      await page.waitForFunction(() => document.getElementById('olEntryErr').textContent.trim().length > 0, null, { timeout: 10000 });
      assert(await visible(page, '#olEntry'), 'still on the entry');
      await page.fill('#olCode', 'ab0');
      assert((await page.inputValue('#olCode')) === 'A', `the bad characters were not dropped: ${await page.inputValue('#olCode')}`);
      assert((await page.textContent('#olCodeHint')).length > 0, 'no inline hint for the bad character');
    } finally { await ctx.close(); srv.close(); }
  });

  await suite.test('A12.3: after 10 bad codes the rate-limit message shows', async () => {
    const srv = await roomServer(base);
    const { ctx, page } = await app(browser, base, srv);
    try {
      await openOnline(page);
      let last = '';
      for (let i = 0; i < 11; i++) {
        await page.evaluate(() => { document.getElementById('olEntryErr').textContent = ''; });
        await join(page, 'ACDEFG');
        await page.waitForFunction(() => document.getElementById('olEntryErr').textContent.trim().length > 0, null, { timeout: 10000 });
        last = (await page.textContent('#olEntryErr')).trim();
      }
      assert(/minute/i.test(last), `the 11th try did not say to wait: ${JSON.stringify(last)}`);
    } finally { await ctx.close(); srv.close(); }
  });

  await suite.test('A12.4: an emote reaches the other player in under a second', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page);
      await openOnline(b.page);
      await join(b.page, code);
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await a.page.click('#olStart');
      await b.page.waitForSelector('#olMatch:not([hidden])', { timeout: 10000 });
      const t0 = Date.now();
      await a.page.click('#olEmotes [data-emote="3"]');
      await b.page.waitForFunction(() => document.getElementById('olEmoteShow').textContent.includes('Mom'), null, { timeout: 2000 });
      const ms = Date.now() - t0;
      assert(ms < 1000, `emote took ${ms} ms`);
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('settings_effect_online_timed: a timed room shows the countdown on every word', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page, { timed: true });
      await openOnline(b.page);
      await join(b.page, code);
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await a.page.click('#olStart');
      await a.page.waitForSelector('#olOrb.ready', { timeout: 10000 });
      await a.page.click('#olOrb');
      await a.page.waitForSelector('#olTimer:not([hidden])', { timeout: 10000 });
      await b.page.waitForSelector('#olTimer:not([hidden])', { timeout: 10000 });
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('settings_effect_online_approve: with approve on, a joiner knocks and the host lets them in', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page, { approve: true });
      await openOnline(b.page);
      await join(b.page, code);
      await b.page.waitForSelector('#olPending:not([hidden])', { timeout: 10000 });
      await a.page.waitForSelector('#olRequests [data-approve]', { timeout: 10000 });
      await a.page.click('#olRequests [data-approve]');
      await b.page.waitForSelector('#olLobby:not([hidden])', { timeout: 10000 });
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('A13.2: twenty seconds offline mid-match shows the reconnect overlay, then the match resumes', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page);
      await openOnline(b.page);
      await join(b.page, code);
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await a.page.click('#olStart');
      await b.page.waitForSelector('#olMatch:not([hidden])', { timeout: 10000 });
      srv.px.cut(20000);
      await b.page.waitForSelector('#olReconnect:not([hidden])', { timeout: 10000 });
      await b.page.waitForSelector('#olReconnect[hidden]', { state: 'attached', timeout: 40000 });
      assert(await visible(b.page, '#olMatch'), 'back in the same match');
      // The match still plays: seat 0 (A) taps and B sees the word start.
      await a.page.waitForSelector('#olOrb.ready', { timeout: 15000 });
      await a.page.click('#olOrb');
      await b.page.waitForFunction(() => /spelling/i.test(document.getElementById('olStatus').textContent), null, { timeout: 10000 });
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('A13.2b: a drain during a match shows the banner', async () => {
    const srv = await roomServer(base);
    const a = await app(browser, base, srv);
    const b = await app(browser, base, srv);
    try {
      await openOnline(a.page);
      const code = await create(a.page);
      await openOnline(b.page);
      await join(b.page, code);
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await a.page.click('#olStart');
      await b.page.waitForSelector('#olMatch:not([hidden])', { timeout: 10000 });
      await fetch(`${srv.admin}/admin/drain`, { method: 'POST' });
      await b.page.waitForSelector('#olDrain:not([hidden])', { timeout: 10000 });
      assert(await visible(b.page, '#olMatch'), 'the match goes on under the banner');
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('F13: an unreachable server shows the resting card, and local Head-to-Head still works', async () => {
    const srv = await roomServer(base);
    srv.close(); // nothing listens any more
    const { ctx, page } = await app(browser, base, srv);
    try {
      await openOnline(page);
      await page.fill('#olName', 'Mom');
      await page.click('#olCreate');
      await page.waitForSelector('#olResting:not([hidden])', { timeout: 20000 });
      await page.click('#olRestingExit');
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      await page.click('#vsStart');
      assert(!(await page.$eval('#vsBar', (e) => e.classList.contains('btn-hide'))), 'local Head-to-Head did not start');
    } finally { await ctx.close(); }
  });

  await suite.test('A13.3: a remote kill while in the lobby shows the resting card', async () => {
    if (!KILL_DOC) throw new Error('NOT RUN: set SPELL_KILL_ONLINE_DOC to a kill document signed for online_enabled (tools/killflags/sign.py --kill online_enabled)');
    const srv = await roomServer(base);
    const { ctx, page } = await app(browser, base, srv);
    try {
      await openOnline(page);
      await create(page);
      await ctx.route('**/api/kill-flags', (r) => r.fulfill({ status: 200, contentType: 'application/json', body: KILL_DOC }));
      // Coming back to the app re-reads the kill file.
      await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
      await page.waitForSelector('#olResting:not([hidden])', { timeout: 10000 });
    } finally { await ctx.close(); srv.close(); }
  });
}
