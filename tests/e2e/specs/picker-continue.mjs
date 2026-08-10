// picker-continue.spec — in-progress on the TILE, plus the family shelves.
//
// CC-SPELLPIC F3 deleted the Continue strip and the "Jump back in" shelf:
// one session, one hub surface, and that surface is the tile. What this
// spec used to assert about #wpResumeRow now asserts the replacement —
// the row is GONE, and the tile carries the {done}/{total} badge.
//
// Housekeeping moved with it. Restart/Remove hung off the strip's
// long-press, so F3 took its only entry point; per Eric (2026-08-10) the
// tile long-press is now contextual — a tile with a live run opens
// housekeeping, anything else stars.
import { openApp, assert, assertEq } from '../harness.mjs';
import { pickTile } from './finale.mjs';

async function openPicker(page) {
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
}

async function findTile(page, pic) {
  // The v3 hub opens to CATEGORY cards, not a flat picture grid, so a tile
  // is reached the way pickTile reaches it: through search.
  await page.evaluate((id) => {
    const s = document.getElementById('wpSearch');
    s.value = id;
    s.dispatchEvent(new Event('input', { bubbles: true }));
  }, pic);
  await page.waitForSelector(`#wpGrid [data-pic="${pic}"]`, { timeout: 5000 });
  return page.$(`#wpGrid [data-pic="${pic}"]`);
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
  await suite.test('in-progress lives on the tile, and the resume strip is gone', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicker(page);
      // F3: the surface is deleted, not hidden. A hidden row would still be
      // a second place a session could appear.
      assertEq(await page.$('#wpResumeRow'), null, 'the Continue strip no longer exists');

      await startAndLeave(page, 'star', 1);
      const expected = await page.evaluate(() => window.__spelltest.picWord());
      await page.evaluate(() => document.getElementById('wpExit')?.click());
      await openPicker(page);

      // the tile itself reports the run
      await findTile(page, 'star');
      const badge = await page.$eval('#wpGrid [data-pic="star"]', (e) => e.textContent || '');
      assert(/\d+\s*\/\s*\d+/.test(badge), `tile shows progress (saw ${JSON.stringify(badge)})`);

      // and tapping it resumes at the exact word the run was waiting on
      await page.click('#wpGrid [data-pic="star"]');
      await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
      await page.waitForTimeout(400);
      assertEq(await page.evaluate(() => window.__spelltest.picWord()), expected,
        'resume lands on the exact next word');
      await page.click('#wpExit');
      await page.waitForTimeout(300);
    } finally { await ctx.close(); }
  });

  await suite.test('contextual long-press: a run offers Remove, a plain tile stars', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPicker(page);
      await startAndLeave(page, 'star', 1);
      await openPicker(page);

      const tile = await findTile(page, 'star');
      assert(tile, 'the tile is present');
      const box = await tile.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.waitForTimeout(750);
      await page.mouse.up();

      // a tile WITH a run opens housekeeping rather than starring
      await page.waitForSelector('#wpHk.show', { timeout: 3000 });
      await page.click('#wpHkRemove');
      await page.waitForTimeout(400);
      await findTile(page, 'star');
      assert(await page.$('#wpGrid [data-pic="star"]'), 'the PICTURE survives (only state died)');
      const badge = await page.$eval('#wpGrid [data-pic="star"]', (e) => e.textContent || '');
      assert(!/\d+\s*\/\s*\d+/.test(badge), 'and the tile no longer reports progress');
    } finally { await ctx.close(); }
  });

  await suite.test('families: shelves render; learn leads with the active script', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ru' });
    try {
      await openPicker(page);
      // v3 hub: categories are CARDS, not sliding shelves. The law is
      // the same one the old header count guarded — every populated
      // category is reachable from the first screen.
      const cards = await page.$$eval('#wpGrid .wp-catcard', (els) =>
        els.map((e) => e.getAttribute('data-cat')));
      assert(cards.length >= 4, `category cards render (saw ${cards.length})`);
      assert(cards.includes('learn'), 'the learn card is on the hub');
      // v3 hub: Learn opens to script FOLDER cards; the player's own
      // script leads. Then the folder opens to its letters.
      await page.click('[data-cat="learn"]');
      await page.waitForTimeout(200);
      const firstFolder = await page.$eval('#wpGrid .wp-catcard', (el) => el.getAttribute('data-folder'));
      assert(firstFolder === 'cyrillic', `ru learner sees Cyrillic first (saw ${firstFolder})`);
      await page.click('#wpGrid [data-folder="cyrillic"]');
      await page.waitForTimeout(200);
      const firstLearn = await page.$eval('#wpGrid [data-pic]', (el) => el.getAttribute('data-pic'));
      assert(firstLearn && firstLearn.startsWith('cyr'),
        `folder holds Cyrillic letters (saw ${firstLearn})`);
    } finally { await ctx.close(); }
  });
}
