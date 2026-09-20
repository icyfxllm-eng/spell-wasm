// spell-cross.spec — CC-WORDGRID v1 Phase B (Spell Cross), on the real screen.
//
// The grids are the real generator's, from the real banks. The test hook only
// READS the served crossword, so a test knows which letters to type.
import { openApp, assert, assertEq } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

const board = (page) => page.evaluate(() => JSON.parse(window.__spelltest.spellCrossBoard() || 'null'));

async function openCross(page) {
  await page.evaluate(() => document.getElementById('xwOpenBtn').click());
  await page.waitForSelector('#xwScreen.show', { timeout: 8000 });
}

/// Type a word into its slot: pick the clue, then its letters in order. A word
/// whose last empty cell is filled by crossings checks itself early (F-C5), so
/// stop as soon as the clue is answered -- the rest would type into whatever
/// comes next, exactly as it would for a player.
async function solve(page, i, letters) {
  await page.click(`[data-xw-clue="${i}"]`);
  for (const ch of letters) {
    if (await page.$eval(`[data-xw-clue="${i}"]`, (e) => e.classList.contains('done'))) break;
    await page.click(`#xwKeys [data-xw-key="${ch}"]`);
  }
  await page.waitForTimeout(60);
}

const learnerLog = (page, lang) => page.evaluate((l) => {
  const s = JSON.parse(localStorage.getItem(`spell_learner_${l}`) || 'null');
  return s ? s.log : [];
}, lang);

