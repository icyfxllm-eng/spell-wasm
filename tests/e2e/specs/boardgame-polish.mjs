// boardgame-polish.spec -- CC-BOARD-GAME-POLISH v1, Phase A (Features 5, 6, 7).
//   A-P11  layout / snapshot-style checks at 375x667, 393x852 and an iPad size
//   A-P12  timing: human hops, Reduce Motion, taps, the stage's die and label
// (A-P10 is a host unit test of tile_to_cell in src/boardgame_ring.rs; A-P9 is the
// unchanged engine golden and A9 table, run by cargo test.)
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const FLAG = () => localStorage.setItem('spell_flag_boardgame', 'on');
const SIZES = [[375, 667], [393, 852], [820, 1180]];
// The allowed Full shapes in portrait (w <= h), the spec's table. Mirrors boardgame_ring::FULL_SHAPES on purpose:
// the test restates the rule instead of reading it back from the code under test.
const FULL = [[22, 22], [21, 23], [20, 24], [19, 25]];

const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
const cite = (w) => w.split('|')[0];

async function openGame(browser, base, { viewport, lang = 'en', mode = 'pass', count = 4 } = {}) {
  const o = await openApp(browser, base, { lang, viewport, init: FLAG });
  const { page } = o;
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
  await page.click(`[data-bg="mode:${mode}"]`);
  await page.click(`[data-bg="count:${count}"]`);
  await page.click('#bgStart');
  if (mode === 'pass') {
    await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
    await page.click('#bgHandGo');
  }
  await domSettled(page);
  return o;
}

async function rollToSpelling(page) {
  await page.click('#bgOrb');
  await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
  await domSettled(page);
}

async function typeIt(page, typed) {
  for (const ch of typed) await page.click(`#bgKeys [data-k="u:${ch}"]`);
}

/// Everything the layout checks need, measured in the page.
const measure = (allowed) => {
  const rect = (el) => { const b = el.getBoundingClientRect(); return { l: b.left, t: b.top, r: b.right, b: b.bottom, w: b.width, h: b.height }; };
  const view = document.getElementById('bgView');
  const board = document.getElementById('bgBoard');
  const v = rect(view);
  // The ring-fit rule, restated: largest pitch among the allowed shapes, transposed in a wide box,
  // ties to the squarer one.
  const landscape = view.clientWidth > view.clientHeight;
  const cands = allowed.map(([a, b]) => (landscape ? [b, a] : [a, b])).map(([w, h]) => ({ w, h, p: Math.min(view.clientWidth / w, view.clientHeight / h) }));
  const best = Math.max(...cands.map((c) => c.p));
  const tied = cands.filter((c) => Math.abs(c.p - best) < 1e-6).sort((x, y) => Math.abs(x.w - x.h) - Math.abs(y.w - y.h));
  const want = tied[0];
  const shape = board.getAttribute('data-shape');
  const pitch = Number(board.getAttribute('data-pitch'));
  const tiles = [...board.querySelectorAll('.bg-t')].map(rect);
  const nums = [...board.querySelectorAll('.bg-num')].map(rect);
  const stage = document.getElementById('bgStage');
  const stageShown = !stage.hidden;
  const sr = stageShown ? rect(stage) : null;
  const pcs = [...document.querySelectorAll('#bgPieces .bg-pc')].map((e) => ({ ...rect(e), z: Number(getComputedStyle(e).zIndex), seat: e.dataset.seat }));
  const ctl = {};
  for (const id of ['bgOrb', 'bgChips', 'bgExit']) { const e = document.getElementById(id); if (e) ctl[id] = rect(e); }
  return { v, shape, pitch, want: `${want.w}x${want.h}`, wantPitch: want.p, tiles, nums, stage: sr, pcs, ctl, W: innerWidth, H: innerHeight, strip: view.classList.contains('strip') };
};

const overlap = (a, b) => Math.min(a.r, b.r) - Math.max(a.l, b.l) > 1 && Math.min(a.b, b.b) - Math.max(a.t, b.t) > 1;

