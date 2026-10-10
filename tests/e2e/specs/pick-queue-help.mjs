// pick-queue-help.spec -- CC-PICK-QUEUE v1 Phase C: help choosing (sort, search,
// Toughest 5) in Misses and My Words, behind `spell_flag_pick_queue_c`
// (default OFF, and only live while Phase A is on).
//
// Browser-only claims: the controls exist only where they should, a sort or a
// search changes what is SHOWN and never what is SERVED, hidden picks stay
// lined up and counted, Toughest 5 fills the lineup and the session serves it
// in that order, and Spell Jr has none of it. The ordering, tie-break, search
// folding and cap rules are host unit tests in src/pick_help.rs.
import { openApp, assert, assertEq, openViaDrawer, openMyWords, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const MISS_KEY = 'byear_misses_v1';
const CUSTOM_KEY = 'byear_custom_v1';
const LISTS_KEY = 'byear_word_lists_v1';
const LINEUP_KEY = 'spell_lineup_v1';
const DAY = 86400000;

// These run in the page (addInitScript serialises them), so each is written
// out in full: a closure over outer variables would arrive as undefined.
const FLAG_AC = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
  localStorage.setItem('spell_flag_pick_queue_c', 'on');
};
const FLAG_A_ONLY = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
  localStorage.setItem('spell_flag_pick_queue_c', 'off');
};
// C without A must do nothing.
const FLAG_C_ALONE = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'off');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
  localStorage.setItem('spell_flag_pick_queue_c', 'on');
};

const lineup = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || 'null'), LINEUP_KEY);
const picksOf = async (page) => Object.values((await lineup(page))?.scopes || {}).flatMap((s) => s.picks.map((p) => p.word));

async function reload(page, base) {
  await page.reload({ waitUntil: 'load' });
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  await page.waitForFunction(() => {
    const o = document.getElementById('orbWrap');
    return !!o && o.getBoundingClientRect().width > 0;
  }, null, { timeout: 10000 }).catch(() => {});
}

/// rows: [word, misses, ageMs (since last miss), dueInDays, lang?]
async function bootMisses(page, base, rows) {
  const now = Date.now();
  const data = rows.map(([word, misses, age, dueIn, lang]) => ({
    word, lang: lang || 'en', tier: 'easy', misses, box_: 1, due: now + dueIn * DAY, ts: now - age,
  }));
  await page.evaluate(([k, d]) => {
    localStorage.setItem(k, JSON.stringify(d));
    localStorage.removeItem('spell_lineup_v1');
    localStorage.removeItem('spell_stars_v1');
  }, [MISS_KEY, data]);
  await reload(page, base);
}

async function bootMine(page, base, words, misses = []) {
  const now = Date.now();
  const rows = misses.map(([word, n, age, dueIn]) => ({
    word, lang: '__mine', tier: 'easy', misses: n, box_: 1, due: now + dueIn * DAY, ts: now - age,
  }));
  await page.evaluate(([ck, lk, mk, set, rows]) => {
    localStorage.setItem(ck, JSON.stringify(set));
    localStorage.removeItem(lk);
    localStorage.setItem(mk, JSON.stringify(rows));
    localStorage.removeItem('spell_lineup_v1');
  }, [CUSTOM_KEY, LISTS_KEY, MISS_KEY, { words, speakLang: 'en-US', wordLang: {} }, rows]);
  await reload(page, base);
}

async function openMisses(page) {
  await openViaDrawer(page, 'misses');
  await page.waitForSelector('#pickScreen.show', { timeout: 5000 });
}

async function openList(page) {
  await openMyWords(page);
  await page.waitForSelector('#listsScreen.show');
  await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
  await domSettled(page);
}

const rowWords = (page, scope) => page.$$eval(`${scope} .lists-entry`, (rows) =>
  rows.map((r) => r.querySelector('.pq-word')?.textContent || r.querySelector('.le-word')?.textContent || ''));
const badgeMap = (page, scope) => page.$$eval(`${scope} .lists-entry`, (rows) =>
  Object.fromEntries(rows.map((r) => [r.querySelector('.pq-word')?.textContent || '', r.querySelector('.pq-badge')?.textContent || ''])));
const tapWord = async (page, scope, word) => {
  await page.click(`${scope} [data-pq-toggle][data-pq-word="${word}"]`);
  await domSettled(page);
};
const sortTo = async (page, scope, id) => {
  await page.click(`${scope} [data-pq-sort="${id}"]`);
  await domSettled(page);
};
const search = async (page, scope, text) => {
  await page.fill(`${scope} [data-pq-search]`, text);
  await domSettled(page);
};
const toughest = async (page, scope) => {
  await page.click(`${scope} [data-pq-toughest]`);
  await domSettled(page);
};
const controls = (page) => page.$$eval('[data-pq-sort],[data-pq-search],[data-pq-toughest]', (els) => els.length);

