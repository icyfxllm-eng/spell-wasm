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
  add('view', document.getElementById('bgView'));
  add('orb', document.getElementById('bgOrb'));
  add('field', document.getElementById('bgField'));
  add('exit', document.getElementById('bgExit'));
  document.querySelectorAll('#bgChips .bg-chip').forEach((c, i) => add(`chip${i}`, c));
  document.querySelectorAll('#bgResults .bg-res').forEach((c, i) => add(`res${i}`, c));
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

/// F6 (v1.2 follow-up): the board uses the leftover height, the keys dock to the bottom
/// inside the safe area, the HUD clears the top inset, the exit is a 48pt+ tap target at
/// the bottom-right. insetTop/insetBottom are the emulated notch and home indicator.
const measureDock = (ins) => {
  const r = (id) => document.getElementById(id).getBoundingClientRect();
  const W = innerWidth, H = innerHeight;
  const keys = r('bgKeys'), hud = document.querySelector('.bg-hud').getBoundingClientRect(), view = r('bgView'), ex = r('bgExit');
  const problems = [];
  if (hud.top < ins.top - 0.5) problems.push(`hud top ${hud.top} is inside the top inset ${ins.top}`);
  // The title row sits right under the inset: not more than ~24pt of gap (a fixed min-height or stacked margin put it ~100pt down).
  const title = document.getElementById('bgTitle').getBoundingClientRect();
  if (title.top < ins.top - 0.5 || title.top > ins.top + 24) problems.push(`title top ${title.top.toFixed(0)} is not within 24pt under the top inset ${ins.top}`);
  // The bottom action row ends at the home-indicator inset (6pt of air at most 8): no dead band under it.
  const act = r('bgAct');
  const under = (H - ins.bottom) - act.bottom;
  if (under < -0.5 || under > 8) problems.push(`bottom action row is ${under.toFixed(0)}pt above the safe bottom (dead space)`);
  // The keys never overlap the exit.
  if (keys.bottom > ex.top + 0.5 && keys.right > ex.left && keys.left < ex.right) problems.push('keys overlap the exit');
  const gap = (H - ins.bottom) - keys.bottom;
  if (gap < -0.5 || gap > 70) problems.push(`keys are not docked: ${gap.toFixed(0)}px above the safe bottom`);
  return { problems, viewH: view.height, viewW: view.width, vb: document.getElementById('bgBoard').getAttribute('viewBox'), exit: { w: ex.width, h: ex.height, r: W - ex.right, b: (H - ins.bottom) - ex.bottom, cx: ex.left + ex.width / 2, cy: ex.top + ex.height / 2 }, keysTop: keys.top, keysBottom: keys.bottom };
};
/// The board panel shows either the whole ring or the unrolled track; a track must span at
/// least 80% of the panel width (the old fallback was a thin sliver at the panel edge).
const measureTrack = () => {
  const view = document.getElementById('bgView').getBoundingClientRect();
  const strip = document.getElementById('bgView').classList.contains('strip');
  if (!strip) return { mode: 'ring', problem: /^0 0 /.test(document.getElementById('bgBoard').getAttribute('viewBox')) ? null : 'ring mode without a full viewBox' };
  const tiles = [...document.querySelectorAll('#bgBoard .bg-t')].map((t) => t.getBoundingClientRect());
  if (tiles.length < 8) return { mode: 'track', problem: `track shows only ${tiles.length} tiles` };
  const l = Math.min(...tiles.map((t) => t.left)), r = Math.max(...tiles.map((t) => t.right));
  const span = (r - l) / view.width;
  const inside = tiles.every((t) => t.left >= view.left - 0.5 && t.right <= view.right + 0.5 && t.top >= view.top - 0.5 && t.bottom <= view.bottom + 0.5);
  return { mode: 'track', span, problem: span < 0.8 ? `track spans ${(span * 100).toFixed(0)}% of the panel width` : !inside ? 'track tiles leave the panel' : null };
};
const setInsets = (page, top, bottom) => page.evaluate(([t, b]) => {
  const el = document.getElementById('bgScreen');
  el.style.setProperty('--bg-inset-top', t + 'px'); el.style.setProperty('--bg-inset-bottom', b + 'px');
}, [top, bottom]);

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
          await page.click('[data-bg="count:4"]'); // four chips must fit
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
            if (lang === 'ja' && m.rows > 6) failed.push(`${id}: ${m.rows} key rows (kana grid is 5 base rows plus one modifier row)`);
            if (lang === 'ja' && (w === 375 || w === 430)) {
              const tr = await page.evaluate(measureTrack);
              if (tr.problem) failed.push(`${id}: ${tr.problem}`);
            }
            const known = KNOWN.has(id);
            if (m.problems.length && !known) failed.push(`${id} (${m.rows} rows, ${m.keys} keys): ${m.problems.slice(0, 3).join('; ')}`);
            if (!m.problems.length && known) failed.push(`${id}: listed as not fitting but it fits now`);
            fitted.push(`${id}:${m.rows}r`);
            // The same sweep again with a notch and a home indicator (tall phones).
            if (h >= 844) {
              await setInsets(page, 47, 34);
              await domSettled(page);
              const mi = await page.evaluate(measure);
              const di = await page.evaluate(measureDock, { top: 47, bottom: 34 });
              await setInsets(page, 0, 0);
              checked++;
              const rowsN = m.rows;
              const bad = [...mi.problems, ...di.problems];
              if (lang === 'ja') {
                const tr = await page.evaluate(measureTrack);
                if (tr.problem) failed.push(`${id}+insets: ${tr.problem}`);
              }
              if (bad.length) failed.push(`${id}+insets: ${bad.slice(0, 3).join('; ')}`);
              // Tall room means the whole ring, big: a full viewBox and a board far taller than the old strip.
              if (rowsN <= 4 && !/^0 0 /.test(di.vb)) failed.push(`${id}: spell state fell back to the strip on a tall phone (${di.vb}, view ${Math.round(di.viewH)})`);
              if (di.viewH < (/^0 0 /.test(di.vb) ? 112 : 0) || (rowsN <= 4 && di.viewH < 250)) failed.push(`${id}: board only ${Math.round(di.viewH)}pt tall in the spell state`);
              if (di.exit.w < 48 || di.exit.h < 48 || di.exit.r > 24 || di.exit.b > 24 || di.exit.cx < w / 2 || di.exit.cy < h / 2) failed.push(`${id}: exit not a 48pt bottom-right target ${JSON.stringify(di.exit)}`);
            }
          }
        } finally { await ctx.close(); }
      }
    }
    process.stdout.write(`  boardgame layout: ${checked} combinations measured\n`);
    assert(failed.length === 0, `${failed.length} layout problem(s):\n    ${failed.join('\n    ')}`);
  });
  await runMore(browser, base, suite);
}

