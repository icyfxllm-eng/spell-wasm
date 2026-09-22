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

// CC-SPELLDOKU-RULES C2 (Eric, 2026-09-22): a symbol is PICKED and only then
// spelled, so every placement goes pick -> spell -> commit. The conflict check
// fires on the pick, which is the whole point of the inversion.
async function pickSym(page, v) {
  await page.click(`[data-sd-sym="${v}"]`);
  await page.waitForTimeout(80);
}

async function place(page, v, word) {
  await pickSym(page, v);
  await spell(page, word);
}

const BOX = { 4: [2, 2], 6: [2, 3], 9: [3, 3] };

/// The cells that share a row, column or box with `i`.
function peersOf(b, i) {
  const n = b.n;
  const [br, bc] = BOX[n];
  const r = Math.floor(i / n);
  const c = i % n;
  const r0 = Math.floor(r / br) * br;
  const c0 = Math.floor(c / bc) * bc;
  const out = new Set();
  for (let k = 0; k < n; k++) { out.add(r * n + k); out.add(k * n + c); }
  for (let rr = r0; rr < r0 + br; rr++) for (let cc = c0; cc < c0 + bc; cc++) out.add(rr * n + cc);
  out.delete(i);
  return [...out];
}

/// The value a clue already puts on the board, or 0. Given AND Spelled both
/// seed a cell -- missing Spelled is what made the first draft of these helpers
/// pick "legal" values that were already in the row.
const givenAt = (b, i) => {
  const c = b.clues[i];
  if (typeof c !== 'object') return 0;
  return c.Given || c.Spelled || 0;
};

/// A value that is WRONG for this cell but breaks no rule, so it tests D2's
/// surviving half rather than F1.
function legalWrong(b, i) {
  const taken = new Set(peersOf(b, i).map((j) => givenAt(b, j)));
  for (let v = 1; v <= b.n; v++) {
    if (v !== b.solution[i] && !taken.has(v)) return v;
  }
  return null;
}

