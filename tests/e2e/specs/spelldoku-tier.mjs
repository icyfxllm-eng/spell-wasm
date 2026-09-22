// spelldoku-tier.spec — CC-SPELLDOKU v1.3 Tier Mode, on the real screen.
//
// Done #11's audit sheet says WHICH words a board serves; this says the board
// plays: the ladder is legible before any cell is touched (I-T3), a digit's
// word comes from the tier its badge promised (F2), a misspelling costs the
// word and not the cell (F4/I-T2), and the same digit in the same cell is
// charged once (F8).
import { openApp, assert, assertEq } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

const board = (page) => page.evaluate(() => JSON.parse(window.__spelltest.spelldokuBoard() || 'null'));
const prompt = (page) => page.evaluate(() => JSON.parse(window.__spelltest.spelldokuPrompt() || 'null'));
const note = (page) => page.$eval('#sdNote', (e) => e.textContent);
const empties = (b) => b.clues.map((c, i) => [c, i]).filter(([c]) => c === 'Empty' || c.Fragment).map(([, i]) => i);

// CC-SPELLDOKU-RULES F5: the header chip is the Numbers/Letters/Mix picker and
// does not expose Tier Mode. Tier Mode keeps its readings there only with its
// own flag on, so every test here turns it on.
const TIER_FLAG = () => localStorage.setItem('spell_flag_sd_tier_mode', 'on');

async function openTier(page, reading, pick) {
  await page.evaluate(() => {
    window.__spelltest.spelldokuPreview(true);
    document.getElementById('sdOpenBtn').click();
  });
  await page.waitForSelector('#sdScreen.show', { timeout: 5000 });
  await page.selectOption('#sdTier', reading);
  await page.waitForTimeout(150);
  await page.selectOption('#sdPick', pick);
  await page.waitForTimeout(300);
}

async function spell(page, word) {
  for (const ch of word) await page.click(`#sdKeys [data-sd-key="${ch}"]`);
  await page.click('#sdKeys [data-sd-go]');
  await page.waitForTimeout(120);
}

