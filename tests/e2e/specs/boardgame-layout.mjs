// boardgame-layout.spec -- CC-BOARD-GAME A4. The spell state for every
// language's key grid at 375x667, 390x844 and 430x932: every element visible,
// nothing scrolls, nothing overlaps, and the keys keep their size (the grid is
// never shrunk to make a layout fit). The locale decides the key grid (the
// shared keyboard's rows plus the units the bank needs), so the sweep is by
// language, for both audiences.
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const FLAG = () => localStorage.setItem('spell_flag_boardgame', 'on');
const LANGS = ['en', 'es', 'fr', 'de', 'pt', 'pl', 'ru', 'vi', 'ko', 'ja', 'fil', 'zh', 'ar', 'sw', 'hi'];
const SIZES = [[375, 667], [390, 844], [430, 932]];
const MIN_KEY_H = 45.5;

/// Combinations known not to fit, each with its reason. A combination listed
/// here that now fits FAILS the test, so the list cannot rot; one that is not
/// listed and does not fit fails too. Empty means every language fits.
const KNOWN = new Map([]);

const measure = () => {
  const r = (el) => { const b = el.getBoundingClientRect(); return { l: b.left, t: b.top, r: b.right, b: b.bottom, w: b.width, h: b.height }; };
  const W = innerWidth, H = innerHeight;
  const leaves = [];
  const add = (name, el) => { if (el) leaves.push({ name, ...r(el) }); };
  add('hud', document.querySelector('.bg-hud'));
  add('view', document.getElementById('bgView'));
  add('orb', document.getElementById('bgOrb'));
  add('field', document.getElementById('bgField'));
  add('del', document.getElementById('bgDel'));
  add('go', document.getElementById('bgGo'));
  const keys = [...document.querySelectorAll('#bgKeys .bg-key')];
  keys.forEach((k, i) => add(`key${i}`, k));
  const problems = [];
  for (const e of leaves) {
    if (e.w <= 0 || e.h <= 0) problems.push(`${e.name} has no size`);
    if (e.t < -0.5 || e.l < -0.5 || e.b > H + 0.5 || e.r > W + 0.5) problems.push(`${e.name} off screen (${Math.round(e.t)}..${Math.round(e.b)} of ${H})`);
  }
  for (let i = 0; i < leaves.length; i++) {
    for (let j = i + 1; j < leaves.length; j++) {
      const a = leaves[i], b = leaves[j];
      const ox = Math.min(a.r, b.r) - Math.max(a.l, b.l), oy = Math.min(a.b, b.b) - Math.max(a.t, b.t);
      if (ox > 1 && oy > 1) problems.push(`${a.name} overlaps ${b.name}`);
    }
  }
  for (const id of ['bgScreen', 'bgPlay', 'bgSpell']) {
    const el = document.getElementById(id);
    if (el.scrollHeight > el.clientHeight + 1) problems.push(`${id} scrolls (${el.scrollHeight} > ${el.clientHeight})`);
  }
  const kh = Math.min(...keys.map((k) => k.getBoundingClientRect().height));
  if (kh < 45.5) problems.push(`keys shrank to ${kh.toFixed(1)}px`);
  const kw = Math.min(...keys.map((k) => k.getBoundingClientRect().width));
  return { problems, rows: document.querySelectorAll('#bgKeys .bg-row').length, keys: keys.length, kw: Math.round(kw) };
};

export async function run(browser, base, suite) {
  await suite.test('boardgame_layout_every_language_every_phone', async () => {
    const failed = [];
    const fitted = [];
    let checked = 0;
    for (const lang of LANGS) {
      for (const kid of [false, true]) {
        const { ctx, page } = await openApp(browser, base, { lang, age: kid ? KID : undefined, init: FLAG });
        try {
          await page.evaluate(() => document.getElementById('bgOpenBtn').click());
          await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
          if (await page.$eval('#bgStart', (b) => b.disabled)) { failed.push(`${lang}${kid ? '/jr' : ''}: Start is disabled (coming soon)`); continue; }
          await page.click('[data-bg="mode:pass"]');
          await page.click('#bgStart');
          await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
          await page.click('#bgHandGo');
          await page.click('#bgOrb');
          await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
          for (const [w, h] of SIZES) {
            await page.setViewportSize({ width: w, height: h });
            await domSettled(page);
            const m = await page.evaluate(measure);
            const id = `${lang}${kid ? '/jr' : ''}@${w}x${h}`;
            checked++;
            const known = KNOWN.has(id);
            if (m.problems.length && !known) failed.push(`${id} (${m.rows} rows, ${m.keys} keys): ${m.problems.slice(0, 3).join('; ')}`);
            if (!m.problems.length && known) failed.push(`${id}: listed as not fitting but it fits now`);
            fitted.push(`${id}:${m.rows}r`);
          }
        } finally { await ctx.close(); }
      }
    }
    process.stdout.write(`  boardgame layout: ${checked} combinations measured\n`);
    assert(failed.length === 0, `${failed.length} layout problem(s):\n    ${failed.join('\n    ')}`);
  });
}
