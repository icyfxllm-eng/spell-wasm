// birthday-months.spec — CC-VISUAL-ECLIPSE Phase B carry-ins B-C1 and B-C2.
//
// B-C2: the birthday prompt's months are in the player's interface language
// (they were a hard-coded English list in every language).
// B-C1: every month name fits its picker on the smallest phones (equal thirds
// clipped 7 of 12 English months on an iPhone SE).
// And the stored value is still the month NUMBER, so no age verdict can move.
import { assert } from '../harness.mjs';

const LOCALES = ['en', 'es', 'fr', 'de', 'pt', 'pl', 'vi', 'ko', 'ja', 'fil', 'zh', 'ru', 'ar', 'sw', 'hi'];
const SIZES = [[320, 568], [375, 667]];

async function openPrompt(browser, base, locale, [w, h]) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 2, isMobile: true });
  await ctx.addInitScript((l) => localStorage.setItem('spellgame.locale', l), locale);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  await ctx.route('**/telemetry/**', (r) => r.fulfill({ status: 404, body: '' }));
  const page = await ctx.newPage();
  await page.goto(base);
  await page.waitForFunction(() => !!window.__spelltest && document.getElementById('ageScrim').classList.contains('show'), null, { timeout: 30000 });
  await page.evaluate(() => document.fonts.ready);
  return { ctx, page };
}

export async function run(browser, base, suite) {
  await suite.test('B-C2: the months are in the interface language, and the value is the month number', async () => {
    const english = [];
    for (const locale of ['en', 'es', 'ru', 'ja']) {
      const { ctx, page } = await openPrompt(browser, base, locale, [375, 667]);
      try {
        const r = await page.evaluate(() => {
          const s = document.getElementById('ageMonth');
          const names = [...s.options].map((o) => o.text);
          const values = [...s.options].map((o) => o.value);
          s.value = '9';
          return { names, values, nine: s.value };
        });
        assert(r.names.length === 12, `${locale}: ${r.names.length} months`);
        assert(r.values.join(',') === '1,2,3,4,5,6,7,8,9,10,11,12', `${locale}: values are not 1..12 (${r.values})`);
        assert(r.nine === '9', `${locale}: September did not read back as 9`);
        if (locale === 'en') english.push(...r.names);
        else assert(r.names[0] !== english[0], `${locale}: January is still the English "${r.names[0]}"`);
      } finally { await ctx.close(); }
    }
  });

  for (const size of SIZES) {
    await suite.test(`B-C1: every month fits its picker at ${size[0]}x${size[1]} in all 15 languages`, async () => {
      const clipped = [];
      for (const locale of LOCALES) {
        const { ctx, page } = await openPrompt(browser, base, locale, size);
        try {
          const r = await page.evaluate(() => {
            const s = document.getElementById('ageMonth');
            const cs = getComputedStyle(s);
            const c = document.createElement('canvas').getContext('2d');
            c.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
            const room = s.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
            const row = s.parentElement.getBoundingClientRect();
            const modal = s.closest('.modal').getBoundingClientRect();
            return {
              room,
              over: [...s.options].map((o) => [o.text, c.measureText(o.text).width]).filter(([, w]) => w > room).map(([t]) => t),
              rowFits: row.right <= modal.right + 0.5,
            };
          });
          if (r.over.length) clipped.push(`${locale}: ${r.over.join(', ')} (room ${Math.round(r.room)}px)`);
          assert(r.rowFits, `${locale}: the date row runs past the prompt`);
        } finally { await ctx.close(); }
      }
      assert(!clipped.length, `clipped month names:\n  ${clipped.join('\n  ')}`);
    });
  }
}
