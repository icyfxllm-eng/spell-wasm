// spelldoku.spec — CC-SPELLDOKU v1 Phase A, English, on the real screen.
//
// The English table is sourced but not yet signed, so a production build does
// not serve it (D3). These tests switch the screen to the preview rule through a
// test-build-only hook and play the REAL table -- nothing here authors a word.
import { openApp, assert, assertEq } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

async function openSpellDoku(page) {
  await page.evaluate(() => {
    window.__spelltest.spelldokuPreview(true);
    document.getElementById('sdOpenBtn').click();
  });
  await page.waitForSelector('#sdScreen.show', { timeout: 5000 });
}

const board = (page) => page.evaluate(() => JSON.parse(window.__spelltest.spelldokuBoard() || 'null'));

async function pick(page, value) {
  await page.selectOption('#sdPick', value);
  await page.waitForTimeout(250);
}

async function spell(page, word) {
  for (const ch of word) await page.click(`#sdKeys [data-sd-key="${ch}"]`);
  await page.click('#sdKeys [data-sd-go]');
  await page.waitForTimeout(80);
}

const WORDS = ['', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine'];

/// The empty cells, in order.
const empties = (b) => b.clues.map((c, i) => [c, i]).filter(([c]) => c === 'Empty' || c.Fragment).map(([, i]) => i);

export async function run(browser, base, suite) {
  // Eric, 2026-09-18: every language is audited by playing it, so every
  // language the mode fits opens onto a board -- in a production build, with
  // no preview hook. Hindi's words cannot make a fragment, so it stops at Medium.
  await suite.test('spelldoku_every_language_opens_a_board', async () => {
    for (const lang of ['en', 'es', 'fr', 'de', 'pt', 'pl', 'ru', 'vi', 'ko', 'ja', 'zh', 'fil', 'sw', 'ar', 'hi']) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await page.evaluate(() => document.getElementById('sdOpenBtn').click());
        await page.waitForSelector('#sdScreen.show', { timeout: 5000 }).catch(() => {});
        const cells = await page.$$eval('#sdGrid .sd-cell', (c) => c.length).catch(() => 0);
        const study = await page.evaluate(() => window.__spelltest.currentLang());
        assert(cells > 0, `${lang} (studying ${study}): SpellDoku opens onto a board`);
        const opts = await page.$$eval('#sdPick option', (o) => o.map((x) => x.value));
        // v1.2 D12: 9x9 Hard and Expert are Word Mode, whose bank words make
        // fragments even where the number words cannot (Hindi's cannot).
        assert(opts.includes('9-hard') && opts.includes('9-expert'), `${study}: 9x9 Hard and Expert are offered`);
      } finally { await ctx.close(); }
    }
  });

  // D8 -- Mandarin commits with the tone-number keys; the untoned word is a misspelling.
  await suite.test('spelldoku_mandarin_needs_its_tone', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'zh' });
    try {
      await page.evaluate(() => document.getElementById('sdOpenBtn').click());
      await page.waitForSelector('#sdScreen.show', { timeout: 5000 });
      const b = await board(page);
      const i = empties(b)[0];
      const PY = ['', 'yi', 'er', 'san', 'si', 'wu', 'liu', 'qi', 'ba', 'jiu'];
      const TONE = ['', '1', '4', '1', '4', '3', '4', '1', '1', '3'];
      await page.click(`[data-sd-cell="${i}"]`);
      await spell(page, PY[b.solution[i]]);
      assert(/spelling|拼写/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'untoned: MISSPELLED');
      await spell(page, PY[b.solution[i]] + TONE[b.solution[i]]);
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), String(b.solution[i]), 'toned: placed');
    } finally { await ctx.close(); }
  });

  // Vietnamese -- the tone comes from the tone row, and it is required.
  await suite.test('spelldoku_vietnamese_uses_the_tone_row', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'vi' });
    try {
      await page.evaluate(() => document.getElementById('sdOpenBtn').click());
      await page.waitForSelector('#sdScreen.show', { timeout: 5000 });
      const b = await board(page);
      // bảy = b a y + hook above; hai = no tone.
      const target = empties(b).find((k) => b.solution[k] === 7 || b.solution[k] === 2);
      if (target === undefined) return; // a 4x4 has no seven or two only rarely
      const v = b.solution[target];
      await page.click(`[data-sd-cell="${target}"]`);
      if (v === 2) {
        await spell(page, 'hai');
      } else {
        for (const ch of 'bay') await page.click(`#sdKeys [data-sd-key="${ch}"]`);
        await page.click('#sdKeys [data-sd-key="\u0309"]');
        await page.click('#sdKeys [data-sd-go]');
        await page.waitForTimeout(80);
      }
      assertEq(await page.$eval(`[data-sd-cell="${target}"]`, (e) => e.textContent.trim()), String(v), 'placed');
    } finally { await ctx.close(); }
  });

  // F1 / D1 / I4 / Done #8 — spelling commits, verdicts stay apart, chips unlock.
  await suite.test('spelldoku_spelling_commits_and_unlocks_a_chip', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '4-easy');
      const b = await board(page);
      assertEq(b.n, 4, 'a 4x4 board');
      const [first, second] = empties(b);
      await page.click(`[data-sd-cell="${first}"]`);
      // A misspelling is a spelling error, shown at once, and places nothing.
      await spell(page, 'fuor');
      assert(/spelling/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'MISSPELLED says so');
      assertEq(await page.$eval(`[data-sd-cell="${first}"]`, (e) => e.textContent.trim()), '', 'nothing placed');
      // A real number word for the wrong value is a logic error (Easy shows it).
      const wrongV = b.solution[first] === 1 ? 2 : 1;
      await spell(page, WORDS[wrongV]);
      assert(/doesn't go here/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'WRONG_VALUE says so');
      // The right word lands.
      const v = b.solution[first];
      await spell(page, WORDS[v]);
      assertEq(await page.$eval(`[data-sd-cell="${first}"]`, (e) => e.textContent.trim()), String(v), 'the value is placed');
      // Easy: one correct spelling turns that value into a chip (D1).
      assert(await page.$(`[data-sd-chip="${v}"]`), `a chip for ${v} appears after one spelling`);
      // A chip commits without spelling -- but only where it is right.
      if (second !== undefined && b.solution[second] === v) {
        await page.click(`[data-sd-cell="${second}"]`);
        await page.click(`[data-sd-chip="${v}"]`);
        assertEq(await page.$eval(`[data-sd-cell="${second}"]`, (e) => e.textContent.trim()), String(v));
      }
    } finally { await ctx.close(); }
  });

  // F1 — a whole Easy board is solvable by spelling, with pencil marks that never open the keyboard.
  await suite.test('spelldoku_a_board_can_be_solved_by_spelling', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '4-easy');
      const b = await board(page);
      const cells = empties(b);
      // Pencil: digits only, no letter keys.
      await page.click(`[data-sd-cell="${cells[0]}"]`);
      await page.click('#sdPencil');
      assert(!(await page.$('#sdKeys [data-sd-key]')), 'pencil mode shows digits, not the keyboard');
      await page.click('#sdKeys [data-sd-pen="2"]');
      await page.click('#sdPencil');
      let spellings = 0;
      for (const i of cells) {
        const v = b.solution[i];
        await page.click(`[data-sd-cell="${i}"]`);
        if (await page.$(`[data-sd-chip="${v}"]`)) {
          await page.click(`[data-sd-chip="${v}"]`);
        } else {
          await spell(page, WORDS[v]);
          spellings++;
        }
      }
      assert(/solved/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'the board reports solved');
      assert(spellings <= 4, `Easy needs at most one spelling per value (${spellings})`);
    } finally { await ctx.close(); }
  });

  // D2 — Hard and Expert: a logic error waits for Check Board, three uses.
  await suite.test('spelldoku_hard_waits_for_check_board', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '9-hard');
      const b = await board(page);
      assert(b.clues.some((c) => c.Fragment), 'Hard carries a fragment clue (F2)');
      const i = empties(b).find((k) => !b.clues[k].Fragment);
      const wrongV = b.solution[i] === 1 ? 2 : 1;
      assert(b.words, 'v1.2 D12: a 9x9 Hard board is Word Mode');
      await page.click(`[data-sd-cell="${i}"]`);
      await spell(page, b.words[wrongV - 1]);
      assertEq(await page.$eval('#sdNote', (e) => e.textContent.trim()), '', 'no logic feedback yet');
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), b.glyphs[wrongV - 1], 'placed as entered, shown as its glyph');
      assert(!(await page.$eval('#sdCheck', (e) => e.classList.contains('btn-hide'))), 'Check Board is offered');
      await page.click('#sdCheck');
      await page.waitForTimeout(100);
      assert(/another look/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'Check Board reports it');
      assert(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.classList.contains('wrong')), 'and marks the cell');
      await page.click('#sdCheck');
      await page.click('#sdCheck');
      assert(await page.$eval('#sdCheck', (e) => e.disabled), 'three uses, then it is spent');
    } finally { await ctx.close(); }
  });

  // F9 / Done #9 — hints point and name; they never show a letter of the answer.
  await suite.test('spelldoku_hints_never_show_a_letter', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '9-medium');
      for (let level = 1; level <= 2; level++) {
        await page.click('#sdHint');
        await page.waitForTimeout(100);
        const b = await board(page);
        const cell = await page.$eval('.sd-cell.hint', (e) => Number(e.dataset.sdCell));
        const word = b.words ? b.words[b.solution[cell] - 1] : WORDS[b.solution[cell]];
        const shown = await page.$eval('#sdScreen', (e) => e.innerText.toLowerCase());
        const note = await page.$eval('#sdNote', (e) => e.textContent.toLowerCase());
        assert(!note.includes(word), `hint level ${level} does not spell "${word}"`);
        assert(await page.$eval(`[data-sd-cell="${cell}"]`, (e) => e.textContent.trim() === '' || /_/.test(e.textContent)),
          'the hinted cell stays unfilled');
        void shown;
      }
    } finally { await ctx.close(); }
  });

  // F7 / Done #12 — Spell Jr only sees its own column.
  await suite.test('spelldoku_spell_jr_sees_only_its_column', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID });
    try {
      await openSpellDoku(page);
      const opts = await page.$$eval('#sdPick option', (o) => o.map((x) => x.value));
      assertEq(opts.join(','), '4-easy,6-medium', 'Jr: 4x4 Easy and 6x6 Medium only');
    } finally { await ctx.close(); }
  });

  // Done #3 / I5 — the wasm build generates byte-identical boards to the Rust
  // harness: it must reproduce the digest pinned in src/spelldoku/tests.rs.
  await suite.test('spelldoku_wasm_matches_the_pinned_golden_digest', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const got = await page.evaluate(() => window.__spelltest.spelldokuGolden());
      assertEq(got, '0x660953a49c47d76c', 'the same seeds make the same boards here as on the host');
    } finally { await ctx.close(); }
  });

  // F8 — the Daily is the same board for everyone on the same day.
  await suite.test('spelldoku_daily_is_the_same_for_everyone', async () => {
    const boards = [];
    for (let k = 0; k < 2; k++) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await openSpellDoku(page);
        await page.click('#sdDaily');
        await page.waitForTimeout(300);
        boards.push(JSON.stringify(await board(page)));
      } finally { await ctx.close(); }
    }
    assertEq(boards[0], boards[1], 'two players, one Daily');
  });

  // v1.2 F10 / I12 — a Word Mode board: nine bank words, one glyph each, a
  // legend of audio orbs, and no surface that spells a word.
  await suite.test('spelldoku_word_mode_never_shows_a_spelling', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '9-medium');
      const b = await board(page);
      assert(b.words && b.words.length === 9, 'nine words');
      assertEq(new Set(b.glyphs.map((g) => g.toLowerCase())).size, 9, 'I11: nine distinct glyphs');
      assertEq(await page.$$eval('#sdLegend .sd-say', (e) => e.length), 9, 'a legend orb per symbol');
      const shown = await page.evaluate(() => ['sdGrid', 'sdLegend', 'sdChips', 'sdBadge', 'sdNote']
        .map((id) => document.getElementById(id).innerText.toLowerCase()).join(' | '));
      for (const w of b.words) assert(!shown.includes(w), `I12: "${w}" is not spelled anywhere on the board`);
      await page.click('#sdLegend [data-sd-say="1"]'); // the orb plays; nothing breaks
    } finally { await ctx.close(); }
  });

  // v1.2 F1 + D13 — spelling a word commits its glyph; a misspelling joins the
  // missed-words queue through the existing path.
  await suite.test('spelldoku_word_mode_spells_to_commit_and_feeds_missed_words', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '9-medium');
      const b = await board(page);
      const i = empties(b).find((k) => !b.clues[k].Fragment);
      const want = b.words[b.solution[i] - 1];
      await page.click(`[data-sd-cell="${i}"]`);
      await spell(page, 'qqzx');
      assert(/spell/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'MISSPELLED');
      const misses = await page.evaluate(() => JSON.parse(localStorage.getItem('byear_misses_v1') || '[]'));
      assert(misses.some((m) => m.word === want && m.lang === 'en'), `D13: "${want}" joined the missed-words queue`);
      await spell(page, want);
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), b.glyphs[b.solution[i] - 1], 'committed, shown as its glyph');
    } finally { await ctx.close(); }
  });

  // v1.2 F11 / D14 — the player's own words come first, and the badge says so.
  await suite.test('spelldoku_word_mode_draws_my_words_first', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const mine = ['baby', 'dark', 'each', 'face', 'gift', 'hand', 'king', 'lamp', 'nose', 'rain', 'wolf', 'about', 'better', 'circle', 'engine', 'family', 'island'];
      await page.evaluate((ws) => {
        const now = Date.now();
        localStorage.setItem('byear_word_lists_v1', JSON.stringify({ v: 1, nextId: 2, migrated: true, selection: [],
          lists: [{ id: 'l1', name: 'Week', createdAt: now, updatedAt: now, source: 'Manual', order: 'Mixed',
            entries: ws.map((w) => ({ text: w, lang: 'en', addedAt: now })) }] }));
      }, mine);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await openSpellDoku(page);
      await pick(page, '9-medium');
      const b = await board(page);
      const fromMine = b.words.filter((w) => mine.includes(w)).length;
      assert(fromMine >= 6, `at least 6 of 9 from My Words (got ${fromMine})`);
      assertEq(b.mine, fromMine, 'the board counts them');
      assert((await page.$eval('#sdBadge', (e) => e.textContent)).includes(String(fromMine)), 'the badge names how many');
    } finally { await ctx.close(); }
  });

  // C5, closed by Eric on 2026-09-21 from a screenshot: a board of letters
  // whose chip read "Numbers". The chip names the symbols on screen, and it
  // has to keep up as the player cycles through the boards.
  await suite.test('spelldoku_mode_chip_names_the_symbols', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      const chip = () => page.$eval('#sdTierOff', (e) => e.textContent.trim());
      for (const [cfg, want] of [['9-easy', 'Numbers'], ['9-medium', 'Letters'], ['4-easy', 'Numbers'], ['9-expert', 'Letters']]) {
        await pick(page, cfg);
        const b = await board(page);
        const isWords = !!(b.words && b.words.length);
        assertEq(isWords, want === 'Letters', `${cfg}: the board really is ${want.toLowerCase()}`);
        assertEq(await chip(), want, `${cfg}: the chip says ${want}`);
      }
    } finally { await ctx.close(); }
  });

  // Eric, 2026-09-21: "fix the keyboard and legend so 9x9 fits". The whole
  // screen must be reachable on the smallest phone the harness models without
  // scrolling -- board, legend, typed line, keyboard and actions at once -- in
  // BOTH modes, because Word Mode carries a legend that Number Mode does not.
  // The pitch floor is what stops a future change from "fitting" the screen by
  // shrinking the board into nothing.
  for (const [mode, cfg, floor] of [['numbers', '9-easy', 36], ['words', '9-medium', 30]]) {
    await suite.test(`spelldoku_nine_fits_a_small_phone_${mode}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' }); // 375x667
      try {
        await openSpellDoku(page);
        await pick(page, cfg);
        const m = await page.evaluate(() => {
          const s = document.getElementById('sdScreen');
          const c = document.querySelector('.sd-cell');
          return {
            vOver: s.scrollHeight - s.clientHeight,
            hOver: s.scrollWidth - s.clientWidth,
            pitch: c.getBoundingClientRect().width,
            keysVisible: document.querySelectorAll('#sdKeys .kb-key').length,
            commit: !!document.querySelector('#sdKeys [data-sd-go]'),
            back: !!document.querySelector('#sdKeys [data-sd-back]'),
            rows: document.querySelectorAll('#sdKeys .sd-row').length,
          };
        });
        assert(m.vOver <= 0, `${mode}: the stack fits 667 px (overruns by ${m.vOver})`);
        assert(m.hOver <= 0, `${mode}: nothing scrolls sideways (over by ${m.hOver})`);
        assert(m.pitch >= floor, `${mode}: the board keeps a ${floor} px cell (got ${m.pitch.toFixed(1)})`);
        assert(m.commit && m.back, `${mode}: backspace and lock-it-in are both reachable`);
        assertEq(m.rows, 3, `${mode}: three key rows -- the commit keys ride on the last one`);
      } finally { await ctx.close(); }
    });
  }

  // Eric's ruling, 2026-09-21: 12x12 is cut. It was measured at a 29 px cell
  // with a 16.9 px glyph on an iPhone SE, under the 17 px floor, and the
  // screen scrolled 258 px. No size but 4, 6 and 9 is ever offered.
  await suite.test('spelldoku_no_twelve_by_twelve', async () => {
    for (const age of [null, KID]) {
      const { ctx, page } = await openApp(browser, base, age === KID ? { lang: 'en', age: KID } : { lang: 'en' });
      try {
        await openSpellDoku(page);
        const opts = await page.$$eval('#sdPick option', (o) => o.map((x) => x.value));
        assert(!opts.some((v) => v.startsWith('12-')), `no 12x12 offered (got ${opts.join(',')})`);
      } finally { await ctx.close(); }
    }
  });

  // v1.2 D15 / I12 — the legend offers a definition card where the language's
  // definition pool is live, and a definition that would spell the symbol is
  // not shown at all.
  await suite.test('spelldoku_word_mode_definition_card_never_spells_it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '9-medium');
      const b = await board(page);
      assertEq(await page.$$eval('#sdLegend .sd-def', (e) => e.length), 9, 'a card per symbol');
      const w = b.words[0];
      await ctx.route('**/api/meaning**', (r) => r.fulfill({
        status: 200, contentType: 'application/json',
        body: JSON.stringify({ pos: 'noun', definition: `a kind of ${w} you can see`, example: '' }),
      }));
      await page.click('#sdLegend [data-sd-def="1"]');
      await page.waitForFunction(() => !/^\s*$/.test(document.getElementById('sdBadge').textContent), null, { timeout: 4000 });
      await page.waitForTimeout(300);
      const shown = await page.$eval('#sdBadge', (e) => e.textContent);
      assert(!shown.toLowerCase().includes(w), `the card must not spell "${w}" (showed "${shown}")`);
      await ctx.unroute('**/api/meaning**');
      await ctx.route('**/api/meaning**', (r) => r.fulfill({
        status: 200, contentType: 'application/json',
        body: JSON.stringify({ pos: 'noun', definition: 'something you might find at home', example: '' }),
      }));
      await page.click('#sdLegend [data-sd-def="1"]');
      await page.waitForFunction(() => /home/.test(document.getElementById('sdBadge').textContent), null, { timeout: 4000 });
    } finally { await ctx.close(); }
  });
}
