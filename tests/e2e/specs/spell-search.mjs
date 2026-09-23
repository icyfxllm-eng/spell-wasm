// spell-search.spec — CC-WORDGRID v1 Phase A (Spell Search), on the real screen.
//
// The grids are the real generator's, from the real banks and the shipped
// decoy table; the test hook only READS the served puzzle so a test knows where
// to drag. Audio is stubbed by the harness, as for every spec.
import { openApp, assert, assertEq } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const LAUNCH = ['en', 'es', 'ru', 'fr', 'de', 'pt', 'pl', 'fil'];
// src/wordsearch/tests.rs GOLDEN: the host's 500-seed grid digest.
// Re-pinned 2026-09-22 with src/wordsearch/tests.rs, which carries the
// reason: CC-AUDIO-CLARITY C10 took five other-variety words out of the
// English bank, so the pool those 500 seeds draw from is a different pool.
const GOLDEN = '0xcc9d3e40658b6357';

const board = (page) => page.evaluate(() => JSON.parse(window.__spelltest.spellSearchBoard() || 'null'));

async function openSearch(page) {
  await page.evaluate(() => document.getElementById('wsOpenBtn').click());
  await page.waitForSelector('#wsScreen.show', { timeout: 8000 });
}

async function center(page, i) {
  const box = await page.$eval(`#wsGrid [data-ws-cell="${i}"]`, (e) => {
    const r = e.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  });
  return box;
}

// A finger drag from the first cell to the last, through the ones between.
async function drag(page, cells) {
  const a = await center(page, cells[0]);
  const b = await center(page, cells[cells.length - 1]);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(60);
}

async function typeWord(page, word) {
  for (const ch of word) await page.click(`#wsKeys [data-ws-key="${ch}"]`);
  await page.click('#wsKeys [data-ws-go]');
  await page.waitForTimeout(60);
}

const learnerLog = (page, lang) => page.evaluate((l) => {
  const s = JSON.parse(localStorage.getItem(`spell_learner_${l}`) || 'null');
  return s ? s.log : [];
}, lang);

