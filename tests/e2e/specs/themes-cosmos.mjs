// themes-cosmos.spec — CC-THEMES-COSMOS Phase A: the Theme Lab and every
// look of the twelve earned themes, on the play screen.
//
// Looks are applied the way Eric will on a device: through the Lab's preview
// (its dev key), which only dev and Lab builds carry (the test build is one).
//
//   AT-T1   the Lab lists all 17; applying each changes the screen; the chip shows
//   AT-T3   applying, clearing and "Play unlock moment" leave every other key byte-identical
//   AT-T4   the Lab clock reaches the Sky band and nothing stored
//   AT-T5   axe color-contrast, every look x standard and Jr
//   AT-T5b  every text box against its WORST rendered background pixel, for the
//           looks drawn over art; a planted label over the bare pavé field fails
//   AT-T11/12/19  during a word nothing behind the play surface animates or
//           repaints; between words Diamond may glint, never under Reduce Motion
//   AT-T13  geometry is identical to Eclipse in every look
//   AT-T16  Appearance lists the Classic five only (Diamond, Great Wall absent)
//   AT-T17  a result card renders identically twice in every look
import AxeBuilder from '@axe-core/playwright';
import { openApp, typeOnKeyboard, domSettled, assert } from '../harness.mjs';
import { decodePng, ratio, PARSE_COLOUR } from '../hotfix-lib.mjs';

const FLAGS = ['visual_type', 'visual_feedback', 'visual_orb', 'visual_icons', 'visual_celebrate', 'visual_cards', 'visual_themes', 'visual_accents'];
const LAB = 'spell_dev_theme_lab';

export const LOOKS = [
  ...['bronze', 'silver', 'gold', 'diamond', 'beach', 'lunar', 'pinkmoon', 'bloodmoon', 'aurora', 'greatwall'].map((theme) => ({ theme })),
  ...['sunrise', 'day', 'sunset', 'night'].map((band) => ({ theme: 'sky', band })),
  ...['mercury', 'venus', 'earth', 'mars', 'jupiter', 'saturn', 'uranus', 'neptune'].map((planet) => ({ theme: 'solar', planet })),
  ...[0, 1, 2, 3, 5, 6, 7].map((phase) => ({ theme: 'lunar', phase })),
];
const nameOf = (l) => [l.theme, l.band, l.planet, l.phase].filter((x) => x !== undefined).join('/');
// The looks drawn over art or a gradient ground: AT-T5b measures their pixels.
const ART = (l) => ['diamond', 'bronze', 'silver', 'gold', 'sky', 'beach', 'pinkmoon', 'aurora', 'greatwall', 'solar'].includes(l.theme);

const seed = (lab) => `(() => {
  for (const f of ${JSON.stringify(FLAGS)}) localStorage.setItem('spell_flag_' + f, 'on');
  ${lab ? `localStorage.setItem('${LAB}', ${JSON.stringify(JSON.stringify(lab))});` : ''}
})()`;

async function openLook(browser, base, lab, device = 'se') {
  const { ctx, page } = await openApp(browser, base, { device, init: seed(lab) });
  await domSettled(page);
  return { ctx, page };
}

const snapshot = (page) => page.evaluate((lab) => {
  const o = {};
  for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k !== lab) o[k] = localStorage.getItem(k); }
  return JSON.stringify(Object.keys(o).sort().map((k) => [k, o[k]]));
}, LAB);

/** Every rendered box outside the theme's own layers, rounded to a half pixel. */
const geometry = (page) => page.evaluate(() => {
  const out = [];
  let i = 0;
  for (const el of document.body.querySelectorAll('*')) {
    i++;
    if (el.closest('#themeArt, #labChip, #themeLab, #themeLabStyle, script, style')) continue;
    const r = el.getBoundingClientRect();
    if (!r.width && !r.height) continue;
    out.push(`${el.id || el.tagName + i}:${Math.round(r.x * 2) / 2},${Math.round(r.y * 2) / 2},${Math.round(r.width * 2) / 2},${Math.round(r.height * 2) / 2}`);
  }
  return out;
});

async function serveWord(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
}

/** AT-T5b: hide all ink, take ONE screenshot, and judge every visible text
 *  box against the worst pixel actually painted under its text runs. */
