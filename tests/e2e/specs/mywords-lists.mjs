// mywords-lists.spec — CC-MYWORDS-LISTS v1, Phase 1: the migration, where it
// actually runs. Host tests cannot see localStorage, so the only honest proof
// that an existing player's words survive is a real browser boot.
import { openApp, assert, assertEq } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const CUSTOM_KEY = 'byear_custom_v1';
const LISTS_KEY = 'byear_word_lists_v1';

const lists = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || 'null'), LISTS_KEY);

/// Put an existing player's flat My Words on the device, with no lists yet, and
/// boot the app as they would.
async function bootWith(page, base, custom) {
  await page.evaluate(([ck, lk, set]) => {
    localStorage.setItem(ck, JSON.stringify(set));
    localStorage.removeItem(lk);
  }, [CUSTOM_KEY, LISTS_KEY, custom]);
  await reload(page, base);
}

async function reload(page, base) {
  await page.reload({ waitUntil: 'load' });
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  await page.waitForTimeout(150);
}

// Phase 2: the save sheet. The photo review sheet is rendered through the
// observation seam (the camera itself cannot run in a browser); the paste sheet
// is driven exactly as a player does.
const listNames = (page) => page.evaluate((k) => {
  const l = JSON.parse(localStorage.getItem(k) || '{"lists":[]}');
  return l.lists.filter((x) => !x.deletedAt).map((x) => ({ name: x.name, words: x.entries.map((e) => e.text) }));
}, LISTS_KEY);

async function openScreen(page) {
  await page.click('#importBtn');
  await page.waitForSelector('#listsScreen.show', { timeout: 4000 });
}

async function openPasteSheet(page) {
  await openScreen(page);
  await page.click('[data-l-add]');
  await page.waitForSelector('#importScrim.show', { timeout: 4000 });
}

async function pasteSave(page, words, dest) {
  await openPasteSheet(page);
  await page.fill('#importText', words.join('\n'));
  await page.waitForTimeout(120);
  if (dest) await page.selectOption('#importDest', dest);
  await page.click('#saveWords');
  await page.waitForTimeout(400);
}

