// hub-row.spec — CC-BUILD219-FIXES F1. The home "Ways to play" row is
// registry-decided (config/hub-tiles.json). This covers the part unit tests
// cannot: that the flag reaches the DOM, and STAYS applied.
//
// Staying applied is the whole reason this file exists. climb::reflect_auth
// toggles `btn-hide` on #climbBtn on every auth change, so a registry sharing
// that class would put the tile back the moment sign-in state moved. The
// suppression class is `tile-off` precisely to avoid that collision, and only
// a live run can prove the two never fight.
//
// The button is SUPPRESSED, NOT DELETED, and that is load-bearing too: it
// carries the leaderboard scrim's click handler, and ghost.rs opens the board
// by synthesising a click on it. A hidden button still dispatches click; a
// deleted one does not.
import { openApp, assert } from '../harness.mjs';

export async function run(browser, base, suite) {
  await suite.test('hub row: The Climb is out, the rest stay', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const row = await page.$$eval('.modes-row button', (els) =>
        els.map((e) => ({ id: e.id, shown: e.offsetParent !== null, cls: e.className })));
      const shown = row.filter((t) => t.shown).map((t) => t.id);
      assert(!shown.includes('climbBtn'), `Climb is out of the row (saw ${shown.join(', ')})`);
      for (const id of ['vsBtn', 'dailyBtn', 'wordPicTile']) {
        assert(shown.includes(id), `${id} stays in the row (saw ${shown.join(', ')})`);
      }
      const climb = row.find((t) => t.id === 'climbBtn');
      assert(climb, 'the Climb button stays in the DOM — it owns the leaderboard handler');
      assert(climb.cls.includes('tile-off'), `suppressed by the registry class, got "${climb.cls}"`);
    } finally { await ctx.close(); }
  });

  await suite.test('hub row: the mode is untouched and the board still routes', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const levels = await page.$$eval('#levelSel option', (o) => o.map((x) => x.value));
      assert(levels.includes('climb'), `Climb stays selectable (levels: ${levels.join(', ')})`);
      // Exactly what ghost.rs does: dom::click on the suppressed button.
      await page.evaluate(() => document.getElementById('climbBtn').click());
      await page.waitForTimeout(400);
      const open = await page.$eval('#climbScrim', (e) => e.classList.contains('show'));
      assert(open, 'a suppressed tile still routes — ghost.rs reaches the board through it');
    } finally { await ctx.close(); }
  });
}
