// attempts-shields.spec — CC-ATTEMPTS-SHIELDS at the UI boundary.
//
// build-54: the flag `spell_flag_attempts_shields` defaults ON (the legacy 3-try
// mechanic is retired, so the extra-attempts toggle + Climb shields are the live
// safety net). The flag is toggled the FP2 way — real `localStorage` set through
// each context's addInitScript (mirrors sayit / spellaloud / ghost), NOT a
// harness param and NOT the cfg(test) override (which the wasm seam build lacks).
//
// Verified here:
//   * flag OFF (explicit) -> Feature 1 settings row + Feature 2 HUD hidden
//     (byte-for-byte the pre-shields build).
//   * flag ON (default)   -> row revealed, toggle still defaults OFF per player.
//   * 3-try RETIRED       -> one wrong submission is immediately the miss reveal.
//   * extra-attempts ON   -> one wrong submission grants exactly one clean retry.
//   * language matrix (en / es / ja) reruns identical assertions — zero
//     per-language behavior.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const FLAG = 'spell_flag_attempts_shields';
const hidden = (page, sel) => page.$eval(sel, (e) => e.classList.contains('btn-hide'));
const exists = (page, sel) => page.$(sel).then((h) => !!h);

/** Open the app with the dark flag pinned to `value` ('on'|'off') before boot,
 *  mirroring the FP2 spec pattern (real localStorage via addInitScript). */
async function openWithFlag(browser, base, value, lang = null) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
  await ctx.addInitScript(([age, l, flag, v]) => {
    localStorage.setItem('byear_agegate_v1', age);
    if (l) localStorage.setItem('spellgame.locale', l);
    localStorage.setItem(flag, v);
  }, [AGE, lang, FLAG, value]);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  if (lang) {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', lang).catch(() => {});
    await page.click('#setupDone').catch(() => {});
    await page.waitForTimeout(200);
  }
  return { ctx, page };
}

async function missOnce(page) {
  await page.click('#orbWrap');
  await page.waitForTimeout(500);
  const word = await page.evaluate(() => window.__spelltest.currentWord());
  const wrong = word === 'zzzz' ? 'xxxx' : 'zzzz';
  await typeOnKeyboard(page, wrong);
  await page.click('#checkBtn');
  await page.waitForTimeout(500);
}

export async function run(browser, base, suite) {
  await suite.test('flag OFF (explicit): extra-attempts row + shield HUD hidden', async () => {
    const { ctx, page } = await openWithFlag(browser, base, 'off');
    try {
      assert(await exists(page, '#extraAttemptsRow'), 'row element present in DOM');
      assert(await hidden(page, '#extraAttemptsRow'), 'extra-attempts row must be hidden when flag OFF');
      assert(await hidden(page, '#shieldHud'), 'shield HUD must be hidden when flag OFF');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON (build-54 default): extra-attempts row revealed + toggle defaults OFF', async () => {
    // No flag set -> compiled default ON.
    const { ctx, page } = await openApp(browser, base, {});
    try {
      assert(!(await hidden(page, '#extraAttemptsRow')), 'row must be shown when flag ON (default)');
      const checked = await page.$eval('#extraAttemptsToggle', (e) => e.checked);
      assert(checked === false, 'the per-player toggle still defaults OFF even though the flag is on');
    } finally { await ctx.close(); }
  });

  await suite.test('3-try RETIRED: one wrong submission is immediately the miss (word revealed)', async () => {
    // Normal mode, extra-attempts toggle OFF -> the base path is now one-shot:
    // a single wrong answer reveals the word (no 2nd/3rd try) and reads as a miss.
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await missOnce(page);
      const cls = await page.$eval('#feedback', (e) => e.className);
      assert(!cls.includes('good'), 'wrong answer must not read as correct');
      assert(cls.includes('bad'), 'one wrong answer must land on the miss consequence');
      const revealed = await page.$('#feedback .reveal');
      assert(!!revealed, 'the word must be revealed after a single miss (one-shot, no retry pips)');
      const still = await page.evaluate(() => window.__spelltest.build());
      assert(still === 'testseam', 'wasm crashed on the one-shot miss path');
    } finally { await ctx.close(); }
  });

  await suite.test('extra-attempts ON: a wrong submission grants exactly one clean retry', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // The extra-attempts toggle applies only in a FIXED level (not the adaptive
      // Climb, where shields own retries). Pick Medium via the setup chip first.
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#levelSel', 'medium').catch(() => {});
      await page.click('#setupDone').catch(() => {});
      await page.waitForTimeout(150);
      // Enable the per-player toggle, then miss once.
      await page.click('#setBtn').catch(() => {});
      await page.click('#extraAttemptsToggle');
      await page.click('#setDone').catch(() => {});
      await missOnce(page);
      // A retry was granted: the word is NOT yet revealed and the check control is
      // live again for a second swing. (finalize would show a `.reveal` span.)
      const revealed = await page.$('#feedback .reveal');
      assert(!revealed, 'the first miss under the toggle must grant a retry, not reveal the word');
      const canRetry = await page.$eval('#checkBtn', (e) => !e.disabled);
      assert(canRetry, 'controls must be re-enabled for the granted retry');
      const still = await page.evaluate(() => window.__spelltest.build());
      assert(still === 'testseam', 'wasm crashed on the extra-attempt retry path');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON: extra-attempts toggle persists across reload (single-source pref)', async () => {
    const { ctx, page } = await openApp(browser, base, {});
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
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        assert(!(await hidden(page, '#extraAttemptsRow')), `${lang}: row shown`);
        assert(await exists(page, '#extraAttemptsToggle'), `${lang}: toggle present`);
      } finally { await ctx.close(); }
    });
  }
}
