// placement.spec — CC-LEARNING-ENGINE L1 feature 5, the kid-facing flow
// (the QA pass that earns the learner_surfaces flag its ON default).
//
// The offer appears ONCE per language at the top of a solo serve, is
// skippable (skip leaves language-default priors standing and never
// re-offers), and Try serves the deterministic placement set through
// the REAL session path — no placement-specific scoring exists.
import { openApp, assert } from '../harness.mjs';

async function surfacesOn(page) {
  await page.evaluate(() => {
    localStorage.setItem('spell_flag_learner_surfaces', 'on');
    localStorage.setItem('spell_flag_learner_select', 'on');
  });
  await page.reload();
  await page.waitForTimeout(600);
}

export async function run(browser, base, suite) {
  await suite.test('placement: offered once, skip stands, no re-offer', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcSkip');
      await page.waitForTimeout(300);
      assert(!(await page.$('#plcCard.show')), 'card dismissed on skip');
      // a second serve never re-offers — skipped IS an answer
      await page.click('#orbWrap');
      await page.waitForTimeout(600);
      assert(!(await page.$('#plcCard.show')), 'no re-offer after skip');
    } finally { await ctx.close(); }
  });

  await suite.test('placement: Try serves the set through the real session', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcTry');
      await page.waitForTimeout(500);
      // D6: 10-14 words, served by the normal engine (the visible word
      // is a real serve; spelling it advances normally).
      const first = await page.evaluate(() => window.__spelltest.currentWord());
      assert(first && first.length > 0, 'a placement word is live in the real session');
    } finally { await ctx.close(); }
  });

  // CC-PLACEMENT-IDEMPOTENCE F4 — the offer may not interrupt a live run.
  //
  // Eric's screenshot: the sheet landing on top of a twenty-word chain. The
  // predicate going true mid-run must SUPPRESS the offer, not fire it — and
  // must not lose it either, so it appears at the next boundary.
  await suite.test('placement: the offer never interrupts a live run', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Answer the probe away first, so the offer is settled, then clear the
      // record to make the predicate true again WHILE a chain is running.
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcSkip');
      await page.waitForTimeout(300);

      // Build a real chain: answer words correctly until the streak is up.
      let streak = 0;
      for (let i = 0; i < 6 && streak < 3; i++) {
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        if (!w || !/^[a-z]+$/i.test(w)) { await page.click('#orbWrap'); await page.waitForTimeout(350); continue; }
        for (const ch of w.toLowerCase()) {
          const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
          if (k) await k.click();
        }
        await page.click('#checkBtn');
        await page.waitForFunction((prev) => window.__spelltest.currentWord() !== prev, w, { timeout: 6000 }).catch(() => {});
        streak = await page.evaluate(() => window.__spelltest.streak());
      }
      assert(streak > 0, `needed a live run to test against, streak=${streak}`);

      // Predicate true again, mid-run.
      await page.evaluate(() => localStorage.removeItem('spell_learner_en'));
      const w2 = await page.evaluate(() => window.__spelltest.currentWord());
      for (const ch of (w2 || 'a').toLowerCase()) {
        const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
        if (k) await k.click();
      }
      await page.click('#checkBtn');
      await page.waitForTimeout(2800);
      assert(!(await page.$('#plcCard.show')),
        'the offer rendered OVER a live run — F4 violated');
    } finally { await ctx.close(); }
  });

  // CC-PLACEMENT-IDEMPOTENCE — "it asked me again, with the same words."
  //
  // The two tests above only prove SKIP does not re-offer, and only within one
  // session. Nothing covered a run that is COMPLETED, and nothing covered
  // survival across a reload -- which is the exact gap the report lives in.
  //
  // This has to run in a browser. The equivalent unit test is meaningless:
  // storage::set_raw writes to web_sys localStorage, which does not exist in a
  // host cargo test, so it is a silent no-op and every assertion passes or
  // fails for the wrong reason.
  await suite.test('placement: a COMPLETED probe never re-offers, across a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcTry');
      await page.waitForTimeout(400);

      // Answer the whole set. The set is 10-14 words (D6); the cap is a
      // guard against a runaway loop, not an expectation.
      let served = 0;
      for (let i = 0; i < 30; i++) {
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        if (!w) break;
        for (const ch of w.toLowerCase()) {
          const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
          if (k) await k.click();
        }
        await page.click('#checkBtn');
        // WAIT FOR THE AUTO-ADVANCE. Do not tap the orb: a correct answer
        // already schedules a serve (CORRECT_DELAY_MS), and an orb tap 500ms
        // later races it, draining two words from the placement queue for one
        // answered. That is the harness driving the app in a way no player
        // does, and it drained the queue before the last answer could close
        // the probe.
        const before = w;
        await page.waitForFunction(
          (prev) => window.__spelltest.currentWord() !== prev,
          before, { timeout: 6000 }).catch(() => {});
        served++;
        // The card must NEVER reappear mid-run (F4: no interruption).
        assert(!(await page.$('#plcCard.show')),
          `the probe offer re-rendered during the run, after ${served} words`);
        if (await page.evaluate(() => !!document.querySelector('#plcCard.show'))) break;
      }
      assert(served >= 5, `expected the probe to serve its set, only saw ${served}`);

      // The whole point: a completed probe is a decision, and it survives.
      assert(!(await page.$('#plcCard.show')), 'no re-offer immediately after completing');
      // The record is the whole point: it must be DURABLE, not in-memory.
      const placed = await page.evaluate(() =>
        JSON.parse(localStorage.getItem('spell_learner_en') || '{}').placed);
      assert(placed === true,
        `a completed probe must write placed=true; localStorage says ${JSON.stringify(placed)}`);
      await page.reload();
      await page.waitForTimeout(900);
      await page.click('#orbWrap');
      await page.waitForTimeout(700);
      assert(!(await page.$('#plcCard.show')),
        'RE-OFFERED after a reload — a completed placement was not recorded durably');
    } finally { await ctx.close(); }
  });
}
