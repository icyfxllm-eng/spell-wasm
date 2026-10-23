// spell-keyboard.spec — CC-SPELL-KEYBOARD Phase A, with keyboard.core ON.
//
// The rest of the suite proves the shared keyboard changes nothing a test can
// see (run it with SPELL_FLAGS=keyboard_core=on). This spec checks what the
// shared keyboard ADDS:
//   * every key on every surface is drawn by it and has a VoiceOver label (KF10);
//   * the five Vietnamese tone keys are told apart by name;
//   * labels are lowercase until Shift is armed (K-D19);
//   * Bee, which had no long-press, now offers a language's alternates;
//   * BoardGame offers German its Impossible trap word, with a Shift to type it;
//   * O1 "Use device keyboard": the device's keyboard types a right answer, Spell's
//     keyboard steps away, and a whole word inserted at once is still refused.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const ON = () => { localStorage.setItem('spell_flag_keyboard_core', 'on'); };
const openMode = (page, id) => page.evaluate((id) => document.getElementById(id).click(), id);

/** Every key in `host` is a shared key with a non-empty label. */
async function labelled(page, host) {
  return page.evaluate((h) => {
    const keys = [...document.querySelectorAll(`#${h} .skb-key`)];
    return { n: keys.length, unlabelled: keys.filter((k) => !(k.getAttribute('aria-label') || '').trim()).map((k) => k.outerHTML.slice(0, 80)) };
  }, host);
}

