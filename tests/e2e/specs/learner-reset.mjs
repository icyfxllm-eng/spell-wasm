// learner-reset.spec — CC-LEARNING-ENGINE Done #8: "reset this language"
// deletes what the learner model knows about THAT language, and nothing else.
//
// Before this, the button cleared stats only; spell_learner_{lang} survived a
// reset, so a parent who reset a language still had its mastery, attempt log
// (with typed misses) and placement on the device. A browser test, because the
// claim is about stored bytes and host tests have no localStorage.
import { openApp, assert } from '../harness.mjs';

export async function run(browser, base, suite) {
  await suite.test('learner: "reset this language" deletes its learner state and backup, only for that language', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const state = (lang) => JSON.stringify({
        version: 1, lang, placed: true,
        skills: [{ id: 'silent_letters', mastery: 0.8, fsrs: { stability: 9, difficulty: 5.1618, due_day: 20400, reps: 4, lapses: 0 } }],
        log: [{ day: 20390, word: 'knight', skills: ['silent_letters'], correct: false, channel: 'typed', typed: 'nite' }],
      });
      await page.evaluate(([en, es]) => {
        localStorage.setItem('spell_learner_en', en);
        localStorage.setItem('spell_learner_en_unreadable', 'old corrupt bytes');
        localStorage.setItem('spell_learner_es', es);
      }, [state('en'), state('es')]);
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(400);

      // The real button's handler (it lives in the stats sheet).
      await page.evaluate(() => document.getElementById('resetStats').click());
      await page.waitForTimeout(300);

      const got = await page.evaluate(() => ({
        en: localStorage.getItem('spell_learner_en'),
        bak: localStorage.getItem('spell_learner_en_unreadable'),
        es: localStorage.getItem('spell_learner_es'),
      }));
      assert(got.en === null, `English learner state survived the reset: ${got.en && got.en.slice(0, 80)}`);
      assert(got.bak === null, 'the English C10 backup survived the reset');
      assert(got.es === state('es'), 'resetting English touched Spanish learner state');
    } finally { await ctx.close(); }
  });
}
