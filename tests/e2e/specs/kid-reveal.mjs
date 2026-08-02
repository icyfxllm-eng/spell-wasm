// kid-reveal.spec — CC-FINALE Done #6, the automatable half: in Kid Mode
// the reveal offers Save and never Share (the D4 outcome: body.kid hides
// #wpShare at the stylesheet level, so there is no code path to leak it).
// The other half of Done #6 — Eric personally reviewing the Kid Mode
// reveal screen — cannot live in CI and is tracked in the session notes.
import { openApp, assert } from '../harness.mjs';
import { completePicture } from './finale.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

export async function run(browser, base, suite) {
  await suite.test('kid reveal: Share is gone, Save stays', async () => {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true });
    try {
      await ctx.addInitScript(([age]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', 'en');
      }, [AGE_KID]);
      await ctx.route('**/api/speak**', (r) =>
        r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
      const page = await ctx.newPage();
      await page.goto(base);
      await page.waitForFunction(
        () => window.__spelltest && window.__spelltest.build() === 'testseam',
        null, { timeout: 30000 });
      assert(await page.evaluate(() => document.body.classList.contains('kid')),
        'the agegate verdict did not put the app in Kid Mode');

      await completePicture(page, 'star');
      await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
      await page.click('#wpRevealStage'); // settle to rest
      await page.waitForTimeout(120);

      const vis = await page.evaluate(() => ({
        share: getComputedStyle(document.getElementById('wpShare')).display,
        save: getComputedStyle(document.getElementById('wpSave')).display,
      }));
      assert(vis.share === 'none', `Kid Mode must hide Share (display: ${vis.share})`);
      assert(vis.save !== 'none', 'Kid Mode must keep Save — their art goes home');
    } finally { await ctx.close(); }
  });

  await suite.test('kid reveal control: an adult sees both actions', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await completePicture(page, 'star');
      await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
      await page.click('#wpRevealStage');
      await page.waitForTimeout(120);
      const vis = await page.evaluate(() => ({
        share: getComputedStyle(document.getElementById('wpShare')).display,
        save: getComputedStyle(document.getElementById('wpSave')).display,
      }));
      assert(vis.share !== 'none', 'an adult reveal keeps Share');
      assert(vis.save !== 'none', 'an adult reveal keeps Save');
    } finally { await ctx.close(); }
  });
}
