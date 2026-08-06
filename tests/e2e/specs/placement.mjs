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
}
