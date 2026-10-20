// web-speech-fallback.spec — with no native layer (the web build, any browser),
// the router's last source is the browser's Web Speech voice. Before
// 2026-10-20 the web had no last source: a word whose clips were all
// verdict-FAIL, or any server outage, played nothing. Eric chose Web Speech.
//
// The harness answers /api/speak with an empty body, which cannot play, so the
// router falls through every rendered source here, exactly as it does for a
// FAIL-verdict word (that skip is the same "advance to the next source" step).
import { openApp, assert } from '../harness.mjs';

// Keep Web Speech silent but remember what the app asked it to say.
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
  await suite.test('no playable clip: the browser voice speaks the word, and no failure banner shows', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: captureSpeech });
    try {
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      await page.waitForFunction(
        () => window.__spoken.includes(window.__spelltest.currentSpoken()), null, { timeout: 8000 },
      ).catch(() => {});
      const r = await page.evaluate(() => ({
        word: window.__spelltest.currentSpoken(),
        spoken: window.__spoken.slice(),
        note: (document.getElementById('audioSourceNote') || {}).textContent || '',
        banner: (document.getElementById('voiceNote') || {}).textContent || '',
      }));
      assert(r.spoken.includes(r.word), `Web Speech never spoke "${r.word}" (spoken: ${JSON.stringify(r.spoken)})`);
      assert(r.note === 'browser voice', `the source note should say "browser voice" (got "${r.note}")`);
      assert(!/couldn.t reach the audio/i.test(r.banner), `the audio-failure banner showed: "${r.banner}"`);
    } finally { await ctx.close(); }
  });
}
