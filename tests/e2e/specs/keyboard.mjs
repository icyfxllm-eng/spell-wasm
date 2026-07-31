// keyboard.spec — per-language on-screen keyboard: keys exist + visible + have a
// ≥44pt-ish hit area at iPhone SE width; rapid-fire doesn't drop characters;
// composition-engine languages compose at the UI layer (mirrors the Rust unit
// tests with a small representative set).
import { openApp, assert, assertEq } from '../harness.mjs';

// Active study languages only — coming-soon languages are gated from play, so
// their on-screen keyboard is not UI-reachable. Their layouts remain in the code
// (preserved for reactivation) and stay covered by the Rust `keyboard::tests`
// (json_layouts_match_rust) + `hangul`/`jamo` unit tests. Only English is active
// now, so it is the only keyboard reachable through the play UI.
const LANGS = ['en'];

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

  // Pinned keyboard metrics at every phone class. Build 106 shipped a
  // keyboard whose keys were 13.8px wide instead of 28.5px: #stage is a
  // column flex with align-items:center, and the #kbHome wrapper added by the
  // F7 keyboard-borrow work had no width, so it shrink-wrapped its content
  // and .game-kb{width:100%} resolved against the shrunken wrapper. Nothing
  // was persisted -- it was in the build, which is why relaunching never
  // helped. These assertions pin the geometry so no future wrapper, layout or
  // language can quietly collapse it again.
  const CLASSES = [[320, 568], [375, 667], [390, 844], [428, 926]];
  for (const [width, height] of CLASSES) {
    await suite.test(`keyboard[en]: metrics hold at ${width}x${height}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width, height } });
      try {
        const m = await page.evaluate(() => {
          const kb = document.getElementById('gameKeyboard');
          const home = kb.parentElement;
          const keys = [...document.querySelectorAll('#kbLetters .kb-key')]
            .filter((e) => e.offsetParent !== null)
            .map((e) => e.getBoundingClientRect().width);
          const rows = [...document.querySelectorAll('#kbLetters .kb-row')]
            .map((r) => ({ scroll: r.scrollWidth, client: r.clientWidth }));
          return {
            kbW: kb.getBoundingClientRect().width,
            homeW: home.getBoundingClientRect().width,
            stageW: document.getElementById('stage').getBoundingClientRect().width,
            minKey: Math.min(...keys), maxKey: Math.max(...keys), n: keys.length,
            overflowing: rows.filter((r) => r.scroll > r.client + 1).length,
          };
        });
        assert(m.n > 0, 'no visible keys');
        // The wrapper must not shrink-wrap: keyboard fills the stage.
        assertEq(Math.round(m.homeW), Math.round(m.stageW), 'keyboard wrapper is not stage-width');
        assertEq(Math.round(m.kbW), Math.round(m.homeW), 'keyboard does not fill its wrapper');
        // Legibility floor lives here, not in CSS -- a CSS floor would clamp a
        // collapsed key back up and overflow the row, hiding the collapse.
        assert(m.minKey >= 20, `smallest key ${m.minKey.toFixed(1)}px is below the 20px floor`);
        assert(m.maxKey <= 46.5, `largest key ${m.maxKey.toFixed(1)}px exceeds the 46px cap`);
        // Uniformity: one --kb-key for every letter, whatever the row length.
        assert(m.maxKey - m.minKey < 0.5, `keys are not uniform (${m.minKey} vs ${m.maxKey})`);
        assertEq(m.overflowing, 0, 'a key row overflows its container');
      } finally { await ctx.close(); }
    });
  }

  // Korean Hangul composition (ㅎㅏㄴ→한) and Japanese kana + dakuten long-press
  // were UI-driven here, but ko/ja are now coming-soon (gated from play), so their
  // keyboards aren't reachable via the UI. That behavior is preserved and covered
  // at the unit level: `hangul` (composition), `jamo` (grading), and
  // `keyboard::tests::json_layouts_match_rust` (kana/dakuten layout).
}
