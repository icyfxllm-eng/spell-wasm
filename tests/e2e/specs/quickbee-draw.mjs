// quickbee-draw.spec -- CC-QUICKBEE-DRAW Phase A: the entry is ABSENT (not greyed)
// for every language, with the flag on or off, because the run loop is not built.
// A7 and I2: the draw entry never appears for vi, en or any other language, and it
// does not appear for zh or ja either until Phase B flips PHASE_B_BUILT.
import { openApp, assert, clickSettled } from '../harness.mjs';

const ALL_ON = () => {
  for (const l of ['zh', 'ja', 'ko']) localStorage.setItem(`spell_flag_quickbee_draw_${l}`, 'on');
};

export async function run(browser, base, suite) {
  const modes = async (page) => {
    await clickSettled(page, '#navBurger');
    return page.$$eval('#navDrawerPanel .nav-row', (els) => els.map((e) => e.dataset.mode));
  };

  for (const lang of ['zh', 'ja', 'ko', 'vi', 'en', 'es']) {
    await suite.test(`quickbee_draw_entry_is_absent_in_${lang}_even_with_every_flag_on`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, init: ALL_ON });
      try {
        const m = await modes(page);
        assert(m.length > 0, 'the drawer rendered no rows');
        assert(!m.includes('quickbee_draw'), `${lang}: the draw entry is offered: ${m.join(',')}`);
        // The hidden launcher exists (the registry names it) but is not a row.
        assert(await page.evaluate(() => document.getElementById('qbdOpenBtn') !== null), 'the launcher element is missing');
      } finally { await ctx.close(); }
    });
  }

  await suite.test('quickbee_draw_entry_is_absent_with_the_flags_off', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'zh' });
    try {
      const m = await modes(page);
      assert(!m.includes('quickbee_draw'), 'offered with every flag off');
    } finally { await ctx.close(); }
  });
}
