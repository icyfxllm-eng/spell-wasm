// menu.spec — the language selector shows each language's endonym in its own
// script, and selecting a language switches the whole UI to match (menu
// integrity: selector is generated from the language registry, no drift).
import { openApp, assert } from '../harness.mjs';

const ENDONYMS = {
  en: 'English', es: 'Español', fr: 'Français', de: 'Deutsch', ko: '한국어',
  ja: '日本語', zh: '中文', th: 'ไทย', vi: 'Tiếng Việt', fil: 'Filipino',
};

export async function run(browser, base, suite) {
  await suite.test('menu: every language shows its own endonym', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const opts = await page.$$eval('#langSel option', (els) => els.map((e) => [e.value, e.textContent]));
      const map = Object.fromEntries(opts);
      for (const [code, name] of Object.entries(ENDONYMS)) {
        assert(map[code] === name, `${code}: expected "${name}", got "${map[code]}"`);
      }
    } finally { await ctx.close(); }
  });

  for (const [lang, tag] of [['ko', '철자'], ['ja', 'つづ'], ['zh', '拼'], ['th', 'สะกด'], ['fil', 'baybayin']]) {
    await suite.test(`menu: selecting ${lang} switches UI chrome`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        const brand = await page.$eval('.tag', (e) => e.textContent);
        assert(brand.includes(tag), `UI not switched to ${lang}: tag="${brand}"`);
        const htmlLang = await page.$eval('html', (e) => e.getAttribute('lang'));
        assert(htmlLang === lang, `<html lang> is "${htmlLang}", expected "${lang}"`);
      } finally { await ctx.close(); }
    });
  }
}
