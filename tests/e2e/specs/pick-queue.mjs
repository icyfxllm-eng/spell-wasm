// pick-queue.spec -- CC-PICK-QUEUE v1 Phase A: tap words to line them up, in
// Misses and in My Words, behind `spell_flag_pick_queue_a` (default OFF).
//
// Browser-only claims: the stored bytes (`spell_lineup_v1`), the real tap and
// badge wiring, persistence across a force reload (A7), and that the flag-off
// app is the app it was. The ordering, caps and scoping rules themselves are
// host unit tests in src/pick_queue.rs and src/misses.rs.
import { openApp, assert, assertEq, openViaDrawer, openMyWords, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const MISS_KEY = 'byear_misses_v1';
const CUSTOM_KEY = 'byear_custom_v1';
const LISTS_KEY = 'byear_word_lists_v1';
const LINEUP_KEY = 'spell_lineup_v1';
const DAY = 86400000;

const FLAG_ON = () => localStorage.setItem('spell_flag_pick_queue_a', 'on');
const FLAG_OFF = () => localStorage.setItem('spell_flag_pick_queue_a', 'off');

const lineup = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || 'null'), LINEUP_KEY);
const scopes = async (page) => Object.values((await lineup(page))?.scopes || {});
const picksOf = async (page) => (await scopes(page)).flatMap((s) => s.picks.map((p) => p.word));
const misses = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || '[]'), MISS_KEY);

async function reload(page, base) {
  await page.reload({ waitUntil: 'load' });
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  await page.waitForFunction(() => {
    const o = document.getElementById('orbWrap');
    return !!o && o.getBoundingClientRect().width > 0;
  }, null, { timeout: 10000 }).catch(() => {});
}

/// Put a miss queue on the device and boot with it. `dueIn` is in days
/// (negative = already due).
async function bootMisses(page, base, words) {
  const now = Date.now();
  const rows = words.map(([word, dueIn], i) => ({
    word, lang: 'en', tier: 'easy', misses: 1 + i, box_: 1, due: now + dueIn * DAY, ts: now - i * 1000,
  }));
  await page.evaluate(([k, rows]) => {
    localStorage.setItem(k, JSON.stringify(rows));
    localStorage.removeItem('spell_lineup_v1');
  }, [MISS_KEY, rows]);
  await reload(page, base);
}

async function bootMine(page, base, words, wordLang = {}) {
  await page.evaluate(([ck, lk, set]) => {
    localStorage.setItem(ck, JSON.stringify(set));
    localStorage.removeItem(lk);
    localStorage.removeItem('spell_lineup_v1');
  }, [CUSTOM_KEY, LISTS_KEY, { words, speakLang: 'en-US', wordLang }]);
  await reload(page, base);
}

async function openMisses(page) {
  await openViaDrawer(page, 'misses');
  await page.waitForSelector('#pickScreen.show', { timeout: 5000 });
}

const badges = (page) => page.$$eval('#pickBody .lists-entry', (rows) =>
  rows.map((r) => [r.querySelector('.pq-word')?.textContent || '', r.querySelector('.pq-badge')?.textContent || '']));

const tapWord = async (page, scope, word) => {
  await page.click(`${scope} [data-pq-toggle][data-pq-word="${word}"]`);
  await domSettled(page);
};

async function type(page, text) {
  for (const ch of text.toLowerCase()) {
    const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
    assert(k, `no key for ${JSON.stringify(ch)}`);
    await k.click({ timeout: 5000 });
  }
}

/// Serve the next word from the orb and return it.
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

const inReview = (page) => page.$eval('#missesBtn', (e) => e.classList.contains('on'));

const FIVE = [['apple', -1], ['bread', -1], ['crane', -1], ['delta', -1], ['eagle', -1]];