async function type(page, text) {
  for (const ch of text.toLowerCase()) {
    const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
    assert(k, `no key for ${JSON.stringify(ch)}`);
    await k.click({ timeout: 5000 });
  }
}
async function serve(page) {
  const before = await page.evaluate(() => window.__spelltest.currentWord());
  await page.click('#orbWrap');
  await page.waitForFunction((b) => { const w = window.__spelltest.currentWord(); return w && w !== b; }, before, { timeout: 5000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}
async function answerCorrect(page) {
  const w = await page.evaluate(() => window.__spelltest.currentWord());
  await page.waitForFunction(() => !document.getElementById('gameKeyboard').classList.contains('locked'),
    null, { timeout: 5000 }).catch(() => {});
  await type(page, w);
  await page.click('#checkBtn');
  await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 5000 });
  return w;
}

// word, misses, age since last miss (ms), due in days. Stored order is the
// array order (what the surface shows today).
//   stored:        apple bread crane delta eagle finch glove
//   missed most:   eagle(9) crane(5, newer) bread(5) glove(3) apple(2) finch(2, older) delta(1)
const SEVEN = [
  ['apple', 2, 1000, -1],
  ['bread', 5, 4000, -3],
  ['crane', 5, 2000, 2],
  ['delta', 1, 9000, -2],
  ['eagle', 9, 500, 9],
  ['finch', 2, 7000, 1],
  ['glove', 3, 3000, -1],
];
const STORED = SEVEN.map((r) => r[0]);

