// menu.spec — the language selector shows each language's endonym in its own
// script, and selecting a language switches the whole UI to match (menu
// integrity: selector is generated from the language registry, no drift).
import { openApp, assert } from '../harness.mjs';

// Spot-checks for STUDY languages whose endonym would silently regress to a
// Latin name. Not the full registry — `consts.rs` owns which languages exist, and
// duplicating that list here is what made this test rot: `th: 'ไทย'` sat in it
// after 5fc69ff cut Thai from BUILTIN_LANGS, and the suite went red unnoticed
// because E2E is not wired into CI.
//
// NOTE: a study language is not a UI locale. Thai remains a UI locale (th.json
// ships), which is why `selecting th switches UI chrome` below still passes — it
// is the #langSel *study* list Thai left.
const ENDONYMS = {
  en: 'English', es: 'Español', fr: 'Français', de: 'Deutsch', ko: '한국어',
  ja: '日本語', zh: '中文', vi: 'Tiếng Việt', fil: 'Filipino',
};

export async function run(browser, base, suite) {
  await suite.test('menu: every language shows its own endonym', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const opts = await page.$$eval('#langSel option', (els) => els.map((e) => [e.value, e.textContent.trim()]));
      const map = Object.fromEntries(opts);

      // The INVARIANT, derived from whatever the registry rendered: every option
      // shows a real name, never a bare code. This is what the spec's own header
      // promises ("generated from the language registry, no drift") and it cannot
      // go stale when a language is cut or added — unlike the map above.
      assert(opts.length > 0, 'the language selector rendered no options at all');
      for (const [code, label] of opts) {
        assert(label && label !== code, `${code}: option renders "${label}" — expected an endonym, not the code`);
      }

      // Then the spot-checks, for the codes actually in the registry. A cut
      // language is consts.rs's business (registry_tests::only_en_active pins
      // that); this test's business is that what IS listed reads correctly.
      for (const [code, name] of Object.entries(ENDONYMS)) {
        if (!(code in map)) continue;
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
