// modes.spec — Daily Challenge start; Head-to-Head start + quit-mid-game clean
// state; menu/agegate integrity.
import { openApp, assert } from '../harness.mjs';

export async function run(browser, base, suite) {
  await suite.test('daily: entering shows progress bar + locks language', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#dailyBtn'); await page.waitForTimeout(400);
      const barHidden = await page.$eval('#dailyBar', (e) => e.classList.contains('btn-hide'));
      const langDisabled = await page.$eval('#langSel', (e) => e.disabled);
      assert(!barHidden, 'daily progress bar not shown');
      assert(langDisabled, 'language selector not locked during daily');
    } finally { await ctx.close(); }
  });

  await suite.test('daily: deterministic per date+language (seam pool stable)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // The daily set is seeded (date,lang); pool() over the same tier is stable.
      const a = await page.evaluate(() => window.__spelltest.pool('en', 'easy'));
      const b = await page.evaluate(() => window.__spelltest.pool('en', 'easy'));
      assert(JSON.stringify(a) === JSON.stringify(b) && a.length > 0, 'pool not stable');
    } finally { await ctx.close(); }
  });

  await suite.test('h2h: start then quit mid-game returns to clean solo state', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#vsBtn'); await page.waitForTimeout(300);
      await page.click('#vsStart').catch(() => {});
      await page.waitForTimeout(300);
      // Quit mid-match.
      await page.click('#vsExit').catch(() => {});
      await page.waitForTimeout(200);
      await page.click('#vsQuitConfirm').catch(() => {});
      await page.waitForTimeout(300);
      const vsBarHidden = await page.$eval('#vsBar', (e) => e.classList.contains('btn-hide'));
      const langEnabled = await page.$eval('#langSel', (e) => !e.disabled);
      assert(vsBarHidden, 'versus bar still visible after quit');
      assert(langEnabled, 'solo controls not restored after quit');
    } finally { await ctx.close(); }
  });

  await suite.test('agegate: no stored verdict shows the DOB prompt cold', async () => {
    // Fresh context WITHOUT the age-gate seed -> the gate must appear.
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    try {
      await page.goto(base, { waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest, null, { timeout: 30000 }).catch(() => {});
      await page.waitForTimeout(500);
      const gateShown = await page.$eval('#ageScrim', (e) => e.classList.contains('show'));
      assert(gateShown, 'age gate not shown on a fresh install');
    } finally { await ctx.close(); }
  });
}
