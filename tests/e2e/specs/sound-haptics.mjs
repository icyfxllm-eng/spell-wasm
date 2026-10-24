// sound-haptics.spec — CC-VISUAL-ECLIPSE Phase C (F5, F6; AT-16, AT-17 in a
// running app), behind visual.sound and visual.haptics.
//
// The observables are the seam's counters: sounds played and dropped
// (sfxCounts) and haptics fired (hapticsFired, counted before the web no-op,
// since there is no haptic engine in a browser). The four settings_effect_*
// tests are the effect tests config/settings-effects.json names.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

// Both flags, plus a silent browser voice that FINISHES: the harness's stub
// never fires its events, and a voice that never ends holds the game sounds off
// (by design) until the 4 s watchdog.
const BOTH = () => {
  localStorage.setItem('spell_flag_visual_sound', 'on');
  localStorage.setItem('spell_flag_visual_haptics', 'on');
  try {
    Object.defineProperty(window, 'speechSynthesis', {
      configurable: true,
      get: () => ({
        speak(u) { setTimeout(() => { u.onstart && u.onstart(); setTimeout(() => u.onend && u.onend(), 200); }, 10); }, // sleep-ok: a fake voice's own duration
        cancel() {}, pause() {}, resume() {},
        getVoices: () => [], speaking: false, pending: false, paused: false,
        addEventListener() {}, removeEventListener() {},
      }),
    });
  } catch (_) { /* the flags alone still drive the server-clip path */ }
};

const counts = (page) => page.evaluate(() => ({ sfx: window.__spelltest.sfxCounts(), hap: window.__spelltest.hapticsFired() }));
const setToggle = (page, id, on) => page.evaluate(([id, on]) => { const t = document.getElementById(id); t.checked = on; t.dispatchEvent(new Event('change')); }, [id, on]);

async function serve(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}
async function answer(page, text) {
  await typeOnKeyboard(page, text);
  await page.click('#checkBtn');
  await page.waitForFunction(() => /\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
}

export async function run(browser, base, suite) {
  await suite.test('Phase C off (default): no sound, no Settings rows, today\'s haptics uncapped', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const rows = await page.evaluate(() => [...document.querySelectorAll('.snd-only, .hap-only')].map((r) => getComputedStyle(r).display));
      assert(rows.length === 4 && rows.every((d) => d === 'none'), `rows shown with the flags off: ${rows}`);
      const word = await serve(page);
      await answer(page, word);
      const c = await counts(page);
      assert(c.sfx[0] === 0, `a sound played with visual.sound off: ${c.sfx}`);
      assert(c.hap >= word.length + 1, `today's haptics did not all fire: ${c.hap} for ${word.length} keys and a verdict`);
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_game_sounds', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: BOTH });
    try {
      assert(await page.evaluate(() => document.getElementById('gameSoundsToggle').checked), 'Game sounds does not default on');
      const start = (await counts(page)).sfx[0];
      let word = await serve(page);
      // Answer once the voice is done, as a player would.
      await page.waitForTimeout(500); // sleep-ok: the fake voice's 200 ms must have ended
      await answer(page, word);
      assert((await counts(page)).sfx[0] > start, 'no verdict sound with Game sounds on');
      await setToggle(page, 'gameSoundsToggle', false);
      await page.evaluate(() => document.getElementById('checkBtn').click());
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(!(await page.evaluate(() => document.getElementById('gameSoundsToggle').checked)), 'Game sounds off did not persist');
      const before = (await counts(page)).sfx[0];
      word = await serve(page);
      await answer(page, word);
      assert((await counts(page)).sfx[0] === before, 'a sound played with Game sounds off');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_game_sounds_level', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: BOTH });
    try {
      const level = () => page.$eval('#gameSoundsLevel', (e) => e.value);
      assert((await level()) === '0.4', `the level does not default to 40%: ${await level()}`);
      // AT-16: the voice volume slider does not move the game sounds level.
      await page.evaluate(() => { const v = document.getElementById('volumeSlider'); v.value = '2'; v.dispatchEvent(new Event('input')); });
      assert((await level()) === '0.4', 'moving the voice volume moved the game sounds level');
      await page.evaluate(() => { const v = document.getElementById('gameSoundsLevel'); v.value = '0'; v.dispatchEvent(new Event('input')); });
      assert((await page.$eval('#volumeSlider', (e) => e.value)) === '2', 'moving the game sounds level moved the voice volume');
      // The harness's setup-sheet taps have already played; count from here.
      const before = (await counts(page)).sfx[0];
      const word = await serve(page);
      await answer(page, word);
      assert((await counts(page)).sfx[0] === before, 'a sound played at level 0');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_haptics', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: BOTH });
    try {
      await setToggle(page, 'keyFeelToggle', false);
      const word = await serve(page);
      await answer(page, word);
      const on = (await counts(page)).hap;
      assert(on === 1, `with Key feel off, one answer should fire exactly its verdict haptic, got ${on}`);
      await setToggle(page, 'hapticsToggle', false);
      const before = (await counts(page)).hap;
      await answer(page, (await serve(page)) + 'q');
      assert((await counts(page)).hap === before, 'an answer haptic fired with Haptics off');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_key_feel', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: BOTH });
    try {
      await setToggle(page, 'hapticsToggle', false);
      await serve(page);
      await typeOnKeyboard(page, 'abc');
      const on = (await counts(page)).hap;
      assert(on === 3, `three keys with Key feel on fired ${on}`);
      await setToggle(page, 'keyFeelToggle', false);
      await typeOnKeyboard(page, 'de');
      assert((await counts(page)).hap === on, 'keys fired with Key feel off');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-17: a scripted word never gets more than 2 answer haptics; keys are exempt', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: BOTH });
    try {
      await setToggle(page, 'keyFeelToggle', false);
      const word = await serve(page);
      // Wrong answers on the same word (a retry keeps the word), then hear it again.
      await answer(page, word + 'q');
      const after = (await counts(page)).hap;
      assert(after <= 2, `one word fired ${after} answer haptics`);
    } finally { await ctx.close(); }
  });
}
