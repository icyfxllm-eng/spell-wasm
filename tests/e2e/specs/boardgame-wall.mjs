// boardgame-wall.spec -- CC-BOARD-GAME A12 / I13, the crawl half. Runs ONLY
// against the site build: the rendered site has no Board Game DOM, no tile, no
// styles and none of its strings in any of the fifteen locales. The byte scan
// (scripts/web-boardgame-wall-scan.mjs) proves the bundle; this proves what the
// browser actually shows. Strings are generated from the locale tables.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { openApp, assert, assertEq, IS_WEB_BUILD, domSettled } from '../harness.mjs';

const LOCALES = join(process.cwd(), 'src', 'i18n', 'locales');

function strings() {
  const out = [];
  for (const f of readdirSync(LOCALES).filter((n) => n.endsWith('.json'))) {
    const t = JSON.parse(readFileSync(join(LOCALES, f), 'utf8'));
    for (const [k, v] of Object.entries(t)) {
      if (!k.startsWith('bg.')) continue;
      const s = String(v).replace(/\{\w+\}/g, '').trim();
      if (s.length >= 10) out.push({ lang: f.replace('.json', ''), key: k, s });
    }
  }
  return out;
}

export async function run(browser, base, suite) {
  if (!IS_WEB_BUILD) return;
  const STR = strings();

  await suite.test('boardgame_wall: no screen, launcher, styles or strings on the site', async () => {
    assert(STR.length > 50, `only ${STR.length} generated strings`);
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      for (const lang of [...new Set(STR.map((x) => x.lang))]) {
        await page.evaluate((c) => { localStorage.setItem('spellgame.locale', c); location.reload(); }, lang);
        await page.waitForLoadState('load');
        await domSettled(page);
        const r = await page.evaluate((needles) => {
          const html = document.documentElement.outerHTML;
          const text = document.body.innerText;
          return {
            dom: ['bgScreen', 'bgOpenBtn', 'bgKeys', 'bgOrb'].filter((id) => document.getElementById(id)),
            css: /\.bg-screen|bg-key|BOARDGAME/.test(html),
            strings: needles.filter((n) => text.includes(n.s)).map((n) => `${n.lang}:${n.key}`),
          };
        }, STR);
        assertEq(r.dom.length, 0, `${lang}: Board Game DOM present ${r.dom}`);
        assert(!r.css, `${lang}: Board Game markup or styles in the page`);
        assertEq(r.strings.length, 0, `${lang}: Board Game strings rendered ${r.strings.slice(0, 3)}`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('boardgame_wall: the registry on the site has no boardgame entry', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Even with the dev flag set, there is nothing to turn on.
      await page.evaluate(() => { localStorage.setItem('spell_flag_boardgame', 'on'); location.reload(); });
      await page.waitForLoadState('load');
      await domSettled(page);
      const hit = await page.evaluate(() => !!document.getElementById('bgScreen') || /boardgame/i.test(document.documentElement.outerHTML));
      assert(!hit, 'the flag revived a Board Game on the site');
    } finally { await ctx.close(); }
  });
}
