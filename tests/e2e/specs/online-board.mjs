// online-board.spec — CC-BOARD-GAME-ONLINE Phase C: Board Game in an online
// room, two app players and one NPC, against a REAL spell-server.
//
// The server's TTS is a stub, and the stub is the one place the test learns a
// word: the server asks it to voice the word before the seat hears it. The
// app never sees the word (A9), so this is how one player can spell right and
// the game can reach its finish.
//
// Prerequisite: cargo build -p spell-server --features test-modes
import { assert, assertEq, domSettled } from '../harness.mjs';
import { roomServer, app, openOnline, join } from './online.mjs';

const FLAGS = { online_enabled: 'on', online_boardgame: 'on' };
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
const visible = (page, sel) => page.$eval(sel, (e) => !e.hidden && e.offsetParent !== null).catch(() => false);

async function hostBoardRoom(page, { seats = 3 } = {}) {
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgOnline:not([hidden])', { timeout: 10000 });
  await page.click('#bgOnline');
  await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
  assertEq(await page.$eval('#olMode', (e) => e.value), 'boardgame', 'the board entry opens on Board Game');
  assert(await visible(page, '#olBoardOpts'), 'the board settings show');
  assert(!(await visible(page, '#olH2hOpts')), 'Spell Off settings hide');
  await page.fill('#olName', 'Mom');
  await page.selectOption('#olSeats', String(seats));
  await page.selectOption('#olSize', 'sprint');
  await page.click('#olCreate');
  await page.waitForSelector('#olLobby:not([hidden])', { timeout: 10000 });
  return (await page.textContent('#olRoomCode')).trim();
}

const pieceButton = (page, n) => `#olPieces [data-piece="${n}"]`;

/** One move for this page if it is its turn. `right`: spell the word the server voiced. */
async function act(page, word, right) {
  const s = await state(page);
  if (!s || !s.online || !s.online.mine || s.online.locked) return false;
  switch (s.phase) {
    case 'AwaitRoll':
      if (await page.$eval('#bgOrb', (b) => b.disabled)) return false;
      await page.click('#bgOrb');
      return true;
    case 'AwaitSpelling': {
      const w = right && word() ? word() : 'zq';
      for (const ch of w) {
        const k = await page.$(`#bgKeys [data-k="u:${ch}"]`);
        if (k) await k.click();
      }
      await page.click('#bgGo');
      return true;
    }
    case 'AwaitStretch':
      await page.click('#bgStretch .bg-st-opt');
      return true;
    case 'AwaitSwitchTarget':
      await page.click('#bgSwap [data-sw="none"]');
      return true;
    default:
      return false;
  }
}

