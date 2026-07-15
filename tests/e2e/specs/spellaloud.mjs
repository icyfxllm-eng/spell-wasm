// spellaloud.spec — "Spell It Out Loud" voice-spelling INPUT ships DARK and is
// iOS-only. On the web the mic beside the answer field must never be visible: it's
// hidden when the flag is off (default), AND it stays hidden even with the flag
// forced on, because the native on-device speech bridge (Capacitor) isn't present
// in the browser (Invariant I3 — voiceSpell config AND on-device availability).
// The letter parser itself is exhaustively covered by the Rust unit tests; live
// mic recognition needs a physical device and isn't covered here.
import { openApp, assert } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

export async function run(browser, base, suite) {
  await suite.test('spell-aloud: mic hidden by default (flag off = zero diff)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Present in the DOM (dark), but hidden and not clickable.
      const hidden = await page.$eval('#voiceSpellMic', (e) => e.classList.contains('btn-hide'));
      const visible = await page.$eval('#voiceSpellMic', (e) => e.offsetParent !== null);
      assert(hidden, 'voiceSpellMic should carry btn-hide when the flag is off');
      assert(!visible, 'voiceSpellMic should not be visible when the flag is off');
      // Typed input must be entirely unaffected by the (dark) feature.
      const permHidden = await page.$eval('#voiceSpellPerm', (e) => e.classList.contains('btn-hide'));
      assert(permHidden, 'the permission explainer must stay hidden');
    } finally { await ctx.close(); }
  });

  await suite.test('spell-aloud: still hidden with flag ON but no native bridge (not iOS)', async () => {
    const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
    await ctx.addInitScript(([age]) => {
      localStorage.setItem('byear_agegate_v1', age);
      // Force the feature flag ON — the mic must STILL stay hidden on the web,
      // because it requires the native SFSpeechRecognizer bridge (I3: on-device
      // availability is false in a browser).
      localStorage.setItem('spell_flag_spell_aloud', '1');
    }, [AGE]);
    await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
    const page = await ctx.newPage();
    await page.goto(base, { waitUntil: 'load' });
    await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
    await page.waitForTimeout(200);
    try {
      const hidden = await page.$eval('#voiceSpellMic', (e) => e.classList.contains('btn-hide'));
      assert(hidden, 'voiceSpellMic must stay hidden off-iOS even with the flag on (no on-device bridge)');
    } finally { await ctx.close(); }
  });
}
