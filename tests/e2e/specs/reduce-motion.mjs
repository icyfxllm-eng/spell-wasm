// reduce-motion.spec — CC-VISUAL-ECLIPSE I4 at runtime: with the OS asking for
// Reduce Motion, nothing that moves (transform, translate, scale, rotate) runs,
// through a real round, the speaking orb, the drawer and the settings switches.
//
// A sampler installed before the app loads reads document.getAnimations()
// every 50 ms and records any running animation or transition whose keyframes
// move something. scripts/reduce-motion-check.mjs proves the same statically
// for every rule in the stylesheet; this proves the running app agrees, for
// the surfaces a player always passes through. The control run (Reduce Motion
// off) must see motion, or the sampler is not looking.
import { openApp, assert, openSettings } from '../harness.mjs';

const sampler = () => {
  window.__moving = new Set();
  const MOVES = /^(transform|translate|scale|rotate)$/;
  setInterval(() => {
    for (const a of document.getAnimations()) {
      if (a.playState !== 'running' || !a.effect || !a.effect.getKeyframes) continue;
      const moves = a.effect.getKeyframes().some((k) => Object.keys(k).some((p) => MOVES.test(p)));
      if (!moves) continue;
      const t = a.effect.target;
      const who = t ? `${t.tagName.toLowerCase()}${t.id ? '#' + t.id : ''}${t.classList && t.classList.length ? '.' + [...t.classList].join('.') : ''}` : '?';
      window.__moving.add(`${a.animationName || a.transitionProperty || 'animation'} on ${who}`);
    }
  }, 50);
};

const tap = (page, sel) => page.evaluate((s) => { const e = document.querySelector(s); if (e) e.click(); }, sel);
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

async function tour(page) {
  // sleep-ok: the sampler needs wall time to observe looping idle animations
  // (the orb breathes for 4.5 s); there is no event for "it did not move".
  await pause(700);
  await page.evaluate(() => document.getElementById('orbWrap').classList.add('speaking'));
  await pause(700); // sleep-ok: the speaking orb's rings and pulse, as above
  await page.evaluate(() => document.getElementById('orbWrap').classList.remove('speaking'));
  await tap(page, '#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  for (const k of ['q', 'z', 'x']) {
    await tap(page, `#gameKeyboard .kb-key[data-k="${k}"]`);
    await pause(60); // sleep-ok: one sampler tick per keystroke's pop
  }
  await tap(page, '#checkBtn');
  await pause(700); // sleep-ok: the wrong-answer shake lasts 400 ms
  await openSettings(page);
  await page.waitForSelector('#setScrim.show', { timeout: 4000 });
  await tap(page, '#setScrim .switch');
  await pause(400); // sleep-ok: the switch knob's travel
}

export async function run(browser, base, suite) {
  await suite.test('I4: under Reduce Motion nothing moves through a round, the orb, the drawer and settings', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: sampler });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await page.reload({ waitUntil: 'load' });
      await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches), 'Reduce Motion was not emulated');
      await tour(page);
      const moving = await page.evaluate(() => [...window.__moving]);
      assert(!moving.length, `moved under Reduce Motion: ${moving.join('; ')}`);
    } finally { await ctx.close(); }
  });

  await suite.test('I4 control: without Reduce Motion the same tour does move (the sampler is looking)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: sampler });
    try {
      await tour(page);
      const moving = await page.evaluate(() => [...window.__moving]);
      assert(moving.length > 0, 'the sampler saw no motion at all with Reduce Motion off');
    } finally { await ctx.close(); }
  });
}
