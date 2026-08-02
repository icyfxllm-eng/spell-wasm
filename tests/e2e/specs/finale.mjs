// finale.spec — CC-FINALE feature 1, the reveal moment.
//
// Done #2: final word -> HUD fades -> the build plays -> a tap skips to rest
// -> rest persists with no auto-advance -> Continue advances. Run at every
// phone class, because the reveal is the one screen that is nothing but
// layout: a piece that overflows or an action row that wraps off-screen
// would be invisible to every other test in the suite.
//
// Reaching the reveal means actually finishing a picture, so this drives the
// real loop -- read the waiting word from the seam, type it on the real
// keyboard -- rather than forcing state. `star` is the shortest piece.
import { openApp, assert, assertEq } from '../harness.mjs';

const CLASSES = [[320, 568], [375, 667], [390, 844], [428, 926]];

/** Play `pic` to completion through the UI. Returns the word count.
 *  Exported: the gallery spec builds its trophies the same honest way. */
export async function completePicture(page, pic) {
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
  await page.click(`[data-pic="${pic}"]`);
  await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
  // The how-to card appears a beat AFTER the play screen and sits over the
  // keyboard, swallowing every key tap. Wait for it rather than sampling
  // once -- checking immediately races it and the failure looks like a
  // mysteriously unclickable keyboard.
  await page.waitForTimeout(500);
  for (let i = 0; i < 6 && (await page.$('#wpHow.show')); i++) {
    await page.click('#wpHowNext');   // three pages, then it closes
    await page.waitForTimeout(150);
  }
  assert(!(await page.$('#wpHow.show')), 'the how-to card never closed');

  let typed = 0;
  for (let guard = 0; guard < 120; guard++) {
    const word = await page.evaluate(() => window.__spelltest.picWord());
    if (!word) break;
    for (const ch of word) {
      await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`, { delay: 0 });
    }
    typed++;
    await page.waitForTimeout(90);
    if (await page.$('#wpReveal.show')) break;
  }
  return typed;
}

export async function run(browser, base, suite) {
  for (const [width, height] of CLASSES) {
    await suite.test(`finale: reveal rests and advances at ${width}x${height}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width, height } });
      try {
        const typed = await completePicture(page, 'star');
        assert(typed > 0, 'never typed a word');
        await page.waitForSelector('#wpReveal.show', { timeout: 8000 });

        // The HUD is gone: the reveal covers it, and the keyboard with it.
        const hudHidden = await page.evaluate(() => {
          const r = document.getElementById('wpReveal').getBoundingClientRect();
          const kb = document.getElementById('gameKeyboard').getBoundingClientRect();
          // reveal covers the screen; the keyboard sits under it
          return r.width > 0 && r.height > 0 && kb.bottom <= r.bottom + 1;
        });
        assert(hudHidden, 'the HUD is not covered by the reveal');

        // A tap on the piece skips the build; it must not also advance.
        await page.click('#wpRevealStage');
        await page.waitForTimeout(120);
        assert(await page.$('#wpReveal.show.rest'), 'tap did not skip to the rest state');
        assert(await page.$('#wpPlay.show'), 'skipping advanced past the picture');

        // Rest is indefinite. Nothing may advance on its own.
        await page.waitForTimeout(3000);
        assert(await page.$('#wpReveal.show'), 'the reveal auto-advanced -- rest must be indefinite');

        // Every action is reachable and on-screen at this width.
        const acts = await page.$$eval('.wp-reveal-acts button', (els) =>
          els.filter((e) => e.offsetParent !== null).map((e) => ({
            id: e.id, r: e.getBoundingClientRect().toJSON(),
          })));
        assert(acts.some((a) => a.id === 'wpContinue'), 'no Continue affordance');
        for (const a of acts) {
          assert(a.r.left >= -0.5 && a.r.right <= width + 0.5,
            `${a.id} is off-screen at ${width}px (${a.r.left}..${a.r.right})`);
          assert(a.r.height >= 30, `${a.id} is only ${a.r.height}px tall`);
        }

        // Continue is the one thing that advances.
        await page.click('#wpContinue');
        await page.waitForTimeout(400);
        assert(!(await page.$('#wpReveal.show')), 'Continue left the reveal up');
      } finally { await ctx.close(); }
    });
  }


  await suite.test('finale: rest holds a full minute — no timer may advance it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', device: 'se' });
    try {
      // Fake timers from BEFORE the reveal exists, so any auto-advance
      // timer the reveal might set is born under our control — then jump
      // 65 virtual seconds and prove nothing moved. The existing 3s check
      // above guards real time; this one guards the spec's ">= 60s" bar
      // without making the battery a minute slower.
      await page.clock.install();
      await completePicture(page, 'star');
      await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
      await page.click('#wpRevealStage');
      await page.waitForTimeout(120);
      assert(await page.$('#wpReveal.show.rest'), 'not at rest before the jump');
      await page.clock.fastForward('01:05');
      await page.waitForTimeout(200);
      assert(await page.$('#wpReveal.show.rest'), 'rest did not survive 65 virtual seconds');
      assert(await page.$('#wpPlay.show'), 'something advanced past the picture');
      await page.click('#wpContinue');
      await page.waitForTimeout(400);
      assert(!(await page.$('#wpReveal.show')), 'Continue still advances after the jump');
    } finally { await ctx.close(); }
  });

  await suite.test('finale: the build replays on request', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', device: 'se' });
    try {
      await completePicture(page, 'star');
      await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
      await page.click('#wpRevealStage');
      await page.waitForTimeout(120);
      assert(await page.$('#wpReveal.rest'), 'not at rest before replay');
      await page.click('#wpReplayBuild');
      await page.waitForTimeout(120);
      // Replay leaves rest and staggers the words again.
      assert(!(await page.$('#wpReveal.rest')), 'replay did not restart the build');
      const delays = await page.$$eval('#wpRevealStage .wp-word',
        (els) => els.map((e) => e.style.animationDelay));
      assert(delays.length > 0, 'no words to rebuild');
      assert(new Set(delays).size > 1, 'words are not staggered — that is not a build');
    } finally { await ctx.close(); }
  });
}
