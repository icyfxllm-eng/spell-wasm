// audit.spec — the review-gated Filipino audit build (Feature 1 + Feature 7).
//
// Two bundles are exercised:
//   auditBase  → dist-test-audit  (seam + `--features audit`, AUDIT_LANGS=fil)
//   base       → dist-test        (seam only; production-parity, NO audit flag)
//
// On the AUDIT bundle: fil is unlocked (a full round is playable), fil is
// PRESELECTED on first load with no clicks, the picker pins fil FIRST with an
// AUDIT badge, and the one-time first-launch banner renders then stays dismissed.
// On the PROD bundle: NONE of that exists — fil is still gated (coming-soon),
// there is no AUDIT badge, no banner, and no preselect. This is the guard that
// the audit surface never leaks into a shipped build.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

export async function run(browser, base, suite, auditBase) {
  // ---- AUDIT bundle: fil unlocked + Feature 7 ----------------------------
  if (!auditBase) {
    await suite.test('audit: dist-test-audit built (AUDIT_LANGS=fil)', async () => {
      assert(false, 'dist-test-audit missing — run: AUDIT_LANGS=fil bash scripts/build-web-test.sh');
    });
    return;
  }

  await suite.test('audit: fil is PRESELECTED on first load (no clicks)', async () => {
    const { ctx, page } = await openApp(browser, auditBase, {});
    try {
      const lang = await page.evaluate(() => window.__spelltest.currentLang());
      assert(lang === 'fil', `expected preselected fil, got "${lang}"`);
      const sel = await page.$eval('#langSel', (e) => e.value).catch(() => null);
      assert(sel === 'fil', `langSel value should be fil, got "${sel}"`);
    } finally { await ctx.close(); }
  });

  await suite.test('audit: picker pins fil FIRST with an AUDIT badge', async () => {
    const { ctx, page } = await openApp(browser, auditBase, {});
    try {
      const opts = await page.$$eval('#langSel option', (els) => els.map((e) => [e.value, e.textContent]));
      assert(opts.length > 0, 'no options in #langSel');
      assert(opts[0][0] === 'fil', `first option should be fil, got "${opts[0][0]}"`);
      assert(/AUDIT/.test(opts[0][1]), `first option should carry an AUDIT badge, got "${opts[0][1]}"`);
      // fil appears exactly once (pinned entry replaces the in-registry position).
      const filCount = opts.filter(([v]) => v === 'fil').length;
      assert(filCount === 1, `fil should appear once, found ${filCount}`);
    } finally { await ctx.close(); }
  });

  await suite.test('audit: fil is playable — hear word, type, get a result', async () => {
    const { ctx, page } = await openApp(browser, auditBase, { lang: 'fil' });
    try {
      // fil is Active here, so play is NOT gated.
      const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
      assert(!gated, 'fil should be playable (not coming-soon) in the audit build');
      // Pin the easy tier (ascii-only fil words) so the on-screen keyboard path runs.
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#levelSel', 'easy').catch(() => {});
      await page.click('#setupDone').catch(() => {});
      await page.waitForTimeout(200);
      await page.click('#orbWrap'); await page.waitForTimeout(500); // hear the word
      const lang = await page.evaluate(() => window.__spelltest.currentLang());
      assert(lang === 'fil', `round should be in fil, got "${lang}"`);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      assert(word && word.length > 0, 'no fil word from seam');
      const baseOnly = [...word.toLowerCase()].every((c) => /[a-zñ'-]/.test(c));
      if (baseOnly) {
        await typeOnKeyboard(page, word.toLowerCase());
        await page.click('#checkBtn');
        await page.waitForTimeout(400);
        const cls = await page.$eval('#feedback', (e) => e.className);
        assert(cls.includes('good'), `expected good feedback for "${word}", got "${cls}"`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('audit: first-launch banner renders once and stays dismissed', async () => {
    const { ctx, page } = await openApp(browser, auditBase, {});
    try {
      const shown = await page.$('#auditBanner');
      assert(shown, 'first-launch audit banner did not render');
      const txt = await page.$eval('#auditBanner', (e) => e.textContent);
      assert(/Filipino/.test(txt), `banner should mention Filipino, got "${txt}"`);
      await page.click('#auditBannerX');
      await page.waitForTimeout(150);
      const gone = await page.$('#auditBanner');
      assert(!gone, 'banner did not disappear after dismiss');
      // reload in the same context (localStorage persists the dismissal)
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(200);
      const stillGone = await page.$('#auditBanner');
      assert(!stillGone, 'dismissed banner reappeared after reload');
    } finally { await ctx.close(); }
  });

  // ---- PROD bundle: NONE of the audit surface exists ---------------------
  await suite.test('prod: fil is NOT preselected (default is not fil)', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const lang = await page.evaluate(() => window.__spelltest.currentLang());
      assert(lang !== 'fil', `prod must not preselect fil, got "${lang}"`);
    } finally { await ctx.close(); }
  });

  await suite.test('prod: picker has NO AUDIT badge and does not pin fil first', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const opts = await page.$$eval('#langSel option', (els) => els.map((e) => [e.value, e.textContent]));
      assert(opts[0][0] !== 'fil', 'prod must not pin fil to the top');
      const anyBadge = opts.some(([, t]) => /AUDIT/.test(t));
      assert(!anyBadge, 'prod picker must not carry an AUDIT badge');
    } finally { await ctx.close(); }
  });

  await suite.test('prod: fil is still gated (coming-soon), not playable', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'fil' });
    try {
      const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
      assert(gated, 'prod fil must be coming-soon (gated), not playable');
    } finally { await ctx.close(); }
  });

  await suite.test('prod: no first-launch audit banner', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const banner = await page.$('#auditBanner');
      assert(!banner, 'prod build must not render an audit banner');
    } finally { await ctx.close(); }
  });
}
