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

/// Wait for the game to move on from `before` (a serialised state): a roll, a
/// submit or a card changes it. No clock: the app is the signal.
async function moved(page, before) {
  await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 4000 }).catch(() => {});
}
const raw = (page) => page.evaluate(() => window.__spelltest.boardgameState());

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
        const before = await raw(page);
        const s = JSON.parse(before);
        if (s.phase === 'AwaitRoll') { await page.click('#bgOrb'); await moved(page, before); continue; }
        if (s.phase === 'AwaitSpelling') {
          await typeIt(page, cite(s.word));
          await page.click('#bgGo');
          await moved(page, before);
          continue;
        }
        // Between a submit and the next card: wait for the state or a card to change.
        await page.waitForFunction(() => document.getElementById('bgHand')?.hidden === false || document.getElementById('bgOver')?.hidden === false
          || JSON.parse(window.__spelltest.boardgameState()).phase === 'AwaitRoll', null, { timeout: 4000 }).catch(() => {});
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
      // A frame-by-frame watcher: the key grid must never be visible while an
      // NPC is playing. It samples every frame, so nothing is missed between polls.
      await page.evaluate(() => {
        window.__bgKeysSeenDuringNpc = false;
        const tick = () => {
          const k = document.getElementById('bgKeys');
          const s = JSON.parse(window.__spelltest.boardgameState() || 'null');
          if (s && s.npc && k && k.offsetParent !== null) window.__bgKeysSeenDuringNpc = true;
          requestAnimationFrame(tick);
        };
        tick();
      });
      let untouched = null;
      let tapped = null;
      for (let guard = 0; guard < 80 && (untouched === null || tapped === null); guard++) {
        const before = await raw(page);
        const s = JSON.parse(before);
        if (s.phase === 'Finished') break;
        if (s.npc) {
          let t0 = Date.now();
          const wantTap = untouched !== null;
          if (wantTap) {
            // Tap once the first NPC has actually moved (its chip is up).
            const chips = await page.$eval('#bgResults', (e) => e.innerHTML);
            await page.waitForFunction((c) => document.getElementById('bgResults').innerHTML !== c, chips, { timeout: 4000 });
            t0 = Date.now(); // the budget runs from the tap
            await page.click('#bgTitle');
          }
          await page.waitForFunction(() => !JSON.parse(window.__spelltest.boardgameState()).npc, null, { timeout: 12000, polling: 'raf' });
          const dt = Date.now() - t0;
          if (wantTap) tapped = dt; else untouched = dt;
          continue;
        }
        if (s.phase === 'AwaitRoll') { await page.click('#bgOrb'); await moved(page, before); continue; }
        if (s.phase === 'AwaitSpelling') {
          await typeIt(page, cite(s.word));
          await page.click('#bgGo');
          await moved(page, before);
          continue;
        }
        if (s.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); await moved(page, before); continue; }
      }
      const keysSeen = await page.evaluate(() => window.__bgKeysSeenDuringNpc);
      assert(untouched !== null, 'an NPC run was observed');
      assert(untouched <= 7000, `three NPC turns untouched took ${untouched} ms (budget 7000)`);
      assert(tapped !== null, 'a second NPC run was observed');
      assert(tapped <= 1000, `an NPC run after a tap took ${tapped} ms (budget 1000)`);
      assert(!keysSeen, 'the key grid was visible during an NPC turn (I11)');
    } finally { await ctx.close(); }
  });

  // v1.2 follow-up -- the landing callout: before a human types, the board names the
  // destination's tier (never the word); when a trap is triggered, it names the trap. With the
  // centre stage (Polish Feature 5) the callout lives in the stage when the ring is full-size,
  // and in the old pill otherwise; the loop below reads whichever is on screen.
  await suite.test('boardgame_landing_callout_names_tier_and_trap', async () => {
    const TIERS = { easy: 'Easy', medium: 'Medium', hard: 'Hard', expert: 'Expert' };
    let sawTier = 0, sawTrap = null;
    for (let game = 0; game < 4 && !sawTrap; game++) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG });
      try {
        await startGame(page, { mode: 'pass', count: 2 });
        await page.evaluate(() => {
          window.__bgTrapPills = [];
          new MutationObserver(() => {
            const t = document.querySelector('#bgView [data-land="trap"]');
            if (t) window.__bgTrapPills.push(t.textContent);
          }).observe(document.getElementById('bgView'), { childList: true, subtree: true, characterData: true });
        });
        for (let i = 0; i < 400; i++) {
          if (await page.isVisible('#bgOver')) break;
          // Deterministic hand-off handling: the engine already says the next seat owes a card
          // (state.hand) a beat before the card is on screen, so wait for the card itself rather
          // than race it (the old loop clicked the orb under a card that was about to appear).
          const st0 = await state(page);
          if (st0.hand) {
            await page.waitForFunction(() => !document.getElementById('bgHand').hidden || !document.getElementById('bgMiss').hidden || !document.getElementById('bgOver').hidden, null, { timeout: 8000 });
            if (await page.isVisible('#bgOver')) break;
            if (await page.isVisible('#bgMiss')) await page.click('#bgMissGo');
            else await page.click('#bgHandGo');
            continue;
          }
          if (await page.isVisible('#bgMiss')) { await page.click('#bgMissGo'); continue; }
          const before = await raw(page);
          const st = JSON.parse(before);
          if (st.phase === 'AwaitRoll') { await page.click('#bgOrb'); await moved(page, before); continue; }
          if (st.phase === 'AwaitSpelling') {
            if (st.kind === 'Landing') {
              await page.waitForFunction((t) => {
                const el = document.querySelector('#bgView [data-tier]');
                return !!el && el.textContent.trim() === t;
              }, TIERS[st.tier], { timeout: 3000 });
              const sw = await page.$eval('#bgView [data-tier]', (e) => e.previousElementSibling.className);
              assert(sw.includes(`t-${st.tier}`), `the swatch matches the tier: ${sw}`);
              const shown = await page.$eval('#bgView', (e) => e.textContent);
              assert(!shown.toLowerCase().includes(cite(st.word).toLowerCase()), 'the callout never shows the word');
              sawTier++;
            }
            await typeIt(page, cite(st.word));
            await page.click('#bgGo');
            await moved(page, before);
            continue;
          }
          if (st.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); await moved(page, before); continue; }
          await page.waitForFunction(() => document.getElementById('bgHand')?.hidden === false || document.getElementById('bgOver')?.hidden === false
            || JSON.parse(window.__spelltest.boardgameState()).phase === 'AwaitRoll', null, { timeout: 4000 }).catch(() => {});
        }
        const pills = await page.evaluate(() => window.__bgTrapPills);
        if (pills.length) sawTrap = pills[0];
      } finally { await ctx.close(); }
    }
    assert(sawTier > 0, 'a tier callout was seen before spelling');
    assert(sawTrap !== null, 'a trap landing never showed its callout in four games');
    assert(/(Back to Start|Long Word|Lose a Roll|Switch Tiles|Double Expert)/.test(sawTrap), `the trap callout names the trap: ${sawTrap}`);
  });
}
