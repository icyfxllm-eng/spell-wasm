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

  // ---- CC-RTL F1: the study language reaches the play surface ----

  await suite.test('rtl F1: play surface carries the WORD\'s language + dir from the registry', async () => {
    // CC-RTL F1/D3. The play surfaces must carry the language of the word being
    // SPELLED, with `dir` read from the registry and nowhere else.
    //
    // NOTE, because it corrects an assumption worth writing down: in this app the
    // language picker ALSO switches the UI chrome, so <html lang> and the picker
    // agree and cannot be used to prove the plumbing. The real divergence is
    // `cur_lang` — Misses replays each word in ITS OWN language and Daily uses its
    // own locale, so the word on screen can be a different language from the
    // picker. That is why reflect_play_direction reads `cur_lang`, not `lang`.
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      await page.click('#orbWrap').catch(() => {});
      await page.waitForTimeout(400);
      const got = await page.evaluate(() => {
        const q = (id) => {
          const e = document.getElementById(id);
          return { lang: e.getAttribute('lang'), dir: e.getAttribute('dir') };
        };
        return { picker: document.getElementById('langSel').value, letters: q('letters'), feedback: q('feedback'), meaning: q('meaning') };
      });
      for (const id of ['letters', 'feedback', 'meaning']) {
        assert(got[id].lang === got.picker, `#${id} lang=${got[id].lang}, expected the word's language (${got.picker})`);
        assert(got[id].dir === 'ltr', `#${id} dir=${got[id].dir}, expected ltr for Spanish`);
      }
    } finally { await ctx.close(); }
  });
}
