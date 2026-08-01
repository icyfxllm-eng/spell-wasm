// gallery.spec — CC-FINALE feature 5 / Done #5. Trophies are piece data +
// seed, re-rendered on view (D5): the grid shows finished pieces only, a
// trophy opens byte-identical to its completion frame, and clearing the
// piece store empties the shelf.
//
// Honesty note on the reset leg: FINALE D4 says gallery state is "included
// in the existing progress-reset flow", but no bulk progress-reset exists in
// the codebase (grep: no bulk remove/clear path outside per-feature stores).
// This spec therefore asserts the data-level truth — store cleared → shelf
// empty — and the missing user-facing flow is flagged in the session notes
// rather than quietly invented here.
import { openApp, assert, assertEq } from '../harness.mjs';
import { completePicture } from './finale.mjs';

async function backToPicker(page) {
  await page.click('#wpContinue');
  await page.waitForTimeout(400);
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
}

export async function run(browser, base, suite) {
  await suite.test('gallery: trophies appear per completion, in-progress stays off the shelf', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', device: 'large' });
    try {
      // Two tiers: star (easy), eiffel (hard). Long pieces are the finale
      // spec's problem; two suffice to prove per-completion accrual.
      await completePicture(page, 'star');
      await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
      await page.click('#wpRevealStage');

      // Capture the completion frame for the D5 identity check later.
      const frame = await page.evaluate(() => document.getElementById('wpRevealStage').innerHTML);
      assert(frame.includes('<svg'), 'no completion frame captured');

      await backToPicker(page);
      let shelf = await page.$$eval('#wpGallery [data-gallery]', (els) => els.map((e) => e.getAttribute('data-gallery')));
      assertEq(JSON.stringify(shelf), JSON.stringify(['star']), 'one finished piece → one trophy');

      // Start a second picture and leave it mid-flight: it must NOT shelve.
      await page.click('[data-pic="eiffel"]');
      await page.waitForSelector('#wpPlay.show');
      await page.waitForTimeout(500);
      for (let i = 0; i < 6 && (await page.$('#wpHow.show')); i++) {
        await page.click('#wpHowNext'); await page.waitForTimeout(150);
      }
      const w = await page.evaluate(() => window.__spelltest.picWord());
      for (const ch of w) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`, { delay: 0 });
      await page.waitForTimeout(300);
      await page.click('#wpExit');
      await page.waitForTimeout(300);
      await page.evaluate(() => document.getElementById('wordPicOpen').click());
      await page.waitForSelector('#wpPicker.show');
      shelf = await page.$$eval('#wpGallery [data-gallery]', (els) => els.map((e) => e.getAttribute('data-gallery')));
      assert(!shelf.includes('eiffel'), 'an in-progress piece must not shelve (D4 boundary)');

      // D5: opening the trophy re-renders byte-identical to the completion
      // frame. One normalisation, for the same reason the Rust parity test
      // has one: start_build stamps animation-delay styles onto the
      // completion frame's words, and those are live-play state -- by the
      // time the frame is settled the delays have burned off, and the
      // gallery renders at rest with none. Geometry must match exactly.
      const settle = (html) => html.replace(/ style="animation-delay:\d+ms"/g, '');
      await page.click('#wpGallery [data-gallery="star"]');
      await page.waitForSelector('#wpReveal.show.rest', { timeout: 5000 });
      const reopened = await page.evaluate(() => document.getElementById('wpRevealStage').innerHTML);
      assertEq(settle(reopened), settle(frame), 'trophy re-render drifted from the completion frame (D5)');
      assert(!/animation-delay/.test(reopened), 'a trophy opens at rest, not mid-build');

      // Clearing the piece store empties the shelf (data-level reset).
      await page.evaluate(() => { localStorage.removeItem('spell_wordpic'); location.reload(); });
      await page.waitForLoadState('load');
      await page.waitForTimeout(1500);
      await page.evaluate(() => document.getElementById('wordPicOpen').click());
      await page.waitForSelector('#wpPicker.show');
      const after = await page.$$eval('#wpGallery [data-gallery]', (els) => els.length);
      assertEq(after, 0, 'cleared store must empty the gallery');
      const hidden = await page.evaluate(() => document.getElementById('wpGallery').classList.contains('btn-hide'));
      assert(hidden, 'an empty shelf hides rather than showing a bare strip');
    } finally { await ctx.close(); }
  });
}
