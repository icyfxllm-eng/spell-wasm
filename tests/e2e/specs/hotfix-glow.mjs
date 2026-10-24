// hotfix-glow.spec — CC-ECLIPSE-HOTFIX F2 / I5: the answer-box letters fill
// with the box's ink for every glow on both themes; the glow is only the halo.
//
// The five glows are the four swatches plus a custom pick (the fifth swatch is
// the custom-colour well; there is no Rainbow glow in the app). The pale
// yellow custom pick is the worst case on a light field. Measured on the fill,
// shadows ignored (I5).
import { typeOnKeyboard, domSettled, assert } from '../harness.mjs';
import { THEMES, openThemed, ratio, PARSE_COLOUR, decodePng } from '../hotfix-lib.mjs';

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

  // I6 (F3, D2): the orb never takes the glow. In the states that used to read
  // --glow (awaiting's underglow, milestone's halo) and at rest, swapping the
  // glow changes at most 1% of the pixels inside the orb's circle.
  for (const theme of THEMES) {
    await suite.test(`I6 ${theme}: changing the glow changes no pixel inside the orb (<= 1%), at rest, awaiting and milestone`, async () => {
      const { ctx, page } = await openThemed(browser, base, theme);
      const bad = [];
      try {
        const glows = await page.$$eval('#glowPick .swatch[data-c]', (s) => s.map((e) => e.dataset.c));
        const box = await page.evaluate(() => { const r = document.querySelector('#orbWrap .orb').getBoundingClientRect(); return { x: r.left, y: r.top, width: r.width, height: r.height }; });
        const shot = async () => decodePng(await page.screenshot({ clip: box, scale: 'css', animations: 'disabled' }));
        for (const state of ['idle', 'awaiting', 'milestone']) {
          await page.evaluate((st) => document.getElementById('orbWrap').setAttribute('data-orb', st), state);
          const shots = [];
          for (const g of [glows[0], glows[1], 'custom']) { await pickGlow(page, g); await domSettled(page); shots.push(await shot()); }
          const { w, h } = shots[0];
          for (let k = 1; k < shots.length; k++) {
            let inside = 0, changed = 0;
            for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
              const dx = x + 0.5 - w / 2, dy = y + 0.5 - h / 2;
              if (dx * dx + dy * dy > (Math.min(w, h) / 2) ** 2) continue;
              inside++;
              const o = (y * w + x) * 4, a = shots[0].px, b = shots[k].px;
              if (Math.abs(a[o] - b[o]) + Math.abs(a[o + 1] - b[o + 1]) + Math.abs(a[o + 2] - b[o + 2]) > 6) changed++;
            }
            const pct = (100 * changed) / inside;
            if (pct > 1) bad.push(`${state}: ${pct.toFixed(2)}% of the orb changed with the glow`);
          }
        }
      } finally { await ctx.close(); }
      assert(!bad.length, bad.join('; '));
    });
  }
}