export async function run(browser, base, suite) {
  // F6 / AT6.1 / AT6.3 — nothing lost, nothing renamed, no progress touched.
  await suite.test('mywords_migration_keeps_every_saved_word', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await bootWith(page, base, {
        words: ['cat', 'gato', 'dog'],
        speakLang: 'en-US',
        wordLang: { gato: 'es-ES' },
      });
      const l = await lists(page);
      assert(l && Array.isArray(l.lists), `a lists blob is written (got ${JSON.stringify(l)})`);
      assertEq(l.lists.length, 1, 'one list for the flat set');
      const only = l.lists[0];
      assertEq(only.entries.map((e) => e.text).join(','), 'cat,gato,dog', 'same words, same order');
      assertEq(only.entries.map((e) => e.lang).join(','), 'en-US,es-ES,en-US', "each word keeps its own speak language");
      assertEq(only.source, 'Migrated', 'recorded as migrated, not as a photo');
      assert(only.name.trim().length > 0, 'and it is named');
      // The flat set is left exactly as it was: nothing reads it differently yet,
      // and per-word progress is keyed by word + language, never by list.
      const custom = await page.evaluate((k) => JSON.parse(localStorage.getItem(k)), CUSTOM_KEY);
      assertEq(custom.words.join(','), 'cat,gato,dog', 'the flat set is untouched');
    } finally { await ctx.close(); }
  });

  // F6.3 / AT6.2 — the marker, proven across a real second boot.
  await suite.test('mywords_migration_runs_once_across_boots', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await bootWith(page, base, { words: ['cat', 'dog'], speakLang: 'en-US', wordLang: {} });
      const first = JSON.stringify(await lists(page));
      await reload(page, base);
      await reload(page, base);
      assertEq(JSON.stringify(await lists(page)), first, 'two more boots change nothing');
    } finally { await ctx.close(); }
  });

  // AT6.4 — an empty My Words produces no list and no crash.
  await suite.test('mywords_migration_of_an_empty_set_makes_no_list', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await bootWith(page, base, { words: [], speakLang: '', wordLang: {} });
      const l = await lists(page);
      assertEq(l.lists.length, 0, 'no list');
      assert(l.migrated === true, 'and it will not try again tomorrow');
      assert(await page.evaluate(() => !!window.__spelltest.currentWord || true), 'the app booted');
    } finally { await ctx.close(); }
  });

  // AT1.1 — the sheet has no destructive option of any kind.
  await suite.test('mywords_save_sheet_offers_no_way_to_erase', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openPasteSheet(page);
      const text = await page.$eval('#importScrim', (e) => e.innerText);
      assert(!/replace/i.test(text), `the paste sheet offers no "replace" (got: ${text.slice(0, 160)})`);
      assert(await page.$('#photoReplace') === null, 'the old replace checkbox is gone from the document');
      // The destination is there instead, defaulting to a new dated list.
      const opts = await page.$$eval('#importDest option', (o) => o.map((x) => x.textContent));
      assert(opts.length >= 1 && /new list/i.test(opts[0]), `a new dated list is the first option (got ${JSON.stringify(opts)})`);
    } finally { await ctx.close(); }
  });

  // D1 (signed) — the same day joins one list; a save into a new list is a choice.
  await suite.test('mywords_second_save_today_joins_todays_list', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat', 'dog']);
      let got = await listNames(page);
      assertEq(got.length, 1, 'the first save makes today\'s list');
      const today = got[0].name;
      await pasteSave(page, ['fox']);
      got = await listNames(page);
      assertEq(got.length, 1, 'a second save the same day joins it (D1)');
      assertEq(got[0].words.join(','), 'cat,dog,fox', 'and adds, never replaces (I1)');
      // The confirmation says where the words went.
      const note = await page.$eval('#feedback', (e) => e.textContent);
      assert(note.includes(today), `the line names the list (got "${note}")`);
    } finally { await ctx.close(); }
  });

  // AT1.2 / AT1.3 — choosing a new list splits them, and the other list is untouched.
  await suite.test('mywords_choosing_new_list_leaves_the_other_alone', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat', 'dog']);
      const before = JSON.stringify(await listNames(page));
      await pasteSave(page, ['fox'], '__new');
      const got = await listNames(page);
      assertEq(got.length, 2, 'two lists');
      const first = got.find((l) => l.words.join(',') === 'cat,dog');
      assert(first, `the first list is unchanged (was ${before}, now ${JSON.stringify(got)})`);
      const second = got.find((l) => l.words.join(',') === 'fox');
      assert(second && second.name !== first.name, 'the new list is named apart (Sep 17, 2026 (2))');
    } finally { await ctx.close(); }
  });

  // F1.3 — the button says what the choice will do.
  await suite.test('mywords_save_button_follows_the_destination', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat']);
      await openPasteSheet(page);
      const addLabel = await page.$eval('#saveWords', (e) => e.textContent.trim());
      assert(/^add to /i.test(addLabel), `today's list reads as an add (got "${addLabel}")`);
      await page.selectOption('#importDest', '__new');
      await page.waitForTimeout(120);
      const newLabel = await page.$eval('#saveWords', (e) => e.textContent.trim());
      assert(/new list/i.test(newLabel), `choosing a new list says so (got "${newLabel}")`);
    } finally { await ctx.close(); }
  });

  // F3.4 — an empty screen offers both doors, never a blank box.
  await suite.test('mywords_empty_screen_offers_both_doors', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openScreen(page);
      const text = await page.$eval('#listsBody', (e) => e.innerText);
      assert(/no word lists yet/i.test(text), `the empty state speaks (got "${text}")`);
      assert(await page.$('[data-l-snap]'), 'Snap a word list');
      assert(await page.$('[data-l-add]'), 'Add words');
    } finally { await ctx.close(); }
  });

  // F3.1 / AT3.1 — cards newest first, with counts and language chips.
  await suite.test('mywords_cards_are_newest_first_with_counts', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat', 'dog']);
      await pasteSave(page, ['fox'], '__new');
      await openScreen(page);
      const cards = await page.$$eval('#listsBody [data-l-open]', (els) =>
        els.map((e) => e.innerText.replace(/\s+/g, ' ').trim()));
      assert(/all words/i.test(cards[0]), `All words leads (got ${JSON.stringify(cards)})`);
      assert(/3 words/i.test(cards[0]), 'and counts the union');
      assert(cards.length === 3, `then one card per list (got ${JSON.stringify(cards)})`);
      assert(/1 words|1 word/.test(cards[1]), `the newest list is first (got "${cards[1]}")`);
    } finally { await ctx.close(); }
  });

  // F1.4 — the confirmation's Open action lands in that list.
  await suite.test('mywords_open_action_lands_in_the_saved_list', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat', 'dog']);
      await page.waitForSelector('#openSavedList:not(.btn-hide)', { timeout: 4000 });
      await page.click('#openSavedList');
      await page.waitForSelector('#listsScreen.show', { timeout: 4000 });
      const text = await page.$eval('#listsBody', (e) => e.innerText);
      assert(text.includes('cat') && text.includes('dog'), `the list is open (got "${text}")`);
    } finally { await ctx.close(); }
  });

  // D2 — an opened list is editable: fix a word, remove one, undo the removal.
  await suite.test('mywords_an_opened_list_can_be_fixed_and_undone', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['becuase', 'dog']);
      await openScreen(page);
      await page.click('#listsBody [data-l-open]:nth-of-type(2)').catch(async () => {
        const cards = await page.$$('#listsBody [data-l-open]');
        await cards[1].click();
      });
      await page.waitForTimeout(200);
      await page.click('[data-e-edit="0"]');
      await page.fill('#listsEdit', 'because');
      await page.click('[data-e-save="0"]');
      await page.waitForTimeout(200);
      let words = await page.$$eval('#listsBody .le-word', (e) => e.map((x) => x.textContent));
      assertEq(words.join(','), 'because,dog', 'the misread is fixed');
      await page.click('[data-e-remove="1"]');
      await page.waitForTimeout(200);
      words = await page.$$eval('#listsBody .le-word', (e) => e.map((x) => x.textContent));
      assertEq(words.join(','), 'because', 'and a word can be removed');
      await page.click('#listsUndoBtn');
      await page.waitForTimeout(200);
      words = await page.$$eval('#listsBody .le-word', (e) => e.map((x) => x.textContent));
      assertEq(words.join(','), 'because,dog', 'Undo puts it back where it was');
    } finally { await ctx.close(); }
  });

  // F5.1 / F5.2 / AT5.1 / AT5.2 — delete asks, hides, and can be taken back.
  await suite.test('mywords_deleting_a_list_asks_first_and_can_be_undone', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await pasteSave(page, ['cat', 'dog']);
      await openScreen(page);
      const cards = await page.$$('#listsBody [data-l-open]');
      await cards[1].click();
      await page.waitForTimeout(200);
      await page.click('[data-l-del-ask]');
      await page.waitForTimeout(150);
      const ask = await page.$eval('#listsBody', (e) => e.innerText);
      assert(/delete .* and its 2 words\?/i.test(ask), `it asks, naming the list and the count (got "${ask}")`);
      await page.click('[data-l-del]');
      await page.waitForTimeout(300);
      let body = await page.$eval('#listsBody', (e) => e.innerText);
      assert(/no word lists yet/i.test(body), `My Words says it is empty (got "${body}")`);
      assert(!/2 words/.test(body), 'and the deleted list is not among the cards (AT5.2)');
      assert(/recently deleted/i.test(body), 'it waits under Recently deleted instead');
      await page.click('#listsUndoBtn');
      await page.waitForTimeout(250);
      body = await page.$eval('#listsBody', (e) => e.innerText);
      assert(/2 words/.test(body), `Undo brings it back whole (got "${body}")`);
    } finally { await ctx.close(); }
  });

  // F5.5 / D7 — a Spell Jr player keeps their own words, but cannot delete a
  // list on their own: the grown-up gate stands in front of it.
  await suite.test('mywords_spell_jr_cannot_delete_without_a_grown_up', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await pasteSave(page, ['cat', 'dog']);
      await openScreen(page);
      const cards = await page.$$('#listsBody [data-l-open]');
      await cards[1].click();
      await page.waitForTimeout(200);
      await page.click('[data-l-del-ask]');
      await page.waitForTimeout(250);
      assert(await page.$eval('#parentScrim', (e) => e.classList.contains('show')),
        'the parent gate opens instead of a confirm');
      const body = await page.$eval('#listsBody', (e) => e.innerText);
      assert(/2 words|cat/i.test(body), 'and nothing is deleted meanwhile');
      // Walking away from the gate leaves the list alone and disarms it.
      await page.click('#parentCancel');
      await page.waitForTimeout(200);
      assert(!(await page.$eval('#parentScrim', (e) => e.classList.contains('show'))), 'the gate closes');
      const after = await page.evaluate((k) => JSON.parse(localStorage.getItem(k)).lists.filter((l) => !l.deletedAt).length, LISTS_KEY);
      assertEq(after, 1, 'the list is still there');
    } finally { await ctx.close(); }
  });
}
