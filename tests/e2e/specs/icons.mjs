// icons.spec — CC-VISUAL-ECLIPSE Phase D F8 (AT-20 at runtime), behind visual.icons.
//
// Every UI icon is an icon slot: the drawn icon (a <use> of the inline sprite)
// and the emoji it replaced. Off, the emoji show and the screen is as it was;
// on, the drawn icons show and no emoji icon is visible anywhere a player looks
// (the play screen and the drawer).
import { openApp, assert } from '../harness.mjs';

const ON = () => localStorage.setItem('spell_flag_visual_icons', 'on');
const EP = '\\p{Extended_Pictographic}';

const slots = (page) => page.evaluate(() => {
  const all = [...document.querySelectorAll('.icon-slot')].filter((s) => s.getClientRects().length);
  return {
    n: all.length,
    svgShown: all.filter((s) => getComputedStyle(s.querySelector('svg')).display !== 'none').length,
    emojiShown: all.filter((s) => getComputedStyle(s.querySelector('.ic-emoji')).display !== 'none').length,
  };
});

async function openDrawer(page) {
  await page.click('.burger');
  await page.waitForSelector('.nav-row', { timeout: 5000 });
}

export async function run(browser, base, suite) {
  await suite.test('F8 off (default): every icon slot shows its emoji, no drawn icon', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDrawer(page);
      const r = await slots(page);
      assert(r.n >= 10, `only ${r.n} visible icon slots`);
      assert(r.emojiShown === r.n && r.svgShown === 0, `off: ${JSON.stringify(r)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('F8 on: every icon slot draws its icon, every drawer row has a real symbol', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      await openDrawer(page);
      const r = await slots(page);
      assert(r.svgShown === r.n && r.emojiShown === 0, `on: ${JSON.stringify(r)}`);
      const rows = await page.evaluate(() => [...document.querySelectorAll('.nav-row')].map((row) => {
        const use = row.querySelector('.nav-ico use');
        const ref = use && use.getAttribute('href');
        const box = use && row.querySelector('.nav-ico svg').getBoundingClientRect();
        return { id: row.id, ref, symbol: !!(ref && document.querySelector(ref)), w: box ? box.width : 0 };
      }));
      const bad = rows.filter((x) => !x.symbol || x.w < 10);
      assert(rows.length >= 8 && !bad.length, `drawer rows without a drawn icon: ${JSON.stringify(bad)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('AT-20: with icons on, no emoji icon is visible on the play screen or in the drawer', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      const scan = () => page.evaluate((ep) => {
        const re = new RegExp(ep, 'u');
        const hits = [];
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
        for (let n = walker.nextNode(); n; n = walker.nextNode()) {
          if (!re.test(n.textContent)) continue;
          const el = n.parentElement;
          if (!el || !el.getClientRects().length || getComputedStyle(el).visibility === 'hidden') continue;
          // Exempt as in scripts/icons-check.mjs: the dev door, typography, achievement badges (content).
          if (el.closest('#devMenu, #trTranslitToggle, .ach-badge, [hidden]')) continue;
          hits.push(`${el.id || el.className}: ${n.textContent.trim().slice(0, 30)}`);
        }
        return hits;
      }, EP);
      const play = await scan();
      assert(!play.length, `emoji icons on the play screen: ${play.join(' | ')}`);
      await openDrawer(page);
      const drawer = await scan();
      assert(!drawer.length, `emoji icons in the drawer: ${drawer.join(' | ')}`);
    } finally { await ctx.close(); }
  });
}
