// spelluzzle-par.spec -- CC-SPELLUZZLE v1.1 Phase F: Par boards (F15) and the board-type row (F22).
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const ON = () => {
  localStorage.setItem('spell_flag_spelluzzle', 'on');
  localStorage.setItem('spell_flag_spelluzzle_par', 'on');
  localStorage.setItem('spell_spz_how_v1', '1');
  window.__plays = [];
  const orig = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () { try { window.__plays.push(this.src || this.currentSrc || ''); } catch (_) {} return orig.apply(this, arguments); };
};
const OFF = () => { localStorage.setItem('spell_flag_spelluzzle', 'on'); localStorage.setItem('spell_spz_how_v1', '1'); };

export async function run(browser, base, suite) {
  const start = async ({ init = ON, viewport = { width: 390, height: 844 }, age, tier = 'hard', par = true } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init, ...(age ? { age } : {}) });
    await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
    await o.page.waitForSelector('#szScreen.show');
    if (par) {
      await o.page.click('#szTypes [data-type="par"]');
      await o.page.click(`#szTiers [data-tier="${tier}"]`);
    }
    await o.page.click('#szStart');
    await o.page.waitForSelector('#szBoard .sz-row', { timeout: 8000 });
    if (await o.page.evaluate(() => !document.getElementById('szHow').hidden)) await o.page.click('#szHowOk');
    return o;
  };
  const words = (page) => page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
  const typeWord = async (page, w) => { for (const c of w) await page.click(`#szKb [data-k="u:${c}"]`); };
  const select = (page, i) => page.click(`#szBoard .sz-row[data-slot="${i}"] .sz-cells`);
  const heard = (page, w) => page.evaluate((x) => window.__plays.filter((u) => u.includes('word=' + x)).length, w);

  await suite.test('spelluzzle_par_is_absent_with_its_flag_off_and_for_jr', async () => {
    for (const [init, age] of [[OFF, null], [ON, KID]]) {
      const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init, ...(age ? { age } : {}) });
      try {
        await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
        await o.page.waitForSelector('#szScreen.show');
        const n = await o.page.evaluate(() => document.querySelectorAll('#szTypes [data-type]').length);
        assert(n === 0, `a board-type row showed (${n}) where Par must be absent`);
      } finally { await o.ctx.close(); }
    }
  });

  await suite.test('spelluzzle_par_boards_start_silent_and_one_listen_is_one_stroke', async () => {
    const { ctx, page } = await start();
    try {
      const m = await page.evaluate(() => ({
        header: document.getElementById('szParLine').textContent.trim(),
        rows: document.querySelectorAll('#szBoard .sz-row').length,
        quiet: document.querySelectorAll('#szBoard .sz-row.quiet').length,
        rowListen: document.querySelectorAll('#szBoard .sz-listen').length,
      }));
      const par = parseInt((m.header.match(/Par (\d)/) || [])[1], 10);
      assert(/^Listens 0 · Par [23]$/.test(m.header), `header: ${m.header}`);
      assert(m.rows === 7 && m.quiet === 6 && m.rowListen === 0, `shape ${JSON.stringify(m)}`);
      const ws = await words(page);
      await select(page, 0);
      assert((await heard(page, ws[0])) === 0, 'a silent word spoke on a tap');
      assert((await page.evaluate(() => document.querySelectorAll('#szParBar .sz-bar-listen').length)) === 1, 'no shared Listen bar for the selected word');
      await page.click('#szParBar .sz-bar-listen');
      assert((await heard(page, ws[0])) === 1, 'Listen did not play the word');
      assert((await page.evaluate(() => document.getElementById('szParLine').textContent.trim())) === `Listens 1 · Par ${par}`, 'the count did not move');
      // Replaying a heard word is free.
      await select(page, 0);
      assert((await page.evaluate(() => document.getElementById('szParLine').textContent.trim())) === `Listens 1 · Par ${par}`, 'a replay cost a stroke');
      // The secret word never has a Listen.
      await select(page, 6);
      assert((await page.evaluate(() => document.querySelectorAll('#szParBar .sz-bar-listen').length)) === 0, 'the secret word offers a Listen');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_par_solved_at_par_says_par_and_over_par_says_so', async () => {
    // Over par: Listen to every word, then solve.
    const { ctx, page } = await start();
    try {
      const ws = await words(page);
      for (let i = 0; i < 6; i++) { await select(page, i); await page.click('#szParBar .sz-bar-listen'); }
      for (let i = 0; i < ws.length; i++) { await select(page, i); await typeWord(page, ws[i]); }
      await domSettled(page);
      const m = await page.evaluate(() => ({ line: document.querySelector('#szResult .sz-parres')?.textContent.trim(), stars: [...document.querySelectorAll('#szResult .sz-star')].map((s) => s.classList.contains('on')) }));
      assert(/^\d+ over par$/.test(m.line || ''), `result line: ${m.line}`);
      assert(m.stars.length === 3 && m.stars[0] && !m.stars[2], `stars ${JSON.stringify(m.stars)}: over par must lose Codebreaker`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_par_deduced_with_no_listens_is_under_par', async () => {
    // Nothing is heard, so the player must work every word out; typing the answers (from the seam)
    // is the best a script can do, and it solves the board with zero Listens: "n under par".
    const { ctx, page } = await start();
    try {
      const ws = await words(page);
      for (let i = 0; i < ws.length; i++) { await select(page, i); await typeWord(page, ws[i]); }
      await domSettled(page);
      const line = await page.evaluate(() => document.querySelector('#szResult .sz-parres')?.textContent.trim());
      assert(/^\d+ under par$/.test(line || ''), `result line: ${line}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_par_expert_is_par_3', async () => {
    const { ctx, page } = await start({ tier: 'expert' });
    try {
      const h = await page.evaluate(() => document.getElementById('szParLine').textContent.trim());
      assert(/Par 3$/.test(h), `Expert Par header ${h}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_par_layout_at_375x667_keeps_7_rows_with_the_keyboard_up', async () => {
    const { ctx, page } = await start({ viewport: { width: 375, height: 667 }, tier: 'expert' });
    try {
      await select(page, 0);
      const m = await page.evaluate(() => {
        const board = document.getElementById('szBoard');
        const kb = document.getElementById('szKb').getBoundingClientRect();
        const cells = [...document.querySelectorAll('#szBoard .sz-cell')];
        return { scrolls: board.scrollHeight - board.clientHeight, kbBottom: kb.bottom, H: innerHeight, minW: Math.min(...cells.map((c) => c.getBoundingClientRect().width)), over: document.documentElement.scrollWidth - innerWidth, bar: !!document.querySelector('#szParBar .sz-bar-listen') };
      });
      assert(m.bar && m.over <= 0 && m.kbBottom <= m.H + 0.5, JSON.stringify(m));
      assert(m.minW >= 32, `cells shrank to ${m.minW}px`);
      assert(m.scrolls <= 1, `Par board scrolls ${m.scrolls}px with the keyboard up at 375x667`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_par_resumes_and_remembers_the_board_type', async () => {
    const { ctx, page } = await start();
    try {
      const ws = await words(page);
      await select(page, 0);
      await page.click('#szParBar .sz-bar-listen');
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show');
      // The type chosen last time is remembered.
      assert((await page.evaluate(() => document.querySelector('#szTypes [data-type].on')?.dataset.type)) === 'par', 'the last board type was forgotten');
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      assert(JSON.stringify(await words(page)) === JSON.stringify(ws), 'resume drew a different Par board');
      assert(/Listens 1/.test(await page.evaluate(() => document.getElementById('szParLine').textContent)), 'the Listen count was lost');
    } finally { await ctx.close(); }
  });
}
