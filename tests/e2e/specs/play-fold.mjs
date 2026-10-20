// play-fold.spec — CC-VISUAL-ECLIPSE AT-4 (as rewritten on the signing sheet,
// 2026-10-20). Play screens never open the iOS keyboard; the app's own keyboard
// is always on screen. So AT-4 is about the fold: on every supported size, the
// spellbox, the in-app keyboard, the Check button and the result line must all
// be fully visible WITHOUT scrolling, before Check and after it.
//
// Sizes: iPhone SE 1st gen (320x568), SE 2nd/3rd gen (375x667, also the AT-19
// device), a 6.1" phone (390x844), the largest Pro Max (440x956), and iPad 13"
// portrait (1032x1376). Every tap is dispatched from page.evaluate, because a
// Playwright click scrolls its target into view and would hide the very
// failure this test exists to catch.
import { openApp, assert } from '../harness.mjs';

const SIZES = [
  [320, 568],
  [375, 667],
  [390, 844],
  [440, 956],
  [1032, 1376],
];

// Kid Mode (Spell Jr / Little Speller) carries taller keys, so it is checked
// at every size as well.
const PROFILES = [
  ['standard', null],
  ['Kid Mode', JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 })],
];

const tap = (page, sel) => page.evaluate((s) => document.querySelector(s).click(), sel);

// Rectangles of the four things AT-4 names, and whether each sits fully inside
// the viewport (with the page still at its top).
const measure = (page) => page.evaluate(() => {
  const r = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const b = el.getBoundingClientRect();
    return { top: Math.round(b.top), bottom: Math.round(b.bottom), h: Math.round(b.height) };
  };
  const rows = [...document.querySelectorAll('#gameKeyboard .kb-row')].filter((e) => e.offsetParent !== null);
  const kbTop = Math.min(...rows.map((e) => e.getBoundingClientRect().top));
  const kbBottom = Math.max(...rows.map((e) => e.getBoundingClientRect().bottom));
  return {
    vh: innerHeight,
    scrollY: Math.round(scrollY),
    spellbox: r('#spellbox'),
    keyboard: { top: Math.round(kbTop), bottom: Math.round(kbBottom) },
    check: r('#checkBtn'),
    feedback: r('#feedback'),
    feedbackText: document.getElementById('feedback').textContent.trim(),
  };
});

function inView(m, names) {
  const bad = [];
  for (const n of names) {
    const b = m[n];
    if (!b) { bad.push(`${n} missing`); continue; }
    if (b.top < 0 || b.bottom > m.vh) bad.push(`${n} ${b.top}..${b.bottom} outside 0..${m.vh}`);
  }
  if (m.scrollY !== 0) bad.push(`page scrolled to ${m.scrollY}`);
  return bad;
}

export async function run(browser, base, suite) {
  for (const [profile, age] of PROFILES) for (const [w, h] of SIZES) {
    await suite.test(`AT-4 ${profile} ${w}x${h}: spellbox, keyboard, Check and the result line fit without scrolling`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: w, height: h }, ...(age ? { age } : {}) });
      if (age) assert(await page.evaluate(() => document.body.classList.contains('kid')), 'the kid verdict did not put the app in Kid Mode');
      try {
        await tap(page, '#orbWrap');
        await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
        // A deliberate miss, so the result line carries the revealed spelling.
        const wrong = await page.evaluate(() => {
          const w = window.__spelltest.currentWord().toLowerCase();
          return w[0] === 'q' ? 'z' : 'q';
        });
        await tap(page, `#gameKeyboard .kb-key[data-k="${wrong}"]`);
        const before = await measure(page);
        const b = inView(before, ['spellbox', 'keyboard', 'check']);
        assert(!b.length, `before Check: ${b.join('; ')}`);
        await tap(page, '#checkBtn');
        await page.waitForFunction(() => document.getElementById('feedback').textContent.trim().length > 0, null, { timeout: 5000 });
        const after = await measure(page);
        const a = inView(after, ['spellbox', 'keyboard', 'check', 'feedback']);
        assert(!a.length, `after Check ("${after.feedbackText.slice(0, 40)}"): ${a.join('; ')}`);
      } finally { await ctx.close(); }
    });
  }
}
