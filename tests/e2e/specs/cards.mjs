// cards.spec — CC-VISUAL-ECLIPSE Phase D F11 (AT-26), behind visual.cards.
//
// Every card mode in three palettes, plus an Arabic (right-to-left) card, is
// rendered through the real renderer (the renderCard seam): twice, to prove a
// seed always draws the same card, at 1080x1350, and against a committed
// golden in tests/e2e/visual/cards/ (under 0.5% of pixels may differ, for font
// hinting). A missing golden is captured on first run; CARD_CAPTURE=1 recaptures.
// AT-27 (no name, avatar or free text) is result_card::tests.
import { openApp, assert } from '../harness.mjs';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'visual', 'cards');
const CAPTURE = process.env.CARD_CAPTURE === '1';

const MODES = {
  chain: { mode: 'Spell It', hero: '12', heroLabel: 'word chain', line: 'best: 15' },
  daily: { mode: 'Daily Challenge', hero: '9/10', heroLabel: 'correct', line: '7-day streak' },
  practice: { mode: 'Practice', hero: '20', heroLabel: 'first words in Spanish' },
  boardgame: { mode: 'Board Game', hero: '#1', heroLabel: 'place' },
  randospell: { mode: 'Randospell', hero: '340', heroLabel: 'points' },
};
const PALETTES = {
  default: {},
  teal: { bg: '#151d2e', orb: '#46d6b8', glow: '#46d6b8', accent: '#46d6b8' },
  rose: { bg: '#241318', orb: '#ff6b7a', glow: '#ff7a9a', accent: '#ff7a9a' },
};

async function fonts(page) {
  await page.evaluate(async () => {
    await document.fonts.load("800 260px 'Bricolage Grotesque'", '0123456789#/');
    await document.fonts.load("700 50px 'Bricolage Grotesque'", 'Spell It');
    await document.fonts.load("700 50px 'Noto Naskh Arabic'", 'سلسلة كلمات');
    await document.fonts.ready;
  });
}

const render = (page, spec) => page.evaluate((j) => window.__spelltest.renderCard(j), JSON.stringify(spec));

/** Pixels that differ, as a share of the card, between two data URLs. */
const diff = (page, a, b) => page.evaluate(async ([a, b]) => {
  const load = (src) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = src; });
  const [ia, ib] = await Promise.all([load(a), load(b)]);
  const px = (img) => { const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const x = c.getContext('2d'); x.drawImage(img, 0, 0); return x.getImageData(0, 0, img.width, img.height).data; };
  if (ia.width !== ib.width || ia.height !== ib.height) return { size: [ia.width, ia.height, ib.width, ib.height], share: 1 };
  const da = px(ia), db = px(ib);
  let n = 0;
  for (let i = 0; i < da.length; i += 4) if (Math.abs(da[i] - db[i]) + Math.abs(da[i + 1] - db[i + 1]) + Math.abs(da[i + 2] - db[i + 2]) > 48) n++;
  return { size: [ia.width, ia.height], share: n / (da.length / 4) };
}, [a, b]);

export async function run(browser, base, suite) {
  await suite.test('AT-26: every card mode x palette, and an Arabic card, is deterministic, 1080x1350 and matches its golden', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: () => localStorage.setItem('spell_flag_visual_cards', 'on') });
    try {
      await fonts(page);
      const cases = [];
      for (const [m, s] of Object.entries(MODES)) for (const [p, pal] of Object.entries(PALETTES)) cases.push([`${m}-${p}`, { ...s, lang: 'en', tagline: 'hear it · spell it', palette: pal, seed: 7 }]);
      cases.push(['chain-ar', { mode: 'تهجَّ', hero: '12', heroLabel: 'سلسلة كلمات', line: 'الأفضل: 15', lang: 'ar', tagline: 'استمع · تهجَّ', palette: {}, seed: 7 }]);
      const problems = [];
      let captured = 0;
      for (const [name, spec] of cases) {
        const a = await render(page, spec);
        const b = await render(page, spec);
        if (!a.startsWith('data:image/png')) { problems.push(`${name}: no image`); continue; }
        if (a !== b) problems.push(`${name}: two renders of one seed differ`);
        const self = await diff(page, a, a);
        if (self.size[0] !== 1080 || self.size[1] !== 1350) problems.push(`${name}: ${self.size.join('x')}`);
        const file = join(DIR, `${name}.png`);
        if (CAPTURE || !existsSync(file)) {
          writeFileSync(file, Buffer.from(a.split(',')[1], 'base64'));
          captured++;
          continue;
        }
        const golden = `data:image/png;base64,${readFileSync(file).toString('base64')}`;
        const d = await diff(page, a, golden);
        if (d.share > 0.005) problems.push(`${name}: ${(d.share * 100).toFixed(2)}% of pixels differ from the golden`);
      }
      if (captured) process.stdout.write(`    (captured ${captured} golden card(s) in tests/e2e/visual/cards)\n`);
      assert(!problems.length, problems.join('\n  '));
    } finally { await ctx.close(); }
  });
}