export async function run(browser, base, suite) {
  // F3 / I-T3: the strip is there from board load, one badge per digit, and it
  // is the F1 Easy row -- Medium on 4, 8, 9 and Easy everywhere else.
  await suite.test('spelldoku_tier_ladder_is_legible_from_load', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: TIER_FLAG });
    try {
      await openTier(page, 'numbersIndexed', '9-easy');
      const bands = await page.$$eval('#sdTiers .sd-tierkey', (ks) =>
        ks.map((k) => [k.getAttribute('data-sd-tier'), [...k.classList].find((c) => c.startsWith('band-'))]));
      assertEq(bands.length, 9, 'nine badges, before a single cell is touched');
      const want = ['band-easy', 'band-easy', 'band-easy', 'band-medium', 'band-easy',
                    'band-easy', 'band-easy', 'band-medium', 'band-medium'];
      assertEq(bands.map(([, b]) => b).join(','), want.join(','), 'the F1 Easy row');
      assertEq(bands.map(([d]) => d).join(','), '1,2,3,4,5,6,7,8,9', 'one badge per digit');
    } finally { await ctx.close(); }
  });

  // F2 / F4 / F8: the drawn word commits the digit; a miss redraws the same
  // tier and leaves the cell empty; the same digit in that cell is then free.
  await suite.test('spelldoku_tier_word_commits_and_recommit_is_free', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: TIER_FLAG });
    try {
      await openTier(page, 'numbersIndexed', '9-easy');
      const b = await board(page);
      const i = empties(b)[0];
      const v = b.solution[i];
      await page.click(`[data-sd-cell="${i}"]`);
      // Typing the number word commits nothing now: the ladder sets the price.
      await spell(page, 'one');
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), '', 'the number word places nothing');
      await page.click(`#sdTiers [data-sd-tier="${v}"]`);
      await page.waitForTimeout(150);
      const first = await prompt(page);
      assert(first && first.spelling, 'a word was drawn');
      assertEq(String(first.digit), String(v), 'drawn for the digit that was tapped');
      // A misspelling: same tier, a NEW word, cell untouched (F4, I-T2).
      await spell(page, first.spelling + 'x');
      const second = await prompt(page);
      assert(second && second.spelling !== first.spelling, 'a miss draws a different word');
      assertEq(second.band, first.band, 'of the same tier');
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), '', 'the cell stays empty');
      // Spelling the word it is now asking for places the digit.
      await spell(page, second.spelling);
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), String(v), 'the digit is placed');
      // F8: that (cell, digit) is paid for -- tapping it again asks nothing.
      await page.click(`[data-sd-cell="${i}"]`);
      await page.click(`#sdTiers [data-sd-tier="${v}"]`);
      await page.waitForTimeout(150);
      assertEq(await prompt(page), null, 'the same digit in the same cell is free');
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), String(v), 'and it stays placed');
    } finally { await ctx.close(); }
  });

  // I-T4: the words a board served enter the player's D9 repeat window, in the
  // ledger Word Search and Spell Cross share, and the next board skips them.
  // Only the browser can see this: the window lives in localStorage.
  await suite.test('spelldoku_tier_served_words_enter_the_repeat_window', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: TIER_FLAG });
    try {
      await openTier(page, 'numbersIndexed', '9-easy');
      const b = await board(page);
      const drawn = [];
      for (const i of empties(b).slice(0, 3)) {
        await page.click(`[data-sd-cell="${i}"]`);
        await page.click(`#sdTiers [data-sd-tier="${b.solution[i]}"]`);
        await page.waitForTimeout(120);
        const p = await prompt(page);
        if (p) drawn.push(p.spelling);
      }
      assert(drawn.length === 3, 'three words were drawn');
      const before = await page.evaluate(() => localStorage.getItem('spell_wordgrid_seen_v1'));
      assertEq(before, null, 'nothing is recorded while the board is still being played');
      // A new board ends the old one, so its words enter the window.
      await page.click('#sdNew');
      await page.waitForTimeout(400);
      const led = await page.evaluate(() => JSON.parse(localStorage.getItem('spell_wordgrid_seen_v1') || 'null'));
      assert(led, 'the window was written');
      assertEq(led.counter, 1, 'one board counted as one puzzle');
      const held = led.seen.filter((s) => drawn.includes(s.w));
      assertEq(held.length, drawn.length, `all three words are held: ${JSON.stringify(led.seen)}`);
      assert(led.seen.every((s) => s.k.startsWith('en:')), 'keyed by language and band');
      // And the next board does not serve them back.
      const b2 = await board(page);
      const i = empties(b2)[0];
      await page.click(`[data-sd-cell="${i}"]`);
      await page.click(`#sdTiers [data-sd-tier="${b2.solution[i]}"]`);
      await page.waitForTimeout(120);
      const next = await prompt(page);
      assert(next && !drawn.includes(next.spelling), `${next && next.spelling} came straight back`);
    } finally { await ctx.close(); }
  });

  // I-T7: a pencil mark never fires a gate.
  await suite.test('spelldoku_tier_pencil_marks_are_free', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: TIER_FLAG });
    try {
      await openTier(page, 'numbersIndexed', '9-easy');
      const b = await board(page);
      const i = empties(b)[0];
      await page.click(`[data-sd-cell="${i}"]`);
      await page.click('#sdPencil');
      await page.click('#sdKeys [data-sd-pen="3"]');
      await page.waitForTimeout(120);
      assertEq(await prompt(page), null, 'no word was drawn for a mark');
      assertEq(await page.$eval(`[data-sd-cell="${i}"]`, (e) => e.textContent.trim()), '3', 'the mark is on the cell');
      assertEq(await page.$$eval('#sdChips .sd-chip.free', (c) => c.length), 0, 'and nothing was unlocked');
    } finally { await ctx.close(); }
  });

  // D-T5 / F7: Jr asks for tier symbols and gets Reading A, never a Hard or
  // Expert word. The whole Easy row on a Jr board is Easy or Medium.
  await suite.test('spelldoku_tier_jr_never_leaves_easy_medium', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID, init: TIER_FLAG });
    try {
      await openTier(page, 'tierSymbols', '4-easy');
      const bands = await page.$$eval('#sdTiers .sd-tierkey', (ks) =>
        ks.map((k) => [...k.classList].find((c) => c.startsWith('band-'))));
      assert(bands.length > 0, 'Jr gets a ladder');
      assert(bands.every((b) => b === 'band-easy' || b === 'band-medium'), `Jr bands: ${bands}`);
    } finally { await ctx.close(); }
  });
}
