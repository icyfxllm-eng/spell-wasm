// celebrate.spec — CC-VISUAL-ECLIPSE Phase D F7 (AT-18 in a running app),
// behind visual.celebrate.
//
// Real play to a perfect run (10 in a row): the big celebration plays in the
// gap after the 10th answer, never over an active word, holds the next word
// until it ends, lasts 2.5 s at most, and a tap skips it. An achievement is a
// small one. Under Reduce Motion the big one is a calm card with no sparks.
// (The 60 s downgrade needs two bigs inside a minute; celebrate::tests covers it.)
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const ON = () => localStorage.setItem('spell_flag_visual_celebrate', 'on');

/** Answer correctly until the streak reaches `target`; returns attempts used. */
async function runTo(page, target) {
  await page.click('#orbWrap');
  for (let i = 0; i < 40; i++) {
    const streak = Number(await page.$eval('#streakNum', (e) => e.textContent) || 0);
    if (streak >= target) return i;
    await page.waitForFunction(() => !!window.__spelltest.currentWord() && !/\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 15000 });
    const w = await page.evaluate(() => window.__spelltest.currentWord());
    await typeOnKeyboard(page, w);
    await page.click('#checkBtn');
    await page.waitForFunction(() => /\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
    if (/bad/.test(await page.$eval('#feedback', (e) => e.className))) await page.click('#orbWrap');
  }
  throw new Error(`streak never reached ${target}`);
}

/** Record every time the big overlay shows or hides, and whether a word stood answered. */
const watch = (page) => page.evaluate(() => {
  window.__cel = [];
  const el = document.getElementById('celebrate');
  new MutationObserver(() => {
    window.__cel.push({ show: el.classList.contains('show'), calm: el.classList.contains('calm'), t: performance.now(), word: window.__spelltest.currentWord(),
      answered: /\b(good|bad)\b/.test(document.getElementById('feedback').className) });
  }).observe(el, { attributes: true, attributeFilter: ['class'] });
});

export async function run(browser, base, suite) {
  await suite.test('F7 off (default): a perfect run plays no celebration; achievements keep today\'s toast', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await runTo(page, 1);
      await page.waitForFunction(() => document.getElementById('toast').classList.contains('show'), null, { timeout: 5000 });
      const toast = await page.$eval('#toast', (e) => e.textContent);
      assert(/Achievement/.test(toast), `the off toast changed: ${toast}`);
      assert((await page.evaluate(() => window.__spelltest.celebrations().length)) === 0, 'a celebration played with the flag off');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-18: a perfect run plays one big celebration, in the gap, under 2.5 s, holding the next word', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      await watch(page);
      await runTo(page, 10);
      await page.waitForFunction(() => window.__cel.some((x) => x.show), null, { timeout: 5000 });
      const wordAtShow = await page.evaluate(() => window.__spelltest.currentWord());
      await page.waitForFunction(() => window.__cel.some((x) => !x.show), null, { timeout: 4000 });
      const ev = await page.evaluate(() => window.__cel);
      const on = ev.find((x) => x.show), off = ev.find((x) => !x.show && x.t > on.t);
      assert(on.answered, 'the big celebration started while a word was active');
      assert(off.t - on.t <= 2600, `the big celebration lasted ${Math.round(off.t - on.t)} ms`);
      // The next word waited: the overlay closed on the same word it opened on.
      assert(off.word === on.word && on.word === wordAtShow, `the next word was served during the celebration (${on.word} -> ${off.word})`);
      const plays = await page.evaluate(() => window.__spelltest.celebrations().map((c) => c[0]));
      assert(plays.filter((p) => p === 'big').length === 1, `bigs: ${plays}`);
      assert(plays.includes('small'), `no small celebration for the achievements on the way: ${plays}`);
    } finally { await ctx.close(); }
  });

  await suite.test('AT-18: a tap skips the big celebration at once', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      await watch(page);
      await runTo(page, 10);
      await page.waitForFunction(() => document.getElementById('celebrate').classList.contains('show'), null, { timeout: 5000 });
      const t0 = await page.evaluate(() => performance.now());
      await page.mouse.click(30, 30);
      await page.waitForFunction(() => !document.getElementById('celebrate').classList.contains('show'), null, { timeout: 1000 });
      const t1 = await page.evaluate(() => performance.now());
      assert(t1 - t0 < 500, `the tap took ${Math.round(t1 - t0)} ms to skip`);
    } finally { await ctx.close(); }
  });

  await suite.test('F7 Reduce Motion: the big celebration is a calm card, no sparks', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await runTo(page, 10);
      await page.waitForFunction(() => document.getElementById('celebrate').classList.contains('show'), null, { timeout: 5000 });
      const r = await page.evaluate(() => {
        const c = document.getElementById('celebrateFx');
        const ctx = c.getContext('2d');
        let lit = 0;
        if (c.width && c.height) {
          const d = ctx.getImageData(0, 0, c.width, c.height).data;
          for (let i = 3; i < d.length; i += 4 * 97) if (d[i]) lit++;
        }
        return { calm: document.getElementById('celebrate').classList.contains('calm'), anim: getComputedStyle(document.getElementById('celebrateMsg')).animationName, lit };
      });
      assert(r.calm && r.anim === 'celFade' && r.lit === 0, `reduce: ${JSON.stringify(r)}`);
    } finally { await ctx.close(); }
  });
}
