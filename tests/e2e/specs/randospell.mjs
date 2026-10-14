// randospell.spec -- CC-RANDOSPELL Phase A (Sprint), in the browser: the exit,
// the roll, the length law (I2), scoring, a miss, the end card (A7), the 30 s
// timer's miss (A14) and Reduce Motion (A16). The suite types through the real
// keys and reads the owed word from the observation-only seam
// (__spelltest.randospellState), the way boardgame.mjs does.
import { openApp, assert } from '../harness.mjs';

const FLAG = () => { localStorage.setItem('spell_flag_randospell', 'on'); };
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.randospellState() || 'null'));

export async function run(browser, base, suite) {
  const open = async ({ lang = 'en', start = true, reduce = false } = {}) => {
    const o = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 }, init: FLAG });
    if (reduce) await o.page.emulateMedia({ reducedMotion: 'reduce' });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
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
}
