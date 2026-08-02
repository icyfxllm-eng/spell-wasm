// audio-gate.spec — CC-PICTURE-BANK feature 5 / Done #4, the tier-gated
// audio modifiers, ON THE UI:
//
//   starter  (star, easy):   Replay visible, and the tier's word audio is
//                            the SLOW variant (the slow voice rides it).
//   advanced (eiffel, hard): Replay visible, but the audio is NORMAL —
//                            Slow is gone.
//   expert   (mona):         Replay HIDDEN (hide, never disable): the word
//                            plays once, full stop.
//
// The observable is the audio router's own output: sources are
// server-primary (D1), and the browser mechanism constructs an
// Audio(/api/speak?...&variant=...) element — so the spec patches the
// Audio constructor BEFORE the app boots and reads the variant off the
// URLs. Counting network requests does not work here: replaying a word
// already loaded rides play_word_html's `already_current` fast path
// (rewind + play, no new fetch), which is correct behavior the first
// version of this spec misread as "the click never reached the router."
import { openApp, assert } from '../harness.mjs';

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

/** Patch the Audio constructor before any app code runs, then reboot the
 *  app so the patch is in place from the first instruction. Every clip
 *  the router starts lands in window.__speakLog as its source URL. */
async function armAudioProbe(ctx, page) {
  await ctx.addInitScript(() => {
    window.__speakLog = [];
    const Orig = window.Audio;
    const Patched = function (src) {
      if (src) window.__speakLog.push(String(src));
      return new Orig(src);
    };
    Patched.prototype = Orig.prototype;
    window.Audio = Patched;
  });
  await page.reload();
  await page.waitForFunction(
    () => window.__spelltest && window.__spelltest.build() === 'testseam',
    null, { timeout: 30000 });
}

/** The /api/speak variants seen so far, in order. */
const variantsSeen = (page) =>
  page.evaluate(() =>
    (window.__speakLog || [])
      .filter((u) => u.includes('/api/speak'))
      .map((u) => new URL(u, location.href).searchParams.get('variant')));

const replayHidden = (page) =>
  page.$eval('#wpReplay', (el) => el.classList.contains('btn-hide'));

/** Open `pic`, nudge Replay when the tier shows it, and return the
 *  variants the router produced. */
async function playAndCollect(page, pic) {
  await openPicture(page, pic);
  if (!(await replayHidden(page))) {
    await page.click('#wpReplay');
  }
  await page.waitForTimeout(600);
  return variantsSeen(page);
}

export async function run(browser, base, suite) {
  await suite.test('audio gate: starter keeps Replay AND the slow voice', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await armAudioProbe(ctx, page);
      const seen = await playAndCollect(page, 'star');
      assert(!(await replayHidden(page)), 'starter shows the Replay button');
      assert(seen.length > 0, 'the word audio reached the router');
      assert(seen.includes('slow'), `starter plays the SLOW clip (saw ${seen})`);
      assert(!seen.includes('normal'), `starter never plays normal (saw ${seen})`);
    } finally { await ctx.close(); }
  });

  await suite.test('audio gate: advanced keeps Replay but loses Slow', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await armAudioProbe(ctx, page);
      const seen = await playAndCollect(page, 'eiffel');
      assert(!(await replayHidden(page)), 'advanced still shows Replay');
      assert(seen.length > 0, 'the word audio reached the router');
      assert(seen.includes('normal'), `advanced plays NORMAL (saw ${seen})`);
      assert(!seen.includes('slow'), `advanced lost the slow voice (saw ${seen})`);
    } finally { await ctx.close(); }
  });

  await suite.test('audio gate: expert hides Replay — the word plays once', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await armAudioProbe(ctx, page);
      await openPicture(page, 'mona');
      assert(await replayHidden(page), 'expert HIDES the Replay button');
      // Hidden, not disabled-and-clickable: the control is off the
      // surface, so nothing the player taps can start a second clip. The
      // open may play the word ONCE; nothing else is allowed to.
      await page.waitForTimeout(600);
      const seen = await variantsSeen(page);
      assert(seen.length <= 1, `expert heard at most one clip (saw ${seen})`);
    } finally { await ctx.close(); }
  });
}
