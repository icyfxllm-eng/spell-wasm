// sayit.spec — Feature F2 "Say It" ships DARK and is iOS-only. On the web the
// launcher must never be visible: it's hidden when the flag is off (default),
// AND it stays hidden even with the flag forced on, because the native on-device
// speech bridge (Capacitor) isn't present in the browser. Live mic recognition
// itself needs a physical device and isn't covered here.
import { openApp, assert } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

export async function run(browser, base, suite) {
  await suite.test('say-it: launcher hidden by default (flag off = zero diff)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Present in the DOM (dark), but hidden and not clickable.
      const hidden = await page.$eval('#sayItBtn', (e) => e.classList.contains('btn-hide'));
      const visible = await page.$eval('#sayItBtn', (e) => e.offsetParent !== null);
      assert(hidden, 'sayItBtn should carry btn-hide when the flag is off');
      assert(!visible, 'sayItBtn should not be visible when the flag is off');
    } finally { await ctx.close(); }
  });

  await suite.test('say-it: still hidden with flag ON but no native bridge (not iOS)', async () => {
    const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
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
      const hidden = await page.$eval('#sayItBtn', (e) => e.classList.contains('btn-hide'));
      const scrimShown = await page.$eval('#sayItScrim', (e) => e.classList.contains('show'));
      assert(hidden, 'sayItBtn must stay hidden off-iOS even with the flag on (no on-device bridge)');
      assert(!scrimShown, 'the Say-It overlay must never open on the web');
    } finally { await ctx.close(); }
  });
}