export async function run(browser, base, suite) {
  // Dual config: Phase C off (A on) is Phase A's surface exactly -- no control
  // exists and the rows are in stored order.
  await suite.test('pick_help_flag_off_is_the_phase_a_surface', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_A_ONLY });
    try {
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      assertEq((await rowWords(page, '#pickBody')).join(','), STORED.join(','), 'stored order');
      assertEq(await controls(page), 0, 'no sort, search or Toughest 5');
      assert((await page.$('#pickBody .pq-help')) === null);
      assertEq(await lineup(page), null, 'nothing written');
    } finally { await ctx.close(); }
  });

  // C without A is nothing: the lineup surface itself does not exist.
  await suite.test('pick_help_flag_c_alone_does_nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_C_ALONE });
    try {
      await bootMisses(page, base, SEVEN);
      await openViaDrawer(page, 'misses');
      await page.waitForFunction(() => document.getElementById('missesBtn').classList.contains('on'), null, { timeout: 5000 });
      assert((await page.$('#pickScreen')) === null, 'no lineup surface');
      assertEq(await controls(page), 0);
    } finally { await ctx.close(); }
  });

  // Dual config, ON, defaults and zero picks: the same rows in the same order
  // as with the flag off, and nothing is written (I4).
  await suite.test('pick_help_defaults_show_todays_order_and_write_nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      assertEq((await rowWords(page, '#pickBody')).join(','), STORED.join(','), 'default sort is the order shown today');
      assertEq(await page.$$eval('#pickBody [data-pq-sort]', (b) => b.length), 5, 'five sort choices');
      assertEq(await page.$eval('#pickBody [data-pq-sort="listed"]', (b) => b.getAttribute('aria-pressed')), 'true');
      assert(await page.$('#pickBody [data-pq-search][aria-label]'), 'the search box has an accessible label');
      assert(await page.$('#pickBody [data-pq-toughest]'));
      assertEq(await lineup(page), null, 'looking and sorting write no lineup');
    } finally { await ctx.close(); }
  });

  // A12 / I1: a sort changes the display, never the serve order, the badges or
  // the stored lineup.
  await suite.test('pick_help_sort_changes_display_only_not_the_serve_order', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'finch');
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'eagle');
      const stored = await lineup(page);
      assertEq((await picksOf(page)).join(','), 'finch,apple,eagle');

      await sortTo(page, '#pickBody', 'missed');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'eagle,crane,bread,glove,apple,finch,delta', 'Missed most, ties: newer first');
      let b = await badgeMap(page, '#pickBody');
      assertEq(`${b.finch}${b.apple}${b.eagle}`, '123', 'badges did not move');

      await sortTo(page, '#pickBody', 'due');
      // due first, soonest first: bread(-3) delta(-2) apple(-1) glove(-1) ... then finch(1) crane(2) eagle(9)
      const due = await rowWords(page, '#pickBody');
      assertEq(due.slice(0, 2).join(','), 'bread,delta', 'overdue first, longest overdue first');
      assertEq(due.slice(-3).join(','), 'finch,crane,eagle', 'then not due, soonest first');

      await sortTo(page, '#pickBody', 'newest');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'eagle,apple,crane,glove,bread,finch,delta', 'last missed first');

      await sortTo(page, '#pickBody', 'never');
      assertEq((await rowWords(page, '#pickBody')).length, 7, 'every word still listed');

      await sortTo(page, '#pickBody', 'listed');
      assertEq((await rowWords(page, '#pickBody')).join(','), STORED.join(','), 'back to as listed');
      assertEq(JSON.stringify(await lineup(page)), JSON.stringify(stored), 'the stored lineup never changed');

      // The session serves tap order whatever the surface was showing.
      await sortTo(page, '#pickBody', 'missed');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const served = [];
      for (let i = 0; i < 3; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.join(','), 'finch,apple,eagle', 'serve order is the lineup, not the sort');
    } finally { await ctx.close(); }
  });

  // I5 / F3: a search hides rows, never picks. Hidden picks stay lined up with
  // their numbers and are counted; the count is visible.
  await suite.test('pick_help_search_hides_rows_but_hidden_picks_stay_lined_up_and_counted', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'crane');
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'glove');
      await search(page, '#pickBody', 'GLO');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'glove', 'case-insensitive substring');
      assertEq((await badgeMap(page, '#pickBody')).glove, '3', 'its badge number is unchanged');
      const note = await page.$eval('#pickBody .pq-note', (e) => e.textContent);
      assert(/2/.test(note), `hidden picks are counted (${note})`);
      assertEq((await picksOf(page)).join(','), 'crane,apple,glove', 'still lined up, still in order');
      assert(/3/.test(await page.$eval('#pickBody [data-pq-start]', (e) => e.textContent)), 'the start button still says 3');
      // The text survives a sort change (same visit).
      await sortTo(page, '#pickBody', 'newest');
      assertEq(await page.$eval('#pickBody [data-pq-search]', (e) => e.value), 'GLO');
      // A search with no match says so, and the picks are still there.
      await search(page, '#pickBody', 'zzz');
      assertEq((await rowWords(page, '#pickBody')).length, 0);
      assert(await page.$('#pickBody .lists-empty[role="status"]'), 'a no-match message');
      assertEq((await picksOf(page)).length, 3);
      await search(page, '#pickBody', '');
      assertEq((await rowWords(page, '#pickBody')).length, 7, 'clearing the search shows everything');
      assertEq(JSON.stringify(await badgeMap(page, '#pickBody')).includes('"crane":"1"'), true, 'numbers intact');
    } finally { await ctx.close(); }
  });

  // Search folds case, diacritics and non-Latin scripts; typing keeps focus.
  await suite.test('pick_help_search_folds_diacritics_and_non_latin_and_keeps_focus', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, [['café', 2, 1000, -1], ['niño', 1, 2000, -1], ['apple', 1, 3000, -1]]);
      await openMisses(page);
      await search(page, '#pickBody', 'CAFE');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'café');
      await search(page, '#pickBody', 'nin');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'niño');
      // Typing char by char: the box keeps focus and the caret through each re-render.
      await search(page, '#pickBody', '');
      await page.click('#pickBody [data-pq-search]');
      await page.keyboard.type('app', { delay: 20 });
      await domSettled(page);
      assertEq(await page.evaluate(() => document.activeElement?.hasAttribute('data-pq-search')), true, 'focus kept');
      assertEq(await page.$eval('#pickBody [data-pq-search]', (e) => e.value), 'app', 'every keystroke landed');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'apple');
      // Non-Latin: a Cyrillic miss in its own language chip.
      await bootMisses(page, base, [['Замок', 2, 1000, -1, 'ru'], ['книга', 1, 2000, -1, 'ru']]);
      await openMisses(page);
      await search(page, '#pickBody', 'зам');
      assertEq((await rowWords(page, '#pickBody')).join(','), 'Замок');
      assertEq(await page.$eval('#pickBody [data-pq-search]', (e) => e.getAttribute('dir')), 'auto', 'RTL-safe text direction');
    } finally { await ctx.close(); }
  });

  // F3: Toughest 5 fills the lineup with the five most-missed (ties: newer),
  // and the session serves it in that order.
  await suite.test('pick_help_toughest_five_fills_the_lineup_and_is_served_in_order', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      await toughest(page, '#pickBody');
      assertEq((await picksOf(page)).join(','), 'eagle,crane,bread,glove,apple', 'most missed first, ties newest first');
      const b = await badgeMap(page, '#pickBody');
      assertEq(`${b.eagle}${b.crane}${b.bread}${b.glove}${b.apple}`, '12345');
      assertEq(`${b.finch}${b.delta}`, '', 'the sixth and seventh are not picked');
      // Pressing it again changes nothing.
      await toughest(page, '#pickBody');
      assertEq((await picksOf(page)).length, 5);
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const served = [];
      for (let i = 0; i < 5; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.join(','), 'eagle,crane,bread,glove,apple', 'served in lineup order');
    } finally { await ctx.close(); }
  });

  // Already-picked words keep their slot; new ones append most-missed first.
  // And the scope is the CURRENT language only (D9).
  await suite.test('pick_help_toughest_keeps_existing_picks_and_never_mixes_languages', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMisses(page, base, [...SEVEN, ['hund', 40, 100, -1, 'de'], ['katze', 30, 100, -1, 'de']]);
      await openMisses(page);
      // English is the study language, so its chip is showing.
      assert(!(await rowWords(page, '#pickBody')).includes('hund'), 'German rows are not on the English chip');
      await tapWord(page, '#pickBody', 'delta'); // 1 miss: would not make the top five
      await tapWord(page, '#pickBody', 'bread');
      await toughest(page, '#pickBody');
      assertEq((await picksOf(page)).join(','), 'delta,bread,eagle,crane,glove,apple',
        'delta and bread keep slots 1 and 2; eagle, crane, glove, apple append most-missed first');
      const sc = Object.values((await lineup(page)).scopes);
      assertEq(sc.length, 1);
      assertEq(sc[0].lang, 'en', 'one language only');
    } finally { await ctx.close(); }
  });

  // Fewer than five missed words: it adds what exists. Never-missed words in
  // My Words are not padded in.
  await suite.test('pick_help_my_words_sort_search_and_toughest', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AC });
    try {
      await bootMine(page, base, ['cat', 'dog', 'fox', 'owl', 'bee'], [['dog', 4, 1000, -1], ['owl', 2, 500, 3]]);
      await openList(page);
      const dflt = await rowWords(page, '#listsBody');
      assertEq(dflt.join(','), 'cat,dog,fox,owl,bee', 'list order by default');
      await sortTo(page, '#listsBody', 'missed');
      assertEq((await rowWords(page, '#listsBody')).slice(0, 2).join(','), 'dog,owl', 'misses read from the Misses store');
      await sortTo(page, '#listsBody', 'due');
      assertEq((await rowWords(page, '#listsBody')).slice(0, 2).join(','), 'dog,owl', 'due, then scheduled');
      await sortTo(page, '#listsBody', 'never');
      assertEq((await rowWords(page, '#listsBody')).slice(-2).join(','), 'dog,owl', 'words with a miss entry are practiced');
      await sortTo(page, '#listsBody', 'listed');
      await search(page, '#listsBody', 'O');
      assertEq((await rowWords(page, '#listsBody')).join(','), 'dog,fox,owl', 'substring over the spelling');
      await search(page, '#listsBody', '');
      await toughest(page, '#listsBody');
      assertEq((await picksOf(page)).join(','), 'dog,owl', 'only the two missed words; never-missed words are not padded in');
      // Editing a row still works from the sorted view (stored index, not display index).
      await sortTo(page, '#listsBody', 'missed');
      const stored = await page.$$eval('#listsBody .lists-entry', (rows) =>
        rows.find((r) => r.querySelector('.pq-word')?.textContent === 'bee')?.querySelector('[data-e-remove]')?.getAttribute('data-e-remove'));
      assertEq(stored, '4', 'the row for bee still carries its stored index');
    } finally { await ctx.close(); }
  });

  // I7 / D4 / A10: Spell Jr gets none of it -- absent, not locked -- on both
  // surfaces, with every flag on.
  await suite.test('pick_help_spell_jr_has_no_sort_search_or_toughest', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG_AC });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await bootMisses(page, base, SEVEN);
      await openMisses(page);
      assertEq((await rowWords(page, '#pickBody')).join(','), STORED.join(','), 'rows in stored order');
      assertEq(await controls(page), 0, 'no controls on Misses');
      assert((await page.$('#pickBody .pq-help')) === null);
      await bootMine(page, base, ['cat', 'dog'], [['dog', 4, 1000, -1]]);
      await openList(page);
      assertEq(await controls(page), 0, 'no controls on My Words');
      assertEq((await rowWords(page, '#listsBody')).join(','), 'cat,dog');
      // Lining a word up still works for Jr.
      await tapWord(page, '#listsBody', 'dog');
      assertEq((await picksOf(page)).join(','), 'dog');
    } finally { await ctx.close(); }
  });
}
