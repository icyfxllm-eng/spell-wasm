// learner-surfaces.spec — CC-LEARNING-ENGINE-L0 Phase 1: every surface that
// reads learner data now reads it through LearnerQuery. No other spec opens
// Reports, Guardian Dash, Calendar or the Stats guardian section, so without
// this the consumer migration would ship having never rendered.
//
// One seeded learner, every surface, the content each must show, and zero
// page errors (a wasm panic in a render path surfaces as a pageerror).
import { openApp, assert } from '../harness.mjs';

export async function run(browser, base, suite) {
  await suite.test('learner surfaces render through LearnerQuery: Reports, Stats, Guardian Dash, Calendar', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    try {
      await page.evaluate(() => {
        const day = Math.floor(Date.now() / 86400000);
        localStorage.setItem('spell_learner_en', JSON.stringify({
          version: 1, lang: 'en', placed: true,
          skills: [
            { id: 'silent_letters', mastery: 0.3, fsrs: { stability: 2, difficulty: 5.2, due_day: day - 9, reps: 6, lapses: 2 } },
            { id: 'doubled_consonant', mastery: 0.9, fsrs: { stability: 30, difficulty: 4.0, due_day: day + 20, reps: 5, lapses: 0 } },
          ],
          log: [
            { day: day - 1, word: 'receive', skills: [], correct: false, channel: 'typed', typed: 'recieve' },
            { day: day - 1, word: 'rabbit', skills: ['doubled_consonant'], correct: true, channel: 'typed' },
          ],
        }));
        const now = Date.now();
        localStorage.setItem('byear_misses_v1', JSON.stringify([
          { word: 'rhythm', lang: 'en', tier: 'medium', misses: 2, box_: 1, due: now - 60000, ts: now - 60000 },
          { word: 'faraway', lang: 'en', tier: 'medium', misses: 1, box_: 4, due: now + 10 * 86400000, ts: now },
        ]));
        localStorage.setItem('spell_flag_learner_surfaces', 'on');
      });
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(500);
      const html = (id) => page.evaluate((i) => (document.getElementById(i) || {}).innerHTML || '', id);

      // Stats guardian section: review_stats -> totals and skill groups.
      const stats = await html('statsGuardian');
      assert(/2 words attempted, 1 spelled right/.test(stats), `Stats guardian totals: ${stats.slice(0, 200)}`);
      assert(stats.includes('g-strong') && stats.includes('g-focus'), `Stats guardian groups: ${stats.slice(0, 300)}`);

      // Reports: at_risk_set -> the due miss is ready for a rematch; the
      // far-future one is not.
      await page.evaluate(() => document.getElementById('repOpenBtn').click());
      await page.waitForTimeout(300);
      const rematch = await html('repRematch');
      assert(rematch.includes('rhythm'), `Reports rematch missing the due word: ${rematch}`);
      assert(!rematch.includes('faraway'), 'Reports rematch showed a word not due for 10 days');

      // Guardian Dash, through the parental gate ("What is {a} times {b}?").
      await page.evaluate(() => document.getElementById('gdashOpen').click());
      const qtext = await page.evaluate(() => document.getElementById('gdashQ').textContent);
      const n = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
      const [a, b] = (qtext.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => n[w]);
      assert(a && b, `could not read the gate question: ${qtext}`);
      await page.fill('#gdashAnswer', String(a * b));
      await page.evaluate(() => document.getElementById('gdashGo').click());
      await page.waitForTimeout(400);
      const gd = await html('gdashBody');
      assert(/2\b/.test(gd) && gd.includes('gd-chip'), `Guardian Dash mastery groups did not render: ${gd.slice(0, 300)}`);
      assert(/1 words? due for review/.test(gd), `Guardian "What's next" should count the one due miss: ${gd.slice(0, 600)}`);
      assert(gd.includes('Trouble spots'), 'Guardian trouble spots (confusion pairs from miss_log) missing');

      // Calendar: opens and renders its forecast from at_risk_set.
      await page.evaluate(() => document.getElementById('calOpenBtn').click());
      await page.waitForTimeout(400);
      assert(await page.evaluate(() => document.getElementById('calScrim').classList.contains('show')), 'Calendar did not open');
      assert((await html('calScrim')).length > 200, 'Calendar rendered nothing');

      assert(errors.length === 0, `page errors while rendering learner surfaces:\n${errors.join('\n')}`);
    } finally { await ctx.close(); }
  });
}
