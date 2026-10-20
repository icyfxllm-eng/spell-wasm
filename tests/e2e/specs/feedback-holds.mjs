// feedback-holds.spec — two pieces of feedback that used to be cleared in the
// same tick they were drawn, so a player never saw them (found by the
// CC-VISUAL-ECLIPSE recon, 2026-10-20):
//   - Word Bee, Kid Mode miss: #beeSpelling showed the right spelling, then
//     render_round cleared it before a frame could paint.
//   - Impostor, right answer: the card turned green, then advance() re-rendered
//     the cards at once.
// Each now holds before moving on. These tests read the screen after a delay a
// player would actually see, and check that the game still moves on afterwards.
import { openApp, assert, domSettled } from '../harness.mjs';

// These tests are about the feedback, not the door, so they open each mode
// through its own (hidden) open control rather than whichever hub row leads to it.
const openMode = (page, id) => page.evaluate((id) => document.getElementById(id).click(), id);

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// The harness silences Web Speech; this keeps it silent but remembers what the
// app asked it to say, which for Impostor is the real spelling.
const captureSpeech = () => {
  window.__spoken = [];
  Object.defineProperty(window, 'speechSynthesis', {
    configurable: true,
    get: () => ({
      speak(u) { window.__spoken.push(u.text); }, cancel() {}, pause() {}, resume() {},
      getVoices: () => [], speaking: false, pending: false, paused: false,
      addEventListener() {}, removeEventListener() {},
    }),
  });
};

export async function run(browser, base, suite) {
  await suite.test('bee, kid miss: the right spelling stays on screen, then the next word comes', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: AGE_KID });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'not in Kid Mode');
      await openMode(page, 'beeOpen');
      await page.waitForSelector('#bee.show', { timeout: 5000 });
      const round = () => page.evaluate(() => document.getElementById('beeRound').textContent);
      const before = await round();
      // One letter is never a whole bank word, so this is a miss.
      await page.click('#beeKeys [data-k="q"]');
      await page.click('#beeSubmit');
      // sleep-ok: the assertion is that the spelling is STILL on screen after a
      // delay a player can read in; there is no positive event to wait for.
      await sleep(400);
      const shown = await page.evaluate(() => document.getElementById('beeSpelling').textContent);
      assert(shown.length > 1, `the right spelling must be visible after a kid miss (got "${shown}")`);
      assert((await round()) === before, 'the bee moved on before the spelling could be read');
      await page.waitForFunction((b) => document.getElementById('beeRound').textContent !== b,
        before, { timeout: 5000 }).catch(() => {});
      await domSettled(page);
      assert((await round()) !== before, 'the bee never moved on after the hold');
      const cleared = await page.evaluate(() => document.getElementById('beeSpelling').textContent);
      assert(cleared === '', `the next word must start clean (got "${cleared}")`);
    } finally { await ctx.close(); }
  });

  await suite.test('impostor, right answer: the green card stays up, then the next set comes', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: captureSpeech });
    try {
      await openMode(page, 'impostorOpen');
      await page.waitForSelector('#impostor.show #impCards .imp-card', { timeout: 5000 });
      const progress = () => page.evaluate(() => document.getElementById('impProgress').textContent);
      const before = await progress();
      const idx = await page.evaluate(() => {
        const word = window.__spoken[window.__spoken.length - 1];
        return [...document.querySelectorAll('#impCards .imp-card')].findIndex((c) => c.textContent === word);
      });
      assert(idx >= 0, 'could not find the real spelling among the cards');
      await page.click(`#impCards .imp-card[data-i="${idx}"]`);
      // sleep-ok: asserting the green card is STILL up after a visible delay.
      await sleep(250);
      const held = await page.evaluate((i) => {
        const c = document.querySelector(`#impCards .imp-card[data-i="${i}"]`);
        return !!c && c.classList.contains('right');
      }, idx);
      assert(held, 'the right card must still be green 250 ms after the tap');
      assert((await progress()) === before, 'the next set came before the green card could be seen');
      await page.waitForFunction((b) => document.getElementById('impProgress').textContent !== b,
        before, { timeout: 5000 }).catch(() => {});
      await domSettled(page);
      const greenLeft = await page.evaluate(() => document.querySelectorAll('#impCards .imp-card.right').length);
      assert(greenLeft === 0, 'the next set must start with no green card');
      assert((await progress()) !== before, 'the game never moved on after the hold');
    } finally { await ctx.close(); }
  });
}