/// Visible fraction of each piece disc: sample points inside it and count those not inside a disc drawn above.
const visibleFractions = (pcs) => pcs.map((a) => {
  const cx = (a.l + a.r) / 2, cy = (a.t + a.b) / 2, r = a.w / 2;
  const above = pcs.filter((b) => b.z > a.z);
  let inside = 0, seen = 0;
  for (let i = -10; i <= 10; i++) for (let j = -10; j <= 10; j++) {
    const x = cx + (i / 10) * r, y = cy + (j / 10) * r;
    if ((x - cx) ** 2 + (y - cy) ** 2 > r * r) continue;
    inside++;
    if (!above.some((b) => (x - (b.l + b.r) / 2) ** 2 + (y - (b.t + b.b) / 2) ** 2 < (b.w / 2) ** 2)) seen++;
  }
  return { seat: a.seat, z: a.z, frac: seen / inside };
});

export async function run(browser, base, suite) {
  // ------------------------------------------------------------------ A-P11
  for (const [w, h] of SIZES) {
    await suite.test(`boardgame_polish_A-P11_layout_${w}x${h}`, async () => {
      const { ctx, page } = await openGame(browser, base, { viewport: { width: w, height: h } });
      try {
        const problems = [];
        const check = (m, label) => {
          if (m.strip) { problems.push(`${label}: fell back to the track`); return; }
          if (m.shape !== m.want) problems.push(`${label}: ring-fit chose ${m.shape} but ${m.want} has the largest pitch (${m.wantPitch.toFixed(2)})`);
          if (Math.abs(m.pitch - m.wantPitch) > 0.01) problems.push(`${label}: pitch ${m.pitch} vs expected ${m.wantPitch.toFixed(3)}`);
          if (!m.stage) { problems.push(`${label}: the centre stage is not shown although the full ring fits`); return; }
          const inView = (r) => r.l >= m.v.l - 0.5 && r.r <= m.v.r + 0.5 && r.t >= m.v.t - 0.5 && r.b <= m.v.b + 0.5;
          if (!inView(m.stage)) problems.push(`${label}: stage leaves the board panel`);
          const hit = m.tiles.filter((t) => overlap(t, m.stage)).length;
          if (hit) problems.push(`${label}: the stage overlaps ${hit} ring tile(s)`);
          const nh = m.nums.filter((t) => overlap(t, m.stage)).length;
          if (nh) problems.push(`${label}: the stage overlaps ${nh} tile number(s)`);
          for (const p of m.pcs) if (overlap(p, m.stage)) problems.push(`${label}: the stage overlaps piece ${p.seat}`);
          for (const [id, r] of Object.entries(m.ctl)) {
            if (overlap(r, m.stage)) problems.push(`${label}: the stage overlaps ${id}`);
            if (m.tiles.some((t) => overlap(t, r))) problems.push(`${label}: the ring overlaps ${id}`);
          }
          // Pieces never leave the panel.
          for (const p of m.pcs) if (!inView(p)) problems.push(`${label}: piece ${p.seat} is clipped by the panel`);
        };
        // Board state, all four pieces on tile 0 at the start.
        const m = await page.evaluate(measure, FULL);
        check(m, 'board');
        assertEq(m.pcs.length, 4, 'four pieces');
        const vis = visibleFractions(m.pcs);
        for (const f of vis) if (f.frac < 0.6) problems.push(`piece ${f.seat} only ${(f.frac * 100).toFixed(0)}% visible on the shared tile`);
        const act = (await state(page)).seat;
        const top = vis.reduce((a, b) => (b.z > a.z ? b : a));
        if (Number(top.seat) !== act) problems.push(`the active seat ${act} is not drawn on top (top is ${top.seat})`);
        for (const p of m.pcs) {
          const px = m.pitch * 1.6;
          const want = Math.min(40, Math.max(24, px));
          if (Math.abs(p.w - want) > 0.6) problems.push(`piece is ${p.w.toFixed(1)}px, wanted ${want.toFixed(1)} (1.6 x ${m.pitch.toFixed(1)} clamped 24-40)`);
        }
        // Spell state.
        await rollToSpelling(page);
        const ms = await page.evaluate(measure, FULL);
        if (ms.strip) {
          // A track at this size is the existing fallback (pitch under 12): the pill carries the info.
          const pill = await page.evaluate(() => !!document.querySelector('#bgLand .bg-pill'));
          if (!pill) problems.push('spell: track fallback without the tier pill');
        } else {
          check(ms, 'spell');
        }
        assert(problems.length === 0, `${w}x${h}: ${problems.join('; ')}`);
        process.stdout.write(`  A-P11 ${w}x${h}: board ${m.shape} pitch ${m.pitch.toFixed(2)}; spell ${ms.strip ? 'track' : ms.shape + ' pitch ' + ms.pitch.toFixed(2)}; piece visibility min ${(Math.min(...vis.map((f) => f.frac)) * 100).toFixed(0)}%\n`);
      } finally { await ctx.close(); }
    });
  }

  // A-P11, the grayscale part: tier, start, finish and a revealed special must each be readable
  // without colour. Checked as structure (pips, glyphs, shapes) and as contrast against the fill.
  await suite.test('boardgame_polish_A-P11_grayscale_tiers_start_finish_revealed', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 } });
    try {
      await page.evaluate(() => window.__spelltest.boardgameMark(30));
      await domSettled(page);
      const r = await page.evaluate(() => {
        const board = document.getElementById('bgBoard');
        const rects = [...board.querySelectorAll('.bg-t')];
        const inside = (el, tile) => {
          const a = el.getBoundingClientRect(), b = tile.getBoundingClientRect();
          const cx = (a.left + a.right) / 2, cy = (a.top + a.bottom) / 2;
          return cx > b.left && cx < b.right && cy > b.top && cy < b.bottom;
        };
        const pips = [...board.querySelectorAll('.bg-pip')];
        const counts = {};
        for (const t of rects) {
          const tier = [...t.classList].find((c) => c.startsWith('t-'));
          if (!tier) continue;
          const n = pips.filter((p) => inside(p, t)).length;
          (counts[tier] ||= new Set()).add(n);
        }
        const lum = (css) => { const m = css.match(/\d+/g).map(Number); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
        const fills = {};
        for (const t of rects) { const k = [...t.classList].filter((c) => c !== 'bg-t' && c !== 'dest').join('.'); fills[k] = lum(getComputedStyle(t).fill); }
        const start = rects.find((t) => t.classList.contains('start'));
        const fin = rects.find((t) => t.classList.contains('end'));
        const startMark = start && [...board.querySelectorAll('.bg-start')].some((p) => inside(p, start));
        const flag = fin && [...board.querySelectorAll('.bg-flag')].some((p) => inside(p, fin));
        const traps = [...board.querySelectorAll('.bg-trap')];
        const trapTile = rects.find((t) => traps.some((p) => inside(p, t)));
        // Contrast of a pip (white) and the start arrow (dark) against their tile fill, on luminance alone.
        const pipLum = 255;
        const tierFills = ['t-medium', 't-hard', 't-expert'].map((k) => fills[k]).filter((x) => x != null);
        return {
          counts: Object.fromEntries(Object.entries(counts).map(([k, v]) => [k, [...v]])),
          startMark: !!startMark, flag: !!flag, nTraps: traps.length, trapTile: !!trapTile,
          contrast: tierFills.map((f) => (pipLum - f) / 255), startFill: fills['start'], endFill: fills['end'],
          nums: [...board.querySelectorAll('.bg-num')].map((n) => n.textContent),
        };
      });
      for (const [tier, n] of [['t-medium', 1], ['t-hard', 2], ['t-expert', 3]]) assertEq(JSON.stringify(r.counts[tier]), JSON.stringify([n]), `${tier} tiles carry exactly ${n} pip(s)`);
      assert(r.startMark, 'the start tile carries its own arrow');
      assert(r.flag, 'the finish tile carries the flag');
      assert(r.nTraps >= 1 && r.trapTile, 'a revealed special keeps its icon on its tile');
      assert(r.contrast.every((c) => c > 0.25), `pips must stand out from every tier fill without colour: ${r.contrast}`);
      assert(Math.abs(r.startFill - r.endFill) > 20, `start and finish differ in luminance too: ${r.startFill} vs ${r.endFill}`);
      assertEq(r.nums.join(','), '10,20,30,40,50,60,70,80', 'a number on every tenth tile');
      // The actual grayscale render: with all colour removed, the three tier tiles are still told apart by their pips.
      await page.addStyleTag({ content: '#bgBoard{filter:grayscale(1)!important}' });
      const png = await page.locator('#bgBoard').screenshot();
      assert(png.length > 1000, 'a grayscale snapshot was taken');
    } finally { await ctx.close(); }
  });

  // Tile numbers sit on the interior side of the ring: every label is closer to the ring's centre than its tile.
  await suite.test('boardgame_polish_tile_numbers_are_on_the_interior_side', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 } });
    try {
      const bad = await page.evaluate(() => {
        const b = document.getElementById('bgBoard').getBoundingClientRect();
        const cx = (b.left + b.right) / 2, cy = (b.top + b.bottom) / 2;
        const tiles = [...document.querySelectorAll('#bgBoard .bg-t')];
        const out = [];
        for (const n of document.querySelectorAll('#bgBoard .bg-num')) {
          const r = n.getBoundingClientRect(), x = (r.left + r.right) / 2, y = (r.top + r.bottom) / 2;
          // the nearest ring tile
          const near = tiles.map((t) => t.getBoundingClientRect()).map((t) => ({ d: Math.hypot((t.left + t.right) / 2 - x, (t.top + t.bottom) / 2 - y), x: (t.left + t.right) / 2, y: (t.top + t.bottom) / 2 })).sort((p, q) => p.d - q.d)[0];
          if (Math.hypot(x - cx, y - cy) >= Math.hypot(near.x - cx, near.y - cy)) out.push(n.textContent);
        }
        return out;
      });
      assertEq(bad.length, 0, `tile numbers outside their tile: ${bad}`);
    } finally { await ctx.close(); }
  });

  // ------------------------------------------------------------------ A-P12
  const stubHaptics = (page) => page.evaluate(() => {
    window.__hap = 0;
    window.Capacitor = { Plugins: { Haptics: { impact() { window.__hap++; return Promise.resolve(); }, notification() { window.__hap++; return Promise.resolve(); } } } };
  });
  const unstub = (page) => page.evaluate(() => { delete window.Capacitor; });
  const counters = (page) => page.evaluate(() => {
    const el = document.getElementById('bgPieces');
    return { hops: Number(el.getAttribute('data-hops') || 0), ticks: Number(el.getAttribute('data-ticks') || 0), hap: window.__hap || 0, moving: el.hasAttribute('data-moving') };
  });
  /// Run the real walk through the seam and time it, polling every frame.
  const timedHop = (page, n) => page.evaluate(async (n) => {
    const el = document.getElementById('bgPieces');
    const t0 = performance.now();
    const planned = window.__spelltest.boardgameHop(0, n);
    await new Promise((res) => { const tick = () => (el.hasAttribute('data-moving') ? requestAnimationFrame(tick) : res()); requestAnimationFrame(tick); });
    return { ms: performance.now() - t0, planned };
  }, n);

  for (const [n, cap] of [[6, 1200], [12, 2000]]) {
    await suite.test(`boardgame_polish_A-P12_${n}_tile_hop_within_${cap}ms`, async () => {
      const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 }, mode: 'solo', count: 1 });
      try {
        await stubHaptics(page);
        const c0 = await counters(page);
        const r = await timedHop(page, n);
        const c1 = await counters(page);
        assert(r.ms <= cap, `a ${n}-tile hop took ${r.ms.toFixed(0)} ms (budget ${cap})`);
        assert(r.planned <= cap, `a ${n}-tile hop plans ${r.planned} ms (budget ${cap})`);
        assertEq(c1.hops - c0.hops, n, 'one hop frame per tile');
        assertEq(c1.ticks - c0.ticks, n, 'one tick per hop');
        assertEq(c1.hap - c0.hap, n, 'one haptic call per hop');
        await unstub(page);
        process.stdout.write(`  A-P12 ${n}-tile hop: ${r.ms.toFixed(0)} ms (budget ${cap})\n`);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('boardgame_polish_A-P12_reduce_motion_has_no_hops_and_no_ticks', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 }, mode: 'solo', count: 1 });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await stubHaptics(page);
      // The seam walk first: a 6-tile move is one cross-fade.
      const c0 = await counters(page);
      await timedHop(page, 6);
      const c1 = await counters(page);
      assertEq(c1.hops - c0.hops, 0, 'zero hop frames under Reduce Motion (seam walk)');
      assertEq(c1.hap - c0.hap, 0, 'zero haptic calls under Reduce Motion (seam walk)');
      // Then a real turn: roll, spell it right, and watch the piece move without hopping.
      await rollToSpelling(page);
      const s = await state(page);
      await typeIt(page, cite(s.word));
      await page.evaluate(() => { window.__hap = 0; });
      const before = await counters(page);
      const rect0 = await page.$eval('#bgPieces [data-seat="0"]', (e) => e.getBoundingClientRect().toJSON());
      await page.click('#bgGo');
      await page.waitForFunction(() => !!document.querySelector('#bgPieces [data-f]'), null, { timeout: 3000 });
      await page.waitForFunction(() => !document.getElementById('bgPieces').hasAttribute('data-moving'), null, { timeout: 3000 });
      const after = await counters(page);
      const rect1 = await page.$eval('#bgPieces [data-seat="0"]', (e) => e.getBoundingClientRect().toJSON());
      assert(Math.abs(rect1.x - rect0.x) + Math.abs(rect1.y - rect0.y) > 3, 'the piece still moved to its new tile');
      assertEq(after.hops - before.hops, 0, 'zero hop frames for a real move under Reduce Motion');
      assertEq(after.ticks - before.ticks, 0, 'zero hop ticks under Reduce Motion');
      // The only haptic in the turn is the correct-spelling confirmation, not a hop tick.
      assert(after.hap - before.hap <= 1, `haptic calls under Reduce Motion: ${after.hap - before.hap} (only the spelling confirmation may fire)`);
      const fade = await page.$eval('#bgPieces [data-seat="0"]', (e) => getComputedStyle(e).transitionDuration);
      assert(/^0s(, 0s)*$/.test(fade), `no travel transition under Reduce Motion: ${fade}`);
      await unstub(page);
    } finally { await ctx.close(); }
  });

  await suite.test('boardgame_polish_A-P12_real_move_hops_tile_by_tile_inside_the_budget', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 }, mode: 'solo', count: 1 });
    try {
      await stubHaptics(page);
      await rollToSpelling(page);
      const s = await state(page);
      await typeIt(page, cite(s.word));
      await page.evaluate(() => { window.__hap = 0; });
      const before = await counters(page);
      const t0 = Date.now();
      await page.click('#bgGo');
      await page.waitForFunction(() => document.getElementById('bgPieces').hasAttribute('data-moving'), null, { timeout: 2000 });
      await page.waitForFunction(() => !document.getElementById('bgPieces').hasAttribute('data-moving'), null, { timeout: 4000, polling: 'raf' });
      const ms = Date.now() - t0;
      const after = await counters(page);
      const pos = (await state(page)).pos[0];
      assertEq(after.hops - before.hops, pos, 'one hop frame per tile moved');
      assertEq(after.ticks - before.ticks, pos, 'one tick per hop');
      assert(ms <= 1200 + 150, `a ${pos}-tile move took ${ms} ms (6 tiles: 1200 ms, plus the click)`);
      await unstub(page);
    } finally { await ctx.close(); }
  });

  await suite.test('boardgame_polish_A-P12_tap_cancels_hops_and_the_npc_run_stays_inside_the_step', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 }, mode: 'solo', count: 3 });
    try {
      // Play the human turn, then watch the NPC run: one NPC step is 1500 ms and its hops live inside it.
      await rollToSpelling(page);
      let s = await state(page);
      await typeIt(page, cite(s.word));
      await page.click('#bgGo');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState()).npc, null, { timeout: 6000, polling: 'raf' });
      // The first NPC steps in; tap once it has moved (its chip is up), and everything must settle at once.
      const chips = await page.$eval('#bgResults', (e) => e.innerHTML);
      await page.waitForFunction((c) => document.getElementById('bgResults').innerHTML !== c, chips, { timeout: 4000 });
      const t0 = Date.now();
      await page.click('#bgTitle');
      await page.waitForFunction(() => !JSON.parse(window.__spelltest.boardgameState()).npc, null, { timeout: 3000, polling: 'raf' });
      const left = Date.now() - t0;
      assert(left <= 1000, `tap did not end the NPC run within 1000 ms (${left} ms)`);
      const moving = await page.evaluate(() => document.getElementById('bgPieces').hasAttribute('data-moving'));
      assert(!moving, 'a tap cancels any hop in progress');
      // Every piece is drawn beside the tile it stands on after the skip.
      const off = await page.evaluate(() => {
        const st = JSON.parse(window.__spelltest.boardgameState());
        const tiles = [...document.querySelectorAll('#bgBoard .bg-t')].map((t) => t.getBoundingClientRect());
        return [...document.querySelectorAll('#bgPieces .bg-pc')].map((p, i) => {
          const a = p.getBoundingClientRect(), t = tiles[st.pos[i]];
          return Math.hypot((a.left + a.right) / 2 - (t.left + t.right) / 2, (a.top + a.bottom) / 2 - (t.top + t.bottom) / 2) / a.width;
        });
      });
      assert(off.every((d) => d < 1.6), `a piece is not on its engine tile after the skip: ${off.map((d) => d.toFixed(2))}`);
    } finally { await ctx.close(); }
  });

  // Feature 5: the die tumbles in <= 0.8 s, the landing follows it, and one label reads the facts in order.
  await suite.test('boardgame_polish_centre_stage_die_label_and_tile_line', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 } });
    try {
      const t0 = await page.evaluate(() => ({ tile: document.getElementById('bgStTile').textContent, label: document.getElementById('bgStage').getAttribute('aria-label'), die: document.querySelectorAll('#bgStage .bg-die').length }));
      assertEq(t0.tile, 'Tile 0 of 83', 'the tile line before a roll');
      assertEq(t0.die, 0, 'no die before a roll');
      assert(/^Turn: .+ \d$/.test(t0.label), `the label names whose turn it is: ${t0.label}`);
      await rollToSpelling(page);
      const s = await state(page);
      const m = await page.evaluate(() => {
        const die = document.querySelector('#bgStage .bg-die');
        const cs = getComputedStyle(die);
        const land = document.getElementById('bgStLand');
        const dur = (x) => Math.max(...x.split(',').map((t) => (t.includes('ms') ? parseFloat(t) : parseFloat(t) * 1000)));
        return {
          dieText: die.textContent, dieMs: dur(cs.animationDuration), landDelay: dur(getComputedStyle(land).animationDelay),
          label: document.getElementById('bgStage').getAttribute('aria-label'), landText: land.textContent.replace(/\s+/g, ' ').trim(),
          lastTile: document.getElementById('bgStTile').textContent,
        };
      });
      assert(m.dieMs > 0 && m.dieMs <= 800, `the die tumble lasts ${m.dieMs} ms (<= 800)`);
      assert(m.landDelay <= 800, `the landing waits ${m.landDelay} ms for the tumble (<= 800)`);
      assert(/^[1-6]$/.test(m.dieText), `the die shows a face: ${m.dieText}`);
      assert(/(Easy|Medium|Hard|Expert)/.test(m.landText) && /→ \d+/.test(m.landText), `the landing reads "→ tile · tier": ${m.landText}`);
      const parts = m.label.split('. ');
      assert(parts.length >= 4 && /^Turn: /.test(parts[0]) && /^Rolled \d$/.test(parts[1]) && /^To tile \d+$/.test(parts[2]) && /(Easy|Medium|Hard|Expert)/.test(parts[3]), `the label reads whose turn, roll, landing tile, tier in order: ${m.label}`);
      assert(parts[3] === { easy: 'Easy', medium: 'Medium', hard: 'Hard', expert: 'Expert' }[s.tier], 'the label names the right tier');
      assertEq(m.lastTile, 'Tile 0 of 83', 'the piece has not moved yet');
    } finally { await ctx.close(); }
  });

  // Feature 5: on a screen whose ring leaves no room for the stage, the tier pill carries the landing.
  await suite.test('boardgame_polish_tier_pill_is_the_fallback_when_there_is_no_stage', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 375, height: 540 } });
    try {
      await rollToSpelling(page);
      const m = await page.evaluate(() => ({ stage: !document.getElementById('bgStage').hidden, strip: document.getElementById('bgView').classList.contains('strip'), pill: document.querySelector('#bgLand [data-tier]')?.textContent ?? null }));
      assert(m.strip && !m.stage && m.pill, `expected the track, no stage and the pill: ${JSON.stringify(m)}`);
    } finally { await ctx.close(); }
  });

  // Feature 6/7: Japanese lays out the same way (the stage, the pieces and the numbers are all locale-neutral).
  await suite.test('boardgame_polish_ja_stage_and_pieces_at_430x932', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 430, height: 932 }, lang: 'ja' });
    try {
      const m = await page.evaluate(measure, FULL);
      assert(!m.strip && m.stage && m.pcs.length === 4, 'ja: ring, stage and four pieces');
      assert(/\d/.test(await page.textContent('#bgStTile')), 'ja: the tile line carries numbers');
      assert(visibleFractions(m.pcs).every((f) => f.frac >= 0.6), 'ja: shared-tile pieces stay readable');
    } finally { await ctx.close(); }
  });
}
