// finale-relaunch.spec — CC-FINALE Done #6, the LITERAL version of the
// unit test spellpic.rs already holds in miniature: complete layer 1 of
// the constellation map (orion, the sky pack), kill the app — a real
// page reload against the same localStorage, not a serde round-trip —
// and find layer 2 unlocked, the run resumed mid-picture, nothing reset.
//
// Reads ride the observation-only seam (__spelltest.picLadder): the spec
// still TYPES every word through the real keyboard like a player would.
import { openApp, assert, assertEq } from '../harness.mjs';

async function openPicture(page, pic) {
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
  await page.click(`[data-pic="${pic}"]`);
  await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
  await page.waitForTimeout(500);
  for (let i = 0; i < 6 && (await page.$('#wpHow.show')); i++) {
    await page.click('#wpHowNext');
    await page.waitForTimeout(150);
  }
  assert(!(await page.$('#wpHow.show')), 'the how-to card never closed');
}

const ladder = (page) =>
  page.evaluate(() => JSON.parse(window.__spelltest.picLadder()));

async function typeWord(page, word) {
  for (const ch of word) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`);
  await page.waitForTimeout(250);
}

export async function run(browser, base, suite) {
  await suite.test('finale: layer 1 of orion survives the kill — layer 2 unlocked on relaunch', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicture(page, 'orion');
      const before = await ladder(page);
      assert(before.ladder.length >= 2, `orion has a ladder to climb (saw ${before.ladder.length} rungs)`);
      assertEq(before.placed, 0, 'a fresh run starts at zero');
      const rung1 = before.ladder[0][1];
      // Climb rung 1 the honest way: one word at a time, real keys.
      for (let i = 0; i < rung1; i++) {
        const word = await page.evaluate(() => window.__spelltest.picWord());
        assert(word, `a word is waiting (${i + 1}/${rung1})`);
        await typeWord(page, word);
      }
      const mid = await ladder(page);
      assertEq(mid.placed, rung1, 'rung 1 is complete');

      // The kill and the relaunch — the REAL one.
      await page.reload();
      await page.waitForFunction(
        () => window.__spelltest && window.__spelltest.build() === 'testseam',
        null, { timeout: 30000 });
      await openPicture(page, 'orion');
      const after = await ladder(page);
      assertEq(after.placed, rung1, 'the placed words survived the relaunch');
      assert(
        JSON.stringify(after.ladder) === JSON.stringify(before.ladder),
        'same seed, same ladder after relaunch');
      // Layer 2 is unlocked: the next word belongs to rung 2, and typing
      // it advances the run past the rung-1 boundary.
      const word = await page.evaluate(() => window.__spelltest.picWord());
      assert(word, 'layer 2 is serving words');
      await typeWord(page, word);
      const deeper = await ladder(page);
      assertEq(deeper.placed, rung1 + 1, 'the relaunched run kept climbing, not restarting');
    } finally { await ctx.close(); }
  });
}
