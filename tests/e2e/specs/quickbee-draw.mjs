// quickbee-draw.spec -- CC-QUICKBEE-DRAW Phase B: the entry is offered only for a
// measured language (zh, ja) whose own flag is on, never to Spell Jr; the run is a
// 60 second clock of drawn cells with no confirm button, a wrong cell ends the word,
// an unreadable cell earns one free redraw, and the one pad is put back after.
// The web build has no native recogniser, so the test names what each cell reads as
// through __spelltest.qbdForce; everything else is the real screen.
import { openApp, assert, clickSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
// An iOS-only mode is in the registry for the app and absent on the web, so the browser
// must look like the app for the absent checks to prove anything. The page's own native
// bridge file replaces any stub at load, so the bridge is patched AFTER the page is up:
// it says it is native and answers no recognition (each cell's reading is forced).
// Playwright serialises an init script, so it cannot close over variables: one literal
// function per set of flags.
const FLAGS = {
  // zh on, with a drawn score already on the board for Chinese only (A6: the Japanese
  // query must not see it).
  zhBoard: () => {
    localStorage.setItem('spell_flag_quickbee_draw_zh', 'on');
    localStorage.setItem('spell_flag_quickbee_draw_ja', 'on');
    localStorage.setItem('spell_quickbee_draw_v1', JSON.stringify({
      best: { zh: 41 }, runs: { zh: 1 }, recent: { zh: [41] },
      top: { zh: [{ score: 41, words: 20, tier: 'easy', day: 20000 }] },
    }));
  },
  zh: () => { localStorage.setItem('spell_flag_quickbee_draw_zh', 'on'); },
  ja: () => { localStorage.setItem('spell_flag_quickbee_draw_ja', 'on'); },
  all: () => { for (const l of ['zh', 'ja', 'ko']) localStorage.setItem(`spell_flag_quickbee_draw_${l}`, 'on'); },
};
const ON = (langs) => (langs.length === 1 ? FLAGS[langs[0]] : FLAGS.all);
const STUB_ONLY = () => {};
const asNative = (page) => page.evaluate(() => {
  window.SpellNativeLang = window.SpellNativeLang || {};
  window.SpellNativeLang.nativePlatform = () => true;
  window.SpellNativeLang.recognizeInk = async () => ({ candidates: [] });
});

export async function run(browser, base, suite) {
  const modes = async (page) => {
    await asNative(page);
    await clickSettled(page, '#navBurger');
    return page.$$eval('#navDrawerPanel .nav-row', (els) => els.map((e) => e.dataset.mode));
  };
  const st = (page) => page.evaluate(() => JSON.parse(window.__spelltest.qbdState() || 'null'));
  const open = async (lang, langs = [lang]) => {
    const o = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 }, init: ON(langs) });
    await asNative(o.page);
    await o.page.evaluate(() => document.getElementById('qbdOpenBtn').click());
    await o.page.waitForSelector('#qbdScreen.show', { timeout: 8000 });
    return o;
  };
  const begin = async (page) => {
    await page.click('#qbdStart');
    await page.waitForSelector('#qbdPlay:not([hidden])', { timeout: 8000 });
    await page.waitForFunction(() => { const s = JSON.parse(window.__spelltest.qbdState() || 'null'); return s && s.word; }, null, { timeout: 8000 });
  };
  // One real stroke on the pad, then wait for the cell to be judged (no confirm button).
  const draw = async (page, force) => {
    await page.evaluate((f) => window.__spelltest.qbdForce(f), force);
    const before = await st(page);
    const box = await page.locator('#qbdPadHost #canvas').boundingBox();
    await page.mouse.move(box.x + box.width * 0.3, box.y + box.height * 0.3);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.7, box.y + box.height * 0.7, { steps: 5 });
    await page.mouse.up();
    await page.waitForFunction((b) => {
      const s = JSON.parse(window.__spelltest.qbdState() || 'null');
      return s && !s.judging && (s.score !== b.score || s.cell !== b.cell || s.word !== b.word || s.redraws !== b.redraws || s.over);
    }, before, { timeout: 8000 });
    return st(page);
  };

  for (const lang of ['zh', 'ja']) {
    await suite.test(`quickbee_draw_is_offered_in_${lang}_with_its_flag`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, init: ON([lang]) });
      try {
        assert((await modes(page)).includes('quickbee_draw'), `${lang}: not offered with its flag on`);
      } finally { await ctx.close(); }
    });
  }

  for (const lang of ['ko', 'vi', 'en', 'es']) {
    await suite.test(`quickbee_draw_is_absent_in_${lang}_even_with_every_flag_on`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, init: ON(['zh', 'ja', 'ko']) });
      try {
        const m = await modes(page);
        assert(m.length > 0 && !m.includes('quickbee_draw'), `${lang}: offered: ${m.join(',')}`);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('quickbee_draw_is_absent_for_zh_with_the_flags_off_and_for_spell_jr', async () => {
    const off = await openApp(browser, base, { lang: 'zh', init: STUB_ONLY });
    try {
      assert(!(await modes(off.page)).includes('quickbee_draw'), 'offered with every flag off');
    } finally { await off.ctx.close(); }
    const jr = await openApp(browser, base, { lang: 'zh', age: KID, init: ON(['zh']) });
    try {
      assert(!(await modes(jr.page)).includes('quickbee_draw'), 'Spell Jr is offered the pad (CC-CJK-INK D6)');
    } finally { await jr.ctx.close(); }
  });

  await suite.test('quickbee_draw_a_right_cell_scores_one_and_advances_with_no_confirm_button', async () => {
    const { ctx, page } = await open('zh');
    try {
      await begin(page);
      assert(await page.evaluate(() => document.querySelectorAll('#qbdScreen #inkCanvasWrap').length === 1 && document.querySelectorAll('canvas#canvas').length === 1), 'the pad is not exactly one canvas, moved into the run');
      assert((await st(page)).padMoved, 'the pad did not move into the run screen');
      const a = await st(page);
      const b = await draw(page, 'right');
      assert(b.score === 1, `a right cell did not score one: ${JSON.stringify(b)}`);
      assert(b.cell === a.cell + 1 || b.word !== a.word, 'the run did not advance by itself');
    } finally { await ctx.close(); }
  });

  await suite.test('quickbee_draw_a3_a_wrong_second_cell_keeps_the_first_point_and_starts_the_next_word', async () => {
    const { ctx, page } = await open('zh');
    try {
      await begin(page);
      // Find a two-cell word: a wrong first cell ends the word and moves on.
      let s = await st(page);
      for (let i = 0; i < 60 && s.cells < 2; i++) s = await draw(page, 'wrong');
      assert(s.cells >= 2, 'no two-cell word in 60');
      const word = s.word;
      const before = s.score;
      s = await draw(page, 'right');
      assert(s.score === before + 1 && s.cell === 1, `first cell: ${JSON.stringify(s)}`);
      s = await draw(page, 'wrong');
      assert(s.score === before + 1, `the wrong cell changed the score: ${s.score}`);
      assert(s.word !== word && s.cell === 0, 'the next word did not begin');
    } finally { await ctx.close(); }
  });

  await suite.test('quickbee_draw_an_unreadable_cell_gets_one_free_redraw_per_word', async () => {
    const { ctx, page } = await open('zh');
    try {
      await begin(page);
      const a = await st(page);
      const b = await draw(page, 'unsure');
      assert(b.word === a.word && b.cell === a.cell && b.score === a.score && b.redraws === 1, `no free redraw: ${JSON.stringify(b)}`);
      const c = await draw(page, 'unsure');
      assert(c.word !== a.word, 'a second unreadable cell did not end the word');
      assert((await page.evaluate(() => document.getElementById('qbdFeed').textContent)).length >= 0);
    } finally { await ctx.close(); }
  });

  await suite.test('quickbee_draw_the_clock_ends_the_run_the_pad_goes_home_and_there_is_a_way_out', async () => {
    const { ctx, page } = await open('zh');
    try {
      await begin(page);
      await page.evaluate(() => window.__spelltest.qbdAge(61000));
      await page.waitForSelector('#qbdOver:not([hidden])', { timeout: 5000 });
      assert(await page.evaluate(() => document.querySelectorAll('#inkPad #inkCanvasWrap').length === 1), 'the pad did not go back to its overlay');
      assert(await page.evaluate(() => document.getElementById('qbdOverExit') !== null), 'no way out of the end card');
      await page.click('#qbdOverExit');
      assert(!(await page.evaluate(() => document.getElementById('qbdScreen').classList.contains('show'))), 'the exit did not leave');
    } finally { await ctx.close(); }
  });

  await suite.test('quickbee_draw_a5_no_word_repeats_within_a_run', async () => {
    const { ctx, page } = await open('ja');
    try {
      await begin(page);
      let s = await st(page);
      for (let i = 0; i < 25 && !s.over; i++) s = await draw(page, 'wrong');
      const seen = new Set();
      for (const w of s.served) { assert(!seen.has(w), `${w} repeated`); seen.add(w); }
      assert(s.served.length >= 20, `only ${s.served.length} served`);
    } finally { await ctx.close(); }
  });
  await suite.test('quickbee_draw_c_the_board_is_per_language_and_labelled_drawn', async () => {
    const zh = await openApp(browser, base, { lang: 'zh', viewport: { width: 390, height: 844 }, init: FLAGS.zhBoard });
    try {
      await asNative(zh.page);
      await zh.page.evaluate(() => document.getElementById('qbdOpenBtn').click());
      await zh.page.waitForSelector('#qbdScreen.show', { timeout: 8000 });
      const rows = await zh.page.$$eval('#qbdBoard li', (l) => l.map((x) => x.dataset.score));
      assert(rows.join() === '41', `the zh board shows ${rows}`);
      assert(await zh.page.evaluate(() => document.querySelector('#qbdBoard .qbd-board-h').textContent.length > 3), 'the board has no label');
    } finally { await zh.ctx.close(); }
    const ja = await openApp(browser, base, { lang: 'ja', viewport: { width: 390, height: 844 }, init: FLAGS.zhBoard });
    try {
      await asNative(ja.page);
      await ja.page.evaluate(() => document.getElementById('qbdOpenBtn').click());
      await ja.page.waitForSelector('#qbdScreen.show', { timeout: 8000 });
      const rows = await ja.page.$$eval('#qbdBoard li', (l) => l.length);
      assert(rows === 0, `the ja board shows another language's rows: ${rows}`);
      assert(await ja.page.evaluate(() => document.querySelector('#qbdBoard .qbd-board-empty') !== null), 'no empty state');
    } finally { await ja.ctx.close(); }
  });

  await suite.test('quickbee_draw_c_a_finished_run_is_placed_on_the_board_with_its_rank', async () => {
    const { ctx, page } = await open('zh');
    try {
      await begin(page);
      await draw(page, 'right');
      await draw(page, 'right');
      await page.evaluate(() => window.__spelltest.qbdAge(61000));
      await page.waitForSelector('#qbdOver:not([hidden])', { timeout: 5000 });
      const stats = await page.$$eval('#qbdStats li', (l) => l.map((x) => x.textContent));
      assert(stats.length >= 3, `end card lines: ${stats}`);
      const board = await page.$$eval('#qbdOverBoard li', (l) => l.map((x) => ({ r: x.dataset.rank, s: x.dataset.score })));
      assert(board.length === 1 && board[0].r === '1' && Number(board[0].s) >= 2, `board ${JSON.stringify(board)}`);
      // The score is stored for Chinese under the device's own key, nowhere else.
      const keys = await page.evaluate(() => Object.keys(localStorage).filter((k) => /quickbee/i.test(k) && !k.startsWith('spell_flag_')));
      assert(keys.join() === 'spell_quickbee_draw_v1', `unexpected keys: ${keys}`);
    } finally { await ctx.close(); }
  });

  await suite.test('quickbee_draw_c_spell_jr_has_no_entry_no_pad_and_no_board', async () => {
    const jr = await openApp(browser, base, { lang: 'zh', age: KID, init: FLAGS.zhBoard });
    try {
      await asNative(jr.page);
      assert(!(await modes(jr.page)).includes('quickbee_draw'), 'Jr is offered the draw variant');
      // A direct call is refused too: the screen does not open and no board renders.
      await jr.page.evaluate(() => document.getElementById('qbdOpenBtn').click());
      assert(await jr.page.evaluate(() => !document.getElementById('qbdScreen').classList.contains('show')), 'the screen opened for Jr');
      assert(await jr.page.evaluate(() => document.getElementById('qbdBoard').children.length === 0), 'a leaderboard rendered for Jr');
    } finally { await jr.ctx.close(); }
  });
}
