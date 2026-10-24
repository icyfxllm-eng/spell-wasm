// keyboard-moments.spec — CC-SPELL-KEYBOARD Phase C (KF5, KF6, KF7).
//
//   KT-14  the letter flight never delays the real letter, is skipped for fast
//          typing (K-D22) and does not show under Reduce Motion;
//   KT-15  the keyboard reacts only where the mode shows a verdict (K-D23, I7):
//          a correct answer waves and draws its constellation, a wrong one dims,
//          and a mode that withholds its verdict only ever pulses;
//   flags  each moment is off by default (K-D28): nothing moves without its flag.
import { openApp, assert, domSettled } from '../harness.mjs';

const MOMENTS = () => {
  localStorage.setItem('spell_flag_keyboard_flight', 'on');
  localStorage.setItem('spell_flag_keyboard_constellation', 'on');
  localStorage.setItem('spell_flag_keyboard_react', 'on');
};

const serve = async (page) => {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  await domSettled(page);
  return page.evaluate(() => window.__spelltest.currentWord());
};
const key = (ch) => `#kbLetters .skb-key[data-k="${ch}"]`;
const letters = (page) => page.$eval('#letters', (e) => e.textContent.replace(/\s/g, ''));
/** Watch the page for a class or element appearing, so a 500 ms moment is not missed. */
const watch = (page) => page.evaluate(() => {
  window.__seen = new Set();
  new MutationObserver(() => {
    const kb = document.getElementById('kbLetters');
    for (const c of ['skb-wave', 'skb-dim', 'skb-pulse']) if (kb.classList.contains(c)) window.__seen.add(c);
    if (document.querySelector('.skb-constellation')) window.__seen.add('constellation');
    if (document.querySelector('.skb-ghost')) window.__seen.add('ghost');
  }).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ['class'] });
});
const seen = (page) => page.evaluate(() => [...window.__seen]);

export async function run(browser, base, suite) {
  await suite.test('keyboard-moments: KT-14 the flight never delays the letter, and skips fast typing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: MOMENTS });
    try {
      const word = await serve(page);
      await watch(page);
      await page.click(key(word[0]));
      // The real letter is already in the answer while its ghost is in the air.
      const now = await page.evaluate(() => ({ ghost: !!document.querySelector('.skb-ghost'), text: document.getElementById('letters').textContent.replace(/\s/g, '') }));
      assert(now.text === word[0], `the letter is typed at once (${now.text})`);
      assert(now.ghost || (await seen(page)).includes('ghost'), 'a slow key press sends its ghost');
      // Two presses inside 120 ms: the second gets no ghost.
      await page.waitForFunction(() => !document.querySelector('.skb-ghost'), null, { timeout: 2000 });
      const fast = await page.evaluate(([a, b]) => {
        document.querySelector(a).click();
        const before = document.querySelectorAll('.skb-ghost').length;
        document.querySelector(b).click();
        return document.querySelectorAll('.skb-ghost').length - before;
      }, [key(word[1] || word[0]), key(word[2] || word[0])]);
      assert(fast === 0, `a key typed within 120 ms of the last flies nothing (${fast})`);
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard-moments: KT-14 no flight under Reduce Motion', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: MOMENTS });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      const word = await serve(page);
      await page.click(key(word[0]));
      const shown = await page.evaluate(() => [...document.querySelectorAll('.skb-ghost')].some((g) => getComputedStyle(g).display !== 'none'));
      assert(!shown, 'no ghost is visible under Reduce Motion');
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard-moments: KT-15 a correct answer waves and draws its constellation', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: MOMENTS });
    try {
      const word = await serve(page);
      for (const ch of word.toLowerCase()) await page.click(key(ch));
      await watch(page);
      await page.click('#checkBtn');
      await page.waitForFunction(() => /good|bad/.test(document.getElementById('feedback').className), null, { timeout: 5000 });
      await page.waitForFunction(() => window.__seen.has('constellation') || window.__seen.has('skb-wave'), null, { timeout: 2000 }).catch(() => {});
      const s = await seen(page);
      assert(s.includes('skb-wave'), `the keys wave (${s})`);
      assert(s.includes('constellation'), `the constellation is drawn (${s})`);
      assert(!s.includes('skb-dim'), 'and nothing dims');
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard-moments: KT-15 a wrong answer dims and draws no constellation', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: MOMENTS });
    try {
      await serve(page);
      await page.click(key('q'));
      await watch(page);
      await page.click('#checkBtn');
      await page.waitForFunction(() => /good|bad/.test(document.getElementById('feedback').className), null, { timeout: 5000 });
      await page.waitForFunction(() => window.__seen.has('skb-dim'), null, { timeout: 2000 }).catch(() => {});
      const s = await seen(page);
      assert(s.includes('skb-dim'), `the keys dim (${s})`);
      assert(!s.includes('constellation') && !s.includes('skb-wave'), `no correct-answer moment on a miss (${s})`);
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard-moments: KT-15 a solo Spelluzzle board only pulses (its verdict is withheld)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: () => {
      localStorage.setItem('spell_flag_spelluzzle', 'on');
      localStorage.setItem('spell_flag_keyboard_flight', 'on');
      localStorage.setItem('spell_flag_keyboard_constellation', 'on');
      localStorage.setItem('spell_flag_keyboard_react', 'on');
    } });
    try {
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show', { timeout: 8000 });
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row', { timeout: 8000 });
      if (await page.evaluate(() => !document.getElementById('szHow').hidden)) await page.click('#szHowOk');
      const ws = await page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
      await page.click('#szBoard .sz-row[data-slot="0"] .sz-cells');
      await page.evaluate(() => {
        window.__szSeen = new Set();
        new MutationObserver(() => {
          const kb = document.getElementById('szKbKeys');
          for (const c of ['skb-wave', 'skb-dim', 'skb-pulse']) if (kb && kb.classList.contains(c)) window.__szSeen.add(c);
          if (document.querySelector('.skb-constellation')) window.__szSeen.add('constellation');
        }).observe(document.body, { subtree: true, attributes: true, childList: true, attributeFilter: ['class'] });
      });
      // A deliberately wrong word: the board must not say so, and neither may the keys.
      const wrong = ws[0].split('').map((c) => (c === 'q' ? 'x' : 'q')).join('');
      for (const c of wrong) await page.click(`#szKb [data-k="u:${c}"]`);
      await domSettled(page);
      const s = await page.evaluate(() => [...window.__szSeen]);
      assert(!s.includes('skb-dim') && !s.includes('skb-wave') && !s.includes('constellation'), `a solo board shows no verdict on its keys (${s})`);
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard-moments: flags off, nothing moves', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const word = await serve(page);
      await watch(page);
      for (const ch of word.toLowerCase()) await page.click(key(ch));
      await page.click('#checkBtn');
      await page.waitForFunction(() => /good|bad/.test(document.getElementById('feedback').className), null, { timeout: 5000 });
      await domSettled(page);
      const s = await seen(page);
      assert(s.length === 0, `no moment runs with its flag off (${s})`);
    } finally { await ctx.close(); }
  });
}
