// boardgame-boost.spec -- CC-BOARD-GAME-POLISH v1, Phase D (Features 2 and 3: boost tiles, hot streak).
//   streak chips with the "+1" pending marker, the Ward marker, the effects named in the centre
//   stage, a revealed boost vs a revealed trap told apart without colour (I-P11), chips fitting a
//   375-wide row, and an NPC run with the effects on staying inside the 7 s budget.
// Effects that dice would take minutes to reach are staged with the dev-build seam
// __spelltest.boardgameForce (place a special on the tile the current spelling moves to, grant a Ward
// or a pending bonus); the arrival, the effect and the text are the real code paths.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const FLAGS = () => {
  localStorage.setItem('spell_flag_boardgame', 'on');
  localStorage.setItem('spell_flag_boardStretch', 'off');
  localStorage.setItem('spell_flag_boardBoosts', 'on');
  localStorage.setItem('spell_flag_boardStreak', 'on');
};
const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
const raw = (page) => page.evaluate(() => window.__spelltest.boardgameState());
const cite = (w) => w.split('|')[0];

async function typeIt(page, typed) {
  for (const ch of typed) await page.click(`#bgKeys [data-k="u:${ch}"]`);
}

async function openGame(browser, base, { viewport = { width: 393, height: 852 }, mode = 'pass', count = 2, size = 'sprint' } = {}) {
  const o = await openApp(browser, base, { lang: 'en', viewport, init: FLAGS });
  const { page } = o;
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
  await page.click(`[data-bg="mode:${mode}"]`);
  await page.click(`[data-bg="count:${count}"]`);
  if (size) await page.click(`[data-bg="size:${size}"]`);
  await page.click('#bgStart');
  await page.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
  if (mode === 'pass') {
    await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
    await page.click('#bgHandGo');
  }
  await domSettled(page);
  return o;
}

/// Record every text the stage's landing slot and the result chips ever show (effects flash by).
async function watchText(page) {
  await page.evaluate(() => {
    window.__seen = new Set();
    const grab = () => {
      for (const id of ['bgStLand', 'bgResults', 'bgLand']) {
        const t = document.getElementById(id)?.textContent.replace(/\s+/g, ' ').trim();
        if (t) window.__seen.add(`${id}: ${t}`);
      }
      const l = document.querySelector('#bgStLand [data-land]');
      if (l) window.__seen.add(`land:${l.dataset.land}: ${l.textContent.trim()}`);
    };
    new MutationObserver(grab).observe(document.getElementById('bgScreen'), { subtree: true, childList: true, characterData: true });
    grab();
  });
}
const seen = (page) => page.evaluate(() => [...window.__seen].join(' | '));

/// Roll and wait for the spelling phase (or the end).
async function rollToSpelling(page) {
  await page.click('#bgOrb');
  await page.waitForFunction(() => {
    const s = JSON.parse(window.__spelltest.boardgameState() || 'null');
    return s && (s.phase === 'AwaitSpelling' || s.phase === 'Finished');
  }, null, { timeout: 5000 });
  await domSettled(page);
}

async function submit(page, right) {
  const s = await state(page);
  const before = await raw(page);
  await typeIt(page, right ? cite(s.word) : 'q');
  await page.click('#bgGo');
  await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 5000 });
}

/// Clear whichever card sits between turns.
async function passCards(page) {
  for (let i = 0; i < 6; i++) {
    if (await page.isVisible('#bgMiss')) { await page.click('#bgMissGo'); continue; }
    if (await page.isVisible('#bgHand')) { await page.click('#bgHandGo'); await domSettled(page); continue; }
    break;
  }
}

/// Wait until the engine hands the turn to a human who can roll: animations and cards finished, a Switch Tiles
/// target declined, a trap word (Long Word, Double Expert) spelled right.
async function untilRoll(page, timeout = 15000) {
  const end = Date.now() + timeout;
  while (Date.now() < end) {
    await passCards(page);
    const s = await state(page);
    if (s.phase === 'Finished' || (s.phase === 'AwaitRoll' && !s.npc && !s.hand)) break;
    const before = await raw(page);
    if (s.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); }
    else if (s.phase === 'AwaitSpelling' && !s.npc) { await typeIt(page, cite(s.word)); await page.click('#bgGo'); }
    else { await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b || !document.getElementById('bgHand').hidden || !document.getElementById('bgMiss').hidden, before, { timeout: 4000 }).catch(() => {}); continue; }
    await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 4000 }).catch(() => {});
  }
  await passCards(page);
  await domSettled(page);
}

