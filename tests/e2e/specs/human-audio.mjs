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

// Wait for the ROUTER to settle, not for a play to be logged.
//
// This used to wait for `__playLog.length > 0` and then sleep 400ms, and it
// was the flakiest thing in the suite -- three different tests in this file
// and the next failed on it across six gate runs on 2026-10-03, each passing
// on an immediate retry.
//
// Two faults, one in each half. The log wait is the wrong CONDITION: only an
// HTMLMediaElement play is logged, so when the chain falls through to
// native-tts -- which is exactly what the switch-off test forces -- nothing
// is ever logged and the wait burns its five seconds. And the 400ms sleep is
// the wrong KIND of wait: it is a guess about how long the router takes, and
// a guess that was being made on a machine also running a cache warm and a
// gate.
//
// `set_source` in src/api.rs is called for every outcome including "none",
// and writes #audioSourceNote. So a non-empty note means the router has
// finished, whichever source won and even if none did. It is also strictly
// LATER than the play log: set_source("human") runs after audio.play()'s
// promise resolves, while the log records at call time. Waiting on the note
// therefore guarantees any play has already been logged -- which is the
// ordering the assertions below quietly depended on and never had.
async function serveWord(page) {
  // Three steps, and the first one is the subtle one.
  //
  // SETTLE FIRST -- and read the next paragraph before trusting this.
  //
  // The app plays a word on load, and that chain writes the note when it
  // finishes. Clearing the note while that write is still in flight lets it
  // land a moment later and satisfy "wait until non-empty" before this
  // click's chain has run at all: the same stale-note bug as below, entered
  // from the other side. Waiting for the page to go quiet first closes that
  // window, because once the load-time playback has written its note there
  // is no pending write left to mistake for ours.
  //
  // HONESTY NOTE. That race is real in the code, but it is NOT established
  // as the cause of the failure that prompted this. D2 failed on one gate
  // run (app and site both) with an empty play log while two other test
  // suites had the CPU, and passed three times alone on the same commit --
  // and an attempt to reproduce it under synthetic load failed: the OLD
  // helper passed four for four. So this is a mitigation for a theory that
  // fits the symptom, not a fix with a reproduction behind it. If D2 flakes
  // again, this comment is the first thing to disbelieve: capture the note's
  // value and the play log at the moment of failure rather than assuming
  // this closed it.
  //
  // It is tolerant because there may be nothing pending.
  await page
    .waitForFunction(
      () => {
        const n = document.getElementById('audioSourceNote');
        return !!n && n.textContent.trim() !== '';
      },
      null,
      { timeout: 10000 },
    )
    .catch(() => {});

  // CLEAR the note. It is not reset between playbacks, so a note left set by
  // the playback above would satisfy the wait below before this click's chain
  // has run. That mistake was made here and caught by the I6 test, which saw
  // no /api/speak request because the assertion ran before the router reached
  // the server source.
  await page.evaluate(() => {
    const n = document.getElementById('audioSourceNote');
    if (n) n.textContent = '';
  });

  // Now the note can only be written by the chain this click starts.
  await page.click('#orbWrap');
  await page.waitForFunction(
    () => {
      const n = document.getElementById('audioSourceNote');
      return !!n && n.textContent.trim() !== '';
    },
    null,
    { timeout: 10000 },
  );
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
      // Wait for the SECOND play rather than for 400ms. The assertion below is
      // about plays.length >= 2, so that is the condition to wait on; sleeping
      // instead just guesses how long a replay takes on a loaded machine.
      await page.waitForFunction(
        () => (window.__playLog || []).filter((p) => p.src.includes('/human-audio/en/')).length >= 2,
        null,
        { timeout: 10000 },
      );
      const plays = await humanPlays(page);
      const slow = plays[plays.length - 1];
      assert(plays.length >= 2 && slow.src === first.src, `slow replay played a different clip: ${slow && slow.src}`);
      assert(Math.abs(slow.rate - 0.7) < 0.01, `slow rate ${slow.rate}, want 0.7`);
      assert(slow.pitch !== false, 'slow replay must preserve pitch');
    } finally { await ctx.close(); }
  });
}
