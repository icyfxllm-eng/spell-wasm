// translate-screen.spec — CC-TRANSLATE-SCREEN Phase A (app build only, D5).
//
// The screen can only answer for audited glosses, and today no gloss file is
// audited. So each fixture first marks a language's REAL rows as audited
// through a test-build-only hook (__spelltest.translateAudit). The hook cannot
// author a row, so everything these tests render is still a real bank word.
//
// Done-criteria map (signed: "map tests to this repo"):
//   #2 screen state matrix, #3 layout stability (I1), #8 script primacy (I4),
//   #9 declines (I6), #11 RTL + largest text  ->  this file
//   #1 / #7 unit + traceability             ->  cargo test --lib translate_
//   #4 closed space, #5 speech API, #6 no per-language branch -> scripts/gate.sh
import { openApp, assert } from '../harness.mjs';

const actionsY = (page) => page.evaluate(() =>
  Math.round(document.getElementById('trActions').getBoundingClientRect().top));

async function openTranslate(page, audit = []) {
  await page.evaluate((langs) => {
    for (const l of langs) window.__spelltest.translateAudit(l, true);
  }, audit);
  await page.evaluate(() => document.getElementById('trOpenBtn').click());
  await page.waitForSelector('#trScreen.show', { timeout: 5000 });
}

async function typeSource(page, text) {
  await page.fill('#trSrcInput', text);
  await page.waitForTimeout(80);
}

async function commitFirst(page, query) {
  await typeSource(page, query);
  await page.waitForSelector('#trSuggest [data-tr-sugg]', { timeout: 4000 });
  await page.click('#trSuggest [data-tr-sugg]');
  await page.waitForFunction(() => document.getElementById('trTgtWord').textContent.length > 0, null, { timeout: 4000 });
}

