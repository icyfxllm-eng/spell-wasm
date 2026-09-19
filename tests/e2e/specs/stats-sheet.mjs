// stats-sheet.spec — the Stats card's learner surfaces survive boot.
//
// From 64ad436a (ship 120) a misplaced </div> nested #statsGuardian and the
// whole Guardian Dash (#gdash, the "For grown-ups" door) inside #statsBody.
// stats::render rewrites #statsBody at boot, so both were deleted before any
// player could see them: Guardian Dash never reached a shipped build. Nothing
// opened those surfaces in a browser test, so nothing noticed.
import { openApp, assert } from '../harness.mjs';

export async function run(browser, base, suite) {
  await suite.test('stats: the guardian section and Guardian Dash door survive boot and a stats render', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => {
        localStorage.setItem('spell_flag_learner_surfaces', 'on');
        localStorage.setItem('spell_learner_en', JSON.stringify({ version: 1, lang: 'en', placed: true, skills: [],
          log: [{ day: 1, word: 'rabbit', skills: [], correct: true, channel: 'typed' }] }));
      });
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(500);
      const dom = await page.evaluate(() => ({
        guardian: !!document.getElementById('statsGuardian'),
        guardianHtml: (document.getElementById('statsGuardian') || {}).innerHTML || '',
        gdash: !!document.getElementById('gdash'),
        door: !!document.getElementById('gdashOpen'),
        nested: !!document.querySelector('#statsBody #statsGuardian, #statsBody #gdash'),
      }));
      assert(!dom.nested, 'statsGuardian or gdash is inside #statsBody again; stats::render will delete it');
      assert(dom.guardian, '#statsGuardian was deleted at boot');
      assert(dom.gdash && dom.door, 'Guardian Dash (#gdash / "For grown-ups") was deleted at boot');
      assert(/1 words? attempted/.test(dom.guardianHtml), `the guardian section did not render: ${dom.guardianHtml.slice(0, 200)}`);
    } finally { await ctx.close(); }
  });
}
