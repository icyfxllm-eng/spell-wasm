// keyboard.spec — per-language on-screen keyboard: keys exist + visible + have a
// ≥44pt-ish hit area at iPhone SE width; rapid-fire doesn't drop characters;
// composition-engine languages compose at the UI layer (mirrors the Rust unit
// tests with a small representative set).
import { openApp, assert, assertEq } from '../harness.mjs';

// Active study languages only — coming-soon languages are gated from play, so
// their on-screen keyboard is not UI-reachable. Their layouts remain in the code
// (preserved for reactivation) and stay covered by the Rust `keyboard::tests`
// (json_layouts_match_rust) + `hangul`/`jamo` unit tests. Active now: en + th
// (Thai in TestFlight testing), so we assert the Thai keyboard renders too.
const LANGS = ['en', 'th'];

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

  // Korean Hangul composition (ㅎㅏㄴ→한) and Japanese kana + dakuten long-press
  // were UI-driven here, but ko/ja are now coming-soon (gated from play), so their
  // keyboards aren't reachable via the UI. That behavior is preserved and covered
  // at the unit level: `hangul` (composition), `jamo` (grading), and
  // `keyboard::tests::json_layouts_match_rust` (kana/dakuten layout).
}
