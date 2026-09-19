// human-audio.spec — CC-HUMAN-AUDIO F5/F6/D8/I6 on the real UI.
//
// The testseam build compiles a fixture manifest that points every English
// bank word at one bundled test clip (build.rs), inert until this spec arms it
// with `spell_test_human_audio`. The observable is what the media element is
// asked to play: an init script wraps HTMLMediaElement.play and logs the
// source, rate and pitch setting of every playback, and the router's own
// outcome note (#audioSourceNote) says which source won.
//
//   9  precedence   switch on: the verified clip plays; switch off: TTS does
//   10 fall-through a clip that fails to load hands over to the next source
//   11 slow replay  the same clip, 0.7x, pitch preserved (D8)
//   13 parity       this spec runs against the app AND the site build
import { openApp, assert } from '../harness.mjs';

const ARM = () => {
  localStorage.setItem('spell_test_human_audio', '1');
  window.__playLog = [];
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    window.__playLog.push({ src: this.src, rate: this.playbackRate, pitch: this.preservesPitch });
    return play.call(this);
  };
};

const humanPlays = (page) =>
  page.evaluate(() => (window.__playLog || []).filter((p) => p.src.includes('/human-audio/en/')));

async function serveWord(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => (window.__playLog || []).length > 0, null, { timeout: 5000 });
  await page.waitForTimeout(400);
}

export async function run(browser, base, suite) {
  await suite.test('human audio: a verified clip heads the router (D2)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ARM });
    try {
      const speak = [];
      page.on('request', (r) => { if (r.url().includes('/api/speak')) speak.push(r.url()); });
      await serveWord(page);
      const plays = await humanPlays(page);
      assert(plays.length > 0, `no human clip played; log: ${JSON.stringify(await page.evaluate(() => window.__playLog))}`);
      await page.waitForFunction(() => document.getElementById('audioSourceNote').textContent === 'real voice',
        null, { timeout: 3000 });
      assert(speak.length === 0, `TTS was also requested: ${speak[0]}`);
    } finally { await ctx.close(); }
  });

  await suite.test('human audio: switch off plays no human clip (F6)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ARM });
    try {
      assert(await page.$eval('#realVoicesToggle', (e) => e.checked), 'Real voices must default ON (D4)');
      await page.$eval('#realVoicesToggle', (e) => e.click());
      assert(!(await page.$eval('#realVoicesToggle', (e) => e.checked)), 'switch did not turn off');
      await serveWord(page);
      const plays = await humanPlays(page);
      assert(plays.length === 0, `switch off but a human clip played: ${plays[0] && plays[0].src}`);
      // And the choice survives a relaunch.
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(!(await page.$eval('#realVoicesToggle', (e) => e.checked)), 'switch did not persist');
    } finally { await ctx.close(); }
  });

  await suite.test('human audio: a clip that fails to load falls through to TTS (I6)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ARM });
    try {
      await ctx.route('**/human-audio/**', (r) => r.fulfill({ status: 404, body: '' }));
      const speak = [];
      page.on('request', (r) => { if (r.url().includes('/api/speak')) speak.push(r.url()); });
      await serveWord(page);
      await page.waitForFunction(() => document.getElementById('audioSourceNote').textContent !== 'real voice'
        && document.getElementById('audioSourceNote').textContent !== '', null, { timeout: 5000 });
      assert(speak.length > 0, 'a failed human clip must hand over to the server clip');
    } finally { await ctx.close(); }
  });

  await suite.test('human audio: slow replay is the same clip at 0.7x, pitch kept (D8)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ARM });
    try {
      await serveWord(page);
      const first = (await humanPlays(page))[0];
      assert(first, 'no human clip on the first hearing');
      const slowVisible = await page.$eval('#slowBtn', (e) => !e.classList.contains('btn-hide') && !e.disabled);
      if (!slowVisible) return; // the tier hides Slow; the Rust test covers the rate
      await page.click('#slowBtn');
      await page.waitForTimeout(400);
      const plays = await humanPlays(page);
      const slow = plays[plays.length - 1];
      assert(plays.length >= 2 && slow.src === first.src, `slow replay played a different clip: ${slow && slow.src}`);
      assert(Math.abs(slow.rate - 0.7) < 0.01, `slow rate ${slow.rate}, want 0.7`);
      assert(slow.pitch !== false, 'slow replay must preserve pitch');
    } finally { await ctx.close(); }
  });
}