export async function run(browser, base, suite) {
  // Dual config, OFF: the Misses button goes straight to practice, no lineup
  // surface exists anywhere, and nothing is written.
  await suite.test('pick_queue_flag_off_is_todays_app', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_OFF });
    try {
      await bootMisses(page, base, FIVE);
      await openViaDrawer(page, 'misses');
      await page.waitForFunction(() => document.getElementById('missesBtn').classList.contains('on'), null, { timeout: 5000 });
      assert(await inReview(page), 'practice started directly');
      assert((await page.$('#pickScreen')) === null, 'no lineup surface was ever created');
      const w = await serve(page);
      assert(FIVE.some(([x]) => x === w), `served a due miss (${w})`);
      assertEq(await lineup(page), null, 'nothing written');
    } finally { await ctx.close(); }
  });

  // Dual config, ON, zero picks: the sheet's "Practice all" is the old queue.
  await suite.test('pick_queue_zero_picks_is_the_normal_queue', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      assertEq((await badges(page)).length, 5, 'every miss is a row');
      assert((await page.$('#pickBody [data-pq-start]')) === null, 'no start button with nothing picked');
      await page.click('#pickBody [data-pq-all]');
      await domSettled(page);
      assert(await inReview(page), 'ordinary review began');
      const w = await serve(page);
      assert(FIVE.some(([x]) => x === w), `served a due miss (${w})`);
      assertEq(await lineup(page), null, 'a zero-pick session writes no lineup');
    } finally { await ctx.close(); }
  });

  // A1 / A3 / A7 / A9 / I5: numbering, renumbering, persistence, order, Just these.
  await suite.test('pick_queue_misses_lineup_numbers_persists_and_serves_first', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'crane');
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'eagle');
      let b = Object.fromEntries(await badges(page));
      assertEq(`${b.crane}${b.apple}${b.eagle}`, '123', 'badges in tap order');
      assertEq(b.bread + b.delta, '', 'unpicked words carry no badge');
      // Tapping a lined-up word removes it and the rest renumber, no gap.
      await tapWord(page, '#pickBody', 'apple');
      b = Object.fromEntries(await badges(page));
      assertEq(`${b.crane}|${b.apple}|${b.eagle}`, '1||2', 'renumbered without a gap');
      await tapWord(page, '#pickBody', 'apple'); // back on, as 3rd
      assertEq((await picksOf(page)).join(','), 'crane,eagle,apple', 'stored in tap order');
      // Just these, then force-reload: the lineup is still there (A7).
      await page.click('#pickBody [data-pq-just]');
      await reload(page, base);
      assertEq((await picksOf(page)).join(','), 'crane,eagle,apple', 'survived a hard reload');
      await openMisses(page);
      b = Object.fromEntries(await badges(page));
      assertEq(`${b.crane}${b.eagle}${b.apple}`, '123', 'badges are back after reload');
      assertEq(await page.$eval('#pickBody [data-pq-just]', (e) => e.getAttribute('aria-pressed')), 'true', 'and so is Just these');
      assert(/3/.test(await page.$eval('#pickBody [data-pq-start]', (e) => e.textContent)), 'the start button names the count');
      // Start: the lineup is served first, in tap order, then the session ends.
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      assert(await inReview(page), 'in the practice session');
      const served = [];
      for (let i = 0; i < 3; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.join(','), 'crane,eagle,apple', 'picks first, in the order picked (I2)');
      await page.click('#orbWrap');
      await page.waitForFunction(() => !document.getElementById('missesBtn').classList.contains('on'),
        null, { timeout: 5000 });
      assert(!(await inReview(page)), 'Just these: the session ended after the picks (A9)');
      assertEq((await picksOf(page)).join(','), 'crane,eagle,apple', 'a finished session does not clear the lineup');
    } finally { await ctx.close(); }
  });

  // Not "Just these": unpicked words follow, and a pick is not repeated (I3).
  await suite.test('pick_queue_unpicked_words_follow_and_picks_do_not_repeat', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'delta');
      await tapWord(page, '#pickBody', 'bread');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const served = [];
      for (let i = 0; i < 3; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.slice(0, 2).join(','), 'delta,bread', 'the lineup first');
      assert(['apple', 'crane', 'eagle'].includes(served[2]), `then the normal queue (${served[2]})`);
    } finally { await ctx.close(); }
  });

  // A5 / D3 / I6: a correct answer on a NOT-DUE pick is practice, but it is
  // not cleared and its next spaced check does not move.
  await suite.test('pick_queue_not_due_pick_keeps_its_schedule_when_answered', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, [['alpha', 3], ['bravo', -1]]);
      const before = (await misses(page)).find((m) => m.word === 'alpha');
      await openMisses(page);
      await tapWord(page, '#pickBody', 'alpha');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      assertEq(await serve(page), 'alpha', 'a not-due pick is served');
      await answerCorrect(page);
      const after = (await misses(page)).find((m) => m.word === 'alpha');
      assert(after, 'still in Misses: not cleared');
      assertEq(after.due, before.due, 'its next due date did not move');
      assertEq(after.misses, before.misses, 'miss count untouched');
    } finally { await ctx.close(); }
  });

  // End control: the explicit exit clears the lineup (and only that does).
  await suite.test('pick_queue_end_lineup_clears_it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'apple');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      assert(await page.$('#pickEndBtn'), 'a WORD exit is on the play screen');
      await page.click('#pickEndBtn');
      await domSettled(page);
      assertEq((await picksOf(page)).length, 0, 'the lineup went with the session');
      assert((await page.$('#pickEndBtn')) === null, 'and the control is gone');
      assert(!(await inReview(page)), 'out of review');
    } finally { await ctx.close(); }
  });

  // O-E: Reset this language clears that language's lineup.
  await suite.test('pick_queue_reset_this_language_clears_its_lineup', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'apple');
      assertEq((await picksOf(page)).length, 1);
      await page.evaluate(() => document.getElementById('resetStats').click());
      await page.waitForFunction((k) => {
        const l = JSON.parse(localStorage.getItem(k) || 'null');
        return !l || Object.keys(l.scopes).length === 0;
      }, LINEUP_KEY, { timeout: 5000 });
      assertEq((await picksOf(page)).length, 0, 'cleared');
    } finally { await ctx.close(); }
  });

  // My Words: same component, same behaviour, scoped to the list.
  await suite.test('pick_queue_my_words_lineup_numbers_persists_and_serves_first', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMine(page, base, ['cat', 'dog', 'fox', 'owl']);
      await openMyWords(page);
      await page.waitForSelector('#listsScreen.show');
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      await tapWord(page, '#listsBody', 'fox');
      await tapWord(page, '#listsBody', 'cat');
      const nums = await page.$$eval('#listsBody .lists-entry', (rows) =>
        Object.fromEntries(rows.map((r) => [r.querySelector('.pq-word')?.textContent, r.querySelector('.pq-badge')?.textContent || ''])));
      assertEq(`${nums.fox}${nums.cat}`, '12', 'badges in tap order');
      await reload(page, base);
      assertEq((await picksOf(page)).join(','), 'fox,cat', 'survived a hard reload (A7)');
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      await page.click('#listsBody [data-pq-start]');
      await page.waitForFunction(() => window.__spelltest.currentLang() === '__mine', null, { timeout: 5000 });
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      assertEq(await page.evaluate(() => window.__spelltest.currentWord()), 'fox', 'the first pick is served first');
      await answerCorrect(page);
      assertEq(await serve(page), 'cat', 'then the second');
    } finally { await ctx.close(); }
  });

  // O-D: removing a word from My Words removes it from the lineup.
  await suite.test('pick_queue_removing_a_my_words_word_unlines_it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMine(page, base, ['cat', 'dog']);
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      await tapWord(page, '#listsBody', 'dog');
      assertEq((await picksOf(page)).join(','), 'dog');
      const idx = await page.$$eval('#listsBody .lists-entry', (rows) =>
        rows.findIndex((r) => r.querySelector('.pq-word')?.textContent === 'dog'));
      await page.click(`#listsBody [data-e-remove="${idx}"]`);
      await domSettled(page);
      assertEq((await picksOf(page)).length, 0, 'the pick left with the word');
    } finally { await ctx.close(); }
  });

  // D9: one language per lineup. Once a lineup is running, words in another
  // language are not tappable in that list.
  await suite.test('pick_queue_a_lineup_never_mixes_two_languages', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await bootMine(page, base, ['cat', 'gato'], { gato: 'es-ES' });
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      await tapWord(page, '#listsBody', 'cat');
      assert((await page.$('#listsBody [data-pq-toggle][data-pq-word="gato"]')) === null,
        'the Spanish word is not a tap target while an English lineup runs');
      assertEq((await scopes(page)).length, 1, 'one scope');
    } finally { await ctx.close(); }
  });

  // D4 / I7 / A10: Spell Jr -- tap to line up, five at most, no extras.
  await suite.test('pick_queue_spell_jr_caps_at_five_and_refuses_the_sixth', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG_ON });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await bootMisses(page, base, [...FIVE, ['finch', -1], ['glove', -1]]);
      await openMisses(page);
      for (const w of ['apple', 'bread', 'crane', 'delta', 'eagle', 'finch']) await tapWord(page, '#pickBody', w);
      assertEq((await picksOf(page)).join(','), 'apple,bread,crane,delta,eagle', 'the sixth was refused');
      assert(await page.$('#pickBody .pq-note'), 'and the child is told why');
      assert((await page.$('#pickBody [data-pq-sort],#pickBody [data-pq-search],#pickBody [data-pq-star]')) === null,
        'no sort, search or stars (absent, not locked)');
      assert((await page.$('#pickBody [data-pq-just]')) === null, 'no Just these control for Spell Jr (D4)');
      assert(await page.$('#pickBody [data-pq-start]'), 'but the start button is there');
      // Taking one off makes room.
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'finch');
      assertEq((await picksOf(page)).length, 5);
    } finally { await ctx.close(); }
  });
}
