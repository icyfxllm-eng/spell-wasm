// hotfix-overlap.spec — CC-ECLIPSE-HOTFIX I4: no two visible text or
// interactive elements' boxes intersect, on every registry screen and the home
// screen's states, at 375 and 430 pt, at the default and the XL (Big text)
// size. Colour does not move a box, so one theme (Eclipse) is enough.
import { assert } from '../harness.mjs';
import { openThemed, drawerRows, openRow, setBigText, overlaps } from '../hotfix-lib.mjs';

const SIZES = [['375', 'se'], ['430', 'large']];

export async function run(browser, base, suite) {
  for (const [w, device] of SIZES) {
    for (const big of [false, true]) {
      const name = `${w} pt, ${big ? 'XL' : 'default'} text`;
      await suite.test(`I4 ${name}: no text or control overlaps another on any registry screen`, async () => {
        const failures = [];
        let rows;
        {
          const { ctx, page } = await openThemed(browser, base, 'eclipse', { device });
          try { rows = await drawerRows(page); } finally { await ctx.close(); }
        }
        assert(rows.length >= 20, `only ${rows.length} drawer rows`);
        for (const row of ['home', 'lived-in home', ...rows]) {
          const { ctx, page } = await openThemed(browser, base, 'eclipse', { device, lived: row === 'lived-in home' });
          try {
            await setBigText(page, big);
            if (!row.endsWith('home')) await openRow(page, row);
            for (const o of await overlaps(page)) failures.push(`${row}: ${o}`);
          } catch (e) {
            failures.push(`${row}: could not be checked (${e.message.split('\n')[0]})`);
          } finally { await ctx.close(); }
        }
        assert(!failures.length, `${failures.length} overlap(s) at ${name}:\n    ${failures.slice(0, 60).join('\n    ')}`);
      });

      await suite.test(`headers ${name}: Spell Picture's title, Gallery and Surprise share a row (Surprise unwrapped); Board Game's title and Exit show`, async () => {
        const bad = [];
        {
          const { ctx, page } = await openThemed(browser, base, 'eclipse', { device });
          try {
            await setBigText(page, big);
            await openRow(page, 'word_picture');
            const h = await page.evaluate(() => {
              const box = (id) => document.getElementById(id).getBoundingClientRect();
              const label = [...document.querySelectorAll('#wpSurprise [data-i18n]')].find((e) => e.getClientRects().length);
              const t = document.querySelector('#wpPickerExit').parentElement.querySelector('.wp-title').getBoundingClientRect();
              return { t: [t.top, t.bottom], g: [box('wpGalleryBtn').top, box('wpGalleryBtn').bottom], s: [box('wpSurprise').top, box('wpSurprise').bottom],
                lines: label ? label.getClientRects().length : 0, right: box('wpSurprise').right, vw: innerWidth };
            });
            const meet = (a, b) => Math.min(a[1], b[1]) - Math.max(a[0], b[0]) > 4;
            if (!(meet(h.t, h.g) && meet(h.g, h.s) && meet(h.t, h.s))) bad.push(`Spell Picture header is not one row (${JSON.stringify(h)})`);
            if (h.lines !== 1) bad.push(`Surprise wraps onto ${h.lines} lines`);
            if (h.right > h.vw + 0.5) bad.push(`Surprise runs off the screen (${h.right} > ${h.vw})`);
          } finally { await ctx.close(); }
        }
        {
          const { ctx, page } = await openThemed(browser, base, 'eclipse', { device });
          try {
            await setBigText(page, big);
            await openRow(page, 'boardgame');
            for (const id of ['bgTitle', 'bgSetupExit']) {
              const ok = await page.evaluate((id) => {
                const el = document.getElementById(id), r = el.getBoundingClientRect();
                if (r.top < 0 || r.bottom > innerHeight || r.left < 0 || r.right > innerWidth) return false;
                const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
                return !!hit && (hit === el || el.contains(hit));
              }, id);
              if (!ok) bad.push(`#${id} is not visible and unobstructed`);
            }
          } finally { await ctx.close(); }
        }
        assert(!bad.length, bad.join('; '));
      });
    }
  }
}
