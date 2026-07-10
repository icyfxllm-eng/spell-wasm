// gameplay.spec — per language: start a word, read the expected answer from the
// __spelltest seam, type it on the on-screen keyboard, assert acceptance; then
// type a wrong answer and assert rejection UX. Types via real key clicks (the
// anti-dictation keyboard is under test), never input.fill.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

// Latin languages where the answer string is directly clickable key-by-key.
const LANGS = ['en', 'es', 'fr', 'de', 'it', 'nl', 'pl', 'sv', 'nb', 'tr', 'vi', 'fil'];

export async function run(browser, base, suite) {
  for (const lang of LANGS) {
    await suite.test(`gameplay[${lang}]: correct answer is accepted`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await page.click('#orbWrap'); await page.waitForTimeout(500);
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        assert(word && word.length > 0, 'no current word from seam');
        // Only run the click-type path for words whose chars are base keys
        // (accented forms need long-press; covered by keyboard.spec).
        const base_only = [...word.toLowerCase()].every((c) => /[a-zñ'-]/.test(c));
        if (!base_only) return; // skip accented words here
        await typeOnKeyboard(page, word.toLowerCase());
        await page.click('#checkBtn');
        await page.waitForTimeout(400);
        const cls = await page.$eval('#feedback', (e) => e.className);
        assert(cls.includes('good'), `expected good feedback, got "${cls}" for ${word}`);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('gameplay[en]: wrong answer is rejected', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#orbWrap'); await page.waitForTimeout(500);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      const wrong = word === 'zzzz' ? 'xxxx' : 'zzzz';
      await typeOnKeyboard(page, wrong);
      await page.click('#checkBtn');
      await page.waitForTimeout(400);
      const cls = await page.$eval('#feedback', (e) => e.className);
      // wrong shows a retry ("Not quite") or a bad reveal — never "good".
      assert(!cls.includes('good'), 'wrong answer was accepted');
    } finally { await ctx.close(); }
  });
}
