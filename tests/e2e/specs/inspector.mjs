// inspector.spec — CC-LEARNING-ENGINE-L0 R4: the dev-only learner inspector.
//
// It is built by Rust at runtime (no markup in index.html) and only exists in
// a `dev_preview` build, which the e2e bundle is. Its absence from release,
// auditor and education builds is a build-level claim, proved by the cargo
// feature and scripts/seam-absence-check.mjs, not here.
import { openApp, assert } from '../harness.mjs';

const PANEL = '#spellLearnerInspector';

async function openInspector(page) {
  // Five taps on the logo within 1.2 s of each other opens the dev menu.
  for (let i = 0; i < 5; i++) {
    await page.click('#brandMark');
    await page.waitForTimeout(80);
  }
  await page.waitForSelector('#devMenu.show', { timeout: 5000 });
  await page.click('#spellLearnerInspectorOpen');
  await page.waitForSelector(`${PANEL}.show`, { timeout: 5000 });
  return page.evaluate((sel) => document.querySelector(sel).innerText, PANEL);
}

export async function run(browser, base, suite) {
  await suite.test('inspector: shows the review queue, next-review dates and why the word was served', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const now = Date.now(), day = Math.floor(now / 86400000);
      await page.evaluate(([now, day]) => {
        localStorage.setItem('byear_misses_v1', JSON.stringify([
          // overdue, on FSRS
          { word: 'rhythm', lang: 'en', tier: 'medium', misses: 3, box_: 0, due: now - 2 * 86400000, ts: now,
            review: { step: 2, fsrs: { stability: 4, difficulty: 7.6, due_day: day - 2, reps: 2, lapses: 1 } } },
          // still in the same-day learning steps
          { word: 'knight', lang: 'en', tier: 'easy', misses: 1, box_: 0, due: now + 600000, ts: now,
            review: { step: 1, fsrs: { stability: 0.4872, difficulty: 7.6214, due_day: day + 1, reps: 1, lapses: 0 } } },
          // another language: must not show under en
          { word: 'gato', lang: 'es', tier: 'easy', misses: 1, box_: 0, due: now, ts: now,
            review: { step: 0, fsrs: { stability: 0.4872, difficulty: 7.6214, due_day: day, reps: 1, lapses: 0 } } },
        ]));
        localStorage.setItem('spell_learner_en', JSON.stringify({
          version: 1, lang: 'en', placed: true,
          skills: [{ id: 'silent_letters', mastery: 0.3, fsrs: { stability: 2, difficulty: 5.2, due_day: day - 9, reps: 6, lapses: 2 } }],
          log: [{ day, word: 'knight', skills: ['silent_letters'], correct: false, channel: 'typed', typed: 'nite' }],
        }));
      }, [now, day]);
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(400);

      // Serve a word, so the inspector has a reason to report.
      await page.click('#orbWrap');
      await page.waitForFunction(() => window.__spelltest.currentWord(), null, { timeout: 5000 });
      const served = await page.evaluate(() => window.__spelltest.currentWord());

      const text = await openInspector(page);
      assert(text.includes(served), `the served word ${served} should be named: ${text.slice(0, 400)}`);
      assert(/Serving:.*(deck order|due review|learner pick|placement|list order|daily)/.test(text),
        `a reason for the served word: ${text.slice(0, 400)}`);
      assert(text.includes('rhythm') && text.includes('knight'), `both queued en words: ${text}`);
      assert(!text.includes('gato'), 'another language\'s queue must not show');
      assert(text.includes('2 d overdue'), `an overdue interval: ${text}`);
      assert(text.includes('in 10 min'), `a learning card's interval in minutes, not "in 0 d": ${text}`);
      assert(/fsrs/.test(text) && /learning/.test(text), `both phases: ${text}`);
      assert(text.includes('silent_letters') && text.includes('0.30'), `skills with mastery: ${text}`);
      assert(text.includes('placement completed'), `placement status: ${text}`);
      // A next-review date for the overdue word.
      const d = new Date(now - 2 * 86400000).toISOString().slice(0, 10);
      assert(text.includes(d), `the next-review date ${d}: ${text}`);

      // Read-only: opening it changes no learner byte.
      const before = await page.evaluate(() => localStorage.getItem('byear_misses_v1'));
      await page.click('#spellLearnerInspectorClose');
      await openInspector(page);
      const after = await page.evaluate(() => localStorage.getItem('byear_misses_v1'));
      assert(before === after, 'the inspector wrote to learner storage');
    } finally { await ctx.close(); }
  });
}
