// randospell.spec -- CC-RANDOSPELL Phase A (Sprint), in the browser: the exit,
// the roll, the length law (I2), scoring, a miss, the end card (A7), the 30 s
// timer's miss (A14) and Reduce Motion (A16). The suite types through the real
// keys and reads the owed word from the observation-only seam
// (__spelltest.randospellState), the way boardgame.mjs does.
import { openApp, assert } from '../harness.mjs';

// The shipped defaults are randospell + marathon ON, daily/h2h/jr OFF; every test states
// the flags it means so a default change cannot silently change what it proves.
const FLAG = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'off');
  localStorage.setItem('spell_flag_randospell_daily', 'off');
  localStorage.setItem('spell_flag_randospell_h2h', 'off');
  localStorage.setItem('spell_flag_randospell_jr', 'off');
};
const FLAG_ALL = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  localStorage.setItem('spell_flag_randospell_daily', 'on');
  localStorage.setItem('spell_flag_randospell_h2h', 'on');
};
const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const FLAG_JR = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'off');
  localStorage.setItem('spell_flag_randospell_jr', 'on');
};
// v1.1 Phase E: every mechanic on, in Marathon (heat needs a pot).
const FLAG_V11 = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  localStorage.setItem('spell_flag_randospell_daily', 'off');
  localStorage.setItem('spell_flag_randospell_h2h', 'off');
  localStorage.setItem('spell_flag_randospell_jr', 'off');
  for (const f of ['heat', 'call', 'stretch', 'doubles', 'ear']) localStorage.setItem(`spell_flag_randospell_${f}`, 'on');
};
// v1.1 Phase F: the NPC table (heat on so NPCs and the human share one payout).
const FLAG_TABLE = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'off');
  localStorage.setItem('spell_flag_randospell_daily', 'off');
  localStorage.setItem('spell_flag_randospell_h2h', 'off');
  localStorage.setItem('spell_flag_randospell_jr', 'off');
  localStorage.setItem('spell_flag_randospell_npc', 'on');
  localStorage.setItem('spell_flag_randospell_heat', 'on');
};
// v1.1 Phase G: the table with Steal and the Final Round on.
const FLAG_G = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  localStorage.setItem('spell_flag_randospell_npc', 'on');
  localStorage.setItem('spell_flag_randospell_heat', 'on');
  localStorage.setItem('spell_flag_randospell_steal', 'on');
  localStorage.setItem('spell_flag_randospell_lastcall', 'on');
};
// v1.1 Phase I: the Eclipse visuals over the Phase E mechanics (Marathon).
const FLAG_FX = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  for (const f of ['heat', 'call', 'stretch', 'doubles', 'ear', 'fx']) localStorage.setItem(`spell_flag_randospell_${f}`, 'on');
};
const FLAG_FX_OFF = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  for (const f of ['heat', 'call', 'stretch', 'doubles', 'ear']) localStorage.setItem(`spell_flag_randospell_${f}`, 'on');
  localStorage.setItem('spell_flag_randospell_fx', 'off');
};
const FLAG_JR_TABLE = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_jr', 'on');
  localStorage.setItem('spell_flag_randospell_npc', 'on');
};
const FLAG_M = () => {
  localStorage.setItem('spell_flag_randospell', 'on');
  localStorage.setItem('spell_flag_randospell_marathon', 'on');
  localStorage.setItem('spell_flag_randospell_daily', 'off');
  localStorage.setItem('spell_flag_randospell_h2h', 'off');
};
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.randospellState() || 'null'));

