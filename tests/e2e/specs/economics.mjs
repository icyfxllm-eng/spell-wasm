// economics.spec — CC-PICTURE-BANK feature 6 / Done #5, the scripted
// sessions the file asks for, at three tiers:
//
//   starter  (star, easy):  a miss is repaired free — no stroke marked,
//                           unlimited retries, nothing lost.
//   advanced (eiffel, hard): a miss is repaired, but the stroke being
//                           earned dims until the word lands.
//   expert   (mona):        a miss is NOT repaired — the wrong letters
//                           stay, the stroke shows charged, and erasing
//                           them (backspace) is what lifts the charge.
//
// And the iron rule at every tier: no mechanic ever touches a different
// correct stroke — placed-word counts are checked around every miss.
import { openApp, assert, assertEq } from '../harness.mjs';

/** Open picture `pic` and dismiss the how-to card if it appears. */
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

/** A first letter guaranteed wrong for `word` (and on the en keyboard). */
function wrongFirst(word) {
  for (const c of ['z', 'q', 'x', 'k']) if (word[0] !== c) return c;
  throw new Error('no wrong letter found');
}

const placedCount = (page) =>
  page.$$eval('#wpStage .wp-word', (els) => els.length);
const inputValue = (page) =>
  page.$eval('#wpInput', (el) => el.value);

export async function run(browser, base, suite) {
  await suite.test('economics: starter misses are free and unlimited', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicture(page, 'star');
      const word = await page.evaluate(() => window.__spelltest.picWord());
      assert(word, 'no word waiting');
      const before = await placedCount(page);
      // Three misses in a row — unlimited means unlimited.
      for (let i = 0; i < 3; i++) {
        await page.click(`#gameKeyboard .kb-key[data-k="${wrongFirst(word)}"]`);
        await page.waitForTimeout(120);
        assertEq(await inputValue(page), '', `miss ${i + 1}: repaired free`);
      }
      assert(!(await page.$('#wpStage .miss-dim')), 'starter never dims');
      assert(!(await page.$('#wpStage .miss-charged')), 'starter never charges');
      assertEq(await placedCount(page), before, 'no stroke lost to a miss');
      // And the retry still lands.
      for (const ch of word) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`);
      await page.waitForTimeout(200);
      assertEq(await placedCount(page), before + 1, 'the retried word landed');
    } finally { await ctx.close(); }
  });

  await suite.test('economics: an advanced miss dims the earned stroke until it lands', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicture(page, 'eiffel');
      const word = await page.evaluate(() => window.__spelltest.picWord());
      const before = await placedCount(page);
      await page.click(`#gameKeyboard .kb-key[data-k="${wrongFirst(word)}"]`);
      await page.waitForTimeout(120);
      assertEq(await inputValue(page), '', 'advanced still repairs the input');
      assert(await page.$('#wpStage .wp-outline.next.miss-dim'), 'the earned stroke dims');
      assert(!(await page.$('#wpStage .miss-charged')), 'dimmed, never charged');
      assertEq(await placedCount(page), before, 'no other stroke was touched');
      // The dim survives the repair (until corrected = until the word lands)...
      await page.click(`#gameKeyboard .kb-key[data-k="${word[0]}"]`);
      await page.waitForTimeout(120);
      assert(await page.$('#wpStage .wp-outline.next.miss-dim'), 'dim holds until the word lands');
      // ...and landing the word clears it.
      for (const ch of word.slice(1)) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`);
      await page.waitForTimeout(200);
      assertEq(await placedCount(page), before + 1, 'the corrected word landed');
      assert(!(await page.$('#wpStage .miss-dim')), 'landing the word lifts the dim');
    } finally { await ctx.close(); }
  });

  await suite.test('economics: an expert miss charges the stroke; erase-to-recover restores it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicture(page, 'mona');
      const word = await page.evaluate(() => window.__spelltest.picWord());
      const before = await placedCount(page);
      const bad = wrongFirst(word);
      await page.click(`#gameKeyboard .kb-key[data-k="${bad}"]`);
      await page.waitForTimeout(120);
      // No repair: the damage stays in the field.
      assertEq(await inputValue(page), bad, 'expert does NOT repair the input');
      assert(await page.$('#wpStage .wp-outline.next.miss-charged'), 'the stroke is charged');
      assertEq(await placedCount(page), before, 'no other stroke was touched');
      // Erase-to-recover: backspacing out the damage lifts the charge.
      await page.click('#kbBackspace');
      await page.waitForTimeout(120);
      assertEq(await inputValue(page), '', 'the erase removed the damage');
      assert(!(await page.$('#wpStage .miss-charged')), 'erase-to-recover restored the stroke');
      // And the recovered player can still land the word.
      for (const ch of word) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`);
      await page.waitForTimeout(200);
      assertEq(await placedCount(page), before + 1, 'the recovered word landed');
    } finally { await ctx.close(); }
  });
}
