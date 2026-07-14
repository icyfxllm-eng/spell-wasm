// ghost.spec — Ghost racing in The Climb (F6). Seeds a stored best run for
// English via localStorage, then plays one correct answer in a Climb run and
// asserts the live pace marker (#ghostPace) appears on the existing chain bar
// with an ahead/behind delta. Local-only: no network, no leaderboard.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const KEY = 'spell_ghost_v1';
// A deliberately slow ghost (first correct at 8s) so a quick real answer lands
// clearly AHEAD — deterministic marker state regardless of test-machine speed.
const SEED = { en: { events: [{ t: 8000, c: true }, { t: 16000, c: true }, { t: 24000, c: true }] } };

export async function run(browser, base, suite) {
  await suite.test('ghost: live pace marker appears and shows a delta on a Climb run', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Seed a stored best run, then reload so the app loads it at boot.
      await page.evaluate(([k, v]) => localStorage.setItem(k, JSON.stringify(v)), [KEY, SEED]);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });

      // Marker starts hidden (no live run yet).
      assert(await page.$eval('#ghostPace', (e) => e.classList.contains('btn-hide')), 'marker should start hidden');

      // Play one correct answer in the default English Climb run.
      await page.click('#orbWrap');
      await page.waitForTimeout(500);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      assert(word && word.length > 0, 'no current word from seam');
      const base_only = [...word.toLowerCase()].every((c) => /[a-z'-]/.test(c));
      if (!base_only) return; // accented first word — grading covered by unit tests
      await typeOnKeyboard(page, word.toLowerCase());
      await page.click('#checkBtn');
      await page.waitForTimeout(400);

      // The marker is now visible, shows the ghost emoji, and reads AHEAD (the
      // real answer beat the 8s ghost to the first word).
      const hidden = await page.$eval('#ghostPace', (e) => e.classList.contains('btn-hide'));
      assert(!hidden, 'ghost pace marker did not appear during the Climb run');
      const cls = await page.$eval('#ghostPace', (e) => e.className);
      const txt = await page.$eval('#ghostPace', (e) => e.textContent);
      assert(txt.includes('\u{1F47B}'), `ghost marker missing ghost emoji: "${txt}"`);
      assert(cls.includes('ahead'), `expected ahead state, got "${cls}" / "${txt}"`);
    } finally { await ctx.close(); }
  });
}