export async function run(browser, base, suite) {
  await suite.test('spell-keyboard: the main keyboard is the shared one, labelled, lowercase', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kb-core')), 'body carries kb-core');
      const r = await labelled(page, 'kbLetters');
      assert(r.n >= 26, `the letters are shared keys (${r.n})`);
      assertEq(r.unlabelled.length, 0, `unlabelled keys: ${r.unlabelled.join(' | ')}`);
      const lower = async (when) => {
        const faces = await page.$$eval('#kbLetters .skb-key[data-k]', (ks) => ks.map((k) => getComputedStyle(k).textTransform + ':' + k.textContent));
        assert(faces.every((f) => !/[A-Z]/.test(f.split(':')[1]) && !f.startsWith('uppercase')), `${when}: labels show the case a key types (${faces.slice(0, 4)})`);
      };
      await lower('before a word');
      // Serving a word hides the (German-only) Shift; hiding it must not read as arming it.
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      await domSettled(page);
      assert(!(await page.$eval('#gameKeyboard', (k) => k.classList.contains('locked'))), 'the keys are live');
      await lower('with a word live');
    } finally { await ctx.close(); }
  });

  await suite.test('spell-keyboard: the five Vietnamese tone keys are named apart', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'vi', init: ON });
    try {
      const names = await page.$$eval('#kbToneRow .skb-key', (ks) => ks.map((k) => k.getAttribute('aria-label')));
      assertEq(names.length, 5, 'five tone keys');
      assertEq(new Set(names).size, 5, `each tone has its own name (${names})`);
      assert(!names.includes('tone mark'), 'no generic "tone mark" label left');
    } finally { await ctx.close(); }
  });

  await suite.test('spell-keyboard: Bee draws shared keys and offers alternates', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es', init: ON });
    try {
      await openMode(page, 'beeOpen');
      await page.waitForSelector('#bee.show', { timeout: 5000 });
      const r = await labelled(page, 'beeKeys');
      assert(r.n > 0, 'Bee keys are shared keys');
      assertEq(r.unlabelled.length, 0, `unlabelled Bee keys: ${r.unlabelled.join(' | ')}`);
      const acc = await page.$eval('#beeKeys .skb-key[data-k="a"]', (k) => k.dataset.acc || '');
      assert(acc.includes('á'), `á is a long-press on a in Bee (got "${acc}")`);
    } finally { await ctx.close(); }
  });

  // K-D18 / L-D4: German Impossible grades capitals, so BoardGame kept German out of
  // its Impossible trap for want of a Shift. On the shared keyboard it has one.
  await suite.test('spell-keyboard: a German Impossible trap word in BoardGame is typed with Shift', async () => {
    const FLAGS = () => {
      for (const [k, v] of Object.entries({
        spell_flag_keyboard_core: 'on', spell_flag_boardgame: 'on', spell_flag_boardStretch: 'off',
        spell_flag_impossible: 'on', spell_flag_impossible_de: 'on', spell_dev_entitlements: 'on',
      })) localStorage.setItem(k, v);
    };
    const { ctx, page } = await openApp(browser, base, { lang: 'de', viewport: { width: 393, height: 852 }, init: FLAGS });
    const state = () => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
    try {
      await page.evaluate(() => document.getElementById('bgOpenBtn').click());
      await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
      await page.click('[data-bg="mode:pass"]');
      await page.click('[data-bg="count:2"]');
      await page.click('#bgStart');
      await page.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
      await page.waitForSelector('#bgHand:not([hidden])', { timeout: 8000 });
      await page.click('#bgHandGo');
      await domSettled(page);
      assert(await page.$('#bgShift'), 'the German grid carries a Shift key');
      // Roll, stage a Double Expert trap on the landing tile, and spell the word right.
      await page.click('#bgOrb');
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.phase === 'AwaitSpelling', null, { timeout: 5000 });
      assert(await page.evaluate(() => window.__spelltest.boardgameForce('trap:double')), 'a Double Expert trap staged');
      assert(await page.$('#bgShift.btn-hide'), 'Shift stays hidden for an ordinary word');
      const first = (await state()).word.split('|')[0].toLowerCase();
      for (const ch of first) await page.click(`#bgKeys [data-k="u:${ch}"]`);
      const before = await page.evaluate(() => window.__spelltest.boardgameState());
      await page.click('#bgGo');
      await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 5000 });
      // The trap's word is the language's single Impossible word.
      await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState() || 'null')?.kind === 'Impossible', null, { timeout: 8000 });
      await domSettled(page);
      assert(!(await page.$('#bgShift.btn-hide')), 'Shift shows for the Impossible word');
      const word = (await state()).word.split('|')[0];
      for (const ch of word) {
        if (ch !== ch.toLowerCase()) await page.click('#bgShift');
        await page.click(`#bgKeys [data-k="u:${ch.toLowerCase()}"]`);
      }
      assertEq(await page.$eval('#bgField', (e) => e.textContent), word, 'the field holds the word, capitals and all');
    } finally { await ctx.close(); }
  });

  const deviceOn = async (page) => {
    await page.evaluate(() => {
      const el = document.getElementById('deviceKbToggle');
      el.checked = true;
      el.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await domSettled(page);
  };
  const serve = async (page) => {
    await page.click('#orbWrap');
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
    await domSettled(page);
    return page.evaluate(() => window.__spelltest.currentWord());
  };
  const verdict = (page) => page.$eval('#feedback', (e) => e.className);

  await suite.test('spell-keyboard: O1 the device keyboard types a right answer', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      assert(!(await page.$('#deviceKbRow.btn-hide')), 'the setting is offered with keyboard.core');
      await deviceOn(page);
      assert(await page.evaluate(() => document.body.classList.contains('device-kb')), 'body carries device-kb');
      assert(await page.$eval('#kbHome', (e) => getComputedStyle(e).display === 'none'), "Spell's keyboard steps away");
      const word = await serve(page);
      await page.focus('#answerDevice');
      await page.keyboard.type(word.toLowerCase(), { delay: 15 });
      assertEq(await page.$eval('#letters', (e) => e.textContent.replace(/\s/g, '')), word.toLowerCase(), 'the letters show what was typed');
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => /good|bad/.test(document.getElementById('feedback').className), null, { timeout: 5000 });
      assert(/good/.test(await verdict(page)), `typed on the device keyboard, graded right (${await verdict(page)})`);
    } finally { await ctx.close(); }
  });

  await suite.test('spell-keyboard: O1 a whole word inserted at once is still refused', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      await deviceOn(page);
      const word = await serve(page);
      await page.focus('#answerDevice');
      // One insertion of the whole word: what dictation or paste looks like.
      await page.keyboard.insertText(word.toLowerCase());
      assertEq(await page.$eval('#letters', (e) => e.textContent.replace(/\s/g, '')), word.toLowerCase(), 'the word did reach the answer');
      await page.keyboard.press('Enter');
      await domSettled(page);
      assert(!/good/.test(await verdict(page)), `a word that arrived in one piece is not a spelling (${await verdict(page)})`);
    } finally { await ctx.close(); }
  });
}