async function pixelContrast(page) {
  const boxes = await page.evaluate((parse) => {
    const P = eval(parse);
    const out = [];
    for (const el of document.body.querySelectorAll('*')) {
      if (el.closest('#themeArt, #labChip, #themeLab, script, style, [aria-hidden="true"]')) continue;
      if (el.closest(':disabled, [aria-disabled="true"], [inert], .game-kb.locked')) continue;
      const cs = getComputedStyle(el);
      if (cs.visibility === 'hidden' || Number(cs.opacity) === 0) continue;
      // A screen-reader label (the 1px clip behind an icon-only control) is not read by eye.
      const own = el.getBoundingClientRect();
      if (own.width <= 1 || own.height <= 1) continue;
      const rects = [];
      for (const t of el.childNodes) {
        if (t.nodeType !== 3 || !t.textContent.trim()) continue;
        const rg = document.createRange(); rg.selectNodeContents(t);
        for (const q of rg.getClientRects()) if (q.width > 1 && q.bottom > 0 && q.top < innerHeight && q.right > 0 && q.left < innerWidth) rects.push({ l: q.left, t: q.top, r: q.right, b: q.bottom });
      }
      if (!rects.length) continue;
      const mid = rects[0];
      const top = document.elementFromPoint((mid.l + mid.r) / 2, (mid.t + mid.b) / 2);
      if (top && top !== el && !el.contains(top) && !top.contains(el)) continue; // covered: not what the player reads
      const fg = P(cs.color);
      if (!fg || fg[3] === 0) continue;
      const size = parseFloat(cs.fontSize), weight = Number(cs.fontWeight) || 400;
      out.push({ what: (el.id ? '#' + el.id : el.tagName.toLowerCase() + '.' + [...el.classList].join('.')) + ` "${el.textContent.trim().slice(0, 18)}"`, fg, need: size >= 24 || (size >= 18.66 && weight >= 700) ? 3 : 4.5, rects });
    }
    const s = document.createElement('style'); s.id = 'cosmosHide';
    s.textContent = '*{color:transparent!important;-webkit-text-fill-color:transparent!important;text-shadow:none!important;caret-color:transparent!important;transition:none!important}';
    document.head.append(s);
    return out;
  }, PARSE_COLOUR);
  const shot = decodePng(await page.screenshot({ scale: 'css', animations: 'disabled' }));
  await page.evaluate(() => document.getElementById('cosmosHide')?.remove());
  const bad = [];
  for (const b of boxes) {
    let worst = Infinity;
    for (const q of b.rects) {
      for (let y = Math.max(0, Math.floor(q.t)); y < Math.min(shot.h, Math.ceil(q.b)); y++) {
        for (let x = Math.max(0, Math.floor(q.l)); x < Math.min(shot.w, Math.ceil(q.r)); x++) {
          const o = (y * shot.w + x) * 4;
          const bg = [shot.px[o], shot.px[o + 1], shot.px[o + 2]];
          const a = b.fg[3] ?? 1;
          const f = [0, 1, 2].map((k) => b.fg[k] * a + bg[k] * (1 - a));
          worst = Math.min(worst, ratio(f, bg));
        }
      }
    }
    if (worst + 1e-6 < b.need) bad.push(`${b.what} ${worst.toFixed(2)}:1 < ${b.need}:1`);
  }
  return bad;
}