export async function run(browser, base, suite) {
  // translate-empty, translate-single-sense; I1 across fill, swap and clear; F7; F8.
  await suite.test('translate_state_matrix_holds_the_action_row_still', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['es']);
      const y0 = await actionsY(page);
      assert(await page.$eval('#trTgtWord', (e) => e.textContent === ''), 'opens empty');

      await typeSource(page, 'a');
      const n = await page.$$eval('#trSuggest [data-tr-sugg]', (b) => b.length);
      assert(n > 0 && n <= 8, `suggestions from the first letter, at most eight (got ${n})`);
      assert((await actionsY(page)) === y0, 'suggestions overlay the spine; the action row stays (I1)');

      await page.click('#trSuggest [data-tr-sugg]');
      await page.waitForFunction(() => document.getElementById('trTgtWord').textContent.length > 0, null, { timeout: 4000 });
      assert((await actionsY(page)) === y0, 'filling does not move the action row (I1)');
      assert((await page.$eval('#trSrcSub', (e) => e.textContent)).length > 0, 'the chosen sense stays visible (F3)');

      const before = await page.evaluate(() => [trSrcLang.textContent, trTgtLang.textContent, trTgtWord.textContent]);
      await page.click('#trSwap');
      const after = await page.evaluate(() => [trSrcLang.textContent, trTgtLang.textContent, trSrcInput.value]);
      assert(after[0] === before[1] && after[1] === before[0], `swap exchanges the languages (F7): ${JSON.stringify([before, after])}`);
      assert(after[2] === before[2], 'and the words: the old answer is the new question');
      assert((await actionsY(page)) === y0, 'swapping does not move the action row (I1)');

      await page.click('#trClear');
      const cleared = await page.evaluate(() => [trSrcLang.textContent, trTgtLang.textContent, trSrcInput.value, trTgtWord.textContent]);
      assert(cleared[0] === after[0] && cleared[1] === after[1], 'clear keeps both languages (F8)');
      assert(cleared[2] === '' && cleared[3] === '', 'and empties the question');
      assert((await actionsY(page)) === y0, 'clearing does not move the action row (I1)');
    } finally { await ctx.close(); }
  });

  // I2 / F12 #1 / I6 — a miss is a pre-commit state with a way forward.
  await suite.test('translate_no_committable_miss', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['es']);
      await typeSource(page, 'zzqxjv');
      const state = await page.$eval('#trSuggest', (e) => ({
        shown: !e.hidden, decline: !!e.querySelector('.tr-decline'), buttons: e.querySelectorAll('button').length,
      }));
      assert(state.shown && state.decline, 'the bound is announced in the suggestion area, before any commit');
      assert(state.buttons >= 1, 'with a live way forward (I6)');
      await page.press('#trSrcInput', 'Enter');
      await page.waitForTimeout(150);
      assert(await page.$eval('#trTgtWord', (e) => e.textContent === ''), 'Enter commits nothing (I2)');
    } finally { await ctx.close(); }
  });

  // translate-no-target-answer / F12 #2 / I6.
  await suite.test('translate_no_target_answer_declines_with_a_way_out', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['es', 'ja']);
      const gap = await page.evaluate(() => window.__spelltest.translateGap('en', 'es', 'ja'));
      assert(gap, 'the real banks carry at least one concept Spanish has and Japanese lacks');
      await commitFirst(page, gap);
      await page.click('#trTgtLang');
      await page.click('#trPicker [data-tr-lang="ja"]');
      const d = await page.$eval('#trTgtCard', (e) => ({
        word: e.querySelector('#trTgtWord').textContent,
        text: e.querySelector('#trTgtSub').textContent.trim(),
        way: !!e.querySelector('[data-tr-pick]'),
      }));
      assert(d.word === '' && d.text.length > 0, `the target card says there is no answer: ${JSON.stringify(d)}`);
      assert(d.way, 'and offers a live way forward (I6)');
    } finally { await ctx.close(); }
  });

  // translate-single-sense in a non-Latin script: I4 and D1.
  await suite.test('translate_script_primacy_and_d1_default', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['zh']);
      await commitFirst(page, 'a');
      const m = await page.evaluate(() => {
        const w = document.getElementById('trTgtWord');
        const r = document.getElementById('trTgtTranslit');
        return { word: parseFloat(getComputedStyle(w).fontSize), translit: parseFloat(getComputedStyle(r).fontSize),
                 wordText: w.textContent, translitText: r.textContent };
      });
      assert(m.wordText.length > 0, 'the native script renders');
      assert(m.translitText.length > 0, 'D1: transliteration is on by default for a non-Latin target under a Latin-script UI');
      assert(m.word > m.translit, `I4: native script (${m.word}px) outsizes transliteration (${m.translit}px)`);
    } finally { await ctx.close(); }
  });

  // translate-rtl-ar and the largest text size.
  await suite.test('translate_rtl_mirrors_the_card_not_the_spine', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['ar']);
      await commitFirst(page, 'a');
      assert(await page.$eval('#trTgtCard', (e) => e.getAttribute('dir') === 'rtl'), 'an RTL target mirrors its own card');
      assert(await page.$eval('#trSpine', (e) => getComputedStyle(e).direction === 'ltr'), 'the spine does not mirror');
      await page.evaluate(() => document.body.classList.add('big-text'));
      await page.waitForTimeout(80);
      const yFull = await actionsY(page);
      await page.click('#trClear');
      assert((await actionsY(page)) === yFull, 'at the largest text size the action row still holds still (I1)');
    } finally { await ctx.close(); }
  });

  // F9 / D10 — save the target word, stay on the screen.
  await suite.test('translate_save_keeps_the_target_word', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['es']);
      await commitFirst(page, 'a');
      const target = await page.$eval('#trTgtWord', (e) => e.textContent);
      await page.click('#trSave');
      await page.waitForTimeout(150);
      assert((await page.$eval('#trNote', (e) => e.textContent)).length > 0, 'an inline confirmation');
      assert(await page.$eval('#trScreen', (e) => e.classList.contains('show')), 'no navigation away');
      const saved = await page.evaluate(() => JSON.parse(localStorage.getItem('byear_custom_v1') || '{}'));
      assert((saved.words || []).includes(target), `the TARGET word is saved (D10): ${JSON.stringify(saved.words)}`);
      assert(saved.wordLang && saved.wordLang[target], 'tagged with its own speak-in language');
    } finally { await ctx.close(); }
  });

  // F10 / D3 (translate-unentitled-target) — served in its language, or an honest decline.
  await suite.test('translate_spell_it_serves_or_declines_honestly', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openTranslate(page, ['es']);
      await commitFirst(page, 'a');
      const target = await page.$eval('#trTgtWord', (e) => e.textContent);
      await page.click('#trSpell');
      await page.waitForTimeout(300);
      const closed = await page.$eval('#trScreen', (e) => !e.classList.contains('show'));
      if (closed) {
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        assert(word === target, `Spell it serves the translation itself (got ${word}, expected ${target})`);
      } else {
        assert((await page.$eval('#trNote', (e) => e.textContent)).length > 0,
          'a word the player may not be served gets an inline decline, never a silent drop');
      }
    } finally { await ctx.close(); }
  });

  // D4 — Spell Jr gets Translate.
  await suite.test('translate_spell_jr_gets_the_screen', async () => {
    const kidVerdict = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: kidVerdict });
    try {
      await openTranslate(page, ['es']);
      await typeSource(page, 'a');
      const panel = await page.$eval('#trSuggest', (e) => !e.hidden);
      assert(panel, 'a Spell Jr player can search');
    } finally { await ctx.close(); }
  });
}
