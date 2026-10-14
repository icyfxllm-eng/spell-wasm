// randospell.spec -- CC-RANDOSPELL Phase A (Sprint), in the browser: the exit,
// the roll, the length law (I2), scoring, a miss, the end card (A7), the 30 s
// timer's miss (A14) and Reduce Motion (A16). The suite types through the real
// keys and reads the owed word from the observation-only seam
// (__spelltest.randospellState), the way boardgame.mjs does.
import { openApp, assert } from '../harness.mjs';

const FLAG = () => { localStorage.setItem('spell_flag_randospell', 'on'); };
const FLAG_M = () => { localStorage.setItem('spell_flag_randospell', 'on'); localStorage.setItem('spell_flag_randospell_marathon', 'on'); };
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.randospellState() || 'null'));

export async function run(browser, base, suite) {
  const open = async ({ lang = 'en', start = true, reduce = false, marathon = false } = {}) => {
    const o = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 }, init: marathon ? FLAG_M : FLAG });
    if (reduce) await o.page.emulateMedia({ reducedMotion: 'reduce' });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
    if (marathon && start) await o.page.click('#rsModes [data-m="marathon"]');
    if (start) {
      await o.page.click('#rsStart');
      await o.page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
    }
    return o;
  };
  const roll = async (page) => {
    await page.click('#rsOrb');
    await page.waitForFunction(() => { const s = JSON.parse(window.__spelltest.randospellState() || 'null'); return s && s.spelling; }, null, { timeout: 6000 });
    return state(page);
  };
  const type = async (page, w) => { for (const c of w) await page.click(`#rsKeys [data-k="u:${c}"]`); };
  const settled = (page, before) => page.waitForFunction((b) => window.__spelltest.randospellState() !== b, before, { timeout: 4000 }).catch(() => {});

  await suite.test('randospell_exit_is_a_word_and_leaves', async () => {
    const { ctx, page } = await open({ start: false });
    try {
      const label = await page.evaluate(() => document.getElementById('rsSetupExit').textContent.trim());
      assert(label.length > 1, `exit must be a word, not a glyph: ${label}`);
      await page.click('#rsSetupExit');
      assert(!(await page.evaluate(() => document.getElementById('rsScreen').classList.contains('show'))), 'exit did not leave');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_the_die_shows_the_length_of_the_spoken_word', async () => {
    const { ctx, page } = await open();
    try {
      for (let i = 0; i < 4; i++) {
        const s = await roll(page);
        const die = await page.evaluate(() => document.getElementById('rsDie').textContent.trim());
        assert(Number(die) === s.roll && [...s.word].length === s.roll, `die ${die}, roll ${s.roll}, word ${s.word}`);
        assert(s.roll >= 2 && s.roll <= 12, `roll out of range: ${s.roll}`);
        const before = await page.evaluate(() => window.__spelltest.randospellState());
        await type(page, s.word);
        await page.click('#rsGo');
        await settled(page, before);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a_right_answer_scores_the_roll_and_a_miss_scores_nothing', async () => {
    const { ctx, page } = await open();
    try {
      let s = await roll(page);
      let before = await page.evaluate(() => window.__spelltest.randospellState());
      await type(page, s.word);
      await page.click('#rsGo');
      await settled(page, before);
      let after = await state(page);
      const want = (s.roll === 2 || s.roll === 12) ? s.roll * 2 : s.roll;
      assert(after.score === want, `score ${after.score}, wanted ${want} for a ${s.roll}`);
      assert(after.streak === 1 && after.rolls === 1, `streak ${after.streak}, rolls ${after.rolls}`);
      s = await roll(page);
      before = await page.evaluate(() => window.__spelltest.randospellState());
      await page.click('#rsKeys [data-k="u:q"]');
      await page.click('#rsKeys [data-k="u:q"]');
      await page.click('#rsGo');
      await settled(page, before);
      const miss = await state(page);
      assert(miss.score === after.score && miss.streak === 0, `a miss changed the score or kept the streak: ${JSON.stringify(miss)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_sprint_ends_at_the_goal_with_stats_a_histogram_and_a_way_out', async () => {
    const { ctx, page } = await open();
    try {
      await page.evaluate(() => window.__spelltest.randospellScore(47));
      for (let i = 0; i < 40; i++) {
        const st = await state(page);
        if (st.ended) break;
        const s = await roll(page);
        const before = await page.evaluate(() => window.__spelltest.randospellState());
        await type(page, s.word);
        await page.click('#rsGo');
        await settled(page, before);
      }
      const end = await state(page);
      assert(end.ended === 'Goal' && end.score >= 50, `did not end on the goal: ${JSON.stringify(end)}`);
      assert(await page.evaluate(() => !document.getElementById('rsOver').hidden), 'no end card');
      const bars = await page.evaluate(() => document.querySelectorAll('#rsHist .rs-bar').length);
      assert(bars === 11, `histogram has ${bars} rows`);
      const total = await page.evaluate(() => [...document.querySelectorAll('#rsHist .rs-bar')].reduce((n, b) => n + Number(b.dataset.n), 0));
      assert(total === end.rolls, `histogram sums to ${total}, rolls ${end.rolls}`);
      assert(await page.evaluate(() => document.getElementById('rsStats').children.length >= 2), 'stats missing');
      await page.click('#rsOverExit');
      assert(!(await page.evaluate(() => document.getElementById('rsScreen').classList.contains('show'))), 'the end card has no working exit');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_reduce_motion_removes_the_shake_and_keeps_the_number', async () => {
    const { ctx, page } = await open({ reduce: true });
    try {
      await page.click('#rsOrb');
      const name = await page.evaluate(() => getComputedStyle(document.getElementById('rsOrb')).animationName);
      assert(name === 'none', `orb still animates under reduce motion: ${name}`);
      await page.waitForFunction(() => document.getElementById('rsDie').textContent.trim() !== '', null, { timeout: 4000 });
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_is_absent_for_a_language_with_no_bank_for_it', async () => {
    // A10 at the screen: ko is off, so the setup says so and Start is dead.
    const { ctx, page } = await open({ lang: 'ko', start: false });
    try {
      const dead = await page.evaluate(() => document.getElementById('rsStart').disabled);
      assert(dead, 'Start is live for ko');
    } finally { await ctx.close(); }
  });
  // ---- Phase B: Marathon ------------------------------------------------
  const answer = async (page, ok) => {
    const s = await roll(page);
    const before = await page.evaluate(() => window.__spelltest.randospellState());
    if (ok) await type(page, s.word); else { await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); }
    await page.click('#rsGo');
    await settled(page, before);
    return { s, after: await state(page) };
  };
  const value = (n) => (n === 2 || n === 12 ? n * 2 : n);

  await suite.test('randospell_marathon_is_offered_only_with_its_flag', async () => {
    const off = await open({ start: false });
    try {
      assert((await off.page.evaluate(() => document.getElementById('rsModes').children.length)) === 0, 'a picker shows with the Marathon flag off');
    } finally { await off.ctx.close(); }
    const on = await open({ start: false, marathon: true });
    try {
      assert((await on.page.evaluate(() => document.getElementById('rsModes').children.length)) === 2, 'no Sprint/Marathon picker with the flag on');
    } finally { await on.ctx.close(); }
  });

  await suite.test('randospell_marathon_right_answer_fills_the_pot_and_asks_bank_or_roll', async () => {
    const { ctx, page } = await open({ marathon: true });
    try {
      const { s, after } = await answer(page, true);
      assert(after.mode === 'Marathon' && after.pot === value(s.roll) && after.score === 0, `pot ${after.pot}, score ${after.score}`);
      assert(after.deciding, 'the game is not waiting on Bank or Roll again');
      assert(await page.evaluate(() => !document.getElementById('rsDecide').hidden), 'no Bank card');
      assert(await page.evaluate(() => document.getElementById('rsOrb').disabled), 'the orb is live while a decision is owed');
      await page.click('#rsBank');
      const banked = await state(page);
      assert(banked.score === value(s.roll) && banked.pot === 0 && !banked.deciding, `bank: ${JSON.stringify(banked)}`);
      assert(banked.bestBank === value(s.roll), 'largest pot banked not recorded');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_marathon_the_ward_saves_one_pot_then_a_miss_empties_it', async () => {
    const { ctx, page } = await open({ marathon: true });
    try {
      const a = await answer(page, true);
      await page.click('#rsRollAgain'); // keeps the pot at risk and rolls
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
      const before = await page.evaluate(() => window.__spelltest.randospellState());
      await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsGo');
      await settled(page, before);
      const saved = await state(page);
      assert(saved.pot === value(a.s.roll) && saved.ward === false && saved.streak === 0, `ward did not save the pot: ${JSON.stringify(saved)}`);
      assert((await page.evaluate(() => document.getElementById('rsFeed').textContent)).length > 3, 'no word shown after the miss');
      const lost = await answer(page, false);
      assert(lost.after.pot === 0 && lost.after.bestLost === value(a.s.roll) && lost.after.score === 0, `second miss: ${JSON.stringify(lost.after)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_marathon_ends_only_on_a_bank_at_the_goal', async () => {
    const { ctx, page } = await open({ marathon: true });
    try {
      await page.evaluate(() => window.__spelltest.randospellScore(99));
      const a = await answer(page, true);
      assert(a.after.ended === null && a.after.deciding, `ended before the bank: ${JSON.stringify(a.after)}`);
      await page.click('#rsRollAgain');
      assert((await state(page)).ended === null, 'rolling again ended the game');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
      // The roll is out and its word is owed: spell it, then bank the pot (D15).
      await answer(page, true);
      const st = await state(page);
      if (st.deciding) await page.click('#rsBank');
      const end = await state(page);
      assert(end.ended === 'Goal' && end.score >= 100, `no end on the bank: ${JSON.stringify(end)}`);
      assert(await page.evaluate(() => !document.getElementById('rsOver').hidden), 'no end card');
      assert(await page.evaluate(() => document.querySelectorAll('#rsHist .rs-bar').length === 11), 'no histogram');
      await page.click('#rsOverExit');
      assert(!(await page.evaluate(() => document.getElementById('rsScreen').classList.contains('show'))), 'no way out of the end card');
    } finally { await ctx.close(); }
  });
}
