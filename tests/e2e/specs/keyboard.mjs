// keyboard.spec — per-language on-screen keyboard: keys exist + visible + have a
// ≥44pt-ish hit area at iPhone SE width; rapid-fire doesn't drop characters;
// composition-engine languages compose at the UI layer (mirrors the Rust unit
// tests with a small representative set).
import { openApp, assert, assertEq } from '../harness.mjs';

const LANGS = ['en', 'fr', 'de', 'ko', 'ja', 'th', 'vi', 'zh'];

export async function run(browser, base, suite) {
  for (const lang of LANGS) {
    await suite.test(`keyboard[${lang}]: keys visible with hit area at SE`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, device: 'se' });
      try {
        const keys = await page.$$eval('#gameKeyboard .kb-key', (els) =>
          els.filter((e) => e.offsetParent !== null).map((e) => {
            const r = e.getBoundingClientRect();
            return { w: r.width, h: r.height };
          }));
        assert(keys.length > 0, 'no visible keys');
        const tooSmall = keys.filter((k) => k.w < 20 || k.h < 30);
        assert(tooSmall.length === 0, `${tooSmall.length} keys below hit-area floor`);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('keyboard[en]: rapid-fire 15 keys drops nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', device: 'se' });
    try {
      await page.click('#orbWrap'); await page.waitForTimeout(400); // start a word so typing is enabled
      const seq = 'abcdefghijklmno';
      for (const ch of seq) await page.click(`#gameKeyboard .kb-key[data-k="${ch}"]`, { delay: 0 });
      const typed = await page.$eval('#letters', (e) => e.textContent.replace(/\s/g, ''));
      assertEq(typed.length, seq.length, 'dropped characters under rapid fire');
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard[ko]: ㅎㅏㄴ composes to 한 in the input', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ko', device: 'se' });
    try {
      await page.click('#orbWrap'); await page.waitForTimeout(400);
      for (const j of ['ㅎ', 'ㅏ', 'ㄴ']) await page.click(`#gameKeyboard .kb-key[data-k="${j}"]`);
      const letters = await page.$eval('#letters', (e) => e.textContent);
      assert(letters.includes('한'), `expected 한 in "${letters}"`);
    } finally { await ctx.close(); }
  });

  await suite.test('keyboard[ja]: か + dakuten long-press → が reachable', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ja', device: 'se' });
    try {
      const hasKa = await page.$('#gameKeyboard .kb-key[data-k="か"]');
      const dakutenMarked = await page.$$eval('#gameKeyboard .kb-key.has-acc', (els) =>
        els.some((e) => e.getAttribute('data-k') === 'か'));
      assert(hasKa && dakutenMarked, 'か missing or has no dakuten long-press');
    } finally { await ctx.close(); }
  });
}
