// platform.spec — the web/app language split, asserted from the UI in BOTH
// build configs (Eric, 2026-07-31: "For testflight all languages unlocked the
// site english only").
//
// This spec is deliberately in the run-everywhere set. A platform split
// checked in only one configuration is half a check: it proves the site gates
// Spanish, or that the app does not, but never that the same build of the
// same code does the right thing on each side. Both halves here, one file.
import { openApp, assert, assertEq, IS_WEB_BUILD } from '../harness.mjs';

const WHERE = IS_WEB_BUILD ? 'site' : 'app';

export async function run(browser, base, suite) {
  await suite.test(`platform[${WHERE}]: English always plays`, async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
      assert(!gated, 'English must be playable in every build');
    } finally { await ctx.close(); }
  });

  await suite.test(`platform[${WHERE}]: non-English follows the platform`, async () => {
    for (const lang of ['es', 'ja', 'sw']) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
        assertEq(gated, IS_WEB_BUILD,
          IS_WEB_BUILD ? `${lang} must be gated on the site`
                       : `${lang} must be playable in the app`);
      } finally { await ctx.close(); }
    }
  });

  await suite.test(`platform[${WHERE}]: every language is still LISTED`, async () => {
    // The site gates play; it does not pretend the languages do not exist.
    // Losing the roadmap tiles would be a silent product change, so pin it.
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const n = await page.$$eval('#langSel option', (els) => els.length);
      assert(n >= 15, `only ${n} languages listed — the roadmap should stay visible`);
    } finally { await ctx.close(); }
  });
}
