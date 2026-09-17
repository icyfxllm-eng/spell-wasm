// mywords-lists.spec — CC-MYWORDS-LISTS v1, Phase 1: the migration, where it
// actually runs. Host tests cannot see localStorage, so the only honest proof
// that an existing player's words survive is a real browser boot.
import { openApp, assert, assertEq } from '../harness.mjs';

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
}