export async function run(browser, base, suite) {
  await suite.test('AT-T1/T3: the Lab lists all 17, each applies with its chip, and nothing but the Lab key is written', async () => {
    const { ctx, page } = await openLook(browser, base, null);
    try {
      const before = await snapshot(page);
      for (let i = 0; i < 5; i++) await page.click('#brandMark');
      await page.click('#devOpenThemeLab');
      const tiles = await page.$$eval('#themeLab [data-lab-apply]', (t) => t.map((x) => x.dataset.labApply));
      assert(tiles.length === 17 && new Set(tiles).size === 17, `the Lab lists ${tiles.length}: ${tiles}`);
      let lastBg = null;
      for (const id of tiles) {
        await page.click(`#themeLab [data-lab-apply="${id}"]`);
        const s = await page.evaluate(() => ({ theme: document.documentElement.getAttribute('data-theme'), chip: document.getElementById('labChip')?.textContent || '', bg: getComputedStyle(document.documentElement).getPropertyValue('--bg-color').trim() + getComputedStyle(document.documentElement).getPropertyValue('--orb-art').length }));
        assert(s.theme === id, `${id}: data-theme is ${s.theme}`);
        assert(s.chip.startsWith('LAB · '), `${id}: no LAB chip (${s.chip})`);
        assert(s.bg !== lastBg, `${id}: the screen did not change`);
        lastBg = s.bg;
      }
      await page.click('#themeLab [data-lab="unlock"]');
      await page.waitForFunction(() => document.getElementById('celebrate')?.classList.contains('show'), null, { timeout: 5000 });
      await page.click('#labChip');
      assert(!(await page.$('#labChip')), 'the chip did not clear the preview');
      assert((await snapshot(page)) === before, 'the Lab wrote something besides its own key (ledger or progress changed)');
      assert((await page.evaluate((k) => JSON.parse(localStorage.getItem(k) || '{}').theme || '', LAB)) === '', 'clearing left a preview behind');
    } finally { await ctx.close(); }
  });

  await suite.test('T-D9: a Lab preview survives a reload until cleared', async () => {
    const { ctx, page } = await openLook(browser, base, { theme: 'gold' });
    try {
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest, null, { timeout: 30000 });
      await domSettled(page);
      assert((await page.evaluate(() => document.documentElement.getAttribute('data-theme'))) === 'gold', 'the preview did not survive a reload');
      assert(await page.$('#labChip'), 'no chip after reload');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-T4: the Lab clock reaches the Sky band and nothing stored', async () => {
    const { ctx, page } = await openLook(browser, base, null);
    try {
      const before = await snapshot(page);
      const next = new Date(); next.setFullYear(next.getFullYear() + 1);
      const day = next.toISOString().slice(0, 10);
      for (const [clock, band] of [[`${day}T21:30`, 'night'], [`${day}T06:15`, 'sunrise'], [`${day}T12:00`, 'day'], [`${day}T18:00`, 'sunset']]) {
        await page.evaluate(([k, v]) => localStorage.setItem(k, v), [LAB, JSON.stringify({ theme: 'sky', clock })]);
        await page.reload({ waitUntil: 'load' });
        await page.waitForFunction(() => window.__spelltest, null, { timeout: 30000 });
        await domSettled(page);
        assert((await page.evaluate(() => document.documentElement.getAttribute('data-band'))) === band, `clock ${clock} did not give ${band}`);
      }
      // A word played under the override records today's real date, not the Lab's.
      await serveWord(page);
      await typeOnKeyboard(page, await page.evaluate(() => window.__spelltest.currentWord()));
      await page.click('#checkBtn');
      await page.waitForFunction(() => /\bgood\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
      // The Lab's own key holds the override by design; every other key must not.
      const leaked = await page.evaluate(([day, lab]) => Object.keys(localStorage).filter((k) => k !== lab && (localStorage.getItem(k) || '').includes(day)), [day, LAB]);
      assert(!leaked.length, `the Lab's date reached stored state: ${leaked}`);
      await page.evaluate((k) => localStorage.removeItem(k), LAB);
      void before;
    } finally { await ctx.close(); }
  });

  await suite.test('AT-T13: no look moves a box (geometry identical to Eclipse)', async () => {
    const { ctx, page } = await openLook(browser, base, { theme: 'eclipse' });
    try {
      const ref = await geometry(page);
      const diffs = [];
      for (const l of LOOKS) {
        await page.evaluate(([k, v]) => localStorage.setItem(k, v), [LAB, JSON.stringify(l)]);
        await page.reload({ waitUntil: 'load' });
        await page.waitForFunction(() => window.__spelltest, null, { timeout: 30000 });
        await domSettled(page);
        const g = await geometry(page);
        const moved = g.filter((x, i) => x !== ref[i]);
        if (g.length !== ref.length || moved.length) diffs.push(`${nameOf(l)}: ${g.length} vs ${ref.length} boxes; first moved ${moved[0]}`);
      }
      assert(!diffs.length, diffs.join('\n'));
    } finally { await ctx.close(); }
  });

  for (const l of LOOKS) {
    await suite.test(`AT-T5/T5b ${nameOf(l)}: axe contrast (standard and Jr)${ART(l) ? ' and worst-pixel contrast over the art' : ''}`, async () => {
      const failures = [];
      for (const jr of [false, true]) {
        const { ctx, page } = await openLook(browser, base, { ...l, jr });
        try {
          await serveWord(page);
          await domSettled(page);
          const r = await new AxeBuilder({ page }).withRules(['color-contrast']).exclude('#labChip').analyze();
          for (const v of r.violations) for (const n of v.nodes) {
            const d = (n.any[0] || {}).data || {};
            failures.push(`${jr ? 'Jr ' : ''}axe ${n.target.join(' ')} ${d.contrastRatio}:1 < ${d.expectedContrastRatio} (${d.fgColor} on ${d.bgColor})`);
          }
          if (ART(l)) for (const b of await pixelContrast(page)) failures.push(`${jr ? 'Jr ' : ''}pixels ${b}`);
        } finally { await ctx.close(); }
      }
      assert(!failures.length, failures.join('\n'));
    });
  }

  await suite.test('AT-T5b: a planted label over the bare pavé field fails, as designed', async () => {
    const { ctx, page } = await openLook(browser, base, { theme: 'diamond' });
    try {
      await serveWord(page);
      await page.evaluate(() => {
        // Between the orb and the keyboard, where no scrim reaches once the scrim token is off.
        document.getElementById('themeArt').style.setProperty('--art-scrim', 'transparent');
        const d = document.createElement('div');
        d.id = 'plantedLabel';
        d.textContent = 'planted label';
        d.style.cssText = 'position:fixed;left:12px;top:46%;color:var(--muted);font-size:14px;z-index:5';
        document.body.append(d);
      });
      const bad = await pixelContrast(page);
      assert(bad.some((b) => b.startsWith('#plantedLabel')), `the pixel check passed a label over bare stones: ${bad}`);
    } finally { await ctx.close(); }
  });

  await suite.test('AT-T11/T12/T19: during a word nothing behind the play surface animates or repaints; Diamond glints between words, never under Reduce Motion', async () => {
    for (const rm of [false, true]) {
      for (const theme of ['diamond', 'aurora', 'sky', 'lunar']) {
        const { ctx, page } = await openLook(browser, base, { theme, rm });
        try {
          await serveWord(page);
          const art = () => page.evaluate(() => {
            const a = document.getElementById('themeArt');
            const anims = document.getAnimations().filter((x) => { const t = x.effect?.target; return t && (t === a || t === document.documentElement || t === document.body); });
            return { bg: a ? getComputedStyle(a).backgroundImage.length + ':' + getComputedStyle(a).backgroundImage.slice(-40) : '', shift: a ? a.style.getPropertyValue('--art-shift') : '', anims: anims.length, glint: a?.classList.contains('glint') };
          });
          const start = await art();
          const word = await page.evaluate(() => window.__spelltest.currentWord());
          await typeOnKeyboard(page, word.slice(0, 2));
          const mid = await art();
          assert(mid.bg === start.bg && mid.shift === start.shift && mid.anims === 0, `${theme}${rm ? ' RM' : ''}: the background changed during a word ${JSON.stringify([start, mid])}`);
          await typeOnKeyboard(page, word.slice(2));
          await page.click('#checkBtn');
          await page.waitForFunction(() => /\bgood\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
          // The boundary runs with the verdict; two frames let any class change land.
          if (theme === 'diamond' && !rm) await page.waitForFunction(() => document.getElementById('themeArt')?.classList.contains('glint'), null, { timeout: 5000 });
          else await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
          const after = await art();
          if (theme === 'diamond') {
            if (rm) assert(!after.glint, 'Diamond glinted under Reduce Motion');
            else assert(after.glint, 'Diamond did not glint between words');
          }
          if (rm) {
            const running = await page.evaluate(() => document.getAnimations().filter((x) => x.effect?.target?.id === 'themeArt' || String(x.effect?.pseudoElement || '').length).filter((x) => x.effect?.target?.id === 'themeArt').length);
            assert(running === 0, `${theme}: art animated under Reduce Motion`);
          }
        } finally { await ctx.close(); }
      }
    }
  });

  await suite.test('AT-T16: Appearance lists the Classic five only; no new theme reaches a player surface', async () => {
    const { ctx, page } = await openLook(browser, base, null);
    try {
      const rows = await page.$$eval('#themePick [data-theme-choice]', (r) => r.map((x) => x.dataset.themeChoice));
      assert(rows.join() === 'system,eclipse,daybreak,chalkboard,typewriter,beestage', `Appearance rows: ${rows}`);
      const leaked = await page.evaluate(() => {
        const text = document.body.innerText;
        return ['Great Wall', 'Diamond', 'Blood Moon', 'Solar System', 'Aurora'].filter((n) => text.includes(n));
      });
      assert(!leaked.length, `a Cosmos theme name is on a player surface: ${leaked}`);
    } finally { await ctx.close(); }
  });

  await suite.test('AT-T17: a result card renders identically twice in every look', async () => {
    const { ctx, page } = await openLook(browser, base, null);
    try {
      const spec = JSON.stringify({ mode: 'Climb', hero: '12', heroLabel: 'chain', lang: 'en', tagline: 'spellgame.net', seed: 7 });
      const seen = new Set();
      for (const l of LOOKS) {
        await page.evaluate(([k, v]) => localStorage.setItem(k, v), [LAB, JSON.stringify(l)]);
        await page.reload({ waitUntil: 'load' });
        await page.waitForFunction(() => window.__spelltest, null, { timeout: 30000 });
        const [a, b] = await page.evaluate((s) => [window.__spelltest.renderCard(s), window.__spelltest.renderCard(s)], spec);
        assert(a && a === b, `${nameOf(l)}: the card is not deterministic`);
        seen.add(a.length + a.slice(-64));
      }
      assert(seen.size > 1, 'every look drew the same card');
    } finally { await ctx.close(); }
  });
}