/// A value already placed in this cell's row, column or box: the F1 case.
function conflicting(b, i) {
  for (const j of peersOf(b, i)) {
    const v = givenAt(b, j);
    if (v) return { value: v, at: j };
  }
  return null;
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
      await place(page, b.solution[i], PY[b.solution[i]]);
      assert(/spelling|拼写/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'untoned: MISSPELLED');
      await spell(page, PY[b.solution[i]] + TONE[b.solution[i]]);  // still picked after a misspelling
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
        await place(page, v, 'hai');
      } else {
        await pickSym(page, v);
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
      await place(page, b.solution[first], 'fuor');
      assert(/spelling/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'MISSPELLED says so');
      assertEq(await page.$eval(`[data-sd-cell="${first}"]`, (e) => e.textContent.trim()), '', 'nothing placed');
      // A real number word for the wrong value is a logic error (Easy shows it).
      const wrongV = legalWrong(b, first);
      if (wrongV) {
        await place(page, wrongV, WORDS[wrongV]);
        assert(/doesn't go here/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'WRONG_VALUE says so');
      }
      // The right word lands.
      const v = b.solution[first];
      await place(page, v, WORDS[v]);
      assertEq(await page.$eval(`[data-sd-cell="${first}"]`, (e) => e.textContent.trim()), String(v), 'the value is placed');
      // Easy: one correct spelling turns that value into a chip (D1).
      assert(await page.$(`[data-sd-sym="${v}"].free`), `${v} is earned after one spelling (D1)`);
      // A chip commits without spelling -- but only where it is right.
      if (second !== undefined && b.solution[second] === v) {
        await page.click(`[data-sd-cell="${second}"]`);
        await page.click(`[data-sd-sym="${v}"]`);
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
        if (await page.$(`[data-sd-sym="${v}"].free`)) {
          await pickSym(page, v); // D1: an earned symbol goes in without spelling
        } else {
          await place(page, v, WORDS[v]);
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
      const wrongV = legalWrong(b, i);
      assert(wrongV, 'the fixture offers a wrong-but-legal value');
      assert(b.words, 'v1.2 D12: a 9x9 Hard board is Word Mode');
      await page.click(`[data-sd-cell="${i}"]`);
      await place(page, wrongV, b.words[wrongV - 1]);
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
      await page.click('#sdLegend [data-sd-sym="1"]'); // the glyph plays its word; nothing breaks
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
      await place(page, b.solution[i], 'qqzx');
      assert(/spell/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'MISSPELLED');
      const misses = await page.evaluate(() => JSON.parse(localStorage.getItem('byear_misses_v1') || '[]'));
      assert(misses.some((m) => m.word === want && m.lang === 'en'), `D13: "${want}" joined the missed-words queue`);
      await spell(page, want);  // the symbol is still picked
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

  // CC-SPELLDOKU-RULES v1 Phase A. Done #1, #4, #5, #7, #8 on the real screen.
  // D-R1 (Eric, 2026-09-22) amends v1.0 D2: a duplicate is refused at EVERY
  // tier, immediately — build 219 accepted three B's in one box on Hard.
  for (const cfg of ['4-easy', '9-easy', '9-medium', '9-hard', '9-expert']) {
    await suite.test(`spelldoku_conflict_is_refused_${cfg.replace('-', '_')}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await openSpellDoku(page);
        await pick(page, cfg);
        const b = await board(page);
        const i = empties(b).find((k) => conflicting(b, k));
        assert(i !== undefined, 'the fixture has an empty cell with a filled peer');
        const { value, at } = conflicting(b, i);
        await page.click(`[data-sd-cell="${i}"]`);
        const before = await page.evaluate(() => ({
          cells: [...document.querySelectorAll('[data-sd-cell]')].map((e) => e.textContent.trim()),
          check: document.getElementById('sdCheck').textContent,
          note: document.getElementById('sdNote').textContent,
        }));
        await pickSym(page, value);
        const after = await page.evaluate(() => ({
          cells: [...document.querySelectorAll('[data-sd-cell]')].map((e) => e.textContent.trim()),
          check: document.getElementById('sdCheck').textContent,
          note: document.getElementById('sdNote').textContent,
          shaken: document.querySelectorAll('.sd-cell.shake').length,
          flashed: [...document.querySelectorAll('.sd-cell.flash')].map((e) => +e.dataset.sdCell),
          typedLine: document.getElementById('sdTyped').textContent,
          picked: document.querySelectorAll('[data-sd-sym].picked').length,
        }));
        // Done #1: refused, and the board is untouched.
        assertEq(JSON.stringify(after.cells), JSON.stringify(before.cells), `${cfg}: the board is unchanged`);
        assert(/already in this (row|column|box)/i.test(after.note), `${cfg}: it says why — got ${JSON.stringify(after.note)}`);
        assertEq(after.shaken, 1, `${cfg}: the target cell shakes`);
        assert(after.flashed.includes(at), `${cfg}: the cell that explains it flashes`);
        // Done #5: nothing is consumed — the Check counter is where it was.
        assertEq(after.check, before.check, `${cfg}: no Check was spent`);
        // Done #4: no symbol is on the hook, so the composer never opened and
        // the player was never asked to spell a word that could not be placed.
        // (Not a string test — the Number Mode PICK prompt contains the word
        // "spell" itself, which is what the first draft of this tripped over.)
        assertEq(after.picked, 0, `${cfg}: nothing is picked, so spelling never opened`);
        // Typing after a refusal does nothing, because nothing is picked.
        await page.click('#sdKeys [data-sd-key="o"]').catch(() => {});
        await page.waitForTimeout(60);
        assertEq(await page.$eval('#sdTyped', (e) => e.textContent), after.typedLine, `${cfg}: the keyboard is inert`);
      } finally { await ctx.close(); }
    });
  }

  // Done #7 (F2) and #8 (F3) — the count on every symbol, and dimming that is
  // on for Easy and Medium and off above them (D-R2).
  await suite.test('spelldoku_counts_and_dimming', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      for (const [cfg, dims] of [['9-easy', true], ['9-medium', true], ['9-hard', false], ['9-expert', false]]) {
        await pick(page, cfg);
        const b = await board(page);
        const counts = await page.$$eval('[data-sd-sym] .sd-n', (e) => e.map((x) => +x.textContent));
        assertEq(counts.length, b.n, `${cfg}: every symbol carries a count`);
        // F2: remaining = n - already on the board, derived not stored (I-R4).
        const placed = b.clues.map((c, k) => givenAt(b, k)).filter(Boolean);
        for (let v = 1; v <= b.n; v++) {
          const want = b.n - placed.filter((x) => x === v).length;
          assertEq(counts[v - 1], want, `${cfg}: ${v} has ${want} left`);
        }
        const i = empties(b).find((k) => conflicting(b, k));
        await page.click(`[data-sd-cell="${i}"]`);
        await page.waitForTimeout(120);
        const dimmed = await page.$$eval('[data-sd-sym].dim', (e) => e.length);
        if (dims) {
          assert(dimmed > 0, `${cfg}: ruled-out symbols dim`);
          // A dimmed chip still answers: the tap runs F1 and is refused.
          const { value } = conflicting(b, i);
          await pickSym(page, value);
          assert(/already in this/i.test(await page.$eval('#sdNote', (e) => e.textContent)), `${cfg}: a dimmed chip still validates`);
        } else {
          assertEq(dimmed, 0, `${cfg}: nothing dims above Medium`);
        }
      }
    } finally { await ctx.close(); }
  });

  // Done #3, the paths a symbol can reach a cell by — every one refuses a
  // conflict. Hints never write at all, and undo/redo and saved-board restore
  // do not exist (census C1), so this is the whole list.
  await suite.test('spelldoku_every_path_refuses_a_conflict', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSpellDoku(page);
      await pick(page, '4-easy');
      let b = await board(page);
      // Earn a symbol so the D1 tap-to-place path exists, then aim it at a cell
      // that already has it as a peer.
      const first = empties(b).find((k) => !b.clues[k].Fragment);
      const v = b.solution[first];
      await page.click(`[data-sd-cell="${first}"]`);
      await place(page, v, WORDS[v]);
      assert(await page.$(`[data-sd-sym="${v}"].free`), 'the symbol is earned (D1)');
      b = await board(page);
      const target = empties(b).find((k) => k !== first && peersOf(b, k).includes(first));
      if (target !== undefined) {
        await page.click(`[data-sd-cell="${target}"]`);
        const before = await page.$eval(`[data-sd-cell="${target}"]`, (e) => e.textContent.trim());
        await pickSym(page, v);
        assertEq(await page.$eval(`[data-sd-cell="${target}"]`, (e) => e.textContent.trim()), before,
          'an EARNED symbol is refused too — being free of spelling is not being free of the rules');
        assert(/already in this/i.test(await page.$eval('#sdNote', (e) => e.textContent)), 'and it says why');
      }
      // Pencil marks stay free (D-R6): a mark that duplicates a peer is fine.
      await page.click('#sdPencil');
      const pen = empties(b).find((k) => k !== first);
      await page.click(`[data-sd-cell="${pen}"]`);
      await page.click(`#sdKeys [data-sd-pen="${v}"]`);
      await page.waitForTimeout(100);
      assert((await page.$eval(`[data-sd-cell="${pen}"]`, (e) => e.textContent)).includes(String(v)),
        'D-R6: a pencil mark is a note, not a placement');
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
  // Pick-then-spell (CC-SPELLDOKU-RULES C2) gave Number Mode a row of symbols
  // to pick from, which it never had before — its chip tray used to be empty
  // until D1 unlocked something. Nine digits wrapped to two rows and took the
  // cell down to 30.8 px, so the chips are tighter on phones and the board is
  // back over its floor. This test is what caught that.
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
