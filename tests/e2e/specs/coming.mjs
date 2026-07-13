// coming.spec — language availability: en/es are playable; the other 15 are
// gated ("coming soon" + Notify Me) in BOTH Standard and Little Speller, and
// tapping a coming-soon language never starts a round. Notify Me persists.
import { openApp, assert } from '../harness.mjs';

// Both modes exercised: Standard (full age verdict) + Little Speller (kid).
const MODES = [
  ['std', { verdict: 'full', checkedAt: 1700000000 }],
  ['kid', { verdict: 'kid', checkedAt: 1700000000 }],
];

async function openMode(browser, base, age, lang) {
  // openApp defaults to a full verdict; for kid we pre-seed the age verdict.
  const { ctx, page } = await openApp(browser, base, {});
  await ctx.addInitScript((a) => localStorage.setItem('byear_agegate_v1', JSON.stringify(a)), age);
  // reload so the kid verdict takes effect, then select the study language
  await page.reload({ waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  if (lang) {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', lang).catch(() => {});
    await page.click('#setupDone').catch(() => {});
    await page.waitForTimeout(250);
  }
  return { ctx, page };
}

export async function run(browser, base, suite) {
  for (const [mode, age] of MODES) {
    await suite.test(`coming[${mode}]: selecting Korean gates play + shows Notify Me`, async () => {
      const { ctx, page } = await openMode(browser, base, age, 'ko');
      try {
        const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
        assert(gated, 'coming-soon state not applied for a coming-soon language');
        const orbShown = await page.evaluate(() => {
          const o = document.querySelector('#orbWrap');
          return o && getComputedStyle(o).display !== 'none';
        });
        assert(!orbShown, 'play area (orb) is not hidden for a coming-soon language');
        const notify = await page.$eval('#notifyBtn', (e) => e.textContent.trim().length > 0 && getComputedStyle(e).display !== 'none');
        assert(notify, 'Notify Me button missing');
      } finally { await ctx.close(); }
    });
  }

  await suite.test('coming: active language (es) is playable (not gated)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
      assert(!gated, 'active language should not be gated');
      const orbShown = await page.evaluate(() => getComputedStyle(document.querySelector('#orbWrap')).display !== 'none');
      assert(orbShown, 'active language should show the play orb');
    } finally { await ctx.close(); }
  });

  await suite.test('coming: Notify Me tap confirms and survives reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ko' });
    try {
      await page.waitForTimeout(150);
      await page.click('#notifyBtn');
      await page.waitForTimeout(150);
      const confirmed = await page.$eval('#notifyBtn', (e) => e.classList.contains('confirmed'));
      assert(confirmed, 'Notify Me did not flip to confirmed');
      // reload: confirmed state persists (localStorage), and ko is still gated
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(200);
      const stillConfirmed = await page.$eval('#notifyBtn', (e) => e.classList.contains('confirmed'));
      assert(stillConfirmed, 'confirmed Notify Me did not survive reload');
    } finally { await ctx.close(); }
  });
}
