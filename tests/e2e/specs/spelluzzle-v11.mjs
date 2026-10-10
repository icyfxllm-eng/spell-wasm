// spelluzzle-v11.spec -- CC-SPELLUZZLE v1.1 Phase E: pencil runes (F16), the rune strip (F17), the
// ripple (F18). Each sits behind its own flag, default off; with all of them off the mode is v1 (I13).
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const V1 = () => { localStorage.setItem('spell_flag_spelluzzle', 'on'); localStorage.setItem('spell_spz_how_v1', '1'); };
const E = () => {
  localStorage.setItem('spell_flag_spelluzzle', 'on');
  localStorage.setItem('spell_flag_spelluzzle_pencil', 'on');
  localStorage.setItem('spell_flag_spelluzzle_strip', 'on');
  localStorage.setItem('spell_flag_spelluzzle_ripple', 'on');
  localStorage.setItem('spell_spz_how_v1', '1');
};

export async function run(browser, base, suite) {
  const open = async ({ init = E, viewport = { width: 390, height: 844 }, age, reduce = false } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init, ...(age ? { age } : {}) });
    if (reduce) await o.page.emulateMedia({ reducedMotion: 'reduce' });
    await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
    await o.page.waitForSelector('#szScreen.show');
    await o.page.click('#szStart');
    await o.page.waitForSelector('#szBoard .sz-row');
    return o;
  };
  const words = (page) => page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
  const key = (page, c) => page.click(`#szKb [data-k="u:${c}"]`);
  const typeWord = async (page, w) => { for (const c of w) await key(page, c); };
  const select = (page, i) => page.click(`#szBoard .sz-row[data-slot="${i}"] .sz-cells`);
  const hold = async (page, selector) => {
    const b = await page.evaluate((s) => { const r = document.querySelector(s).getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; }, selector);
    await page.mouse.move(b.x, b.y);
    await page.mouse.down();
    await page.waitForTimeout(650);
    await page.mouse.up();
  };

  await suite.test('spelluzzle_v11_flags_off_is_v1', async () => {
    const { ctx, page } = await open({ init: V1 });
    try {
      const m = await page.evaluate(() => ({
        strip: document.querySelectorAll('#szRunes .sz-s').length,
        v11: document.getElementById('szScreen').classList.contains('v11'),
        pencil: document.querySelectorAll('.sz-pencil').length,
      }));
      assert(m.strip === 0 && !m.v11 && m.pencil === 0, `v1.1 markup with its flags off: ${JSON.stringify(m)}`);
      await hold(page, '#szBoard .sz-row[data-slot="0"] .sz-cell');
      await page.waitForTimeout(100);
      const after = await page.evaluate(() => ({ target: document.querySelectorAll('.sz-cell.target').length, keys: Object.keys(localStorage).filter((k) => k.startsWith('spell_spz_pencil')) }));
      assert(after.target === 0 && after.keys.length === 0, `a long-press did something with the flag off: ${JSON.stringify(after)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_pencil_marks_are_notes', async () => {
    const { ctx, page } = await open();
    try {
      const ws = await words(page);
      // Long-press a cell of slot 0, then the next key is its pencil mark.
      await hold(page, '#szBoard .sz-row[data-slot="0"] .sz-cell');
      await page.waitForSelector('#szBoard .sz-cell.target');
      assert(await page.evaluate(() => !document.getElementById('szKb').hidden), 'the keyboard did not open for a pencil mark');
      await key(page, 'q');
      const m = await page.evaluate(() => {
        const marked = [...document.querySelectorAll('#szBoard .sz-cell.pencilled')];
        const runes = new Set(marked.map((c) => c.dataset.rune));
        const same = [...document.querySelectorAll('#szBoard .sz-cell')].filter((c) => runes.has(c.dataset.rune)).length;
        return {
          n: marked.length, runes: runes.size, same,
          text: [...document.querySelectorAll('.sz-pencil')].map((x) => x.textContent).join(''),
          msg: document.getElementById('szMsg').textContent.trim(),
          amber: document.querySelectorAll('#szBoard .sz-cell.amber').length,
          typed: document.querySelectorAll('#szBoard .sz-cell.typed').length,
          clear: !document.getElementById('szPencilClear').hidden,
          store: Object.keys(localStorage).filter((k) => k.startsWith('spell_spz_pencil')).length,
        };
      });
      assert(m.runes === 1 && m.n === m.same && m.n >= 1, `the mark is not on every cell of one rune: ${JSON.stringify(m)}`);
      assert(/^q+$/.test(m.text), `mark text ${m.text}`);
      assert(m.msg === '' && m.amber === 0 && m.typed === 0, `a pencil mark reacted: ${JSON.stringify(m)}`);
      assert(m.clear && m.store === 1, `clear button or save missing: ${JSON.stringify(m)}`);
      // The mark is only a note: commit a word that decodes the rune and the mark is hidden.
      await select(page, 0);
      await typeWord(page, ws[0]);
      const hidden = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.pencilled').length);
      assert(hidden === 0, 'a mark is drawn on a decoded rune');
      // Clear the word: the rune is undecoded again and the mark returns.
      await page.click('#szBoard .sz-row[data-slot="0"] [data-clear]');
      assert((await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.pencilled').length)) > 0, 'the mark did not come back');
      // Clear pencil marks removes them all.
      await page.click('#szPencilClear');
      assert((await page.evaluate(() => document.querySelectorAll('.sz-pencil').length)) === 0, 'Clear pencil marks left a mark');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_pencil_survives_leaving', async () => {
    const { ctx, page } = await open();
    try {
      await hold(page, '#szBoard .sz-row[data-slot="1"] .sz-cell');
      await key(page, 'e');
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      assert((await page.evaluate(() => document.querySelectorAll('.sz-pencil').length)) > 0, 'the pencil mark did not come back on resume');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_strip_counts_order_and_highlight', async () => {
    const { ctx, page } = await open();
    try {
      const m = await page.evaluate(() => {
        const rows = [...document.querySelectorAll('#szRunes .sz-s')].map((e) => ({ rune: e.dataset.rune, n: parseInt(e.querySelector('small').childNodes[0].textContent, 10) }));
        const counts = {};
        for (const c of document.querySelectorAll('#szBoard .sz-cell')) counts[c.dataset.rune] = (counts[c.dataset.rune] || 0) + 1;
        return { rows, counts, ariaOk: [...document.querySelectorAll('#szRunes .sz-s')].every((e) => /^Rune \\d+, \\d+ cells/.test(e.getAttribute('aria-label') || '') || (e.getAttribute('aria-label') || '').length > 0) };
      });
      assert(m.rows.length > 0, 'no strip');
      for (const r of m.rows) assert(r.n === m.counts[r.rune], `strip says ${r.n} for rune ${r.rune}, board has ${m.counts[r.rune]}`);
      for (let i = 1; i < m.rows.length; i++) assert(m.rows[i - 1].n >= m.rows[i].n, 'strip is not most cells first');
      // Tap lights the rune's cells; tap again clears.
      await page.click('#szRunes .sz-s');
      const lit = await page.evaluate(() => ({ n: document.querySelectorAll('#szBoard .sz-cell.hl').length, entry: document.querySelectorAll('#szRunes .sz-s.hl').length }));
      assert(lit.n > 0 && lit.entry === 1, `highlight: ${JSON.stringify(lit)}`);
      await page.click('#szRunes .sz-s');
      assert((await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.hl').length)) === 0, 'second tap did not clear');
      // The strip hides while the keyboard is up.
      await select(page, 0);
      assert(await page.evaluate(() => document.getElementById('szRunes').hidden), 'the strip stayed up with the keyboard');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_ripple_counts_cells_and_a_tap_skips_it', async () => {
    const { ctx, page } = await open();
    try {
      const ws = await words(page);
      await select(page, 0);
      for (let i = 0; i < ws[0].length - 1; i++) await key(page, ws[0][i]);
      await key(page, ws[0][ws[0].length - 1]);
      // The counter appears at once and the ripple is over inside 1.2 s.
      const count = await page.evaluate(() => document.getElementById('szCount').textContent.trim());
      assert(/^\+\d+ cells$/.test(count), `counter: ${count}`);
      const n = parseInt(count.slice(1), 10);
      await page.waitForTimeout(1350);
      const done = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.decoded').length);
      assert(done === n, `the ripple counted ${n} cells but ${done} are decoded`);
      // A second word, skipped with a tap: nothing changes afterwards, so it was already finished.
      await select(page, 1);
      await typeWord(page, ws[1]);
      await page.click('#szBoard .sz-row[data-slot="0"] .sz-cells');
      const snap = () => page.evaluate(() => document.getElementById('szBoard').innerHTML);
      const at = await snap();
      await page.waitForTimeout(1400);
      assert((await snap()) === at, 'the board kept changing after a tap: the ripple was not skipped');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_reduce_motion_has_no_flip_and_still_counts', async () => {
    const { ctx, page } = await open({ reduce: true });
    try {
      const ws = await words(page);
      await select(page, 0);
      await typeWord(page, ws[0]);
      // No held-back runes: every decoded cell is there on the next frame.
      const m = await page.evaluate(() => ({ decoded: document.querySelectorAll('#szBoard .sz-cell.decoded').length, count: document.getElementById('szCount').textContent.trim() }));
      assert(m.decoded === parseInt(m.count.slice(1), 10) && /^\+\d+ cells$/.test(m.count), `reduce motion: ${JSON.stringify(m)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_layout_at_375x667', async () => {
    const { ctx, page } = await open({ viewport: { width: 375, height: 667 } });
    try {
      await hold(page, '#szBoard .sz-row[data-slot="0"] .sz-cell');
      await key(page, 'e');
      await page.click('#szBoard .sz-row[data-slot="3"] .sz-cells'); // select a slot to bring the keyboard up
      await page.click('#szBoard .sz-row[data-slot="3"] .sz-cells');
      const m = await page.evaluate(() => {
        const cell = document.querySelector('#szBoard .sz-cell.pencilled');
        const out = { over: document.documentElement.scrollWidth - innerWidth };
        if (cell) {
          const c = cell.getBoundingClientRect(), p = cell.querySelector('.sz-pencil').getBoundingClientRect(), g = cell.querySelector('.sz-rune').getBoundingClientRect();
          out.cell = c.width; out.pencilFont = parseFloat(getComputedStyle(cell.querySelector('.sz-pencil')).fontSize);
          out.inside = p.left >= c.left - 0.5 && p.right <= c.right + 0.5 && p.top >= c.top - 0.5 && p.bottom <= c.bottom + 0.5;
          out.overlap = !(p.right <= g.left || p.left >= g.right || p.bottom <= g.top || p.top >= g.bottom);
        }
        const board = document.getElementById('szBoard');
        out.scrolls = board.scrollHeight - board.clientHeight;
        return out;
      });
      assert(m.over <= 0, `horizontal overflow ${m.over}`);
      assert(m.cell >= 32 && m.pencilFont >= 11, `pencilled cell ${m.cell}px, mark ${m.pencilFont}px`);
      assert(m.inside && !m.overlap, `the mark ${m.inside ? 'overlaps the glyph' : 'is outside the cell'}`);
      assert(m.scrolls <= 1, `board scrolls ${m.scrolls}px`);
      // Strip: idle view, all entries visible in the viewport.
      await page.click('#szNew');
      await page.waitForSelector('#szBoard .sz-row');
      const s = await page.evaluate(() => {
        const e = [...document.querySelectorAll('#szRunes .sz-s')];
        const rs = e.map((x) => x.getBoundingClientRect());
        return { n: e.length, left: Math.min(...rs.map((r) => r.left)), right: Math.max(...rs.map((r) => r.right)), bottom: Math.max(...rs.map((r) => r.bottom)), H: innerHeight, W: innerWidth, minW: Math.min(...rs.map((r) => r.width)) };
      });
      assert(s.left >= 0 && s.right <= s.W + 0.5 && s.bottom <= s.H, `strip off screen ${JSON.stringify(s)}`);
      assert(s.minW >= 28, `strip entries ${s.minW}px wide`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_v11_jr_gets_pencil_strip_ripple_with_a_flat_tick', async () => {
    const { ctx, page } = await open({ age: KID });
    try {
      assert((await page.evaluate(() => document.querySelectorAll('#szRunes .sz-s').length)) > 0, 'no strip for Spell Jr');
      await hold(page, '#szBoard .sz-row[data-slot="0"] .sz-cell');
      await key(page, 'a');
      assert((await page.evaluate(() => document.querySelectorAll('.sz-pencil').length)) > 0, 'no pencil for Spell Jr');
    } finally { await ctx.close(); }
  });
}
