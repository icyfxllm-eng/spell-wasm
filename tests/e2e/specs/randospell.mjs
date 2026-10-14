// randospell.spec -- CC-RANDOSPELL Phase A (Sprint), in the browser: the exit,
// the roll, the length law (I2), scoring, a miss, the end card (A7), the 30 s
// timer's miss (A14) and Reduce Motion (A16). The suite types through the real
// keys and reads the owed word from the observation-only seam
// (__spelltest.randospellState), the way boardgame.mjs does.
import { openApp, assert } from '../harness.mjs';

const FLAG = () => { localStorage.setItem('spell_flag_randospell', 'on'); };
const FLAG_ALL = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  localStorage.setItem('spell_flag_randospell_daily', 'on');
  localStorage.setItem('spell_flag_randospell_h2h', 'on');
};
const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const FLAG_JR = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_jr', 'on');
};
const FLAG_M = () => { localStorage.setItem('spell_flag_randospell', 'on'); localStorage.setItem('spell_flag_randospell_marathon', 'on'); };
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.randospellState() || 'null'));

export async function run(browser, base, suite) {
  const open = async ({ lang = 'en', start = true, reduce = false, marathon = false, kind = null, jr = false, jrFlag = true } = {}) => {
    const init = jr ? (jrFlag ? FLAG_JR : FLAG) : kind ? FLAG_ALL : marathon ? FLAG_M : FLAG;
    const o = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 }, init, ...(jr ? { age: KID } : {}) });
    if (reduce) await o.page.emulateMedia({ reducedMotion: 'reduce' });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
    if (marathon && start) await o.page.click('#rsModes [data-m="marathon"]');
    if (kind && start) await o.page.click(`#rsModes [data-m="${kind}"]`);
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
  // ---- Phase C: Daily and Head-to-Head -----------------------------------
  const served3 = async (page) => {
    const out = [];
    for (let i = 0; i < 3; i++) {
      const s = await roll(page);
      out.push([s.roll, s.word]);
      const before = await page.evaluate(() => window.__spelltest.randospellState());
      await type(page, s.word);
      await page.click('#rsGo');
      await settled(page, before);
      if ((await state(page)).deciding) await page.click('#rsRollAgain'); // keeps rolling on the same stream
    }
    return out;
  };

  await suite.test('randospell_daily_and_h2h_are_offered_only_with_their_flags', async () => {
    const m = await open({ start: false, marathon: true });
    try {
      const ids = await m.page.evaluate(() => [...document.querySelectorAll('#rsModes [data-m]')].map((b) => b.dataset.m));
      assert(ids.join() === 'sprint,marathon', `with only the Marathon flag: ${ids}`);
    } finally { await m.ctx.close(); }
    const a = await open({ start: false, kind: 'sprint' });
    try {
      const ids = await a.page.evaluate(() => [...document.querySelectorAll('#rsModes [data-m]')].map((b) => b.dataset.m));
      assert(ids.join() === 'sprint,marathon,daily,h2h', `with every flag: ${ids}`);
    } finally { await a.ctx.close(); }
  });

  await suite.test('randospell_daily_is_the_same_game_for_everyone_and_one_attempt', async () => {
    const one = await open({ kind: 'daily' });
    let a;
    try {
      const st = await state(one.page);
      assert(st.kind === 'Daily' && st.dailyKey, `not a daily: ${JSON.stringify(st)}`);
      a = await served3(one.page);
      // Leave mid-game: the attempt stays spent.
      await one.page.click('#rsExit');
      await one.page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await one.page.click('#rsModes [data-m="daily"]');
      assert(await one.page.evaluate(() => document.getElementById('rsStart').disabled), 'a second Daily attempt can start');
      assert((await one.page.evaluate(() => document.getElementById('rsNote').textContent)).length > 3, 'no reason given');
    } finally { await one.ctx.close(); }
    const two = await open({ kind: 'daily' });
    try {
      const b = await served3(two.page);
      assert(JSON.stringify(a) === JSON.stringify(b), `different games on one day: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`);
    } finally { await two.ctx.close(); }
  });

  await suite.test('randospell_h2h_a_miss_hands_over_with_a_bonus_roll_and_a_bank_wins', async () => {
    const { ctx, page } = await open({ kind: 'h2h' });
    try {
      let st = await state(page);
      assert(st.kind === 'H2h' && st.turn === 0, `bad start: ${JSON.stringify(st)}`);
      await answer(page, false); // player 1 misses
      assert(await page.evaluate(() => !document.getElementById('rsHand').hidden), 'no hand-off card');
      assert(await page.evaluate(() => document.getElementById('rsHandExit') !== null), 'the card has no exit');
      st = await state(page);
      assert(st.turn === 1 && st.bonus === true, `no bonus roll for the next player: ${JSON.stringify(st)}`);
      await page.click('#rsHandGo');
      // Player 2 is 98 up with a free bonus roll: any right answer is at least 3 points, banked
      // at once, so the game ends on that Bank and player 2 (index 1) wins (D15).
      await page.evaluate(() => window.__spelltest.randospellScore(98));
      const r1 = await answer(page, true);
      assert(r1.after.ended === 'Goal' && r1.after.score >= 100, `the bonus roll did not bank to the goal: ${JSON.stringify(r1.after)}`);
      assert(r1.after.winner === 1, `wrong winner: ${r1.after.winner}`);
      assert(await page.evaluate(() => !document.getElementById('rsOver').hidden), 'no end card');
    } finally { await ctx.close(); }
  });
  // ---- Phase D: Spell Jr -----------------------------------------------
  await suite.test('randospell_jr_is_off_without_its_flag', async () => {
    const { ctx, page } = await open({ jr: true, jrFlag: false, start: false });
    try {
      assert(await page.evaluate(() => document.getElementById('rsStart').disabled), 'Jr can start with randospell_jr off');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_jr_is_off_for_swahili_and_filipino', async () => {
    for (const lang of ['sw', 'fil']) {
      const { ctx, page } = await open({ jr: true, lang, start: false });
      try {
        assert(await page.evaluate(() => document.getElementById('rsStart').disabled), `${lang}: Jr can start`);
      } finally { await ctx.close(); }
    }
  });

  await suite.test('randospell_jr_one_die_no_pot_no_ward_no_timer', async () => {
    const { ctx, page } = await open({ jr: true });
    try {
      assert(await page.evaluate(() => document.getElementById('rsModes').children.length === 0 && document.getElementById('rsTiers').children.length === 0), 'Jr shows a mode or tier picker');
      for (let i = 0; i < 4; i++) {
        const s = await roll(page);
        const st = await state(page);
        assert(st.jr === true && st.mode === 'Jr', `not the Jr game: ${JSON.stringify(st)}`);
        const die = Number(await page.evaluate(() => document.getElementById('rsDie').textContent.trim()));
        assert(die >= 1 && die <= 6 && s.roll === die + 1 && [...s.word].length === die + 1, `die ${die}, length ${s.roll}, word ${s.word}`);
        assert(await page.evaluate(() => document.getElementById('rsPot').hidden && document.getElementById('rsWard').hidden), 'pot or Ward shown in Jr');
        assert(await page.evaluate(() => !document.getElementById('rsTimer').classList.contains('run')), 'a timer runs in Jr');
        const before = await page.evaluate(() => window.__spelltest.randospellState());
        await type(page, s.word);
        await page.click('#rsGo');
        await settled(page, before);
        const after = await state(page);
        assert(after.score >= die, `a right word did not score the die: ${JSON.stringify(after)}`);
        assert(await page.evaluate(() => document.getElementById('rsDecide').hidden), 'Bank / Roll again shown in Jr');
      }
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_jr_a_miss_replays_the_word_for_one_more_try', async () => {
    const { ctx, page } = await open({ jr: true });
    try {
      const s = await roll(page);
      let before = await page.evaluate(() => window.__spelltest.randospellState());
      await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsGo');
      await settled(page, before);
      let st = await state(page);
      assert(st.spelling && st.word === s.word && st.score === 0, `the first miss did not hold the word: ${JSON.stringify(st)}`);
      assert((await page.evaluate(() => document.getElementById('rsFeed').textContent)).length > 3, 'no encouragement shown');
      // The second try, right: scores the die in full.
      before = await page.evaluate(() => window.__spelltest.randospellState());
      await type(page, s.word);
      await page.click('#rsGo');
      await settled(page, before);
      st = await state(page);
      assert(st.score === s.roll - 1 && st.spelling === false, `retry did not score: ${JSON.stringify(st)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_jr_two_misses_move_on_and_the_end_card_has_six_faces', async () => {
    const { ctx, page } = await open({ jr: true });
    try {
      await roll(page);
      for (let i = 0; i < 2; i++) {
        const before = await page.evaluate(() => window.__spelltest.randospellState());
        await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsGo');
        await settled(page, before);
      }
      const st = await state(page);
      assert(st.spelling === false && st.score === 0 && st.streak === 0, `two misses did not move on: ${JSON.stringify(st)}`);
      await page.evaluate(() => window.__spelltest.randospellScore(48));
      for (let i = 0; i < 30; i++) {
        if ((await state(page)).ended) break;
        const s = await roll(page);
        const before = await page.evaluate(() => window.__spelltest.randospellState());
        await type(page, s.word);
        await page.click('#rsGo');
        await settled(page, before);
      }
      assert((await state(page)).ended === 'Goal', 'Jr did not end at the goal');
      assert(await page.evaluate(() => document.querySelectorAll('#rsHist .rs-bar').length === 6), 'the Jr end card is not a six-face histogram');
      assert(await page.evaluate(() => document.getElementById('rsAgain') !== null && document.getElementById('rsOverExit') !== null), 'end card controls missing');
    } finally { await ctx.close(); }
  });
  // ---- Harness gaps closed before the cut --------------------------------
  await suite.test('randospell_a15_a_full_run_changes_only_its_own_storage', async () => {
    // A15 / I9: play Sprint, Marathon and a Daily; every other key (Climb, Daily
    // Challenge, shields, streaks) must be byte-identical afterwards. The only keys
    // allowed to change are the mode's own store and the shared no-repeat history (D9).
    const { ctx, page } = await open({ kind: 'sprint', start: false });
    const dump = () => page.evaluate(() => Object.fromEntries(Object.keys(localStorage).sort().map((k) => [k, localStorage.getItem(k)])));
    // spell_tel_perf_v1 is the shared audio path's anonymous latency counter: api::play_word
    // writes it for every mode that speaks a word, and Randospell's own code never touches it
    // (the screen's telemetry test bans it). Anything else changing is a failure.
    const allowed = new Set(['spell_randospell_v1', 'spell_wordgrid_seen_v1', 'spell_tel_perf_v1']);
    try {
      const before = await dump();
      const play = async (mode, seed) => {
        await page.click(`#rsModes [data-m="${mode}"]`);
        await page.click('#rsStart');
        await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
        if (seed) await page.evaluate((n) => window.__spelltest.randospellScore(n), seed);
      };
      // Sprint to the goal.
      await play('sprint', 48);
      for (let i = 0; i < 30; i++) {
        if ((await state(page)).ended) break;
        const s = await roll(page);
        const b = await page.evaluate(() => window.__spelltest.randospellState());
        await type(page, s.word); await page.click('#rsGo'); await settled(page, b);
      }
      assert((await state(page)).ended === 'Goal', 'the Sprint did not finish');
      await page.click('#rsOverExit');
      // Marathon to a Bank at the goal.
      await page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await play('marathon', 99);
      await answer(page, true);
      await page.click('#rsBank');
      assert((await state(page)).ended === 'Goal', 'the Marathon did not finish');
      await page.click('#rsOverExit');
      // A Daily, abandoned after one roll.
      await page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await play('daily', 0);
      await answer(page, true);
      await page.click('#rsExit');
      const after = await dump();
      const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
      const changed = [...keys].filter((k) => before[k] !== after[k] && !allowed.has(k));
      assert(changed.length === 0, `Randospell touched other storage: ${changed.join(', ')}`);
      assert(after['spell_randospell_v1'], 'the mode wrote no store of its own');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a9_the_info_sheet_lists_a_remap_where_one_exists', async () => {
    const infoLines = async (lang) => {
      const { ctx, page } = await open({ lang, start: false });
      try {
        return await page.evaluate(() => [...document.querySelectorAll('#rsInfo li')].map((li) => li.textContent.trim()));
      } finally { await ctx.close(); }
    };
    const es = await infoLines('es');
    assert(es.length >= 1 && es.some((t) => /2/.test(t) && /3/.test(t)), `es lists no 2 -> 3 remap: ${JSON.stringify(es)}`);
    const en = await infoLines('en');
    assert(en.length === 0, `en has full coverage but lists remaps: ${JSON.stringify(en)}`);
  });
}