export async function run(browser, base, suite) {
  await suite.test('BG-ONLINE F6/F7/F8: two players and an NPC play to a winner, with a drop and a resume mid-word', async () => {
    let last = '';
    const srv = await roomServer(base, { SPELL_ONLINE_TURN: 'true' }, { onWord: (w) => { last = w; } });
    const a = await app(browser, base, srv, { flags: FLAGS });
    const b = await app(browser, base, srv, { flags: FLAGS });
    try {
      const frames = [];
      if (process.env.BG_DEBUG) b.page.on('console', (m) => { if (m.type() === 'error' || /panic/i.test(m.text())) frames.push(`console ${m.text().slice(0, 600)}`); });
      if (process.env.BG_DEBUG) b.page.on('pageerror', (e) => frames.push(`pageerror ${String(e).slice(0, 600)}`));
      if (process.env.BG_DEBUG) b.page.on('websocket', (w) => w.on('framereceived', (f) => frames.push(`${Date.now() % 100000} ${String(f.payload)}`)));
      const code = await hostBoardRoom(a.page);
      // The other player comes in through the Spell Off entry: a code is a code.
      await openOnline(b.page);
      await join(b.page, code, 'Dad');
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 2, null, { timeout: 10000 });
      await b.page.waitForSelector('#olPieces:not([hidden])', { timeout: 10000 });
      assert((await a.page.textContent('#olRulesLobby')).includes('1'), 'the lobby says one seat goes to the computer');

      // D-B12: first come first served, and a taken piece cannot be had.
      await a.page.click(pieceButton(a.page, 3));
      await b.page.waitForSelector(`${pieceButton(b.page, 3)}.taken`, { timeout: 10000 });
      assert(await b.page.$eval(pieceButton(b.page, 3), (e) => e.disabled), 'a taken piece is not offered');
      await b.page.click(pieceButton(b.page, 1));
      await a.page.waitForSelector(`${pieceButton(a.page, 1)}.taken`, { timeout: 10000 });
      await a.page.waitForSelector(`${pieceButton(a.page, 3)}.on`, { timeout: 10000 });

      await a.page.click('#olStart');
      for (const p of [a.page, b.page]) {
        await p.waitForSelector('#bgScreen.online.show #bgPlay:not([hidden])', { timeout: 15000 });
        assert(!(await visible(p, '#olScreen')), 'the online screen steps aside for the board');
      }
      const sa = await state(a.page);
      const sb = await state(b.page);
      assertEq(JSON.stringify(sa.online.table), JSON.stringify(sb.online.table), 'both phones see one table');
      assertEq(sa.online.npc.filter(Boolean).length, 1, 'the empty seat is the computer\'s');
      assertEq(sa.online.pieces[sa.online.me], 3, 'Mom kept her piece');
      assertEq(sb.online.pieces[sb.online.me], 1, 'Dad kept his');
      assertEq(sa.tiles, 42, 'Sprint');
      assert(!sa.word, 'the mirror never holds a word');

      // F6: a reaction reaches the other board.
      await b.page.click('#bgEmotes [data-emote="0"]');
      await a.page.waitForFunction(() => (document.getElementById('bgFeed').textContent || '').startsWith('Dad '), null, { timeout: 10000 });

      let dropped = false;
      const spelled = [];
      const deadline = Date.now() + 240000;
      while (Date.now() < deadline) {
        if ((await visible(a.page, '#bgOver')) && (await visible(b.page, '#bgOver'))) break;
        const s = await state(b.page);
        // A1 (in short): Dad's socket drops while his word is live, and comes back to it.
        if (!dropped && s && s.online && s.online.mine && s.phase === 'AwaitSpelling' && s.online.ticket) {
          dropped = true;
          const seat = s.seat;
          if (process.env.BG_DEBUG) frames.push('---- cut');
          srv.px.cut(3000);
          await b.page.waitForSelector('#bgReconnect:not([hidden])', { timeout: 10000 });
          await b.page.waitForSelector('#bgReconnect', { state: 'hidden', timeout: 20000 });
          await b.page.waitForFunction((seat) => {
            const t = JSON.parse(window.__spelltest.boardgameState() || 'null');
            return t && t.online && t.seat === seat && t.phase === 'AwaitSpelling';
          }, seat, { timeout: 10000 }).catch(async (e) => {
            if (process.env.BG_DEBUG) (await import('node:fs')).writeFileSync(process.env.BG_DEBUG, frames.join('\n'));
            throw new Error(`after the resume: ${JSON.stringify(await state(b.page))} panel=${await b.page.evaluate(() => [...document.querySelectorAll('.ol-panel')].filter((p) => !p.hidden).map((p) => p.id))} bg=${await b.page.$eval('#bgScreen', (e) => e.className)} (${e.message.split('\n')[0]})`);
          });
          continue;
        }
        const before = await state(a.page);
        const moved = (await act(a.page, () => last, true)) || (await act(b.page, () => last, false));
        if (moved && before && before.online && before.online.mine && before.phase === 'AwaitSpelling' && last && !spelled.includes(last)) spelled.push(last);
        const t = await state(a.page);
        if (t && t.word) throw new Error(`the mirror held a word: ${t.word}`);
        // sleep-ok: the loop polls two phones for their turn; there is no single event that covers both.
        if (!moved) await a.page.waitForTimeout(150);
      }
      assert(dropped, 'Dad never had a live word to drop on');
      for (const p of [a.page, b.page]) {
        await p.waitForSelector('#bgOver:not([hidden])', { timeout: 30000 });
        assertEq(await p.$$eval('#bgPodium .bg-pod', (r) => r.length), 3, 'a row per seat');
        assert(await visible(p, '.bg-online'), 'the result says it was online');
        assert(!(await visible(p, '#bgShare')), 'no share card online');
      }
      const ra = (await state(a.page)).online.ranking;
      const rb = (await state(b.page)).online.ranking;
      assertEq(JSON.stringify(ra), JSON.stringify(rb), 'both phones agree who won');
      const played = await a.page.evaluate(() => JSON.parse(localStorage.getItem('bg_online_matches_v1') || 'null'));
      assertEq(played && played.played, 1, 'the local count of online games');
      // F8: each phone's own words reached its own no-repeat history, and only its own.
      const seen = (p) => p.evaluate(() => localStorage.getItem('spell_wordgrid_seen_v1') || '');
      const [la, lb] = [await seen(a.page), await seen(b.page)];
      assert(spelled.length > 0 && spelled.every((w) => la.includes(w)), `Mom's words missing from her history: ${spelled}`);
      assert(lb.includes('boardgame:en:'), 'Dad\'s words never reached his history');
      assert(!spelled.some((w) => lb.includes(`"${w}"`)), 'Mom\'s words were written to Dad\'s history');

      // Back to the room: the lobby again, where the host can start another.
      await a.page.click('#bgAgain');
      await a.page.waitForSelector('#olLobby:not([hidden])', { timeout: 10000 });
      assert(!(await visible(a.page, '#bgScreen')), 'the board closed');
      // The way out of the board leaves the room too (mode-exit-check).
      await b.page.click('#bgOverExit');
      await a.page.waitForFunction(() => document.querySelectorAll('#olPlayers li').length === 1, null, { timeout: 10000 });
    } finally { await a.ctx.close(); await b.ctx.close(); srv.close(); }
  });

  await suite.test('BG-ONLINE: with online.boardgame off there is no board entry and no game choice', async () => {
    const a = await app(browser, base, null, { flags: { online_enabled: 'on' } });
    try {
      await a.page.evaluate(() => document.getElementById('bgOpenBtn').click());
      await domSettled(a.page);
      assert(!(await visible(a.page, '#bgOnline')), 'the board shows Play online with its flag off');
      await a.page.evaluate(() => document.getElementById('bgSetupExit').click());
      await openOnline(a.page);
      assert(!(await visible(a.page, '#olModeRow')), 'the game choice shows with the flag off');
      assert(await visible(a.page, '#olH2hOpts'), 'Spell Off settings are the only ones');
    } finally { await a.ctx.close(); }
  });
}
