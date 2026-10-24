// hotfix-glow.spec — CC-ECLIPSE-HOTFIX F2 / I5: the answer-box letters fill
// with the box's ink for every glow on both themes; the glow is only the halo.
//
// The five glows are the four swatches plus a custom pick (the fifth swatch is
// the custom-colour well; there is no Rainbow glow in the app). The pale
// yellow custom pick is the worst case on a light field. Measured on the fill,
// shadows ignored (I5).
import { typeOnKeyboard, domSettled, assert } from '../harness.mjs';
import { THEMES, openThemed, ratio, PARSE_COLOUR } from '../hotfix-lib.mjs';

const CUSTOM = '#ffe14d';

async function pickGlow(page, which) {
  await page.evaluate(([which, custom]) => {
    if (which === 'custom') {
      const i = document.getElementById('glowColor');
      i.value = custom;
      i.dispatchEvent(new Event('input', { bubbles: true }));
      i.dispatchEvent(new Event('change', { bubbles: true }));
    } else {
      document.querySelector(`#glowPick .swatch[data-c="${which}"]`).click();
    }
  }, [which, CUSTOM]);
  await domSettled(page);
}

export async function run(browser, base, suite) {
  for (const theme of THEMES) {
    await suite.test(`I5 ${theme}: typed letters fill with ink >= 4.5:1 for all five glows; the glow is the halo`, async () => {
      const { ctx, page } = await openThemed(browser, base, theme);
      const bad = [];
      try {
        const glows = await page.$$eval('#glowPick .swatch[data-c]', (s) => s.map((e) => e.dataset.c));
        assert(glows.length === 4, `expected four glow swatches, found ${glows.length}`);
        await page.click('#orbWrap');
        await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        await typeOnKeyboard(page, word.slice(0, 3));
        for (const g of [...glows, 'custom']) {
          await pickGlow(page, g);
          const m = await page.evaluate((parse) => {
            const P = eval(parse);
            const ltr = document.querySelector('#letters .ltr:not(.space)');
            const box = document.getElementById('spellbox');
            const glow = getComputedStyle(document.querySelector('.stage') || document.documentElement).getPropertyValue('--glow').trim();
            return { fill: P(getComputedStyle(ltr).color), field: P(getComputedStyle(box).backgroundColor), shadow: getComputedStyle(ltr).textShadow, glow };
          }, PARSE_COLOUR);
          const r = ratio(m.fill, m.field);
          if (r < 4.5) bad.push(`${g}: fill ${r.toFixed(2)}:1 on the field`);
          if (!m.shadow || m.shadow === 'none') bad.push(`${g}: no halo`);
        }
      } finally { await ctx.close(); }
      assert(!bad.length, bad.join('; '));
    });
  }
}