export async function run(browser, base, suite) {
  // D1 / F-C1: the clue is a number, a length and a play button. The words are
  // the list's own, and no surface writes one down.
  await suite.test('spell_cross_clues_are_heard_never_read', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openCross(page);
      const b = await board(page);
      assert(b && b.words.length >= 5, `I7: five or more words (got ${b && b.words.length})`);
      assertEq(await page.$$eval('#xwGrid .xw-cell', (c) => c.length), b.w * b.h, 'the grid is drawn');
      const shown = await page.evaluate(() => ['xwGrid', 'xwClues', 'xwNote']
        .map((id) => document.getElementById(id).innerText.toLowerCase()).join(' | '));
      for (const w of b.words) assert(!shown.includes(w.word), `"${w.word}" is not written on the screen`);
      assertEq(await page.$$eval('#xwClues [data-xw-say]', (c) => c.length), b.words.length, 'a play button per clue');
      await page.click('#xwClues [data-xw-say="0"]');
      // Across and Down are both listed.
      const heads = await page.$$eval('#xwClues .xw-head', (h) => h.map((e) => e.textContent));
      assertEq(heads.length, 2, 'Across and Down');
    } finally { await ctx.close(); }
  });

  // F-C5: a word checks when its last cell is filled; only wrong cells flash;
  // the result reaches the base game's log (F-X6) and the missed-words queue.
  await suite.test('spell_cross_checks_on_the_last_letter', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openCross(page);
      const b = await board(page);
      const w = b.words[0];
      const wrong = 'x'.repeat(w.word.length);
      await solve(page, 0, wrong);
      const flashing = await page.$$eval('#xwGrid .xw-cell.wrong', (c) => c.length);
      assert(flashing > 0, 'the wrong cells flash');
      assert(/not quite/i.test(await page.$eval('#xwNote', (e) => e.textContent)), 'and it says so');
      const missed = await page.evaluate(() => JSON.parse(localStorage.getItem('byear_misses_v1') || '[]'));
      assert(missed.some((m) => m.word === w.word), `"${w.word}" joined the missed-words queue`);

      await solve(page, 0, w.word);
      assert(await page.$eval('[data-xw-clue="0"]', (e) => e.classList.contains('done')), 'the clue is answered');
      assertEq(await page.$$eval('#xwGrid .xw-cell.wrong', (c) => c.length), 0, 'nothing flashes now');
      const log = await learnerLog(page, 'en');
      assert(log.some((a) => a.word === w.word && a.correct), 'the right answer is in the base-game log');
      assert(log.some((a) => a.word === w.word && !a.correct), 'and so is the miss');
    } finally { await ctx.close(); }
  });

  // F-C6: the whole grid, then the hidden word, then stars.
  await suite.test('spell_cross_finishes_on_the_hidden_word', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openCross(page);
      let b = await board(page);
      // A crossword with a keystone: take a new one until it has a hidden word.
      for (let k = 0; k < 6 && !b.keystone; k++) {
        await page.click('#xwNew');
        await page.waitForTimeout(200);
        b = await board(page);
      }
      assert(b.keystone, 'a crossword with a hidden word');
      for (let i = 0; i < b.words.length; i++) await solve(page, i, b.words[i].word);
      await page.waitForSelector('#xwKey:not(.btn-hide)', { timeout: 3000 });
      assert(await page.$eval('#xwSolve', (e) => e.classList.contains('btn-hide')), 'the grid is hidden for the keystone');
      assertEq(await page.$$eval('#xwGrid .xw-cell.shaded', (c) => c.length), b.shaded.length, 'the shaded cells spelled it');
      for (const ch of b.keystone) await page.click(`#xwKeys [data-xw-key="${ch}"]`);
      await page.click('#xwKeyGo');
      await page.waitForSelector('#xwDone:not(.btn-hide)', { timeout: 3000 });
      assertEq(await page.$eval('#xwStars', (e) => e.textContent), '★★★', 'three stars for a clean grid');
      const log = await learnerLog(page, 'en');
      assert(log.some((a) => a.word === b.keystone && a.correct), 'the keystone is recorded like any answer');
    } finally { await ctx.close(); }
  });

  // D2 / F-C7: a list that will not interlock offers Spell Search, one button.
  await suite.test('spell_cross_offers_spell_search_when_a_list_will_not_interlock', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => {
        const now = Date.now();
        localStorage.setItem('byear_word_lists_v1', JSON.stringify({ v: 1, nextId: 2, migrated: true, selection: [],
          lists: [{ id: 'l1', name: 'Tiny', createdAt: now, updatedAt: now, source: 'Manual', order: 'Mixed',
            entries: ['planet', 'garden', 'yellow'].map((w) => ({ text: w, lang: 'en', addedAt: now })) }] }));
      });
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.evaluate(() => document.getElementById('importBtn').click());
      await page.waitForSelector('#listsScreen.show', { timeout: 5000 });
      await page.click('[data-l-open="l1"]');
      await page.click('[data-l-cross="l1"]');
      await page.waitForSelector('#xwScreen.show', { timeout: 5000 });
      assert(/better spell search/i.test(await page.$eval('#xwNote', (e) => e.textContent)), 'it says which game suits the list');
      assert(!(await page.$eval('#xwFallback', (e) => e.classList.contains('btn-hide'))), 'with one button');
      await page.click('#xwToSearch');
      await page.waitForSelector('#wsScreen.show', { timeout: 5000 });
      assert(!(await page.$eval('#xwScreen', (e) => e.classList.contains('show'))), 'and Spell Cross gets out of the way');
    } finally { await ctx.close(); }
  });

  // The launch set, as Spell Search has it; Korean and Chinese never (D4).
  await suite.test('spell_cross_launch_set', async () => {
    for (const lang of ['es', 'ru', 'fil']) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await openCross(page);
        const b = await board(page);
        assert(b && b.words.length >= 5, `${lang}: a crossword`);
        assertEq(b.lang, lang, `${lang}: in its own language`);
      } finally { await ctx.close(); }
    }
    for (const lang of ['ko', 'zh']) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await page.evaluate(() => document.getElementById('xwOpenBtn').click());
        await page.waitForTimeout(400);
        assert(!(await page.$eval('#xwScreen', (e) => e.classList.contains('show'))), `${lang}: no Spell Cross`);
      } finally { await ctx.close(); }
    }
  });

  // F-X5: an under-13 player gets the Jr grid only.
  await suite.test('spell_cross_spell_jr_gets_jr_only', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID });
    try {
      await openCross(page);
      assertEq(await page.$$eval('#xwPick option', (o) => o.map((x) => x.value).join(',')), 'jr', 'Jr is the only choice');
      assertEq((await board(page)).tier, 'jr');
    } finally { await ctx.close(); }
  });

  // Phase C: the Daily crossword is one puzzle per language per date.
  await suite.test('spell_cross_daily_is_the_same_for_everyone', async () => {
    const seen = [];
    for (let k = 0; k < 2; k++) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await openCross(page);
        await page.click('#xwDaily');
        await page.waitForTimeout(400);
        const b = await board(page);
        assert(b.daily, 'the Daily is marked');
        seen.push(JSON.stringify(b.words.map((w) => w.word)));
      } finally { await ctx.close(); }
    }
    assertEq(seen[0], seen[1], 'two players, one Daily crossword');
  });
}
