// spelluzzle-g.spec -- CC-SPELLUZZLE v1.1 Phase G: Expeditions (F19) and My Words boards (F21).
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const LIST_WORDS = ['bicycle', 'kitchen', 'monster', 'picture', 'brother', 'holiday', 'library', 'machine'];
const lists = (words) => JSON.stringify({
  v: 1, nextId: 3, migrated: true, selection: [],
  lists: [{ id: 'l1', profile: '', name: 'Week 1', createdAt: 1, updatedAt: 1, source: 'Manual', deletedAt: null, order: 'Mixed',
    entries: words.map((t) => ({ text: t, lang: 'en', addedAt: 1, starred: false })) }],
});
const initWith = (extra) => `(() => {
  localStorage.setItem('spell_flag_spelluzzle', 'on');
  localStorage.setItem('spell_spz_how_v1', '1');
  localStorage.setItem('spell_spz_how2_v1', '1');
  ${extra}
})()`;
const mk = (src) => new Function(src);

export async function run(browser, base, suite) {
  const open = async ({ extra = '', age, viewport = { width: 390, height: 844 }, type, tier, list = false } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init: mk(initWith(extra)), ...(age ? { age } : {}) });
    await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
    await o.page.waitForSelector('#szScreen.show');
    if (type) await o.page.click(`#szTypes [data-type="${type}"]`);
    if (tier) await o.page.click(`#szTiers [data-tier="${tier}"]`);
    if (list) await o.page.waitForSelector('#szLists [data-list]');
    return o;
  };
  const go = async (page) => {
    await page.click('#szStart');
    await page.waitForSelector('#szBoard .sz-row', { timeout: 10000 });
    if (await page.evaluate(() => !document.getElementById('szHow').hidden)) await page.click('#szHowOk');
  };
  const words = (page) => page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
  const typeWord = async (page, w) => { for (const c of w) await page.click(`#szKb [data-k="u:${c}"]`); };
  const select = (page, i) => page.click(`#szBoard .sz-row[data-slot="${i}"] .sz-cells`);
  const solveBoard = async (page) => {
    const ws = await words(page);
    for (let i = 0; i < ws.length; i++) { await select(page, i); await typeWord(page, ws[i]); }
    await domSettled(page);
    return ws;
  };
  const EXP = "localStorage.setItem('spell_flag_spelluzzle_expedition','on');";
  const MW = (w) => `localStorage.setItem('spell_flag_spelluzzle_mywords','on'); localStorage.setItem('byear_word_lists_v1', ${JSON.stringify(lists(w))});`;

  await suite.test('spelluzzle_expedition_is_absent_with_its_flag_off_and_for_jr', async () => {
    for (const [extra, age] of [['', null], [EXP, KID]]) {
      const o = await open({ extra, ...(age ? { age } : {}) });
      try {
        const n = await o.page.evaluate(() => [...document.querySelectorAll('#szTypes [data-type]')].filter((b) => b.dataset.type === 'expedition').length);
        assert(n === 0, 'an Expedition entry showed where it must be absent');
      } finally { await o.ctx.close(); }
    }
  });

  await suite.test('spelluzzle_expedition_carries_the_cipher_to_board_2_and_the_vault', async () => {
    const { ctx, page } = await open({ extra: EXP, type: 'expedition', tier: 'medium' });
    try {
      await go(page);
      const h1 = await page.evaluate(() => document.getElementById('szParLine').textContent.trim());
      assert(h1 === 'Board 1 of 3', `header ${h1}`);
      const m1 = await page.evaluate(() => ({ rows: document.querySelectorAll('#szBoard .sz-row').length, quiet: document.querySelectorAll('#szBoard .sz-row.quiet').length, decoded: document.querySelectorAll('#szBoard .sz-cell.decoded').length }));
      assert(m1.rows === 5 && m1.quiet === 0 && m1.decoded === 0, `board 1 ${JSON.stringify(m1)}`);
      assert(await page.evaluate(() => document.getElementById('szNext').hidden), 'a way on before board 1 is solved');
      await solveBoard(page);
      assert(await page.evaluate(() => !document.getElementById('szNext').hidden), 'no way on after board 1');
      await page.click('#szNext');
      const h2 = await page.evaluate(() => document.getElementById('szParLine').textContent.trim());
      assert(h2 === 'Board 2 of 3', `header ${h2}`);
      const m2 = await page.evaluate(() => ({ rows: document.querySelectorAll('#szBoard .sz-row').length, quiet: document.querySelectorAll('#szBoard .sz-row.quiet').length, decoded: document.querySelectorAll('#szBoard .sz-cell.decoded').length, cells: document.querySelectorAll('#szBoard .sz-cell').length }));
      assert(m2.rows === 6 && m2.quiet === 2, `board 2 ${JSON.stringify(m2)}`);
      assert(m2.decoded > 0 && m2.decoded < m2.cells, `carried runes should show decoded at load: ${m2.decoded} of ${m2.cells}`);
      // A wrong unit over a carried rune clashes: type a word with its first decoded cell altered.
      const ws = await words(page);
      const cell = await page.evaluate(() => {
        const rows = [...document.querySelectorAll('#szBoard .sz-row')];
        for (let i = 0; i < rows.length; i++) { const k = [...rows[i].querySelectorAll('.sz-cell')].findIndex((c) => c.classList.contains('decoded')); if (k >= 0) return { i, k }; }
        return null;
      });
      if (cell) {
        const w = [...ws[cell.i]];
        w[cell.k] = w[cell.k] === 'q' ? 'x' : 'q';
        await select(page, cell.i);
        await typeWord(page, w.join(''));
        assert((await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.amber').length)) >= 1, 'a wrong unit over a carried rune did not turn amber');
        await page.click(`#szBoard .sz-row[data-slot="${cell.i}"] [data-clear]`);
      }
      await solveBoard(page);
      await page.click('#szNext');
      const h3 = await page.evaluate(() => document.getElementById('szParLine').textContent.trim());
      assert(h3 === 'The Vault', `header ${h3}`);
      const m3 = await page.evaluate(() => ({ rows: document.querySelectorAll('#szBoard .sz-row').length, quiet: document.querySelectorAll('#szBoard .sz-row.quiet').length, decoded: document.querySelectorAll('#szBoard .sz-cell.decoded').length, listens: document.querySelectorAll('#szBoard .sz-row.quiet.heard').length }));
      assert(m3.rows === 5 && m3.quiet === 4 && m3.decoded > 0, `Vault ${JSON.stringify(m3)}`);
      // The Vault needs no Listen: solve it by typing alone and the run is over.
      await solveBoard(page);
      const end = await page.evaluate(() => ({ line: [...document.querySelectorAll('#szResult .sz-parres')].map((x) => x.textContent.trim()).join('|'), next: document.getElementById('szNext').hidden, nw: document.getElementById('szNew').textContent.trim() }));
      assert(/★ \d\/9/.test(end.line), `run total missing: ${JSON.stringify(end)}`);
      assert(end.next, 'a way on after the Vault');
      assert(end.nw === 'New expedition', `the button says ${end.nw}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_expedition_resumes_on_the_board_you_left', async () => {
    const { ctx, page } = await open({ extra: EXP, type: 'expedition', tier: 'medium' });
    try {
      await go(page);
      const w1 = await solveBoard(page);
      await page.click('#szNext');
      const w2 = await words(page);
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show');
      assert((await page.evaluate(() => document.querySelector('#szTypes [data-type].on')?.dataset.type)) === 'expedition', 'the last board type was forgotten');
      await go(page);
      assert((await page.evaluate(() => document.getElementById('szParLine').textContent.trim())) === 'Board 2 of 3', 'resume did not return to board 2');
      assert(JSON.stringify(await words(page)) === JSON.stringify(w2), 'resume drew a different board 2');
      // Board 1 can be reopened and read.
      await page.click('#szRunTabs [data-board="0"]');
      assert(JSON.stringify(await words(page)) === JSON.stringify(w1), 'tab 1 did not reopen board 1');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_expedition_vault_layout_at_375x667', async () => {
    const { ctx, page } = await open({ extra: EXP, type: 'expedition', tier: 'hard', viewport: { width: 375, height: 667 } });
    try {
      await go(page);
      await solveBoard(page); await page.click('#szNext');
      await solveBoard(page); await page.click('#szNext');
      await select(page, 0);
      const m = await page.evaluate(() => {
        const board = document.getElementById('szBoard');
        const cells = [...document.querySelectorAll('#szBoard .sz-cell')];
        return { scrolls: board.scrollHeight - board.clientHeight, minW: Math.min(...cells.map((c) => c.getBoundingClientRect().width)), over: document.documentElement.scrollWidth - innerWidth };
      });
      assert(m.over <= 0 && m.minW >= 32, JSON.stringify(m));
      assert(m.scrolls <= 1, `the Vault scrolls ${m.scrolls}px at 375x667`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_my_words_builds_a_board_from_the_list_and_badges_top_ups', async () => {
    const { ctx, page } = await open({ extra: MW(LIST_WORDS), type: 'mywords', list: true });
    try {
      const pick = await page.evaluate(() => document.getElementById('szLists').textContent);
      assert(/Week 1/.test(pick), `the picker: ${pick}`);
      await go(page);
      const m = await page.evaluate(() => ({ rows: document.querySelectorAll('#szBoard .sz-row').length, badges: document.querySelectorAll('#szBoard .sz-topup').length, silent: document.querySelectorAll('#szBoard .sz-row[data-kind="silent"]').length }));
      assert(m.rows === 6 && m.silent === 0, JSON.stringify(m));
      const ws = await words(page);
      const listed = ws.filter((w) => LIST_WORDS.includes(w)).length;
      assert(listed >= 3, `only ${listed} list words on the board: ${ws}`);
      assert(m.badges === 6 - listed, `${m.badges} badges for ${6 - listed} top-ups`);
      const label = await page.evaluate(() => document.querySelector('#szBoard .sz-topup')?.getAttribute('aria-label'));
      if (m.badges > 0) assert(label === 'Added from the word bank', `badge label ${label}`);
      await solveBoard(page);
      const stars = await page.evaluate(() => document.querySelectorAll('#szResult .sz-star').length);
      assert(stars === 3, 'stars are shown when the board is solved');
      // Not kept (D45): no stars store for My Words.
      assert((await page.evaluate(() => Object.keys(localStorage).filter((k) => k.startsWith('spell_spz_stars_v1')).length)) === 0, 'My Words kept its stars');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_my_words_needs_three_matched_words_and_leaves_the_list_alone', async () => {
    const o = await open({ extra: MW(['bicycle', 'kitchen', 'zzzxq', 'qqqqqx']) });
    try {
      const n = await o.page.evaluate(() => [...document.querySelectorAll('#szTypes [data-type]')].filter((b) => b.dataset.type === 'mywords').length);
      assert(n === 0, 'My Words was offered for a list with fewer than three matched words');
    } finally { await o.ctx.close(); }
    const o2 = await open({ extra: MW([...LIST_WORDS, 'zzzxq']), type: 'mywords', list: true });
    try {
      await go(o2.page);
      const ws = await words(o2.page);
      assert(!ws.includes('zzzxq'), 'a word with no bank match appeared');
      assert((await o2.page.evaluate(() => JSON.parse(localStorage.getItem('byear_word_lists_v1')).lists[0].entries.length)) === 9, 'the list was changed');
    } finally { await o2.ctx.close(); }
  });

  await suite.test('spelluzzle_my_words_for_jr_has_no_sound_alikes', async () => {
    const o = await open({ extra: MW(LIST_WORDS), type: 'mywords', list: true, age: KID });
    try {
      await go(o.page);
      const rows = await o.page.evaluate(() => document.querySelectorAll('#szBoard .sz-row').length);
      assert(rows === 6, `a Jr My Words board has ${rows} rows`);
    } finally { await o.ctx.close(); }
  });
}