const chips = (page) =>
  page.$$eval('#bgChips .bg-chip', (cs) =>
    cs.map((c) => ({
      label: c.getAttribute('aria-label'),
      sk: c.querySelector('.bg-sk')?.textContent.trim() ?? null,
      bonus: !!c.querySelector('.bg-sk.bonus'),
      ward: !!c.querySelector('[data-ward]'),
    })),
  );

export async function run(browser, base, suite) {
  // Feature 3: every chip shows its streak count; five right answers in a row turn it into the "+1" marker, and
  // the next roll spends the bonus (+1 on the move) whether or not the spelling is right.
  await suite.test('boardgame_boost_streak_chips_count_then_show_the_plus_one_marker', async () => {
    let earned = false;
    for (let attempt = 0; attempt < 4 && !earned; attempt++) {
      const { ctx, page } = await openGame(browser, base, { count: 2 });
      try {
        const s0 = await state(page);
        assert(s0.streakOn && s0.boostsOn, 'both rule flags reached the game');
        let c = await chips(page);
        assertEq(c.length, 2, 'two chips');
        assert(c.every((x) => x.sk.replace(/\D/g, '') === '0' && !x.bonus), `every chip starts at streak 0: ${JSON.stringify(c)}`);
        let finished = false;
        for (let turn = 0; turn < 40 && !earned && !finished; turn++) {
          await untilRoll(page);
          let s = await state(page);
          if (s.phase === 'Finished') { finished = true; break; }
          const me = s.seat;
          if (s.bonus[me]) {
            // Earned on a trap word inside the last untilRoll: the marker must be up all the same.
            c = await chips(page);
            assert(c[me].bonus && c[me].sk.includes('+1'), `the chip shows the +1 marker: ${JSON.stringify(c[me])}`);
            earned = true;
            break;
          }
          await rollToSpelling(page);
          s = await state(page);
          if (s.phase === 'Finished') { finished = true; break; }
          await submit(page, true);
          await domSettled(page);
          s = await state(page);
          c = await chips(page);
          if (s.bonus[me]) {
            // The marker replaces the count while the bonus waits; the label says so in words.
            assert(c[me].bonus && c[me].sk.includes('+1'), `the chip shows the +1 marker: ${JSON.stringify(c[me])}`);
            assert(/\+1 on the next roll/.test(c[me].label), `the chip's label spells the bonus out: ${c[me].label}`);
            assertEq(s.streak[me], 0, 'the counter reset when the bonus was earned');
            earned = true;
            break;
          }
          assert(c[me].sk.replace(/\D/g, '') === String(s.streak[me]) && s.streak[me] >= 1 && s.streak[me] <= 4, `the chip shows the streak (0..4): ${JSON.stringify(c[me])} vs ${s.streak[me]}`);
        }
        if (!earned) continue;
        // Spend it: the next roll actually taken carries +1.
        await untilRoll(page);
        let s = await state(page);
        // The other seat may have played in between; hand the turn back to the one holding the bonus.
        for (let g = 0; g < 6 && !s.bonus[s.seat] && s.phase !== 'Finished'; g++) {
          await rollToSpelling(page);
          await submit(page, true);
          await untilRoll(page);
          s = await state(page);
        }
        if (s.phase === 'Finished' || !s.bonus[s.seat]) { earned = false; continue; }
        const holder = s.seat;
        const from = s.pos[holder];
        await page.click('#bgOrb');
        await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState()).phase === 'AwaitSpelling', null, { timeout: 5000 });
        s = await state(page);
        assert(!s.bonus[holder], 'the roll itself spent the bonus');
        const lab = await page.evaluate(() => document.getElementById('bgStage').getAttribute('aria-label'));
        const m = /Rolled (\d)\. To tile (\d+)/.exec(lab);
        assert(m, `the stage names the roll and the landing: ${lab}`);
        assertEq(Number(m[2]), Math.min(from + Number(m[1]) + 1, s.tiles - 1), 'the bonus adds one tile to the roll');
      } finally { await ctx.close(); }
    }
    assert(earned, 'a five-in-a-row streak was reached within four games');
  });

  // Feature 2: a held Ward is a marker on the chip and in its label; the trap it cancels is named in the stage.
  await suite.test('boardgame_boost_ward_marker_and_blocked_trap_named_in_the_stage', async () => {
    const { ctx, page } = await openGame(browser, base, { count: 2 });
    try {
      await watchText(page);
      let c = await chips(page);
      assert(c.every((x) => !x.ward), 'nobody holds a Ward at the start');
      const me = (await state(page)).seat;
      assert(await page.evaluate((i) => window.__spelltest.boardgameForce(`ward:${i}`), me), 'Ward granted through the seam');
      c = await chips(page);
      assert(c[me].ward && c.filter((x) => x.ward).length === 1, `only the holder shows the marker: ${JSON.stringify(c)}`);
      assert(/Ward/.test(c[me].label), `the label says Ward: ${c[me].label}`);
      await rollToSpelling(page);
      assert(await page.evaluate(() => window.__spelltest.boardgameForce('trap:start')), 'a Back to Start trap staged on the landing tile');
      await submit(page, true);
      await page.waitForFunction(() => { const x = JSON.parse(window.__spelltest.boardgameState()); return x.phase !== 'AwaitSpelling'; }, null, { timeout: 5000 });
      const t = await state(page);
      assert(t.pos[me] > 0, `the Ward kept the piece on the tile (pos ${t.pos[me]}), not back at Start`);
      assertEq(t.ward[me], false, 'the charge is spent');
      await page.waitForFunction(() => /Ward blocked/.test(document.getElementById('bgScreen').textContent) || [...window.__seen].some((x) => /Ward blocked/.test(x)), null, { timeout: 6000 });
      const all = await seen(page);
      assert(/Ward blocked/.test(all), `the blocked trap is named: ${all}`);
      assert(/land:blocked/.test(all), `the stage slot carried it: ${all}`);
    } finally { await ctx.close(); }
  });

  // Feature 2: each boost is named in the centre stage when it fires, and does what it says.
  await suite.test('boardgame_boost_effects_are_named_in_the_stage_and_resolve', async () => {
    for (const [kind, name, check] of [
      ['tailwind', /Tailwind \+3/, (t, s, dest) => assert(t.pos[s.seat] === Math.min(dest + 3, t.tiles - 1), `Tailwind moves 3 more: ${t.pos[s.seat]} vs ${dest}`)],
      ['extra', /Extra Roll/, (t, s) => { assertEq(t.phase, 'AwaitRoll', 'Extra Roll waits for a roll'); assertEq(t.seat, s.seat, 'and it is the same seat'); }],
      ['ward', /Ward gained/, (t, s) => assertEq(t.ward[s.seat], true, 'the Ward charge is held')],
    ]) {
      const { ctx, page } = await openGame(browser, base, { count: 2 });
      try {
        await watchText(page);
        await rollToSpelling(page);
        assert(await page.evaluate((k) => window.__spelltest.boardgameForce(`boost:${k}`), kind), `${kind} staged`);
        const s = await state(page);
        const target = await page.evaluate(() => {
          const lab = document.getElementById('bgStage').getAttribute('aria-label');
          const m = /To tile (\d+)/.exec(lab);
          return m ? Number(m[1]) : -1;
        });
        assert(target > 0, 'the stage names the landing tile');
        await submit(page, true);
        await page.waitForFunction(() => { const x = JSON.parse(window.__spelltest.boardgameState()); return x.phase !== 'AwaitSpelling'; }, null, { timeout: 5000 });
        const t = await state(page);
        check(t, s, target);
        await page.waitForFunction((re) => [...window.__seen].some((x) => new RegExp(re).test(x)), name.source, { timeout: 6000 });
        const all = await seen(page);
        assert(name.test(all), `${kind}: the effect is named: ${all}`);
        assert(/land:boost/.test(all), `${kind}: named in the stage's landing slot: ${all}`);
        if (kind !== 'extra') await untilRoll(page);
        const g = await state(page);
        assert(!(await page.textContent('#bgScreen')).includes('undefined'), 'no undefined text on screen');
        // The tile stays marked for the rest of the game.
        const diamonds = await page.$$eval('#bgBoard .bg-boostbg', (e) => e.length);
        assert(diamonds >= 1, `${kind}: the revealed boost stays on the ring as a diamond (${g.phase})`);
      } finally { await ctx.close(); }
    }
  });

  // I-P11: a revealed boost and a revealed trap differ in shape (diamond vs circle) and glyph, not only in colour.
  await suite.test('boardgame_boost_grayscale_boost_diamond_vs_trap_circle', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 393, height: 852 }, count: 2, size: 'full' });
    try {
      const s = await state(page);
      assert(s.boosts.length === 3, `the Full board holds three boosts (D-P24): ${JSON.stringify(s.boosts)}`);
      const [bt, bk] = s.boosts[0];
      // A plain tile: one that is neither a boost nor a trap, nor an end.
      const taken = new Set(s.boosts.map((b) => b[0]));
      let plain = 5;
      while (taken.has(plain)) plain++;
      await page.evaluate(([a, b]) => { window.__spelltest.boardgameMark(a); window.__spelltest.boardgameMark(b); }, [bt, plain]);
      await domSettled(page);
      const r = await page.evaluate(([bt, plain]) => {
        const board = document.getElementById('bgBoard');
        const rects = [...board.querySelectorAll('.bg-t')];
        const inside = (el, tile) => {
          const a = el.getBoundingClientRect(), b = tile.getBoundingClientRect();
          const cx = (a.left + a.right) / 2, cy = (a.top + a.bottom) / 2;
          return cx > b.left && cx < b.right && cy > b.top && cy < b.bottom;
        };
        const on = (sel, tile) => [...board.querySelectorAll(sel)].filter((e) => inside(e, rects[tile]));
        return {
          boostShape: on('.bg-boostbg', bt).map((e) => e.tagName),
          boostGlyph: on('.bg-boost', bt).map((e) => e.textContent),
          trapShape: on('.bg-trapbg', plain).map((e) => e.tagName),
          trapGlyph: on('.bg-trap', plain).map((e) => e.textContent),
          trapOnBoost: on('.bg-trapbg', bt).length,
        };
      }, [bt, plain]);
      assertEq(r.boostShape.join(), 'rect', 'a revealed boost sits on a diamond');
      assertEq(r.trapShape.join(), 'circle', 'a revealed trap sits on a circle');
      assertEq(r.trapOnBoost, 0, 'a boost tile never also draws the trap circle');
      assert(r.boostGlyph.length === 1 && r.trapGlyph.length === 1 && r.boostGlyph[0] !== r.trapGlyph[0], `the glyphs differ: ${r.boostGlyph} vs ${r.trapGlyph}`);
      assert(r.boostGlyph[0].trim().length > 0 && bk, `the boost carries its own glyph (${bk})`);
      await page.addStyleTag({ content: '#bgBoard{filter:grayscale(1)!important}' });
      const png = await page.locator('#bgBoard').screenshot();
      assert(png.length > 1000, 'a grayscale snapshot was taken');
    } finally { await ctx.close(); }
  });

  // The four chips with a streak count, a pending +1 and a Ward still fit a 375-wide row without overlap or scroll.
  await suite.test('boardgame_boost_four_chips_with_markers_fit_375', async () => {
    const { ctx, page } = await openGame(browser, base, { viewport: { width: 375, height: 667 }, count: 4 });
    try {
      await page.evaluate(() => {
        for (const i of [0, 1, 2, 3]) window.__spelltest.boardgameForce(`ward:${i}`);
        window.__spelltest.boardgameForce('bonus:0');
      });
      await domSettled(page);
      const c = await chips(page);
      assertEq(c.length, 4, 'four chips');
      assert(c[0].bonus && c.every((x) => x.ward), 'a +1 pending and four Wards shown');
      const g = await page.evaluate(() => {
        const row = document.getElementById('bgChips');
        const rs = [...row.querySelectorAll('.bg-chip')].map((e) => { const b = e.getBoundingClientRect(); return { l: b.left, r: b.right, t: b.top, b: b.bottom }; });
        const rb = row.getBoundingClientRect();
        return { rs, rowL: rb.left, rowR: rb.right, vw: document.documentElement.clientWidth, sw: row.scrollWidth, cw: row.clientWidth, docSw: document.documentElement.scrollWidth };
      });
      assert(g.rs.every((r) => r.l >= -0.5 && r.r <= g.vw + 0.5), `every chip is inside the 375 viewport: ${JSON.stringify(g.rs)}`);
      assert(g.sw <= g.cw + 1, `the chip row does not scroll (${g.sw} > ${g.cw})`);
      assert(g.docSw <= g.vw + 1, 'the page does not scroll sideways');
      const rows = new Set(g.rs.map((r) => Math.round(r.t)));
      for (let i = 0; i < g.rs.length; i++) for (let j = i + 1; j < g.rs.length; j++) {
        const a = g.rs[i], b = g.rs[j];
        const overlap = a.l < b.r - 0.5 && b.l < a.r - 0.5 && a.t < b.b - 0.5 && b.t < a.b - 0.5;
        assert(!overlap, `chips ${i} and ${j} overlap: ${JSON.stringify([a, b])}`);
      }
      assert(rows.size <= 2, `the chips wrap at most onto a second line (${rows.size} lines)`);
    } finally { await ctx.close(); }
  });

  // D10 with effects on: a whole solo game, every NPC run (untapped) stays inside 7 s for three NPCs; a tap ends the
  // run within a second and every piece is drawn on the tile the engine says it stands on (final positions applied).
  await suite.test('boardgame_boost_npc_runs_stay_inside_the_budget_and_tap_applies_final_positions', async () => {
    const { ctx, page } = await openGame(browser, base, { mode: 'solo', count: 3, size: 'sprint' });
    try {
      let untappedMax = 0, tappedMax = 0, runs = 0, tapRuns = 0, longRuns = 0;
      // Steps per run, counted in the page (a seat change while an NPC is up is one step); a run can be longer than
      // one round of NPCs when the human's turn is lost, and only the ordinary run is held to the 7 s budget.
      await page.evaluate(() => {
        window.__runSteps = [];
        let n = 0, last = null, was = false;
        const tick = () => {
          const s = JSON.parse(window.__spelltest.boardgameState() || 'null');
          if (s && s.npc) { if (!was || last !== s.seat) n++; last = s.seat; was = true; }
          else if (was) { window.__runSteps.push(n); n = 0; was = false; last = null; }
          requestAnimationFrame(tick);
        };
        tick();
      });
      const piecesOnTiles = () => page.evaluate(() => {
        const st = JSON.parse(window.__spelltest.boardgameState());
        const tiles = [...document.querySelectorAll('#bgBoard .bg-t')].map((t) => t.getBoundingClientRect());
        return [...document.querySelectorAll('#bgPieces .bg-pc')].map((p, i) => {
          const a = p.getBoundingClientRect(), t = tiles[st.pos[i]];
          return Math.hypot((a.left + a.right) / 2 - (t.left + t.right) / 2, (a.top + a.bottom) / 2 - (t.top + t.bottom) / 2) / a.width;
        });
      });
      for (let guard = 0; guard < 120; guard++) {
        const s = await state(page);
        if (s.phase === 'Finished') break;
        if (s.npc) {
          const tap = runs % 2 === 1;
          // The budget is the NPC run's, so the clock starts once the human's own walk has finished.
          await page.waitForFunction(() => !document.getElementById('bgPieces').hasAttribute('data-moving'), null, { timeout: 5000, polling: 'raf' }).catch(() => {});
          let t0 = Date.now();
          if (tap) {
            const c0 = await page.$eval('#bgResults', (e) => e.innerHTML);
            await page.waitForFunction((c) => document.getElementById('bgResults').innerHTML !== c, c0, { timeout: 4000 }).catch(() => {});
            t0 = Date.now();
            await page.click('#bgTitle');
          }
          await page.waitForFunction(() => !JSON.parse(window.__spelltest.boardgameState()).npc, null, { timeout: 12000, polling: 'raf' });
          const dt = Date.now() - t0;
          if (tap) {
            tapRuns++; tappedMax = Math.max(tappedMax, dt);
            const off = await piecesOnTiles();
            assert(off.every((d) => d < 1.6), `after a tap a piece is not on its engine tile: ${off.map((d) => d.toFixed(2))}`);
          } else {
            await page.waitForFunction((k) => window.__runSteps.length > k, runs, { timeout: 2000, polling: 'raf' }).catch(() => {});
            const steps = await page.evaluate((k) => window.__runSteps[k] ?? 0, runs);
            if (steps <= 3) untappedMax = Math.max(untappedMax, dt); else longRuns++;
          }
          runs++;
          continue;
        }
        const before = await raw(page);
        if (s.phase === 'AwaitRoll') { await page.click('#bgOrb'); await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 5000 }).catch(() => {}); continue; }
        if (s.phase === 'AwaitSpelling') { await typeIt(page, cite(s.word)); await page.click('#bgGo'); await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 5000 }).catch(() => {}); continue; }
        if (s.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 5000 }).catch(() => {}); continue; }
        await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState()).phase !== 'AwaitSpelling' || document.getElementById('bgOver')?.hidden === false, null, { timeout: 3000 }).catch(() => {});
      }
      assert(runs >= 4, `several NPC runs were observed (${runs})`);
      assert(untappedMax <= 7000, `an untouched NPC run took ${untappedMax} ms (budget 7000)`);
      assert(tapRuns === 0 || tappedMax <= 1000, `a tapped NPC run took ${tappedMax} ms (budget 1000)`);
    } finally { await ctx.close(); }
  });
}