async function runMore(browser, base, suite) {
  const open = async (viewport) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init: FLAG });
    await o.page.evaluate(() => document.getElementById('bgOpenBtn').click());
    await o.page.waitForSelector('#bgScreen.show', { timeout: 8000 });
    await o.page.click('[data-bg="mode:pass"]');
    await o.page.click('#bgStart');
    await o.page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
    await o.page.click('#bgHandGo');
    return o;
  };
  const shown = (page) => page.evaluate(() => document.getElementById('bgScreen').classList.contains('show'));
  await suite.test('boardgame_setup_screen_has_top_right_exit', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: FLAG });
    try {
      await o.page.evaluate(() => document.getElementById('bgOpenBtn').click());
      await o.page.waitForSelector('#bgScreen.show', { timeout: 8000 });
      const m = await o.page.evaluate(() => {
        const b = document.getElementById('bgSetupExit').getBoundingClientRect();
        const top = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
        return { r: b.toJSON(), W: innerWidth, hit: !!top && !!top.closest('#bgSetupExit') };
      });
      assert(m.r.width >= 44 && m.r.height >= 44, `setup exit too small ${JSON.stringify(m.r)}`);
      assert(m.r.right > m.W - 40 && m.r.top < 120, `setup exit is not top-right ${JSON.stringify(m.r)}`);
      assert(m.hit, 'something covers the setup exit button');
      await o.page.click('#bgSetupExit');
      assert(!(await o.page.evaluate(() => document.getElementById('bgScreen').classList.contains('show'))), 'setup exit did not leave the Board Game');
    } finally { await o.ctx.close(); }
  });
  await suite.test('boardgame_board_state_fills_height_and_exit_works', async () => {
    const { ctx, page } = await open({ width: 430, height: 932 });
    try {
      await setInsets(page, 47, 34);
      await domSettled(page);
      const m = await page.evaluate(() => {
        const v = document.getElementById('bgView').getBoundingClientRect(), a = document.getElementById('bgAct').getBoundingClientRect();
        return { vh: v.height, vw: v.width, actBottom: a.bottom, H: innerHeight, hud: document.querySelector('.bg-hud').getBoundingClientRect().top, over: document.documentElement.scrollWidth - innerWidth, ex: document.getElementById('bgExit').getBoundingClientRect().toJSON() };
      });
      assert(m.vh > 450, `board state: ring panel only ${m.vh}pt tall`);
      assert(m.hud >= 47, `HUD at ${m.hud} is under the top inset`);
      assert(m.over <= 0, 'horizontal overflow');
      assert(m.ex.width >= 48 && m.ex.height >= 48 && m.ex.left > 215 && m.ex.top > 466 && m.ex.bottom <= 932 - 34 + 0.5, `exit rect ${JSON.stringify(m.ex)}`);
      const hit = await page.evaluate(() => { const b = document.getElementById('bgExit').getBoundingClientRect(); return document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2).closest('#bgExit') !== null; });
      assert(hit, 'something covers the exit button');
      await page.click('#bgExit');
      assert(!(await shown(page)), 'exit from the board state did not leave the Board Game');
    } finally { await ctx.close(); }
  });
  await suite.test('boardgame_spell_state_exit_works', async () => {
    const { ctx, page } = await open({ width: 390, height: 844 });
    try {
      await page.click('#bgOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
      await domSettled(page);
      const hit = await page.evaluate(() => { const b = document.getElementById('bgExit').getBoundingClientRect(); return document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2).closest('#bgExit') !== null; });
      assert(hit, 'something covers the exit button in the spell state');
      await page.click('#bgExit');
      assert(!(await shown(page)), 'exit from the spell state did not leave the Board Game');
    } finally { await ctx.close(); }
  });
  await suite.test('boardgame_ja_modifier_keys', async () => {
    const o = await openApp(browser, base, { lang: 'ja', viewport: { width: 390, height: 844 }, init: FLAG });
    const { ctx, page } = o;
    try {
      await page.evaluate(() => document.getElementById('bgOpenBtn').click());
      await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
      await page.click('[data-bg="mode:pass"]');
      await page.click('#bgStart');
      await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
      await page.click('#bgHandGo');
      await page.click('#bgOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
      await domSettled(page);
      const field = () => page.$eval('#bgField', (f) => f.textContent);
      const key = (k) => page.click(`#bgKeys [data-k="${k}"]`);
      assert(await page.$$eval('#bgKeys .bg-key.mod', (ks) => ks.length) === 3, 'expected three tinted modifier keys');
      const tint = await page.evaluate(() => [getComputedStyle(document.querySelector('#bgKeys .bg-key.mod')).backgroundColor, getComputedStyle(document.querySelector('#bgKeys .bg-key:not(.mod)')).backgroundColor]);
      assert(tint[0] !== tint[1], 'modifier keys are not tinted apart from kana keys');
      await key('u:か'); await key('m:d');
      assert((await field()) === 'が', `か + dakuten gave ${await field()}`);
      await page.click('#bgDel');
      assert((await field()) === '', `backspace left ${JSON.stringify(await field())} (must remove the modified char whole)`);
      await key('u:は'); await key('m:h');
      assert((await field()) === 'ぱ', `は + handakuten gave ${await field()}`);
      await page.click('#bgDel');
      await key('u:つ'); await key('m:s');
      assert((await field()) === 'っ', `つ + small gave ${await field()}`);
      await key('u:き'); await key('m:s');
      assert((await field()) === 'っき', 'a modifier on a kana that cannot take it must do nothing');
      const rows = await page.$$eval('#bgKeys .bg-row', (r) => r.length);
      assert(rows <= 6, `${rows} key rows`);
    } finally { await ctx.close(); }
  });
  for (const lang of ['en', 'ar']) {
    await suite.test(`boardgame_unrolled_track_${lang}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, viewport: { width: 375, height: 540 }, init: FLAG });
      try {
        await page.evaluate(() => document.getElementById('bgOpenBtn').click());
        await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
        await page.click('[data-bg="mode:pass"]');
        await page.click('#bgStart');
        await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
        await page.click('#bgHandGo');
        await page.click('#bgOrb');
        await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
        await domSettled(page);
        const m = await page.evaluate(() => {
          const tiles = [...document.querySelectorAll('#bgBoard .bg-t')].map((t) => ({ slot: Number(t.getAttribute('data-slot')), l: t.getBoundingClientRect().left }));
          const me = document.querySelector('#bgBoard .bg-me')?.getBoundingClientRect();
          return { strip: document.getElementById('bgView').classList.contains('strip'), tiles, rtl: getComputedStyle(document.getElementById('bgView')).direction === 'rtl', me: me && me.left + me.width / 2, dest: document.querySelectorAll('#bgBoard .bg-t.dest').length, pill: !!document.querySelector('#bgLand .bg-pill') };
        });
        assert(m.strip && m.tiles.length >= 8 && m.tiles.length <= 12, `expected a track of 8-12 tiles, got ${JSON.stringify({ strip: m.strip, n: m.tiles.length })}`);
        const n = m.tiles.length;
        const want = (k) => (lang === 'ar' ? n - 1 - k : k);
        assert(m.tiles.every((t, k) => t.slot === want(k)), `path direction wrong for ${lang}: ${m.tiles.map((t) => t.slot)}`);
        // The path advances left to right for ltr and right to left for rtl: screen x of tile k.
        const xs = m.tiles.map((t) => t.l);
        assert(xs.every((x, k) => k === 0 || (lang === 'ar' ? x < xs[k - 1] : x > xs[k - 1])), 'tiles are not in reading order on screen');
        assert(m.dest === 1, `destination tile outlined ${m.dest} times`);
        assert(m.pill, 'the tier pill is missing');
        const tr = await page.evaluate(measureTrack);
        assert(!tr.problem, tr.problem);
      } finally { await ctx.close(); }
    });
  }
  await suite.test('boardgame_compact_strip_when_space_is_short', async () => {
    // Not a device name: a viewport too short for a legible ring (and the keys at full size) must fall back to the unrolled track, with nothing lost.
    // The ring-fit rule (Polish Feature 7) may turn the ring on its side, so a ring needs about 19 rows
    // x 12 px of panel: 375x600 now keeps a 12 px ring, and the track starts below that (375x520).
    const { ctx, page } = await open({ width: 375, height: 520 });
    try {
      await page.click('#bgOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
      await domSettled(page);
      const m = await page.evaluate(() => ({ strip: document.getElementById('bgView').classList.contains('strip'), tiles: document.querySelectorAll('#bgBoard .bg-t').length }));
      assert(m.strip && m.tiles >= 8 && m.tiles <= 12, `expected the unrolled track at 375x520, got ${JSON.stringify(m)}`);
    } finally { await ctx.close(); }
  });
}
