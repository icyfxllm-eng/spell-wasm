// boardgame.spec -- CC-BOARD-GAME v1.2 A4, A5, A10 on the real screen.
//
// The mode is dark by default (flag off), so every session turns it on before
// the app boots. The word a human owes is heard, never shown, so the test reads
// it from the observation-only seam (__spelltest.boardgameState) and types it
// through the real key grid, exactly as a player would.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const FLAG = () => localStorage.setItem('spell_flag_boardgame', 'on');

const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));

async function openBg(page) {
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
}

async function startGame(page, { mode = 'pass', count = 2 } = {}) {
  await openBg(page);
  await page.click(`[data-bg="mode:${mode}"]`);
  await page.click(`[data-bg="count:${count}"]`);
  await page.click('#bgStart');
  await page.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
}

/// Type a bank entry through the key grid. The citation (before any pipe) is
/// what a player types; every unit must be a key on screen.
async function typeIt(page, typed) {
  for (const ch of typed) {
    const k = await page.$(`#bgKeys [data-k="u:${ch}"]`);
    assert(k, `no key for ${JSON.stringify(ch)} on the grid`);
    await k.click();
  }
}

const cite = (w) => w.split('|')[0];

/// Roll and wait for the word (or the end of the game).
async function roll(page) {
  await page.click('#bgOrb');
  await page.waitForFunction(() => {
    const s = JSON.parse(window.__spelltest.boardgameState() || 'null');
    return s && (s.phase === 'AwaitSpelling' || s.phase === 'Finished');
  }, null, { timeout: 4000 });
}

export async function run(browser, base, suite) {
  // A5 -- pass-and-play: the hand-off card, a right answer that moves, a wrong
  // answer that does not and is shown to the one who missed.
  await suite.test('boardgame_pass_and_play_handoff_hit_and_miss', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG });
    try {
      await startGame(page, { mode: 'pass', count: 2 });
      let s = await state(page);
      assert(s.hand, 'the first player gets a hand-off card');
      assert(await page.isVisible('#bgHand'), 'and it is on screen');
      assert(/\d/.test(await page.textContent('#bgHandText')), 'it names a player');
      await page.click('#bgHandGo');
      await domSettled(page);
      assert(!(await page.isVisible('#bgHand')), 'Ready dismisses it');

      // Player A answers right.
      const a = (await state(page)).seat;
      await roll(page);
      s = await state(page);
      if (s.phase === 'Finished') return;
      assert(await page.isVisible('#bgKeys'), 'the key grid is up for a human spelling');
      const before = s.pos[a];
      await typeIt(page, cite(s.word));
      await page.click('#bgGo');
      await page.waitForSelector('#bgHand:not([hidden])', { timeout: 5000 });
      s = await state(page);
      assert(s.pos[a] > before, `a correct word moves the piece (${before} -> ${s.pos[a]})`);
      assertEq(await page.$eval('#bgField', (e) => e.textContent), '', 'nothing the last player typed is visible');
      await page.click('#bgHandGo');

      // Player B answers wrong.
      const b = (await state(page)).seat;
      assert(b !== a, 'the turn passed to the other seat');
      await roll(page);
      s = await state(page);
      if (s.phase === 'Finished') return;
      const posB = s.pos[b];
      const word = s.word;
      await typeIt(page, 'q');
      await page.click('#bgGo');
      await page.waitForSelector('#bgMiss:not([hidden])', { timeout: 5000 });
      assert((await page.textContent('#bgMissText')).includes(cite(word)), 'the miss card shows the correct spelling to the one who missed');
      s = await state(page);
      assertEq(s.pos[b], posB, 'a wrong spelling moves nobody (I4)');
      await page.click('#bgMissGo');
      await page.waitForSelector('#bgHand:not([hidden])', { timeout: 5000 });
    } finally { await ctx.close(); }
  });

  // A5 (cont.) -- the game reaches the podium. Spell Jr: 40 tiles, no traps,
  // so a two-human game is a few dozen turns.
  await suite.test('boardgame_pass_and_play_reaches_the_podium', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG });
    try {
      await startGame(page, { mode: 'pass', count: 2 });
      assertEq((await state(page)).variant, 'Jr', 'Spell Jr plays the Jr variant');
      assertEq((await state(page)).traps, 0, 'and has no traps');
      for (let i = 0; i < 400; i++) {
        if (await page.isVisible('#bgOver')) break;
        if (await page.isVisible('#bgHand')) { await page.click('#bgHandGo'); continue; }
        if (await page.isVisible('#bgMiss')) { await page.click('#bgMissGo'); continue; }
        const s = await state(page);
        if (s.phase === 'AwaitRoll') { await page.click('#bgOrb'); await page.waitForTimeout(40); continue; }
        if (s.phase === 'AwaitSpelling') {
          await typeIt(page, cite(s.word));
          await page.click('#bgGo');
          await page.waitForFunction((w) => {
            const t = JSON.parse(window.__spelltest.boardgameState());
            return t.word !== w || t.phase !== 'AwaitSpelling';
          }, s.word, { timeout: 4000 }).catch(() => {});
          await page.waitForTimeout(60);
          continue;
        }
        await page.waitForTimeout(100);
      }
      await page.waitForSelector('#bgOver:not([hidden])', { timeout: 20000 });
      const rows = await page.$$eval('#bgPodium .bg-pod', (r) => r.length);
      assertEq(rows, 2, 'the podium lists every piece');
    } finally { await ctx.close(); }
  });

  // A10 -- the NPC run: <= 7 s untouched, <= 1 s after a tap, and the key grid
  // never appears while it runs (I11).
  await suite.test('boardgame_npc_run_is_fast_and_never_shows_keys', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG });
    try {
      await startGame(page, { mode: 'solo', count: 3 });
      let untouched = null;
      let tapped = null;
      let keysSeen = false;
      for (let guard = 0; guard < 80 && (untouched === null || tapped === null); guard++) {
        const s = await state(page);
        if (s.phase === 'Finished') break;
        if (s.npc) {
          const t0 = Date.now();
          const wantTap = untouched !== null;
          if (wantTap) { await page.waitForTimeout(350); await page.click('#bgTitle'); }
          while (Date.now() - t0 < 12000) {
            if (await page.isVisible('#bgKeys')) keysSeen = true;
            const n = await state(page);
            if (!n.npc) break;
            await page.waitForTimeout(40);
          }
          const dt = Date.now() - t0;
          if (wantTap) tapped = dt; else untouched = dt;
          continue;
        }
        if (s.phase === 'AwaitRoll') { await page.click('#bgOrb'); await page.waitForTimeout(40); continue; }
        if (s.phase === 'AwaitSpelling') {
          await typeIt(page, cite(s.word));
          await page.click('#bgGo');
          await page.waitForTimeout(120);
          continue;
        }
        if (s.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); continue; }
      }
      assert(untouched !== null, 'an NPC run was observed');
      assert(untouched <= 7000, `three NPC turns untouched took ${untouched} ms (budget 7000)`);
      assert(tapped !== null, 'a second NPC run was observed');
      assert(tapped <= 1000, `an NPC run after a tap took ${tapped} ms (budget 1000)`);
      assert(!keysSeen, 'the key grid was visible during an NPC turn (I11)');
    } finally { await ctx.close(); }
  });
}
