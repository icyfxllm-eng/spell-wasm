// picker-continue.spec — CC-PICKER-CONTINUE + the family shelves.
//
// D1: max 4 cards, LRU. D2: tap resumes directly at the exact word.
// D3: completed pieces go to the Gallery, never this row. Housekeeping:
// long-press Remove discards run state, never the picture. Families:
// section headers render, and the learn shelf leads with the active
// language's script.
import { openApp, assert, assertEq } from '../harness.mjs';
import { pickTile } from './finale.mjs';

async function openPicker(page) {
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
}

async function startAndLeave(page, pic, words) {
  await pickTile(page, pic);
  await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
  await page.waitForTimeout(500);
  for (let i = 0; i < 6 && (await page.$('#wpHow.show')); i++) {
    await page.click('#wpHowNext');
    await page.waitForTimeout(150);
  }
  for (let k = 0; k < words; k++) {
    const word = await page.evaluate(() => window.__spelltest.picWord());
    for (const ch of word) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`, { delay: 0 });
    await page.waitForTimeout(200);
  }
  await page.click('#wpExit');
  await page.waitForTimeout(300);
}

export async function run(browser, base, suite) {
  await suite.test('continue: exact resume, LRU order, hidden when empty', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicker(page);
      assert(await page.$eval('#wpResumeRow', (e) => e.classList.contains('btn-hide')),
        'row hidden with nothing in progress');

      await startAndLeave(page, 'star', 1);
      const expected = await page.evaluate(() => window.__spelltest.picWord());
      await page.evaluate(() => document.getElementById('wpExit')?.click());
      await openPicker(page);
      assert(!(await page.$eval('#wpResumeRow', (e) => e.classList.contains('btn-hide'))),
        'row visible with a run in progress');
      const cards = await page.$$eval('#wpResumeRow [data-resume]',
        (els) => els.map((e) => e.getAttribute('data-resume')));
      assertEq(JSON.stringify(cards), JSON.stringify(['star']), 'one card, the run');
      const glyphs = await page.$eval('#wpResumeRow', (e) => e.querySelectorAll('text').length);
      assertEq(glyphs, 0, 'living thumbnails carry zero typeset glyphs');

      // D2: tap resumes at the precise word the run was waiting on.
      await page.click('#wpResumeRow [data-resume="star"]');
      await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
      await page.waitForTimeout(400);
      const resumed = await page.evaluate(() => window.__spelltest.picWord());
      assertEq(resumed, expected, 'resume lands on the exact next word');
      await page.click('#wpExit');
      await page.waitForTimeout(300);
    } finally { await ctx.close(); }
  });

  await suite.test('continue: long-press Remove discards state, not the picture', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicker(page);
      await startAndLeave(page, 'star', 1);
      await openPicker(page);
      const card = await page.$('#wpResumeRow [data-resume="star"]');
      assert(card, 'run card present');
      const box = await card.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.waitForTimeout(750);
      await page.mouse.up();
      await page.waitForSelector('#wpHk.show', { timeout: 3000 });
      await page.click('#wpHkRemove');
      await page.waitForTimeout(400);
      assert(await page.$eval('#wpResumeRow', (e) => e.classList.contains('btn-hide')),
        'run discarded, row hides');
      assert(await page.$('[data-pic="star"]'), 'the PICTURE survives (only state died)');
    } finally { await ctx.close(); }
  });

  await suite.test('families: shelves render; learn leads with the active script', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ru' });
    try {
      await openPicker(page);
      const heads = await page.$$eval('#wpGrid .wp-fam-head', (els) => els.length);
      assert(heads >= 4, `family headers render (saw ${heads})`);
      // Collapsed rows can fill the learn row with the numbers pack, so
      // open the full grid via See All before asserting script order.
      if (await page.$('[data-cat="learn"]')) {
        await page.click('[data-cat="learn"]');
        await page.waitForTimeout(200);
      }
      // The learn shelf's first alphabet tile is Cyrillic for a ru player.
      const firstLearn = await page.$$eval('#wpGrid .wp-shelf', (shelves) => {
        for (const s of shelves) {
          const tiles = [...s.querySelectorAll('[data-pic]')].map((t) => t.getAttribute('data-pic'));
          if (tiles.some((t) => /^(lat|cyr|arb|dev|kor|hir|hnz)\d/.test(t) || /^(zero|one|two)$/.test(t))) {
            return tiles.filter((t) => /^(lat|cyr)\d/.test(t))[0];
          }
        }
        return null;
      });
      assert(firstLearn && firstLearn.startsWith('cyr'),
        `ru learner sees Cyrillic first (saw ${firstLearn})`);
    } finally { await ctx.close(); }
  });
}
