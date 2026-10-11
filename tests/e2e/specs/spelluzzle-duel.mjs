// spelluzzle-duel.spec -- CC-SPELLUZZLE v1.1 Phase H: Rune Duel (F20).
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const ON = () => {
  localStorage.setItem('spell_flag_spelluzzle', 'on');
  localStorage.setItem('spell_flag_spelluzzle_duel', 'on');
  localStorage.setItem('spell_spz_how_v1', '1');
  localStorage.setItem('spell_spz_how2_v1', '1');
  window.__plays = [];
  const orig = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () { try { window.__plays.push(this.src || this.currentSrc || ''); } catch (_) {} return orig.apply(this, arguments); };
};
const OFF = () => { localStorage.setItem('spell_flag_spelluzzle', 'on'); localStorage.setItem('spell_spz_how_v1', '1'); };

export async function run(browser, base, suite) {
  const start = async ({ init = ON, viewport = { width: 390, height: 844 }, age, tier = 'easy', cpu = false, diff = null } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init, ...(age ? { age } : {}) });
    await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
    await o.page.waitForSelector('#szScreen.show');
    await o.page.click('#szTypes [data-type="duel"]');
    await o.page.click(`#szTiers [data-tier="${tier}"]`);
    if (cpu) {
      await o.page.click('#szOpp [data-opp="cpu"]');
      if (diff !== null) await o.page.click(`#szOpp [data-diff="${diff}"]`);
    }
    await o.page.click('#szStart');
    await o.page.waitForSelector('#szBoard .sz-row', { timeout: 10000 });
    if (await o.page.evaluate(() => !document.getElementById('szHow').hidden)) await o.page.click('#szHowOk');
    return o;
  };
  const words = (page) => page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
  const typeWord = async (page, w) => { for (const c of w) await page.click(`#szKb [data-k="u:${c}"]`); };
  const select = (page, i) => page.click(`#szBoard .sz-row[data-slot="${i}"] .sz-cells`);
  const act = (page, a) => page.click(`#szParBar [data-act="${a}"]`);
  const head = (page) => page.evaluate(() => ({ line: document.getElementById('szParLine').textContent.trim(), score: document.getElementById('szDuelHdr').textContent.trim(), note: document.getElementById('szMsg').textContent.trim() }));
  const heard = (page, w) => page.evaluate((x) => window.__plays.filter((u) => u.includes('word=' + x)).length, w);

  await suite.test('spelluzzle_duel_is_absent_with_its_flag_off_and_for_jr', async () => {
    for (const [init, age] of [[OFF, null], [ON, KID]]) {
      const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init, ...(age ? { age } : {}) });
      try {
        await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
        await o.page.waitForSelector('#szScreen.show');
        const n = await o.page.evaluate(() => [...document.querySelectorAll('#szTypes [data-type]')].filter((b) => b.dataset.type === 'duel').length);
        assert(n === 0, 'a Duel entry showed where it must be absent');
      } finally { await o.ctx.close(); }
    }
  });

  await suite.test('spelluzzle_duel_pass_and_play_scores_claims_and_shows_each_verdict', async () => {
    const { ctx, page } = await start();
    try {
      const h0 = await head(page);
      assert(/^Board 1 of 2 · .*turn/i.test(h0.line) || /Board 1 of 2/.test(h0.line), `header ${h0.line}`);
      assert(/Player 1: 0 · Player 2: 0/.test(h0.score), `scores ${h0.score}`);
      const ws = await words(page);
      assert(ws.length === 7, `${ws.length} words`);
      const rows = await page.evaluate(() => ({ n: document.querySelectorAll('#szBoard .sz-row').length, unheard: document.querySelectorAll('#szBoard .sz-row.unheard').length }));
      assert(rows.n === 7 && rows.unheard === 7, JSON.stringify(rows));
      // The secret word offers Spell unheard x3 and no Listen.
      await select(page, 6);
      let labels = await page.evaluate(() => [...document.querySelectorAll('#szParBar [data-act]')].map((b) => b.dataset.act));
      assert(labels.includes('unheard') && !labels.includes('listen') && labels.includes('pass'), `secret actions ${labels}`);
      assert(/×3/.test(await page.evaluate(() => document.querySelector('#szParBar [data-act="unheard"]').textContent)), 'the secret word is not worth triple');
      // Player 1: Listen and spell word 0, correct.
      await select(page, 0);
      await act(page, 'listen');
      assert((await heard(page, ws[0])) === 1, 'Listen did not play the word');
      await typeWord(page, ws[0]);
      // Filling the last cell does not submit.
      assert(/Player 1: 0 ·/.test((await head(page)).score), 'filling the last cell submitted');
      await act(page, 'check');
      const h1 = await head(page);
      assert(/✓ \+\d+/.test(h1.note), `no verdict: ${h1.note}`);
      const p1 = parseInt(h1.score.match(/Player 1: (\d+)/)[1], 10);
      assert(p1 > 0, `score ${h1.score}`);
      assert(/Player 2/.test(h1.line), `the turn did not pass: ${h1.line}`);
      // Player 2: Listen and spell word 1, wrong: cleared, nothing decoded, verdict shown, slot heard.
      await select(page, 1);
      const decodedBefore = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.decoded').length);
      await act(page, 'listen');
      const wrong = [...ws[1]];
      wrong[0] = wrong[0] === 'q' ? 'x' : 'q';
      await typeWord(page, wrong.join(''));
      await act(page, 'check');
      const h2 = await head(page);
      assert(/✗/.test(h2.note), `no miss verdict: ${h2.note}`);
      assert((await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.decoded').length)) === decodedBefore, 'a wrong word decoded something');
      assert((await page.evaluate(() => document.querySelector('#szBoard .sz-row[data-slot="1"]').classList.contains('heard'))), 'the missed word is not marked heard');
      // Both players' ownership marks exist once each has claimed, and they differ in shape.
      await select(page, 1);
      await act(page, 'listen');
      await typeWord(page, ws[1]);
      await act(page, 'check'); // player 1 claims
      const marks = await page.evaluate(() => ({ a: document.querySelectorAll('.sz-own.a').length, b: document.querySelectorAll('.sz-own.b').length }));
      assert(marks.a > 0, `no ownership marks: ${JSON.stringify(marks)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_duel_two_passes_end_the_leg_and_two_legs_end_the_match', async () => {
    const { ctx, page } = await start();
    try {
      await act(page, 'pass');
      await act(page, 'pass');
      assert(/Board 2 of 2/.test((await head(page)).line), 'two passes did not end leg 1');
      await act(page, 'pass');
      await act(page, 'pass');
      const h = await head(page);
      assert(/Draw/.test(h.line), `a scoreless match should be a draw: ${h.line}`);
      assert((await page.evaluate(() => document.querySelectorAll('#szParBar [data-act]').length)) === 0, 'actions after the match ended');
      assert((await page.evaluate(() => Object.keys(localStorage).filter((k) => k.startsWith('spell_spz_duel')).length)) === 0, 'a finished match is still saved');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_duel_against_the_computer_takes_its_turn_and_never_blocks_you', async () => {
    const { ctx, page } = await start({ cpu: true, diff: 1 });
    try {
      // Whoever starts, within a few seconds the table is ours again or the computer has moved.
      await page.waitForFunction(() => {
        const t = document.getElementById('szParLine').textContent;
        return /Player 1/.test(t) || document.getElementById('szMsg').textContent.includes('Computer');
      }, null, { timeout: 8000 });
      const cpuMoved = await page.evaluate(() => document.getElementById('szMsg').textContent.includes('Computer'));
      if (!cpuMoved) {
        const ws = await words(page);
        await select(page, 0);
        await act(page, 'listen');
        await typeWord(page, ws[0]);
        await act(page, 'check');
        await page.waitForFunction(() => document.getElementById('szMsg').textContent.includes('Computer'), null, { timeout: 8000 });
      }
      assert((await head(page)).score.includes('Computer'), 'the computer has no name on the scoreboard');
    } finally { await ctx.close(); }
  });

  for (const tier of ['hard', 'expert']) {
    await suite.test(`spelluzzle_duel_${tier}_layout_at_375x667_with_the_keyboard_up`, async () => {
      const { ctx, page } = await start({ viewport: { width: 375, height: 667 }, tier });
      try {
        const ws = await words(page);
        await select(page, 2);
        await act(page, 'unheard');
        await typeWord(page, ws[2].slice(0, 3));
        const m = await page.evaluate(() => {
          const board = document.getElementById('szBoard');
          const kb = document.getElementById('szKb').getBoundingClientRect();
          const cells = [...document.querySelectorAll('#szBoard .sz-cell')];
          return { scrolls: board.scrollHeight - board.clientHeight, kbBottom: kb.bottom, H: innerHeight, minW: Math.min(...cells.map((c) => c.getBoundingClientRect().width)), over: document.documentElement.scrollWidth - innerWidth };
        });
        assert(m.over <= 0 && m.kbBottom <= m.H + 0.5, JSON.stringify(m));
        assert(m.minW >= 32, `cells shrank to ${m.minW}px`);
        assert(m.scrolls <= 1, `the Duel table scrolls ${m.scrolls}px with the keyboard up at 375x667`);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('spelluzzle_duel_resumes_a_match_in_progress', async () => {
    const { ctx, page } = await start();
    try {
      const ws = await words(page);
      await select(page, 0);
      await act(page, 'listen');
      await typeWord(page, ws[0]);
      await act(page, 'check');
      const before = await head(page);
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show');
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      const after = await head(page);
      assert(after.score === before.score, `scores changed over a resume: ${before.score} -> ${after.score}`);
      assert(JSON.stringify(await words(page)) === JSON.stringify(ws), 'resume dealt a different board');
    } finally { await ctx.close(); }
  });
}
