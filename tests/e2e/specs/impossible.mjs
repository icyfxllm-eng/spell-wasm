// impossible.spec -- CC-IMPOSSIBLE-COMPLETE Phase C at the UI boundary (flags are real localStorage, set
// through addInitScript before boot, like attempts_shields). English only: the free tier owns no other
// language, which is also the case the Mix lock copy is about.
//
//   * every flag OFF (the default)  -> no Impossible entry in the level picker.
//   * flags ON, free English        -> "Impossible" is playable; "Impossible Mix" is shown LOCKED with the
//                                      Complete reason (F3), never "needs two languages".
//   * an Impossible word            -> hint / definition / can't-hear buttons hidden; exactly one replay;
//                                      one wrong submission is a miss with NO retry even with extra attempts
//                                      on; the miss is filed under the impossible tag (M9).
import { typeOnKeyboard, assert, pinBaseline, clickSettled, openSettings } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const ON = { spell_flag_impossible: 'on', spell_flag_impossible_en: 'on', spell_flag_impossible_mix: 'on', spell_flag_en_retier: 'on' };

async function open(browser, base, flags, extra = {}) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
  await pinBaseline(ctx);
  await ctx.addInitScript(([age, f, x]) => {
    localStorage.setItem('byear_agegate_v1', age);
    for (const [k, v] of Object.entries({ ...f, ...x })) localStorage.setItem(k, v);
  }, [AGE, flags, extra]);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  return { ctx, page };
}

const options = (page) => page.$$eval('#levelSel option', (os) => os.map((o) => ({ v: o.value, t: o.textContent, d: o.disabled })));

async function startImpossible(page) {
  await page.click('#setupChip').catch(() => {});
  await page.selectOption('#levelSel', 'impossible');
  await clickSettled(page, '#setupDone').catch(() => {});
  // Picking the level serves the first word. The orb is a REPLAY once a word is live (and so spends
  // the one replay), so only tap it when nothing has been served.
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 3000 }).catch(async () => {
    await page.click('#orbWrap');
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  });
}

// A kill document signed with the REAL key by tools/killflags/sign.py (kills impossible_fr + impossible_mix).
const KILL_DOC = JSON.stringify({ v: 1, issued: '2026-10-19T12:00:00Z', kill: ['impossible_fr', 'impossible_mix'],
  sig: 'bd2e970d4e27620794019a061fe849b2c3a9f7704f98811d4a6f3af78aa23a823657dd22db41ad6ac079cd4ff1bb3162fbf6fb739a77e09b3d59f9408199180e' });

async function answerRight(page, word) {
  // A capital is typed with the on-screen Shift key (German Impossible grades case).
  const first = word[0];
  if (first !== first.toLowerCase()) {
    await page.waitForSelector('#kbShift:not(.btn-hide)', { timeout: 3000 });
    await page.click('#kbShift');
    await page.click(`#gameKeyboard .kb-key[data-k="${first.toLowerCase()}"]`);
    await typeOnKeyboard(page, word.slice(1));
  } else {
    await typeOnKeyboard(page, word);
  }
  await page.click('#checkBtn');
  await page.waitForFunction(() => {
    const c = document.getElementById('feedback')?.className || '';
    return c.includes('good') || c.includes('bad');
  }, null, { timeout: 15000 });
  return page.$eval('#feedback', (e) => e.className.includes('good'));
}

