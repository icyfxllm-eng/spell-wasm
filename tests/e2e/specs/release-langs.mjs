// release-langs.spec — CC-LAUNCH-EN A3.2 (Phase B): in the RELEASE language
// profile every route to a language picker offers exactly one language,
// English, and no surface shows a Hidden language or a coming-soon panel (D3).
//
// Runs against the release-profile app build only:
//   SPELL_RELEASE_LANGS=1 bash scripts/build-web-test.sh   # -> dist-test-release/
//   SPELL_RELEASE_LANGS=1 node tests/e2e/run.mjs release-langs
// The default dist-test/ carries every language (`all_langs`), so this spec is
// release-only in run.mjs, the way `coming` is web-only.
//
// Routes covered: the setup sheet's word-source picker (#langSel), including a
// saved non-English choice, a device in another language and an interface
// locale left in another language (I3.3: the interface language is untouched);
// the My Words paste sheet's "Speak in" picker (#importLang); the Translate
// language picker (flag forced on); the Online entry form's language picker
// (flag forced on). The photo import's "Speak in" picker (#photoLang) is
// iOS-only (no camera in a browser); it is built from the same accessor as
// #importLang (words::speak_in_languages), which the I3.1 unit test pins.
import { openApp, openMyWords, assert, assertEq, domSettled } from '../harness.mjs';

const MINE = '__mine';

const optionValues = (page, sel) =>
  page.$$eval(`${sel} option`, (os) => os.map((o) => o.value));

async function setupPicker(page) {
  await page.click('#setupChip');
  await domSettled(page);
  const values = (await optionValues(page, '#langSel')).filter((v) => v !== MINE);
  const selected = await page.$eval('#langSel', (s) => s.value);
  await page.click('#setupDone').catch(() => {});
  return { values, selected };
}

async function noComingSoon(page, where) {
  const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
  assert(!gated, `${where}: the coming-soon state is applied`);
  const panel = await page.$eval('#comingPanel', (e) => getComputedStyle(e).display);
  assertEq(panel, 'none', `${where}: the coming-soon panel is visible`);
  const text = await page.evaluate(() => document.body.innerText);
  assert(!/coming soon/i.test(text), `${where}: "coming soon" copy is on screen`);
  const orb = await page.$eval('#orbWrap', (e) => getComputedStyle(e).display !== 'none');
  assert(orb, `${where}: the play orb is hidden`);
}

export async function run(browser, base, suite) {
  await suite.test('release-langs: the setup picker lists English and nothing else', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const { values, selected } = await setupPicker(page);
      assertEq(JSON.stringify(values), JSON.stringify(['en']), 'word-source options');
      assertEq(selected, 'en', 'selected word source');
      await noComingSoon(page, 'fresh launch');
    } finally { await ctx.close(); }
  });

  await suite.test('release-langs: a saved Korean choice and a Korean interface open on English play', async () => {
    const init = () => {
      localStorage.setItem('byear_prefs_v1', JSON.stringify({ lastLang: 'ko' }));
    };
    // lang: 'ko' sets the persisted interface locale (spellgame.locale).
    const { ctx, page } = await openApp(browser, base, { lang: null, init: `(${init})()` });
    try {
      await page.evaluate(() => localStorage.setItem('spellgame.locale', 'ko'));
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await domSettled(page);
      const { values, selected } = await setupPicker(page);
      assertEq(JSON.stringify(values), JSON.stringify(['en']), 'word-source options');
      assertEq(selected, 'en', 'a Hidden saved language falls back to English');
      // I3.3: the interface language is its own setting and is left alone.
      assertEq(await page.evaluate(() => document.documentElement.lang), 'ko', 'interface locale');
      await noComingSoon(page, 'saved Korean');
    } finally { await ctx.close(); }
  });

  await suite.test('release-langs: a device in Spanish still opens on English play', async () => {
    const init = () => {
      Object.defineProperty(navigator, 'language', { configurable: true, get: () => 'es-ES' });
      Object.defineProperty(navigator, 'languages', { configurable: true, get: () => ['es-ES'] });
    };
    const { ctx, page } = await openApp(browser, base, { init: `(${init})()` });
    try {
      const { values, selected } = await setupPicker(page);
      assertEq(JSON.stringify(values), JSON.stringify(['en']), 'word-source options');
      assertEq(selected, 'en', 'a Hidden device language is not the study language');
      await noComingSoon(page, 'Spanish device');
    } finally { await ctx.close(); }
  });

  await suite.test('release-langs: the My Words "Speak in" picker lists English only', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await openMyWords(page);
      await page.waitForSelector('#listsScreen.show', { timeout: 4000 });
      await page.click('[data-l-add]');
      await page.waitForSelector('#importScrim.show', { timeout: 4000 });
      assertEq(JSON.stringify(await optionValues(page, '#importLang')), JSON.stringify(['en-US']), 'Speak in options');
    } finally { await ctx.close(); }
  });

  await suite.test('release-langs: the Translate language picker lists English only', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await page.evaluate(() => {
        localStorage.setItem('spell_flag_translate', 'on');
        for (const l of ['es', 'ko']) window.__spelltest.translateAudit(l, true);
      });
      await page.evaluate(() => document.getElementById('trOpenBtn').click());
      await page.waitForSelector('#trScreen.show', { timeout: 5000 });
      await domSettled(page);
      const pick = await page.$('[data-tr-pick]');
      if (pick) {
        await pick.click();
        await domSettled(page);
      }
      const langs = await page.$$eval('#trPickerList [data-tr-lang]', (bs) => bs.map((b) => b.dataset.trLang));
      for (const l of langs) assertEq(l, 'en', 'a Translate picker row');
      const text = await page.$eval('#trScreen', (e) => e.innerText);
      for (const name of ['Español', '한국어']) assert(!text.includes(name), `Translate names a Hidden language: ${name}`);
    } finally { await ctx.close(); }
  });

  await suite.test('release-langs: the Online entry form lists English only', async () => {
    const init = () => localStorage.setItem('spell_flag_online_enabled', 'on');
    const { ctx, page } = await openApp(browser, base, { init: `(${init})()` });
    try {
      await page.evaluate(() => document.getElementById('vsBtn').click());
      await domSettled(page);
      await page.click('#vsOnline');
      await page.waitForSelector('#olEntry:not([hidden])', { timeout: 10000 });
      assertEq(JSON.stringify(await optionValues(page, '#olLang')), JSON.stringify(['en']), 'Online language options');
    } finally { await ctx.close(); }
  });
}
