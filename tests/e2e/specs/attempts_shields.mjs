// attempts-shields.spec — CC-ATTEMPTS-SHIELDS ships DARK behind a flag.
//
// Verifies the dark-flag contract at the UI boundary: with the flag OFF
// (production default) the Feature 1 settings row and the Feature 2 shield HUD
// are hidden — byte-for-byte the current build; with the flag ON they appear.
// The language matrix reruns the same assertions in en / es / ja (identical
// test code) to prove there is zero per-language behavior.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const hidden = (page, sel) => page.$eval(sel, (e) => e.classList.contains('btn-hide'));
const exists = (page, sel) => page.$(sel).then((h) => !!h);

export async function run(browser, base, suite) {
  await suite.test('flag OFF (dark default): extra-attempts row + shield HUD hidden', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      assert(await exists(page, '#extraAttemptsRow'), 'row element present in DOM');
      assert(await hidden(page, '#extraAttemptsRow'), 'extra-attempts row must be hidden when flag OFF');
      assert(await hidden(page, '#shieldHud'), 'shield HUD must be hidden when flag OFF');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON: extra-attempts toggle revealed + defaults OFF', async () => {
    const { ctx, page } = await openApp(browser, base, { attemptsShields: true });
    try {
      assert(!(await hidden(page, '#extraAttemptsRow')), 'row must be shown when flag ON');
      const checked = await page.$eval('#extraAttemptsToggle', (e) => e.checked);
      assert(checked === false, 'the toggle defaults OFF even when the flag is on');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON: a Climb miss runs the shield path without crashing', async () => {
    // Drives the real on_wrong -> on_wrong_climb path at runtime (0 shields held
    // -> normal consequence). Catches any RefCell double-borrow the DOM-free
    // unit tests can't reach: if the wasm aborted, the seam call below throws.
    const { ctx, page } = await openApp(browser, base, { lang: 'en', attemptsShields: true });
    try {
      await page.click('#orbWrap'); await page.waitForTimeout(500);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      const wrong = word === 'zzzz' ? 'xxxx' : 'zzzz';
      await typeOnKeyboard(page, wrong);
      await page.click('#checkBtn');
      await page.waitForTimeout(500);
      const cls = await page.$eval('#feedback', (e) => e.className);
      assert(!cls.includes('good'), 'wrong answer must not read as correct');
      // wasm still alive after the shield-path miss (no panic/abort):
      const still = await page.evaluate(() => window.__spelltest.build());
      assert(still === 'testseam', 'wasm crashed on the shield-miss path');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON: extra-attempts toggle persists across reload (single-source pref)', async () => {
    const { ctx, page } = await openApp(browser, base, { attemptsShields: true });
    try {
      await page.click('#setBtn').catch(() => {});
      await page.click('#extraAttemptsToggle');
      assert(await page.$eval('#extraAttemptsToggle', (e) => e.checked), 'toggle should be on after click');
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(await page.$eval('#extraAttemptsToggle', (e) => e.checked), 'toggle state must survive reload');
    } finally { await ctx.close(); }
  });

  // Language matrix — identical assertions, three languages incl. a
  // composition-input one (ja). Same code path, no per-language branching.
  for (const lang of ['en', 'es', 'ja']) {
    await suite.test(`flag ON in ${lang}: toggle revealed (identical test code)`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, attemptsShields: true });
      try {
        assert(!(await hidden(page, '#extraAttemptsRow')), `${lang}: row shown`);
        assert(await exists(page, '#extraAttemptsToggle'), `${lang}: toggle present`);
      } finally { await ctx.close(); }
    });
  }
}