export async function run(browser, base, suite) {
  // Launch set: every language opens onto a grid whose slots show only length.
  await suite.test('spell_search_opens_in_every_launch_language', async () => {
    for (const lang of LAUNCH) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await openSearch(page);
        const b = await board(page);
        const cells = await page.$$eval('#wsGrid .ws-cell', (c) => c.length);
        assertEq(cells, b.size * b.size, `${lang}: a ${b.size}x${b.size} grid`);
        const slots = await page.$$eval('#wsSlots .ws-word', (s) => s.map((e) => e.textContent));
        assertEq(slots.length, b.targets.length, `${lang}: one slot per target`);
        for (const s of slots) assert(/^_+$/.test(s), `${lang}: F-S1 a slot shows only length, got "${s}"`);
        if (lang !== 'en') assertEq(b.decoys.length, 0, `${lang}: no decoys without a dictionary (D11)`);
        const opts = await page.$$eval('#wsPick option', (o) => o.map((x) => x.value));
        assertEq(JSON.stringify(opts), JSON.stringify(lang === 'en' ? ['easy', 'medium', 'hard', 'expert'] : ['easy']), `${lang}: tiers`);
      } finally { await ctx.close(); }
    }
  });

  // Test 14 / D4: ko and zh never open it.
  await suite.test('spell_search_never_opens_in_korean_or_chinese', async () => {
    for (const lang of ['ko', 'zh']) {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await page.evaluate(() => document.getElementById('wsOpenBtn').click());
        await page.waitForTimeout(400);
        assert(!(await page.$eval('#wsScreen', (e) => e.classList.contains('show'))), `${lang}: no Spell Search`);
      } finally { await ctx.close(); }
    }
  });

  // F-S1: hear it, find it, and the slot shows the word.
  await suite.test('spell_search_a_drag_finds_a_word', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSearch(page);
      const b = await board(page);
      await page.click('#wsSlots [data-ws-play="0"]');
      const t = b.targets[0];
      await drag(page, [...t.cells].reverse()); // backwards counts too
      assertEq(await page.$eval('#wsSlots [data-ws-slot="0"] .ws-word', (e) => e.textContent), t.word, 'the slot reveals the word');
      assert(await page.$eval('#wsSlots [data-ws-slot="0"]', (e) => e.classList.contains('found')), 'and is marked found');
      assert(/found/i.test(await page.$eval('#wsNote', (e) => e.textContent)), 'Found it!');
    } finally { await ctx.close(); }
  });

  // F-S2, F-S4, F-X6, I11: a trap costs a star, never a shield; Lock It In
  // grades through the base game's log; stars add up.
  await suite.test('spell_search_trap_then_lock_in_scores_in_stars_only', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSearch(page);
      const b = await board(page);
      assert(b.tier === 'easy' && b.decoys.length === 2, `English Easy carries 2 decoys (got ${b.decoys.length})`);
      const shieldsBefore = await page.$eval('#shieldCount', (e) => e.textContent);

      const d = b.decoys[0];
      await drag(page, d.cells);
      assertEq(await page.$eval('#wsNote', (e) => e.textContent), "That's the trap spelling", 'F-S2 copy');
      await drag(page, d.cells); // the same trap again costs nothing more
      const traps = (await learnerLog(page, 'en')).filter((a) => a.channel === 'trap');
      assertEq(traps.length, 1, 'one TRAP_MISS in the base-game log');
      assertEq(traps[0].word, b.targets[d.of].word, 'filed against its target');
      assertEq(traps[0].typed, d.word, 'with the decoy as what was taken');

      for (const t of b.targets) await drag(page, t.cells);
      await page.waitForSelector('#wsLock:not(.btn-hide)', { timeout: 3000 });
      assert(await page.$eval('#wsFind', (e) => e.classList.contains('btn-hide')), 'F-S4: the grid is hidden');

      // One deliberate miss, then every word right.
      let first = true;
      for (let k = 0; k < b.targets.length; k++) {
        const now = await board(page);
        if (first) { await typeWord(page, 'x'.repeat(now.asking.length)); first = false; continue; }
        await typeWord(page, now.asking);
      }
      const done = await board(page);
      assertEq(done.stars, 1, '3 stars, less 1 trap, less 1 miss');
      assertEq(await page.$eval('#wsStars', (e) => e.textContent), '★☆☆', 'the stars shown');
      const typed = (await learnerLog(page, 'en')).filter((a) => a.channel === 'typed');
      assertEq(typed.length, b.targets.length, 'every Lock It In answer is in the base-game log');
      assertEq(typed.filter((a) => !a.correct).length, 1, 'the miss among them');
      assertEq(await page.$eval('#shieldCount', (e) => e.textContent), shieldsBefore, 'I11: shields untouched');
    } finally { await ctx.close(); }
  });

  // Test 12 / F-X5: an under-13 player gets Jr only: → ↓, no decoys.
  await suite.test('spell_search_spell_jr_gets_only_jr', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID });
    try {
      await openSearch(page);
      const opts = await page.$$eval('#wsPick option', (o) => o.map((x) => x.value));
      assertEq(JSON.stringify(opts), '["jr"]', 'Jr is the only choice');
      for (let k = 0; k < 3; k++) {
        const b = await board(page);
        assertEq(b.tier, 'jr', 'a Jr grid');
        assertEq(b.decoys.length, 0, 'no decoys');
        for (const t of b.targets) {
          const step = t.cells[1] - t.cells[0];
          assert(step === 1 || step === b.size, `${t.word} runs → or ↓`);
        }
        await page.click('#wsNew');
        await page.waitForTimeout(150);
      }
      await page.click('#wsDaily');
      assertEq((await board(page)).tier, 'jr', 'the Daily is Jr too');
    } finally { await ctx.close(); }
  });

  // Test 2 / I8: the app's WebAssembly builds the same 500 grids as the host.
  await suite.test('spell_search_wasm_matches_the_pinned_golden_digest', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      assertEq(await page.evaluate(() => window.__spelltest.spellSearchGolden()), GOLDEN, 'wasm digest = host digest');
    } finally { await ctx.close(); }
  });

  // F-X1: the Daily is the same grid for everyone.
  await suite.test('spell_search_daily_is_the_same_for_everyone', async () => {
    const hashes = [];
    for (let k = 0; k < 2; k++) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await openSearch(page);
        await page.click('#wsDaily');
        await page.waitForTimeout(150);
        hashes.push((await board(page)).hash);
      } finally { await ctx.close(); }
    }
    assertEq(hashes[0], hashes[1], 'two players, one Daily');
  });

  // F-X1 / test 4: "Make a puzzle" on a My Words list; the same list twice
  // lays out differently.
  await suite.test('spell_search_make_a_puzzle_from_my_words', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const words = ['planet', 'garden', 'yellow', 'rabbit', 'window', 'orange', 'basket', 'candle', 'button', 'pocket'];
      await page.evaluate((ws) => {
        const now = Date.now();
        localStorage.setItem('byear_word_lists_v1', JSON.stringify({
          v: 1, nextId: 2, migrated: true, selection: [],
          lists: [{ id: 'l1', name: 'Spelling test', createdAt: now, updatedAt: now, source: 'Manual', order: 'Mixed',
            entries: ws.map((w) => ({ text: w, lang: 'en', addedAt: now })) }],
        }));
      }, words);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      const hashes = [];
      for (let k = 0; k < 2; k++) {
        await page.evaluate(() => document.getElementById('importBtn').click());
        await page.waitForSelector('#listsScreen.show', { timeout: 5000 });
        await page.click('[data-l-open="l1"]');
        await page.click('[data-l-puzzle="l1"]');
        await page.waitForSelector('#wsScreen.show', { timeout: 5000 });
        const b = await board(page);
        for (const t of b.targets) assert(words.includes(t.word), `${t.word} is from the list`);
        hashes.push(b.hash);
        await page.click('#wsExit');
      }
      assert(hashes[0] !== hashes[1], 'the same list, a different layout');
    } finally { await ctx.close(); }
  });

  // F-X4 / I10: a definition that shows the word is not shown.
  await suite.test('spell_search_meaning_never_gives_the_word_away', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSearch(page);
      const b = await board(page);
      const w = b.targets[0].word;
      await ctx.route('**/api/meaning**', (r) => r.fulfill({
        status: 200, contentType: 'application/json',
        body: JSON.stringify({ pos: 'noun', definition: `a kind of ${w} you can see`, example: '' }),
      }));
      await page.click('#wsSlots [data-ws-play="0"]');
      await page.click('#wsMeaningBtn');
      await page.waitForFunction(() => document.getElementById('wsMeaning').textContent.length > 0, null, { timeout: 4000 });
      const shown = await page.$eval('#wsMeaning', (e) => e.textContent);
      assert(!shown.toLowerCase().includes(w), `the hint must not contain "${w}" (showed "${shown}")`);
      assertEq(shown, 'No meaning hint for this word', 'the word offers no meaning hint instead');
    } finally { await ctx.close(); }
  });

  // Test 11: a 12-word list builds in 300 ms at p95 (desktop WebAssembly here;
  // the oldest-device figure is measured on the Simulator).
  await suite.test('spell_search_twelve_word_list_builds_fast', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const words = JSON.stringify(['planet', 'garden', 'yellow', 'rabbit', 'window', 'orange', 'basket', 'candle', 'button', 'pocket', 'silver', 'forest']);
      const times = await page.evaluate((ws) => {
        const out = [];
        for (let n = 0; n < 40; n++) out.push(window.__spelltest.spellSearchTimeList('en', ws, n));
        return out;
      }, words);
      assert(times.every((t) => t >= 0), 'every build served a puzzle');
      const sorted = [...times].sort((a, b) => a - b);
      const p95 = sorted[Math.floor(sorted.length * 0.95) - 1];
      assert(p95 <= 300, `p95 ${p95.toFixed(1)} ms`);
    } finally { await ctx.close(); }
  });
}
