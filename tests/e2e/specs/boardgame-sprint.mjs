// boardgame-sprint.spec -- CC-BOARD-GAME-POLISH v1, Phase B (Feature 4: Sprint).
//   size row, defaults (solo Sprint, pass-and-play Full + remembered), Jr has no picker,
//   Sprint ring fit (11x12 / 10x13), "Tile 0 of 41", the setup screen fits with the extra row.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
// D-P25 (Oct 9 2026): the three rule flags default ON for TestFlight, so a spec that tests the flag-off game switches them off explicitly.
const FLAG = () => { localStorage.setItem('spell_flag_boardgame', 'on'); for (const f of ['boardStretch', 'boardBoosts', 'boardStreak']) localStorage.setItem('spell_flag_' + f, 'off'); };
const SIZES = [[375, 667], [393, 852], [820, 1180]];
// Restated from the spec, not read back from the code under test.
const SPRINT = [[11, 12], [10, 13]];

const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
const on = (page) => page.$$eval('#bgOptSize .bg-btn.on', (b) => b.map((x) => x.dataset.bg));

async function openSetup(page) {
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
}

async function start(page) {
  await page.click('#bgStart');
  await page.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
  await domSettled(page);
}

export async function run(browser, base, suite) {
  await suite.test('boardgame_sprint_solo_defaults_to_sprint_and_can_pick_full', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG });
    try {
      await openSetup(page);
      assertEq((await on(page)).join(), 'size:sprint', 'solo opens on Sprint');
      assertEq((await page.$$('#bgOptSize .bg-btn')).length, 2, 'the row offers Sprint and Full');
      await start(page);
      let s = await state(page);
      assertEq(s.variant, 'Sprint');
      assertEq(s.tiles, 42);
      assertEq(s.traps, 3);
      assert(/^Tile \d+ of 41$/.test(await page.textContent('#bgStTile')), `centre stage counts to the last tile index: ${await page.textContent('#bgStTile')}`);
      await ctx.close();
      const o = await openApp(browser, base, { lang: 'en', init: FLAG });
      await openSetup(o.page);
      await o.page.click('[data-bg="size:full"]');
      await start(o.page);
      s = await state(o.page);
      assertEq(s.variant, 'Full');
      assertEq(s.tiles, 84);
      assertEq(s.traps, 6);
      assert(/^Tile \d+ of 83$/.test(await o.page.textContent('#bgStTile')), 'Full counts to 83');
      await o.ctx.close();
    } catch (e) { await ctx.close().catch(() => {}); throw e; }
  });

  await suite.test('boardgame_sprint_pass_and_play_opens_on_full_and_remembers', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG });
    try {
      await openSetup(page);
      await page.click('[data-bg="mode:pass"]');
      assertEq((await on(page)).join(), 'size:full', 'pass-and-play opens on Full');
      await page.click('[data-bg="size:sprint"]');
      assertEq(await page.evaluate(() => localStorage.getItem('bg_size_pass_v1')), 'sprint', 'the choice is stored');
      // Solo is not remembered: back to solo is Sprint, and a fresh open of the screen too.
      await page.click('[data-bg="mode:solo"]');
      assertEq((await on(page)).join(), 'size:sprint');
      await page.click('#bgSetupExit');
      await page.waitForSelector('#bgScreen:not(.show)', { state: 'attached' });
      await openSetup(page);
      assertEq((await on(page)).join(), 'size:sprint', 'solo on a fresh open');
      await page.click('[data-bg="mode:pass"]');
      assertEq((await on(page)).join(), 'size:sprint', 'pass-and-play remembers the last choice');
      await page.click('[data-bg="size:full"]');
      await page.click('[data-bg="mode:solo"]');
      await page.click('[data-bg="mode:pass"]');
      assertEq((await on(page)).join(), 'size:full', 'and remembers Full again');
    } finally { await ctx.close(); }
  });

  await suite.test('boardgame_sprint_spell_jr_has_no_size_picker', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG });
    try {
      await openSetup(page);
      assertEq((await page.$$('#bgOptSize .bg-btn')).length, 0, 'no picker for Spell Jr');
      await start(page);
      const s = await state(page);
      assertEq(s.variant, 'Jr');
      assertEq(s.tiles, 40);
    } finally { await ctx.close(); }
  });

  for (const [w, h] of SIZES) {
    await suite.test(`boardgame_sprint_ring_fit_${w}x${h}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: w, height: h }, init: FLAG });
      try {
        await openSetup(page);
        const fit = await page.evaluate(() => {
          const s = document.getElementById('bgSetup');
          const b = document.getElementById('bgStart').getBoundingClientRect();
          return { scroll: s.scrollHeight - s.clientHeight, bottom: b.bottom, H: innerHeight };
        });
        assert(fit.scroll <= 1, `the setup screen scrolls by ${fit.scroll}px with the size row`);
        assert(fit.bottom <= fit.H, `Start is below the fold (${fit.bottom} > ${fit.H})`);
        await start(page);
        const m = await page.evaluate((allowed) => {
          const view = document.getElementById('bgView'), board = document.getElementById('bgBoard');
          const landscape = view.clientWidth > view.clientHeight;
          const c = allowed.map(([a, b]) => (landscape ? [b, a] : [a, b])).map(([x, y]) => ({ w: x, h: y, p: Math.min(view.clientWidth / x, view.clientHeight / y) }));
          const best = Math.max(...c.map((q) => q.p));
          const want = c.filter((q) => Math.abs(q.p - best) < 1e-6).sort((p, q) => Math.abs(p.w - p.h) - Math.abs(q.w - q.h))[0];
          return {
            strip: view.classList.contains('strip'), shape: board.getAttribute('data-shape'), want: `${want.w}x${want.h}`,
            tiles: board.querySelectorAll('.bg-t').length, nums: [...board.querySelectorAll('.bg-num')].map((n) => n.textContent),
          };
        }, SPRINT);
        if (m.strip) return; // the unrolled track is the existing small-screen fallback
        assertEq(m.shape, m.want, 'ring-fit picks the largest-pitch Sprint shape');
        assertEq(m.tiles, 42, 'forty-two tiles on the ring');
        assertEq(m.nums.join(','), '10,20,30,40', 'a number on every tenth tile');
        process.stdout.write(`  Sprint ${w}x${h}: ring ${m.shape}\n`);
      } finally { await ctx.close(); }
    });
  }
}
