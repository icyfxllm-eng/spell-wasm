// sayit.spec — Feature F2 "Say It" ships DARK and is iOS-only. On the web the
// launcher must never be visible: it's hidden when the flag is off (default),
// AND it stays hidden even with the flag forced on, because the native on-device
// speech bridge (Capacitor) isn't present in the browser. Live mic recognition
// itself needs a physical device and isn't covered here.
import { openApp, assert, pinBaseline } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

export async function run(browser, base, suite) {
  await suite.test('say-it: launcher hidden by default (flag off = zero diff)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // The launcher's own visibility stopped being the law when v1.3.1 F2
      // retired the quick-play row: it is display:none for everyone now, so
      // asserting it is hidden would pass for the wrong reason forever. What
      // still has to hold is that the MODE is not offered, and the drawer row
      // is where that shows.
      await page.click('#navBurger');
      await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
      assert(!(await page.$('.nav-row[data-mode="say_it"]')),
        'say_it must have no drawer row when its flag is off');
      const visible = await page.$eval('#sayItBtn', (e) => e.offsetParent !== null);
      assert(!visible, 'and the retired launcher stays invisible');
    } finally { await ctx.close(); }
  });

  await suite.test('say-it: still hidden with flag ON but no native bridge (not iOS)', async () => {
    const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
    await pinBaseline(ctx);
    await ctx.addInitScript(([age]) => {
      localStorage.setItem('byear_agegate_v1', age);
      // Force the feature flag ON — the mode must STILL stay hidden on the web,
      // because it requires the native SFSpeechRecognizer bridge.
      localStorage.setItem('spell_flag_say_it', '1');
    }, [AGE]);
    await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
    const page = await ctx.newPage();
    await page.goto(base, { waitUntil: 'load' });
    await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
    await page.waitForTimeout(200);
    try {
      // Flag ON, still web: the registry's `platforms: [ios]` is what keeps it
      // away, and after v1.3.1 H2 gave say_it a live status that gate is the
      // ONLY thing left holding it back — so this is the assertion that
      // matters most in this file.
      await page.click('#navBurger');
      await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
      assert(!(await page.$('.nav-row[data-mode="say_it"]')),
        'say_it must have no drawer row off-iOS even with the flag on (no on-device bridge)');
      const scrimShown = await page.$eval('#sayItScrim', (e) => e.classList.contains('show'));
      assert(!scrimShown, 'the Say-It overlay must never open on the web');
    } finally { await ctx.close(); }
  });
}