export async function run(browser, base, suite) {
  const open = async ({ lang = 'en', start = true, reduce = false, marathon = false, kind = null, jr = false, jrFlag = true, v11 = false } = {}) => {
    if (v11) marathon = true;
    const init = v11 ? FLAG_V11 : jr ? (jrFlag ? FLAG_JR : FLAG) : kind ? FLAG_ALL : marathon ? FLAG_M : FLAG;
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
    // es needs Full for a viable bank (D10), so the dev entitlement door is on here.
    const DEV = () => { localStorage.setItem('spell_flag_randospell', 'on'); localStorage.setItem('spell_dev_entitlements', 'on'); };
    const infoLines = async (lang, init) => {
      const o = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 }, init });
      try {
        await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
        await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
        return await o.page.evaluate(() => [...document.querySelectorAll('#rsInfo li')].map((li) => li.textContent.trim()));
      } finally { await o.ctx.close(); }
    };
    const es = await infoLines('es', DEV);
    assert(es.length >= 1 && es.some((t) => /2/.test(t) && /3/.test(t)), `es lists no 2 -> 3 remap: ${JSON.stringify(es)}`);
    const en = await infoLines('en', FLAG);
    assert(en.length === 0, `en has full coverage but lists remaps: ${JSON.stringify(en)}`);
  });

  await suite.test('randospell_d10_preview_owns_the_first_tier_and_an_unviable_bank_is_absent', async () => {
    // Free tier: es is Preview (first tier only). Its Easy bank cannot carry the
    // die, so the mode is absent for that player: Start is dead and says why.
    const es = await open({ lang: 'es', start: false });
    try {
      assert(await es.page.evaluate(() => document.getElementById('rsStart').disabled), 'es at Preview can start');
    } finally { await es.ctx.close(); }
    // ru's Easy bank does carry it: Start is live and only Easy is offered.
    const ru = await open({ lang: 'ru', start: false });
    try {
      assert(!(await ru.page.evaluate(() => document.getElementById('rsStart').disabled)), 'ru at Preview cannot start');
      const tiers = await ru.page.evaluate(() => [...document.querySelectorAll('#rsTiers [data-t]')].map((b) => b.dataset.t));
      assert(tiers.join() === '0', `Preview offers tiers ${tiers}`);
    } finally { await ru.ctx.close(); }
  });
  // ---- v1.1 Phase E ---------------------------------------------------------
  // With Stretch or Ear on, a roll has two steps: the die lands, the player
  // chooses, and Listen draws the word. This drives both steps.
  const rollV11 = async (page, { stretch = false, ear = false } = {}) => {
    await page.click('#rsOrb');
    await page.waitForFunction(() => { const s = JSON.parse(window.__spelltest.randospellState() || 'null'); return s && (s.choosing || s.spelling); }, null, { timeout: 6000 });
    let st = await state(page);
    if (st.choosing) {
      if (stretch && st.stretchOffered) await page.click('#rsCharms [data-b="stretch"]');
      if (ear) { const o = await state(page); if (o.earOffered) await page.click('#rsCharms [data-b="ear"]'); }
      await page.click('#rsCharms [data-b="listen"]');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
      st = await state(page);
    }
    return st;
  };
  const finish = async (page, word, ok = true) => {
    const before = await page.evaluate(() => window.__spelltest.randospellState());
    if (ok) await type(page, word); else { await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); }
    await page.click('#rsGo');
    await settled(page, before);
    return state(page);
  };

  await suite.test('randospell_v11_the_choices_come_between_the_die_and_the_word', async () => {
    const { ctx, page } = await open({ v11: true });
    try {
      await page.click('#rsOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).choosing, null, { timeout: 6000 });
      const st = await state(page);
      assert(st.word === null && st.spelling === false, `a word exists before the choice: ${JSON.stringify(st)}`);
      assert(await page.evaluate(() => document.getElementById('rsCharms').querySelector('[data-b="listen"]') !== null), 'no Listen');
      await page.click('#rsCharms [data-b="listen"]');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
      const after = await state(page);
      assert(after.word && [...after.word].length === after.roll, 'the drawn word does not match the roll');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_v11_the_guess_toggles_and_locks_on_the_press', async () => {
    const { ctx, page } = await open({ v11: true });
    try {
      await page.click('#rsCall [data-c="high"]');
      assert((await state(page)).call === 'High', 'High not selected');
      await page.click('#rsCall [data-c="high"]');
      assert((await state(page)).call === null, 'a second tap did not clear it');
      await page.click('#rsCall [data-c="low"]');
      assert((await state(page)).call === 'Low', 'Low not selected');
      await page.click('#rsOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).choosing, null, { timeout: 6000 });
      assert(await page.evaluate(() => document.getElementById('rsCall').hidden), 'the guess can still be changed after the press');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a22_stretch_is_chosen_before_the_word_and_draws_a_tier_up', async () => {
    const { ctx, page } = await open({ v11: true });
    try {
      let got = null;
      for (let i = 0; i < 80 && !got; i++) {
        await page.click('#rsOrb');
        await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).choosing, null, { timeout: 6000 });
        const st = await state(page);
        if (st.stretchOffered) {
          await page.click('#rsCharms [data-b="stretch"]');
          await page.click('#rsCharms [data-b="listen"]');
          await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
          got = await state(page);
        } else {
          await page.click('#rsCharms [data-b="listen"]');
          await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
          const cur = await state(page);
          const a = await finish(page, cur.word, false);
          if (a.deciding) await page.click('#rsRollAgain');
        }
      }
      assert(got, 'a Stretch was never on offer in 80 rolls');
      assert(got.stretch === true && got.wordTier === got.tier + 1, `Stretch did not draw a tier up: ${JSON.stringify({ base: got.tier, word: got.wordTier, s: got.stretch })}`);
      assert(got.events.indexOf('StretchChosen(true)') >= 0 && got.events.lastIndexOf('StretchChosen(true)') < got.events.lastIndexOf('WordDrawn'), `choice was not logged before the draw: ${got.events}`);
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a23_ear_folds_the_replay_away', async () => {
    const { ctx, page } = await open({ v11: true });
    try {
      const st = await rollV11(page, { ear: true });
      assert(st.ear === true, `Ear was not set: ${JSON.stringify(st)}`);
      assert(await page.evaluate(() => document.getElementById('rsOrb').disabled), 'the replay control is live after Ear');
      // A normal word keeps its replay.
      await finish(page, st.word, true);
      const n = await state(page);
      if (n.deciding) await page.click('#rsRollAgain');
      const plain = await rollV11(page, {});
      assert(plain.ear === false && !(await page.evaluate(() => document.getElementById('rsOrb').disabled)), 'a plain word lost its replay');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_v11_heat_climbs_with_each_unbanked_word_and_resets', async () => {
    const { ctx, page } = await open({ v11: true });
    try {
      for (let n = 1; n <= 3; n++) {
        const st = await rollV11(page, {});
        const a = await finish(page, st.word, true);
        assert(a.heat === n, `heat ${a.heat} after ${n} correct words`);
        assert((await page.evaluate(() => document.getElementById('rsHeat').textContent)).includes('●'), 'no filled pip shown');
        await page.click('#rsRollAgain');
      }
      const st = await rollV11(page, {});
      const miss = await finish(page, st.word, false);
      assert(miss.heat === 0, `a miss left heat ${miss.heat}`);
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a24_doubles_glow_and_pay_double_in_sprint', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: () => {
      localStorage.setItem('spell_flag_randospell', 'on');
      localStorage.setItem('spell_flag_randospell_doubles', 'on');
      localStorage.setItem('spell_flag_randospell_marathon', 'off');
    } });
    const { ctx, page } = o;
    try {
      await page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await page.waitForSelector('#rsScreen.show', { timeout: 8000 });
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      let seen = false;
      for (let i = 0; i < 100 && !seen; i++) {
        const st = await roll(page);
        await page.evaluate(() => window.__spelltest.randospellScore(0));
        const before = await state(page);
        if (st.doubles) {
          assert(await page.evaluate(() => document.getElementById('rsDie').classList.contains('glow')), 'doubles without a glow');
          const after = await finish(page, st.word, true);
          const base = (st.roll === 2 || st.roll === 12) ? st.roll * 2 : st.roll;
          assert(after.score - before.score === base * 2, `doubles paid ${after.score - before.score}, wanted ${base * 2}`);
          seen = true;
        } else {
          assert(await page.evaluate(() => !document.getElementById('rsDie').classList.contains('glow')), 'a glow without doubles');
          await finish(page, st.word, false);
        }
      }
      assert(seen, 'no doubles in 100 rolls');
    } finally { await ctx.close(); }
  });
  // ---- v1.1 Phase F: the NPC table ------------------------------------------
  const openTable = async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: FLAG_TABLE });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
    await o.page.click('#rsModes [data-m="table"]');
    return o;
  };
  const settledTable = (page) => page.waitForFunction(() => {
    const s = JSON.parse(window.__spelltest.randospellState() || 'null');
    return s && s.table && !s.table.busy && s.table.turn === 0;
  }, null, { timeout: 15000 });

  await suite.test('randospell_table_is_offered_only_with_its_flag', async () => {
    const off = await open({ start: false });
    try {
      assert((await off.page.evaluate(() => document.querySelector('#rsModes [data-m="table"]'))) === null, 'the table is offered with randospell_npc off');
    } finally { await off.ctx.close(); }
    const on = await openTable();
    try {
      assert(await on.page.evaluate(() => !document.getElementById('rsNpcCfg').hidden), 'no NPC setup');
    } finally { await on.ctx.close(); }
  });

  await suite.test('randospell_table_the_setup_picks_count_personalities_and_difficulty', async () => {
    const { ctx, page } = await openTable();
    try {
      await page.click('#rsNpcCfg [data-n="3"]');
      await page.click('#rsNpcCfg [data-p="1:daring"]');
      await page.click('#rsNpcCfg [data-p="2:mirror"]');
      await page.click('#rsNpcCfg [data-d="tough"]');
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      await settledTable(page);
      const st = await state(page);
      assert(st.table.people.join() === 'cautious,daring,mirror', `personalities: ${st.table.people}`);
      assert(await page.evaluate(() => document.querySelectorAll('#rsTable .rs-seat').length === 4), 'the table does not show four seats');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_table_a_miss_hands_the_turn_to_the_computer_and_back', async () => {
    const { ctx, page } = await openTable();
    try {
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      await settledTable(page);
      const st = await roll(page);
      const before = await page.evaluate(() => window.__spelltest.randospellState());
      await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsKeys [data-k="u:q"]'); await page.click('#rsGo');
      await settled(page, before);
      const mid = await state(page);
      assert(mid.table.turn !== 0 || mid.table.busy, `a miss did not hand the turn on: ${JSON.stringify(mid.table)}`);
      // A tap on the table finishes the run at once; the turn comes back to the human.
      await page.click('#rsTable', { force: true }).catch(() => {});
      await settledTable(page);
      const after = await state(page);
      assert(after.table.turn === 0 && after.table.winner === null, `turn did not return: ${JSON.stringify(after.table)}`);
      assert(!(await page.evaluate(() => document.getElementById('rsOrb').disabled)), 'the orb stays locked after the NPCs');
      void st;
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_table_a_human_bank_past_100_wins_the_table', async () => {
    const { ctx, page } = await openTable();
    try {
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      await settledTable(page);
      await page.evaluate(() => window.__spelltest.randospellScore(99));
      await answer(page, true);
      await page.click('#rsBank');
      const end = await state(page);
      assert(end.ended === 'Goal' && end.table.winner === 0, `the human did not win: ${JSON.stringify(end.table)}`);
      assert(await page.evaluate(() => !document.getElementById('rsOver').hidden), 'no end card');
      assert(await page.evaluate(() => document.getElementById('rsOverExit') !== null), 'no way out');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_jr_table_is_one_cautious_racer_with_no_pot', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: FLAG_JR_TABLE, age: KID });
    const { ctx, page } = o;
    try {
      await page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await page.waitForSelector('#rsScreen.show', { timeout: 8000 });
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      await settledTable(page);
      let st = await state(page);
      assert(st.table.people.join() === 'cautious', `Jr personalities: ${st.table.people}`);
      const s = await roll(page);
      const before = await page.evaluate(() => window.__spelltest.randospellState());
      await type(page, s.word);
      await page.click('#rsGo');
      await settled(page, before);
      await settledTable(page);
      st = await state(page);
      assert(st.table.pots[1] === 0, 'a racer has a pot');
      assert(await page.evaluate(() => document.getElementById('rsPot').hidden), 'Jr shows a pot');
    } finally { await ctx.close(); }
  });
  // ---- v1.1 Phase G: Steal and the Final Round ---------------------------------
  const openG = async (mode) => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: FLAG_G });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
    await o.page.click(`#rsModes [data-m="${mode}"]`);
    if (mode === 'table') await o.page.click('#rsNpcCfg [data-n="2"]');
    await o.page.click('#rsStart');
    await o.page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
    if (mode === 'table') await settledTable(o.page);
    return o;
  };

  // One step of a scripted human who always spells right and never banks, from
  // whatever state the screen is in. Returns the state it acted on.
  const step = async (page, { steal = false } = {}) => {
    const st = await state(page);
    if (st.table && st.table.busy) {
      await page.waitForFunction(() => { const x = JSON.parse(window.__spelltest.randospellState() || 'null'); return !x || !x.table || !x.table.busy; }, null, { timeout: 15000 });
      return st;
    }
    if (st.deciding) { await page.click('#rsRollAgain'); return st; }
    if (st.choosing) {
      if (steal && st.stealOptions.length) return st; // the caller handles the steal
      await page.click('#rsCharms [data-b="listen"]');
      return st;
    }
    if (st.spelling) {
      const before = await page.evaluate(() => window.__spelltest.randospellState());
      await type(page, st.word);
      await page.click('#rsGo');
      await settled(page, before);
      return st;
    }
    await page.click('#rsOrb');
    return st;
  };

  await suite.test('randospell_g_a_goal_bank_opens_the_final_round_instead_of_ending_the_game', async () => {
    const { ctx, page } = await openG('table');
    try {
      await page.evaluate(() => window.__spelltest.randospellScore(99));
      let st = await rollV11(page, {});
      await finish(page, st.word, true);
      await page.click('#rsBank');
      // The NPCs each take one final turn, then it settles on its own.
      await page.waitForSelector('#rsOver:not([hidden])', { timeout: 30000 });
      const end = await state(page);
      assert(end.table.finalLeader === 0, `the final round was not opened by the human: ${JSON.stringify(end.table)}`);
      assert(end.table.finalPending.length === 0, 'someone still owes a final turn');
      assert(end.table.winner !== null, 'no winner');
      assert(await page.evaluate(() => document.getElementById('rsOverExit') !== null), 'no way out');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_g_the_final_round_banner_is_words_not_only_a_colour', async () => {
    const { ctx, page } = await openG('table');
    try {
      await page.evaluate(() => window.__spelltest.randospellScore(99));
      let st = await rollV11(page, {});
      await finish(page, st.word, true);
      await page.click('#rsBank');
      const seen = await page.waitForFunction(() => {
        const el = document.getElementById('rsFinal');
        return el && !el.hidden && el.textContent.trim().length > 3;
      }, null, { timeout: 15000 }).then(() => true).catch(() => false);
      assert(seen, 'no Final Round banner');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_g_steal_moves_floor_half_and_leaves_the_rest', async () => {
    const { ctx, page } = await openG('table');
    try {
      await page.evaluate(() => window.__spelltest.randospellNpcPot(1, 21));
      let done = false;
      for (let i = 0; i < 200 && !done; i++) {
        const st = await step(page, { steal: true });
        if (st.choosing && st.stealOptions.length) {
          assert(st.rolled === 7, `a steal on a roll of ${st.rolled}`);
          assert(st.stealOptions.some(([seat, pot]) => seat === 1 && pot === 21), `options ${JSON.stringify(st.stealOptions)}`);
          const potBefore = st.pot;
          await page.click('#rsCharms [data-b="steal:1"]');
          await page.click('#rsCharms [data-b="listen"]');
          await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).spelling, null, { timeout: 6000 });
          const w = await state(page);
          assert(w.roll === 7 && [...w.word].length === 7, `the steal word is not 7 letters: ${w.word}`);
          assert(w.wordTier >= 2, `a steal word from tier ${w.wordTier}, wanted Hard or Expert`);
          const after = await finish(page, w.word, true);
          assert(after.pot === potBefore + 10, `pot ${after.pot}, wanted ${potBefore + 10}`);
          assert(after.table.pots[1] === 11, `target kept ${after.table.pots[1]}, wanted 11`);
          assert(after.deciding, 'no Bank / Roll again after a landed steal');
          done = true;
        }
      }
      assert(done, 'no roll of 7 in 200 steps');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_g_i18_steal_is_never_offered_in_solo_marathon_or_sprint', async () => {
    for (const mode of ['marathon', 'sprint']) {
      const { ctx, page } = await openG(mode);
      try {
        for (let i = 0; i < 90; i++) {
          const st = await state(page);
          if (st.ended) break;
          assert(st.stealOptions.length === 0, `${mode} offered a steal`);
          assert(await page.evaluate(() => document.querySelector('#rsCharms [data-b^="steal"]') === null), `${mode} shows a steal button`);
          await step(page);
        }
      } finally { await ctx.close(); }
    }
  });
  // ---- v1.1 Phase H and I: Daily with the bets, and the Eclipse visuals ---------
  const openFx = async (init, { reduce = false, mode = 'marathon' } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 375, height: 667 }, init });
    if (reduce) await o.page.emulateMedia({ reducedMotion: 'reduce' });
    await o.page.evaluate(() => document.getElementById('rsOpenBtn').click());
    await o.page.waitForSelector('#rsScreen.show', { timeout: 8000 });
    await o.page.click(`#rsModes [data-m="${mode}"]`);
    await o.page.click('#rsStart');
    await o.page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
    return o;
  };

  await suite.test('randospell_h_daily_carries_the_bets_and_never_steal_or_a_final_round', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: () => {
      localStorage.setItem('spell_flag_randospell', 'on');
      localStorage.setItem('spell_flag_randospell_daily', 'on');
      for (const f of ['heat', 'call', 'stretch', 'doubles', 'ear', 'steal', 'lastcall']) localStorage.setItem(`spell_flag_randospell_${f}`, 'on');
    } });
    const { ctx, page } = o;
    try {
      await page.evaluate(() => document.getElementById('rsOpenBtn').click());
      await page.waitForSelector('#rsScreen.show', { timeout: 8000 });
      await page.click('#rsModes [data-m="daily"]');
      await page.click('#rsStart');
      await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
      assert(await page.evaluate(() => document.querySelectorAll('#rsCall [data-c]').length === 3), 'a Daily has no Guess');
      await page.click('#rsOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.randospellState()).choosing, null, { timeout: 6000 });
      const st = await state(page);
      assert(st.kind === 'Daily' && st.stealOptions.length === 0, `Daily state: ${JSON.stringify({ k: st.kind, s: st.stealOptions })}`);
      assert(await page.evaluate(() => document.getElementById('rsFinal').hidden), 'a Daily shows a Final Round');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_i_fx_draws_the_state_and_fx_off_still_shows_every_number', async () => {
    const on = await openFx(FLAG_FX);
    try {
      assert(await on.page.evaluate(() => document.getElementById('rsScreen').classList.contains('fx')), 'fx class missing');
      const st0 = await rollV11(on.page, {});
      assert(await on.page.evaluate(() => document.querySelectorAll('#rsFxSvg .rs-tick').length === 11), 'the dial does not have eleven faces');
      assert(await on.page.evaluate((n) => document.querySelector('#rsFxSvg .rs-tnum.on')?.textContent === String(n), st0.roll), 'the dial did not stop on the roll');
      await finish(on.page, st0.word, true);
      const st1 = await state(on.page);
      assert(st1.fx.stones.length === 1 && st1.fx.stones[0] === st1.pot, `stones ${JSON.stringify(st1.fx.stones)} vs pot ${st1.pot}`);
      assert(await on.page.evaluate(() => document.querySelectorAll('#rsFxSvg .rs-stone').length === 1), 'no stone in orbit');
      assert(await on.page.evaluate(() => document.querySelectorAll('#rsFxSvg .rs-notch.on').length === 1), 'no heat notch lit');
    } finally { await on.ctx.close(); }
    // fx off: the same state is in plain numerals and pips (I20).
    const off = await openFx(FLAG_FX_OFF);
    try {
      assert(await off.page.evaluate(() => !document.getElementById('rsScreen').classList.contains('fx')), 'fx class present with the flag off');
      const st0 = await rollV11(off.page, {});
      await finish(off.page, st0.word, true);
      const txt = await off.page.evaluate(() => ({ pot: document.getElementById('rsPot').textContent, heat: document.getElementById('rsHeat').textContent }));
      assert(/\d/.test(txt.pot) && txt.heat.includes('●'), `plain state missing: ${JSON.stringify(txt)}`);
      assert(await off.page.evaluate(() => getComputedStyle(document.getElementById('rsFxHost')).display === 'none'), 'the fx host shows with fx off');
    } finally { await off.ctx.close(); }
  });

  await suite.test('randospell_a33_particles_stay_under_200_and_the_canvas_is_gone_between_turns', async () => {
    const { ctx, page } = await openFx(FLAG_FX);
    try {
      let peak = 0;
      for (let i = 0; i < 12; i++) {
        const st = await rollV11(page, {});
        await finish(page, st.word, i % 3 !== 2); // some misses: shatter; the rest bank: stream
        const a = await state(page);
        peak = Math.max(peak, a.fx.peak, a.fx.live);
        if (a.deciding) await page.click('#rsBank');
        await page.waitForFunction(() => !JSON.parse(window.__spelltest.randospellState()).fx.canvas, null, { timeout: 4000 });
        const b = await state(page);
        assert(b.fx.live === 0 && b.fx.canvas === false, `a canvas survived the turn: ${JSON.stringify(b.fx)}`);
        peak = Math.max(peak, b.fx.peak);
      }
      assert(peak <= 200, `peak particles ${peak}`);
      assert(peak > 0, 'no burst ever ran');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a16_reduce_motion_runs_no_bursts_and_no_orbit_animation', async () => {
    const { ctx, page } = await openFx(FLAG_FX, { reduce: true });
    try {
      const st = await rollV11(page, {});
      await finish(page, st.word, true);
      const orbit = await page.evaluate(() => getComputedStyle(document.querySelector('#rsFxSvg .rs-orbit')).animationName);
      assert(orbit === 'none', `the orbit animates under reduce motion: ${orbit}`);
      await page.click('#rsBank');
      const a = await state(page);
      assert(a.fx.peak === 0 && a.fx.canvas === false, `a burst ran under reduce motion: ${JSON.stringify(a.fx)}`);
      assert(a.score > 0, 'the bank did not register');
    } finally { await ctx.close(); }
  });

  await suite.test('randospell_a34_every_state_has_a_shape_and_the_numerals_have_contrast', async () => {
    const { ctx, page } = await openFx(FLAG_FX);
    try {
      // Greyscale the whole page: shapes and words must still carry each state.
      await page.addStyleTag({ content: 'html{filter:grayscale(1)}' });
      for (let i = 0; i < 4; i++) {
        const st = await rollV11(page, {});
        await finish(page, st.word, true);
        if ((await state(page)).deciding) await page.click('#rsRollAgain');
      }
      const shapes = await page.evaluate(() => ({
        stones: document.querySelectorAll('#rsFxSvg .rs-stone').length,
        notches: document.querySelectorAll('#rsFxSvg .rs-notch.on').length,
        corona: !!document.querySelector('#rsFxSvg .rs-corona'),
        numerals: document.getElementById('rsPot').textContent,
      }));
      assert(shapes.stones >= 1 && shapes.notches >= 3 && shapes.corona, `shape channel: ${JSON.stringify(shapes)}`);
      assert(/\d/.test(shapes.numerals), 'the pot numeral is missing');
      // Contrast of the key numerals against the screen background (>= 4.5:1).
      const ratios = await page.evaluate(() => {
        const lum = (c) => { const [r, g, b] = c.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number).map((v) => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
        const bg = getComputedStyle(document.getElementById('rsScreen')).backgroundColor;
        const out = {};
        for (const id of ['rsScore', 'rsPot', 'rsHeat', 'rsFeed']) {
          const el = document.getElementById(id);
          if (!el || el.hidden || !el.textContent.trim()) continue;
          const a = lum(getComputedStyle(el).color), b = lum(bg);
          out[id] = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
        }
        return out;
      });
      for (const [id, r] of Object.entries(ratios)) assert(r >= 4.5, `${id} contrast ${r.toFixed(2)}`);
    } finally { await ctx.close(); }
  });
  await suite.test('randospell_i_the_lantern_never_overlaps_the_title_or_the_score', async () => {
    for (const vp of [{ width: 375, height: 667 }, { width: 390, height: 844 }]) {
      const o = await openApp(browser, base, { lang: 'en', viewport: vp, init: FLAG_FX });
      const { ctx, page } = o;
      try {
        await page.evaluate(() => document.getElementById('rsOpenBtn').click());
        await page.waitForSelector('#rsScreen.show', { timeout: 8000 });
        await page.click('#rsModes [data-m="marathon"]');
        await page.click('#rsStart');
        await page.waitForSelector('#rsPlay:not([hidden])', { timeout: 8000 });
        const hit = await page.evaluate(() => {
          const L = document.getElementById('rsLantern').getBoundingClientRect();
          const ids = ['rsTitle', 'rsScore', 'rsStreak', 'rsPot', 'rsRolls'];
          return ids.filter((id) => {
            const el = document.getElementById(id);
            if (!el || el.hidden || !el.textContent.trim()) return false;
            const r = el.getBoundingClientRect();
            return r.width && r.left < L.right && r.right > L.left && r.top < L.bottom && r.bottom > L.top;
          });
        });
        assert(hit.length === 0, `the lantern overlaps ${hit.join(', ')} at ${vp.width}x${vp.height}`);
      } finally { await ctx.close(); }
    }
  });
}
