// pick-queue.spec -- CC-PICK-QUEUE v1 Phase A: tap words to line them up, in
// Misses and in My Words, behind `spell_flag_pick_queue_a` (default OFF).
// Phase B (focus stars) rides the same file behind `spell_flag_pick_queue_b`,
// also default OFF and only live while Phase A is on.
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
// Phase B: both flags on / lineups only (B explicitly off).
const FLAG_AB = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'on');
};
const FLAG_A_ONLY = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
};
const STARS_KEY = 'spell_stars_v1';
const stars = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || 'null'), STARS_KEY);
const starWords = async (page) => ((await stars(page))?.stars || []).map((p) => p.word);
const starBtn = (scope, word) => `${scope} [data-pq-star][data-pq-word="${word}"]`;
const tapStar = async (page, scope, word) => {
  await page.click(starBtn(scope, word));
  await domSettled(page);
};
const listsData = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || 'null'), LISTS_KEY);

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
    localStorage.removeItem('spell_stars_v1');
  }, [MISS_KEY, rows]);
  await reload(page, base);
}

async function bootMine(page, base, words, wordLang = {}) {
  await page.evaluate(([ck, lk, set]) => {
    localStorage.setItem(ck, JSON.stringify(set));
    localStorage.removeItem(lk);
    localStorage.removeItem('spell_lineup_v1');
    localStorage.removeItem('spell_stars_v1');
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


// ---- Phase D (F4): missed picks come back ----
const FLAG_ABD = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
  localStorage.setItem('spell_flag_pick_queue_d', 'on');
};
const FLAG_AD_OFF = () => {
  localStorage.setItem('spell_flag_pick_queue_a', 'on');
  localStorage.setItem('spell_flag_pick_queue_b', 'off');
  localStorage.setItem('spell_flag_pick_queue_d', 'off');
};

/// Spell the current word wrong (the one attempt the base game gives).
async function answerWrong(page) {
  const w = await page.evaluate(() => window.__spelltest.currentWord());
  await page.waitForFunction(() => !document.getElementById('gameKeyboard').classList.contains('locked'),
    null, { timeout: 5000 }).catch(() => {});
  // Right length, deliberately wrong: the first letter is swapped for one the word does not contain.
  const bad = [...w.toLowerCase()];
  bad[0] = ['q', 'z', 'x', 'j'].find((c) => !bad.includes(c));
  await type(page, bad.join(''));
  await page.click('#checkBtn');
  await page.waitForFunction(() => document.getElementById('feedback').className.includes('bad'), null, { timeout: 5000 });
  return w;
}

const missCount = async (page, word) => (await misses(page)).find((m) => m.word === word)?.misses;

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

  // ================= Phase B: focus stars =================

  // Dual config: stars flag OFF (lineups ON) is Phase A exactly -- no star
  // control anywhere, nothing written to spell_stars_v1.
  await suite.test('pick_stars_flag_off_has_no_star_control_and_writes_nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_A_ONLY });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      assertEq((await badges(page)).length, 5, 'the lineup surface is there');
      assert((await page.$('#pickBody [data-pq-star]')) === null, 'no star control in Misses');
      await tapWord(page, '#pickBody', 'apple');
      assertEq((await stars(page)), null, 'nothing written');
      await bootMine(page, base, ['cat', 'dog']);
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      assert((await page.$('#listsBody [data-pq-star]')) === null, 'no star control in My Words');
    } finally { await ctx.close(); }
  });

  // A1/A2/A7/I2/I3 for stars in Misses: stars lead the session in the order
  // they were starred, once each, then today's queue; persists across reload.
  await suite.test('pick_stars_misses_star_leads_every_session_and_persists', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      assert(await page.$(starBtn('#pickBody', 'crane')), 'a star control on every row');
      // The star sits at the top-right (inline END) of the row, the badge at the START.
      const pos = await page.$eval(starBtn('#pickBody', 'crane'), (el) => {
        const cs = getComputedStyle(el);
        return { position: cs.position, insetInlineEnd: cs.insetInlineEnd, dir: getComputedStyle(el.closest('.lists-entry')).direction };
      });
      assertEq(pos.position, 'absolute');
      await tapStar(page, '#pickBody', 'eagle');
      await tapStar(page, '#pickBody', 'crane');
      assertEq((await starWords(page)).join(','), 'eagle,crane', 'stored in the order starred');
      assertEq(await page.$eval(starBtn('#pickBody', 'eagle'), (e) => e.getAttribute('aria-pressed')), 'true');
      await reload(page, base);
      assertEq((await starWords(page)).join(','), 'eagle,crane', 'survived a hard reload (A7)');
      await openMisses(page);
      assertEq(await page.$eval(starBtn('#pickBody', 'crane'), (e) => e.getAttribute('aria-pressed')), 'true', 'still starred on screen');
      await page.click('#pickBody [data-pq-all]');
      await domSettled(page);
      assert(await inReview(page), 'ordinary practice began');
      const served = [];
      for (let i = 0; i < 5; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.slice(0, 2).join(','), 'eagle,crane', 'starred words lead, in star order');
      assertEq([...served].sort().join(','), 'apple,bread,crane,delta,eagle', 'then the rest; nothing repeated (I3)');
      // Answering correctly on a due word does not unstar it: a star lasts
      // until the word is LEARNED, which only the unchanged schedule decides.
      assertEq((await starWords(page)).join(','), 'eagle,crane', 'stars are still there');
    } finally { await ctx.close(); }
  });

  // Ordering when both exist: picks first (tap order), then stars, then the queue.
  await suite.test('pick_stars_picks_lead_then_stars_then_the_rest', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapStar(page, '#pickBody', 'apple');
      await tapStar(page, '#pickBody', 'bread');
      await tapWord(page, '#pickBody', 'delta');
      await tapWord(page, '#pickBody', 'bread'); // picked AND starred: served once, as a pick
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const served = [];
      for (let i = 0; i < 4; i++) {
        served.push(await serve(page));
        await answerCorrect(page);
      }
      assertEq(served.join(',').startsWith('delta,bread,apple,'), true, `picks, then stars, then the queue (${served})`);
      assertEq(new Set(served).size, 4, 'no repeats');
    } finally { await ctx.close(); }
  });

  // D3 / I6 / A5 for stars: a not-due starred miss leads, counts as practice,
  // but is neither cleared nor does its next spaced check move.
  await suite.test('pick_stars_not_due_star_keeps_its_schedule_when_answered', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMisses(page, base, [['alpha', 3], ['bravo', -1]]);
      const before = (await misses(page)).find((m) => m.word === 'alpha');
      await openMisses(page);
      await tapStar(page, '#pickBody', 'alpha');
      await page.click('#pickBody [data-pq-all]');
      await domSettled(page);
      assertEq(await serve(page), 'alpha', 'the not-due star leads');
      await answerCorrect(page);
      const after = (await misses(page)).find((m) => m.word === 'alpha');
      assert(after, 'still in Misses: not cleared');
      assertEq(after.due, before.due, 'its next due date did not move');
      assertEq(after.misses, before.misses, 'miss count untouched');
      assertEq((await starWords(page)).join(','), 'alpha', 'and the star stays until it is learned');
    } finally { await ctx.close(); }
  });

  // A6: the 11th star in a language is refused, visibly.
  await suite.test('pick_stars_the_eleventh_star_is_refused_with_a_visible_message', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      const words = ['ant', 'bat', 'cow', 'dog', 'eel', 'fox', 'gnu', 'hen', 'ibis', 'jay', 'kiwi', 'lark'];
      await bootMisses(page, base, words.map((w) => [w, -1]));
      await openMisses(page);
      for (const w of words.slice(0, 10)) await tapStar(page, '#pickBody', w);
      assertEq((await starWords(page)).length, 10, 'ten stars fit');
      assert((await page.$('#pickBody .pq-note')) === null, 'and no complaint yet');
      await tapStar(page, '#pickBody', 'kiwi');
      assertEq((await starWords(page)).length, 10, 'the eleventh was refused');
      assert(!(await starWords(page)).includes('kiwi'));
      const note = await page.$('#pickBody .pq-note[role="status"]');
      assert(note, 'a status message is on screen');
      assert(await note.isVisible(), 'and it is visible');
      assert(/ten/i.test(await note.textContent()), 'and it says why');
      assertEq(await page.$eval(starBtn('#pickBody', 'kiwi'), (e) => e.getAttribute('aria-pressed')), 'false');
      // Unstarring is always allowed and makes room.
      await tapStar(page, '#pickBody', 'ant');
      await tapStar(page, '#pickBody', 'kiwi');
      assertEq((await starWords(page)).length, 10);
      assert((await starWords(page)).includes('kiwi'));
    } finally { await ctx.close(); }
  });

  // O-E: Reset this language clears that language's stars; sign-out is not tested here.
  await suite.test('pick_stars_reset_this_language_clears_its_stars', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapStar(page, '#pickBody', 'apple');
      assertEq((await starWords(page)).length, 1);
      await page.evaluate(() => document.getElementById('resetStats').click());
      await page.waitForFunction((k) => {
        const l = JSON.parse(localStorage.getItem(k) || 'null');
        return !l || l.stars.length === 0;
      }, STARS_KEY, { timeout: 5000 });
      assertEq((await starWords(page)).length, 0, 'cleared');
    } finally { await ctx.close(); }
  });

  // A newer build's blob is read as empty and never overwritten.
  await suite.test('pick_stars_a_newer_blob_is_kept_byte_for_byte', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMisses(page, base, FIVE);
      const blob = JSON.stringify({ v: 99, stars: [{ key: 'x', word: 'apple', lang: 'en' }], future: true });
      await page.evaluate(([k, b]) => localStorage.setItem(k, b), [STARS_KEY, blob]);
      await reload(page, base);
      await openMisses(page);
      assertEq(await page.$eval(starBtn('#pickBody', 'apple'), (e) => e.getAttribute('aria-pressed')), 'false', 'read as empty');
      await tapStar(page, '#pickBody', 'bread');
      assertEq(await page.evaluate((k) => localStorage.getItem(k), STARS_KEY), blob, 'never overwritten');
    } finally { await ctx.close(); }
  });

  // My Words: the star IS ListEntry.starred (O-A), leads the session in a Mixed
  // list, persists, and removing the word takes its star (O-D).
  await suite.test('pick_stars_my_words_star_reuses_the_entry_star_and_leads', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      await bootMine(page, base, ['cat', 'dog', 'fox', 'owl']);
      await openMyWords(page);
      await page.waitForSelector('#listsScreen.show');
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      assert((await page.$('#listsBody [data-e-star]')) === null, 'one star control per word, not two');
      await tapStar(page, '#listsBody', 'owl');
      let d = await listsData(page);
      assertEq(d.lists[0].entries.find((e) => e.text === 'owl').starred, true, 'the existing entry field');
      assertEq(await stars(page), null, 'no second store for My Words stars');
      await reload(page, base);
      d = await listsData(page);
      assertEq(d.lists[0].entries.find((e) => e.text === 'owl').starred, true, 'survived a hard reload (A7)');
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      await page.click('#listsBody [data-l-play]');
      await page.waitForFunction(() => window.__spelltest.currentLang() === '__mine', null, { timeout: 5000 });
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      assertEq(await page.evaluate(() => window.__spelltest.currentWord()), 'owl', 'the starred word leads');
      await answerCorrect(page);
      const next = await serve(page);
      assert(next !== 'owl', `and is not repeated (${next})`);
      // Removing the word from My Words removes its star.
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      const idx = await page.$$eval('#listsBody .lists-entry', (rows) =>
        rows.findIndex((r) => r.querySelector('.pq-word')?.textContent === 'owl'));
      await page.click(`#listsBody [data-e-remove="${idx}"]`);
      await domSettled(page);
      d = await listsData(page);
      assert(!d.lists[0].entries.some((e) => e.text === 'owl'), 'owl is gone');
      assertEq(d.lists[0].entries.filter((e) => e.starred).length, 0, 'and no star is left behind');
    } finally { await ctx.close(); }
  });

  // A6 in My Words: same component, same cap, same message.
  await suite.test('pick_stars_my_words_eleventh_star_is_refused_with_a_message', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AB });
    try {
      const words = ['ant', 'bat', 'cow', 'dog', 'eel', 'fox', 'gnu', 'hen', 'ibis', 'jay', 'kiwi', 'lark'];
      await bootMine(page, base, words);
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      for (const w of words.slice(0, 10)) await tapStar(page, '#listsBody', w);
      await tapStar(page, '#listsBody', 'kiwi');
      const d = await listsData(page);
      assertEq(d.lists[0].entries.filter((e) => e.starred).length, 10, 'still ten');
      const note = await page.$('#listsBody .pq-note[role="status"]');
      assert(note && (await note.isVisible()), 'the message is visible');
    } finally { await ctx.close(); }
  });

  // I7: Spell Jr shows no stars at all, and a persisted star changes nothing.
  await suite.test('pick_stars_spell_jr_has_no_stars_and_ignores_persisted_ones', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG_AB });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await bootMisses(page, base, FIVE);
      await page.evaluate(([k, v]) => localStorage.setItem(k, v), [STARS_KEY,
        JSON.stringify({ v: 1, stars: [{ key: 'en::eagle', word: 'eagle', lang: 'en' }] })]);
      await reload(page, base);
      await openMisses(page);
      assertEq((await badges(page)).length, 5, 'the lineup surface is there');
      assert((await page.$('#pickBody [data-pq-star]')) === null, 'no star control rendered');
      // And in My Words order: the persisted entry star does not lead for Jr.
      await bootMine(page, base, ['cat', 'dog', 'fox', 'owl']);
      await page.evaluate((k) => {
        const d = JSON.parse(localStorage.getItem(k));
        d.lists[0].order = 'InMyOrder';
        d.lists[0].entries[3].starred = true;
        localStorage.setItem(k, JSON.stringify(d));
      }, LISTS_KEY);
      await reload(page, base);
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      assert((await page.$('#listsBody [data-pq-star],#listsBody [data-e-star]')) === null, 'no star control in My Words either');
      await page.click('#listsBody [data-l-play]');
      await page.waitForFunction(() => window.__spelltest.currentLang() === '__mine', null, { timeout: 5000 });
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      assertEq(await page.evaluate(() => window.__spelltest.currentWord()), 'cat', 'saved order; the star is ignored');
    } finally { await ctx.close(); }
  });

  // Flag OFF entirely: the existing in-my-order star is exactly today's.
  await suite.test('pick_stars_everything_off_keeps_the_in_my_order_star', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_OFF });
    try {
      await bootMine(page, base, ['cat', 'dog', 'fox', 'owl']);
      await page.evaluate((k) => {
        const d = JSON.parse(localStorage.getItem(k));
        d.lists[0].order = 'InMyOrder';
        localStorage.setItem(k, JSON.stringify(d));
      }, LISTS_KEY);
      await reload(page, base);
      await openMyWords(page);
      await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
      await domSettled(page);
      assert(await page.$('#listsBody [data-e-star="3"]'), 'the old star control is there');
      assert((await page.$('#listsBody [data-pq-star]')) === null, 'and no new one');
      await page.click('#listsBody [data-e-star="3"]');
      await domSettled(page);
      const d = await listsData(page);
      assertEq(d.lists[0].entries[3].starred, true);
      assertEq(await stars(page), null, 'nothing else written');
    } finally { await ctx.close(); }
  });

  // ================= Phase D: F4, missed picks come back =================

  // A4 (Misses): a lined-up word spelled wrong returns 3-5 words later in the
  // same session; the wrong answer is recorded exactly as ever (I1).
  await suite.test('pick_requeue_misses_missed_pick_returns_three_to_five_later', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ABD });
    try {
      await bootMisses(page, base, [...FIVE, ['finch', -1], ['glove', -1]]);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'bread');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const seq = [];
      let first = await serve(page);
      assertEq(first, 'apple', 'the first pick leads');
      seq.push(first);
      await answerWrong(page);
      assertEq(await missCount(page, 'apple'), 2, 'the miss is recorded as it always was (I1)');
      for (let i = 1; i <= 7 && !(i > 1 && seq[i - 1] === 'apple'); i++) {
        seq.push(await serve(page));
        if (seq[i] !== 'apple') await answerCorrect(page);
      }
      assertEq(seq[1], 'bread', 'the rest of the lineup is not displaced');
      const back = seq.indexOf('apple', 1);
      assert(back >= 3 && back <= 5, `apple came back ${back} words later (${seq.join(',')})`);
      assertEq(seq.filter((w) => w === 'bread').length, 1, 'a correct pick is not repeated (I3)');
    } finally { await ctx.close(); }
  });

  // "Just these" with a miss: the session ends only after the re-queue is served.
  await suite.test('pick_requeue_just_these_keeps_a_miss_inside_the_session', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ABD });
    try {
      await bootMisses(page, base, FIVE);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'apple');
      await tapWord(page, '#pickBody', 'bread');
      await page.click('#pickBody [data-pq-just]');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const seq = [];
      seq.push(await serve(page));
      await answerWrong(page);
      seq.push(await serve(page));
      await answerCorrect(page);
      seq.push(await serve(page));
      await answerCorrect(page);
      assertEq(seq.join(','), 'apple,bread,apple', 'the miss goes to the end of a short lineup');
      assert(await inReview(page), 'still in the session until the re-queue is done');
      await page.click('#orbWrap');
      await page.waitForFunction(() => !document.getElementById('missesBtn').classList.contains('on'),
        null, { timeout: 5000 });
      assert(!(await inReview(page)), 'then Just these ends (A9 with a miss)');
    } finally { await ctx.close(); }
  });

  // My Words: the same logic (D10). A4 on, and flag off is today's app.
  async function myWordsMiss(page, serves = 7) {
    await bootMine(page, base, ['cat', 'dog', 'fox', 'owl']);
    await openMyWords(page);
    await page.waitForSelector('#listsScreen.show');
    await page.click('#listsBody [data-l-open]:not([data-l-open="__all"])');
    await domSettled(page);
    await tapWord(page, '#listsBody', 'cat');
    await page.click('#listsBody [data-pq-start]');
    await page.waitForFunction(() => window.__spelltest.currentLang() === '__mine', null, { timeout: 5000 });
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
    assertEq(await page.evaluate(() => window.__spelltest.currentWord()), 'cat', 'the pick leads');
    await answerWrong(page);
    const seq = [];
    for (let i = 1; i <= serves; i++) {
      const w = await serve(page);
      seq.push(w);
      if (w === 'cat') break;
      await answerCorrect(page);
    }
    return seq;
  }

  await suite.test('pick_requeue_my_words_missed_pick_returns_three_to_five_later', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ABD });
    try {
      const seq = await myWordsMiss(page);
      const at = seq.indexOf('cat') + 1;
      assert(at >= 3 && at <= 5, `cat came back ${at} words later (${seq.join(',')})`);
    } finally { await ctx.close(); }
  });

  await suite.test('pick_requeue_flag_off_is_todays_app', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_AD_OFF });
    try {
      // Three other words in the list: the next three serves are exactly them.
      const seq = await myWordsMiss(page, 3);
      assert(!seq.includes('cat'), `Phase D off: a missed pick is not re-queued (${seq.join(',')})`);
    } finally { await ctx.close(); }
  });

  // Spell Jr: a tap-to-line-up pick missed also returns (F4 is not Jr-excluded).
  await suite.test('pick_requeue_spell_jr_missed_pick_returns', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG_ABD });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await bootMisses(page, base, [...FIVE, ['finch', -1], ['glove', -1]]);
      await openMisses(page);
      await tapWord(page, '#pickBody', 'apple');
      await page.click('#pickBody [data-pq-start]');
      await domSettled(page);
      const seq = [await serve(page)];
      assertEq(seq[0], 'apple');
      await answerWrong(page);
      for (let i = 1; i <= 7; i++) {
        const w = await serve(page);
        seq.push(w);
        if (w === 'apple') break;
        await answerCorrect(page);
      }
      const back = seq.indexOf('apple', 1);
      assert(back >= 3 && back <= 5, `apple came back ${back} words later (${seq.join(',')})`);
    } finally { await ctx.close(); }
  });
}