export async function run(browser, base, suite) {
  await suite.test('F7: the Daily button on the Impossible level is the Impossible Daily, with its own record', async () => {
    const { ctx, page } = await open(browser, base, ON);
    try {
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#levelSel', 'impossible');
      await clickSettled(page, '#setupDone').catch(() => {});
      await page.evaluate(() => document.getElementById('dailyBtn').click()); // the hub tile sits under an overlay
      await page.click('#orbWrap');
      const seen = [];
      for (let i = 0; i < 10; i++) {
        await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        seen.push(word);
        assert(await page.$eval('#hintBtn', (e) => e.classList.contains('btn-hide')), 'no hints in the Impossible Daily');
        assert(await answerRight(page, word), `"${word}" typed exactly must be correct`);
        await page.click('#orbWrap'); // next word, or the result
      }
      assert(new Set(seen).size === 10, `ten different words: ${seen}`);
      const imp = await page.evaluate(() => JSON.parse(localStorage.getItem('spell_daily_imp_v1_en') || '{}'));
      assert(Object.values(imp.history || {}).includes(10), `the Impossible record holds today's 10/10: ${JSON.stringify(imp)}`);
      const plain = await page.evaluate(() => localStorage.getItem('spell_daily_v1'));
      assert(!plain || !Object.keys(JSON.parse(plain).history || {}).length, 'the ordinary Daily must stay unplayed');
    } finally { await ctx.close(); }
  });

  await suite.test('German Impossible: a capitalised noun is typed with the Shift key and counts', async () => {
    const { ctx, page } = await open(browser, base, { ...ON, spell_flag_impossible_de: 'on', spell_dev_entitlements: 'on' });
    try {
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#langSel', 'de');
      await page.selectOption('#levelSel', 'impossible');
      await clickSettled(page, '#setupDone').catch(() => {});
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      let capital = false;
      for (let i = 0; i < 25 && !capital; i++) {
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        capital = word[0] !== word[0].toLowerCase();
        assert(await answerRight(page, word), `"${word}" typed exactly (with Shift for a capital) must be correct`);
        if (!capital) { await page.click('#orbWrap'); await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 }); }
      }
      assert(capital, 'no capitalised German word came up in 25 words');
    } finally { await ctx.close(); }
  });

  await suite.test('Impossible Mix: a German word brings the German keyboard and its Shift', async () => {
    // Eric 2026-10-24: playing Mix from English, a German word showed the English keyboard -- no
    // Shift for its capital and no umlauts -- so a right spelling was unreachable.
    const { ctx, page } = await open(browser, base, { ...ON, spell_flag_impossible_de: 'on', spell_dev_entitlements: 'on' });
    page.on('dialog', (d) => d.accept()); // the Mix pre-run notice (M3)
    try {
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#langSel', 'en');
      await page.selectOption('#levelSel', 'impossible_mix');
      await clickSettled(page, '#setupDone').catch(() => {});
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      let german = false;
      for (let i = 0; i < 30 && !german; i++) {
        const lang = await page.evaluate(() => window.__spelltest.currentLang());
        const word = await page.evaluate(() => window.__spelltest.currentWord());
        german = lang === 'de';
        if (german) {
          assert(await page.$('#kbShift:not(.btn-hide)'), 'a German Mix word shows the Shift key');
          assert(await page.$('#gameKeyboard .kb-key[data-k="\u00fc"]'), 'a German Mix word shows the German keyboard (\u00fc)');
          assert(await answerRight(page, word), `"${word}" typed exactly must be correct in Mix`);
        } else {
          assert(await page.$('#kbShift.btn-hide') || !(await page.$('#kbShift')), `no Shift for a ${lang} word`);
        }
        if (!german) {
          // Only en and de are flagged on, so every other word is English.
          await answerRight(page, word);
          await page.click('#orbWrap');
          await page.waitForFunction((w) => !!window.__spelltest.currentWord() && window.__spelltest.currentWord() !== w, word, { timeout: 5000 });
        }
      }
      assert(german, 'no German word came up in 30 Mix words');
    } finally { await ctx.close(); }
  });

  await suite.test('German outside Impossible: the Shift key is there for every word', async () => {
    const { ctx, page } = await open(browser, base, { spell_dev_entitlements: 'on' });
    try {
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#langSel', 'de');
      await clickSettled(page, '#setupDone').catch(() => {});
      await page.click('#orbWrap').catch(() => {});
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      assert(await page.$('#kbShift:not(.btn-hide)'), 'Shift shows for an ordinary German word');
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      await page.click('#kbShift');
      await page.click(`#gameKeyboard .kb-key[data-k="${word[0].toLowerCase()}"]`);
      await typeOnKeyboard(page, word.slice(1));
      await page.click('#checkBtn');
      await page.waitForFunction(() => /good|bad/.test(document.getElementById('feedback')?.className || ''), null, { timeout: 15000 });
      assert(await page.$eval('#feedback', (e) => e.className.includes('good')), `"${word}" with a capital first letter is still correct (case folds outside Impossible)`);
    } finally { await ctx.close(); }
  });

  await suite.test('Mix: the keyboard follows each word into its own language', async () => {
    const { ctx, page } = await open(browser, base,
      { ...ON, spell_flag_impossible_es: 'on', spell_flag_impossible_ru: 'on', spell_dev_entitlements: 'on' });
    page.on('dialog', (d) => d.accept()); // the one-time Mix notice
    try {
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#levelSel', 'impossible_mix');
      await clickSettled(page, '#setupDone').catch(() => {});
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      const seen = new Set();
      for (let i = 0; i < 20 && seen.size < 2; i++) {
        const [word, lang] = await page.evaluate(() => [window.__spelltest.currentWord(), window.__spelltest.currentLang()]);
        seen.add(lang);
        const keys = new Set(await page.$$eval('#kbLetters [data-k]', (ks) => ks.map((k) => k.getAttribute('data-k'))));
        const base = (c) => c.normalize('NFD')[0]; // an accented letter lives on its base key's long-press
        const missing = [...word.toLowerCase()].filter((c) => !keys.has(c) && !keys.has(base(c)));
        assert(missing.length === 0, `${lang} word "${word}" is not typeable on the keyboard shown (missing ${missing.join('')})`);
        await page.click('#giveupBtn');
        await page.click('#orbWrap');
        await page.waitForFunction((w) => window.__spelltest.currentWord() && window.__spelltest.currentWord() !== w, word, { timeout: 5000 });
      }
      assert(seen.size === 2, `Mix served only ${[...seen]} in 20 words`);
    } finally { await ctx.close(); }
  });

  await suite.test('F4: a signed kill file switches Mix off after it is cached; a tampered one does nothing', async () => {
    for (const [doc, killed] of [[KILL_DOC, true], [KILL_DOC.replace('impossible_mix', 'impossible_xx'), false]]) {
      const { ctx, page } = await open(browser, base, ON);
      try {
        await ctx.route('**/api/kill-flags', (r) => r.fulfill({ status: 200, contentType: 'application/json', body: doc }));
        const answered = page.waitForResponse('**/api/kill-flags');
        await page.reload({ waitUntil: 'load' });
        await answered; // the silent refresh has been answered; adoption follows in the same tasks
        await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
        if (killed) await page.waitForFunction(() => !!localStorage.getItem('spell_kill_flags_v1'), null, { timeout: 5000 });
        else await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
        const cached = await page.evaluate(() => localStorage.getItem('spell_kill_flags_v1'));
        assert(killed ? !!cached : !cached, `cache after refresh: killed=${killed} cached=${!!cached}`);
        await page.reload({ waitUntil: 'load' });
        await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
        const o = await options(page);
        const mix = o.find((x) => x.v === 'impossible_mix');
        assert(killed ? !mix : !!mix, `Mix entry with killed=${killed}: ${JSON.stringify(o)}`);
        assert(o.some((x) => x.v === 'impossible'), 'the unnamed Impossible flag keeps the build value');
      } finally { await ctx.close(); }
    }
  });

  await suite.test('flags OFF: no Impossible entry in the level picker', async () => {
    const { ctx, page } = await open(browser, base, {});
    try {
      const o = await options(page);
      assert(!o.some((x) => x.v.startsWith('impossible')), `picker offers Impossible with every flag off: ${JSON.stringify(o)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('flags ON, free English: Impossible playable, Mix locked with the Complete reason', async () => {
    const { ctx, page } = await open(browser, base, ON);
    try {
      const o = await options(page);
      const imp = o.find((x) => x.v === 'impossible');
      const mix = o.find((x) => x.v === 'impossible_mix');
      assert(imp && !imp.d, `Impossible must be selectable: ${JSON.stringify(o)}`);
      assert(mix && mix.d, `Mix must be shown disabled: ${JSON.stringify(o)}`);
      assert(/complete/i.test(mix.t) && !/two/i.test(mix.t), `Mix lock must say Complete, got ${mix.t}`);
    } finally { await ctx.close(); }
  });

  await suite.test('an Impossible word: no hints, one replay, one attempt, miss filed under the tag', async () => {
    const { ctx, page } = await open(browser, base, ON);
    try {
      await openSettings(page).catch(() => {});
      await page.click('#extraAttemptsToggle'); // the toggle is ON: Impossible must still give no retry
      await page.click('#setDone').catch(() => {});
      await startImpossible(page);
      for (const id of ['#hintBtn', '#defBtn', '#sentenceBtn', '#cantHearBtn']) {
        const hid = await page.$eval(id, (e) => e.classList.contains('btn-hide') || e.disabled).catch(() => true);
        assert(hid, `${id} must be unavailable at Impossible`);
      }
      assert(!(await page.$eval('#replayBtn', (e) => e.disabled)), 'the first replay must be allowed');
      await page.click('#replayBtn');
      assert(await page.$eval('#replayBtn', (e) => e.disabled), 'the replay button must lock after the one replay');
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      const wrong = word === 'zzzz' ? 'xxxx' : 'zzzz';
      await typeOnKeyboard(page, wrong);
      await page.click('#checkBtn');
      await page.waitForFunction(() => {
        const c = document.getElementById('feedback')?.className || '';
        return c.includes('good') || c.includes('bad');
      }, null, { timeout: 15000 });
      const bad = await page.$eval('#feedback', (e) => e.className.includes('bad'));
      assert(bad, 'a wrong answer is a miss');
      assert(await page.$eval('#checkBtn', (e) => e.disabled), 'no retry: the answer stays locked after one attempt');
      const misses = await page.evaluate(() => JSON.parse(localStorage.getItem('byear_misses_v1') || '[]'));
      assert(misses.some((m) => m.word === word && m.tier === 'impossible'), `miss not filed under the impossible tag: ${JSON.stringify(misses)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('an Impossible word typed right is correct (strict grader, no server)', async () => {
    const { ctx, page } = await open(browser, base, ON);
    try {
      await startImpossible(page);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      await typeOnKeyboard(page, word);
      await page.click('#checkBtn');
      await page.waitForFunction(() => {
        const c = document.getElementById('feedback')?.className || '';
        return c.includes('good') || c.includes('bad');
      }, null, { timeout: 15000 });
      assert(await page.$eval('#feedback', (e) => e.className.includes('good')), `"${word}" typed exactly must be correct`);
    } finally { await ctx.close(); }
  });
}
