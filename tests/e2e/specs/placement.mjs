// placement.spec — CC-LEARNING-ENGINE L1 feature 5, the kid-facing flow
// (the QA pass that earns the learner_surfaces flag its ON default).
//
// The offer appears ONCE per language at the top of a solo serve, is
// skippable (skip leaves language-default priors standing and never
// re-offers), and Try serves the deterministic placement set through
// the REAL session path — no placement-specific scoring exists.
import { openApp, assert, typeOnKeyboard } from '../harness.mjs';

async function surfacesOn(page) {
  await page.evaluate(() => {
    localStorage.setItem('spell_flag_learner_surfaces', 'on');
    localStorage.setItem('spell_flag_learner_select', 'on');
  });
  await page.reload();
  await page.waitForTimeout(600);
}

export async function run(browser, base, suite) {
  await suite.test('placement: offered once, skip stands, no re-offer', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcSkip');
      await page.waitForTimeout(300);
      assert(!(await page.$('#plcCard.show')), 'card dismissed on skip');
      // a second serve never re-offers — skipped IS an answer
      await page.click('#orbWrap');
      await page.waitForTimeout(600);
      assert(!(await page.$('#plcCard.show')), 'no re-offer after skip');
    } finally { await ctx.close(); }
  });

  await suite.test('placement: Try serves the set through the real session', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcTry');
      await page.waitForTimeout(500);
      // D6: 10-14 words, served by the normal engine (the visible word
      // is a real serve; spelling it advances normally).
      const first = await page.evaluate(() => window.__spelltest.currentWord());
      assert(first && first.length > 0, 'a placement word is live in the real session');
    } finally { await ctx.close(); }
  });

  // CC-PLACEMENT-IDEMPOTENCE F4 — the offer may not interrupt a live run.
  //
  // Eric's screenshot: the sheet landing on top of a twenty-word chain. The
  // predicate going true mid-run must SUPPRESS the offer, not fire it — and
  // must not lose it either, so it appears at the next boundary.
  await suite.test('placement: the offer never interrupts a live run', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Answer the probe away first, so the offer is settled, then clear the
      // record to make the predicate true again WHILE a chain is running.
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcSkip');
      await page.waitForTimeout(300);

      // Build a real chain: answer words correctly until the streak is up.
      let streak = 0;
      for (let i = 0; i < 6 && streak < 3; i++) {
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        if (!w || !/^[a-z]+$/i.test(w)) { await page.click('#orbWrap'); await page.waitForTimeout(350); continue; }
        for (const ch of w.toLowerCase()) {
          const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
          if (k) await k.click();
        }
        await page.click('#checkBtn');
        await page.waitForFunction((prev) => window.__spelltest.currentWord() !== prev, w, { timeout: 6000 }).catch(() => {});
        streak = await page.evaluate(() => window.__spelltest.streak());
      }
      assert(streak > 0, `needed a live run to test against, streak=${streak}`);

      // Predicate true again, mid-run.
      await page.evaluate(() => localStorage.removeItem('spell_learner_en'));
      const w2 = await page.evaluate(() => window.__spelltest.currentWord());
      for (const ch of (w2 || 'a').toLowerCase()) {
        const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
        if (k) await k.click();
      }
      await page.click('#checkBtn');
      await page.waitForTimeout(2800);
      assert(!(await page.$('#plcCard.show')),
        'the offer rendered OVER a live run — F4 violated');
    } finally { await ctx.close(); }
  });

  // CC-PLACEMENT-IDEMPOTENCE — "it asked me again, with the same words."
  //
  // The two tests above only prove SKIP does not re-offer, and only within one
  // session. Nothing covered a run that is COMPLETED, and nothing covered
  // survival across a reload -- which is the exact gap the report lives in.
  //
  // This has to run in a browser. The equivalent unit test is meaningless:
  // storage::set_raw writes to web_sys localStorage, which does not exist in a
  // host cargo test, so it is a silent no-op and every assertion passes or
  // fails for the wrong reason.
  await suite.test('placement: a COMPLETED probe never re-offers, across a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await surfacesOn(page);
      await page.click('#orbWrap');
      await page.waitForSelector('#plcCard.show', { timeout: 5000 });
      await page.click('#plcTry');
      await page.waitForTimeout(400);

      // Answer the whole set. The set is 10-14 words (D6); the cap is a
      // guard against a runaway loop, not an expectation.
      let served = 0;
      for (let i = 0; i < 30; i++) {
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        if (!w) break;
        for (const ch of w.toLowerCase()) {
          const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
          if (k) await k.click();
        }
        await page.click('#checkBtn');
        // WAIT FOR THE AUTO-ADVANCE. Do not tap the orb: a correct answer
        // already schedules a serve (CORRECT_DELAY_MS), and an orb tap 500ms
        // later races it, draining two words from the placement queue for one
        // answered. That is the harness driving the app in a way no player
        // does, and it drained the queue before the last answer could close
        // the probe.
        const before = w;
        await page.waitForFunction(
          (prev) => window.__spelltest.currentWord() !== prev,
          before, { timeout: 6000 }).catch(() => {});
        served++;
        // The card must NEVER reappear mid-run (F4: no interruption).
        assert(!(await page.$('#plcCard.show')),
          `the probe offer re-rendered during the run, after ${served} words`);
        if (await page.evaluate(() => !!document.querySelector('#plcCard.show'))) break;
        // STOP WHEN THE PROBE CLOSES. The last placement answer writes
        // placed=true and the session carries on into ordinary play. Without
        // this break the loop played up to 30 words, the last ~20 of them
        // ordinary deck words this test does not police, and CI's run hit a
        // turn that left the keyboard locked (#kbHome took the click, 30s
        // timeout), failing a test whose subject had already passed.
        if (await page.evaluate(() =>
          JSON.parse(localStorage.getItem('spell_learner_en') || '{}').placed === true)) break;
      }
      assert(served >= 5, `expected the probe to serve its set, only saw ${served}`);

      // The whole point: a completed probe is a decision, and it survives.
      assert(!(await page.$('#plcCard.show')), 'no re-offer immediately after completing');
      // The record is the whole point: it must be DURABLE, not in-memory.
      const placed = await page.evaluate(() =>
        JSON.parse(localStorage.getItem('spell_learner_en') || '{}').placed);
      assert(placed === true,
        `a completed probe must write placed=true; localStorage says ${JSON.stringify(placed)}`);
      await page.reload();
      await page.waitForTimeout(900);
      await page.click('#orbWrap');
      await page.waitForTimeout(700);
      assert(!(await page.$('#plcCard.show')),
        'RE-OFFERED after a reload — a completed placement was not recorded durably');
    } finally { await ctx.close(); }
  });

  // C10 — a failed learner load never wipes. Only a browser can prove this:
  // the danger is a WRITE over stored bytes, and host tests have no storage.
  async function answerOneWord(page) {
    await page.click('#orbWrap');
    await page.waitForTimeout(500);
    const w = await page.evaluate(() => window.__spelltest.currentWord());
    assert(w, 'no word served');
    await typeOnKeyboard(page, w.toLowerCase());
    await page.click('#checkBtn');
    await page.waitForTimeout(800);
  }
  const rawOf = (page, k) => page.evaluate((key) => localStorage.getItem(key), k);

  await suite.test('learner: a NEWER build\'s stored state survives an older build, byte for byte', async () => {
    // What a player on a later build leaves behind when they reinstall this one.
    const newer = JSON.stringify({
      version: 99, lang: 'en', placed: true, profile: 3,
      skills: [{ id: 'silent_letters', mastery: 0.9, fsrs: { stability: 40, difficulty: 5.2, due_day: 20500, reps: 12, lapses: 1 } }],
      log: [{ day: 20400, word: 'knight', skills: ['silent_letters'], correct: true, channel: 'typed' }],
    });
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate((s) => localStorage.setItem('spell_learner_en', s), newer);
      await surfacesOn(page);
      await answerOneWord(page);
      await answerOneWord(page);
      assert((await rawOf(page, 'spell_learner_en')) === newer,
        'the newer build\'s learner state was OVERWRITTEN by an older build');
      assert(!(await page.$('#plcCard.show')), 'placement re-offered to a player whose newer state says placed');
      assert((await rawOf(page, 'spell_learner_en_unreadable')) === null,
        'a newer-schema state is not corruption and must not be backed up as such');
    } finally { await ctx.close(); }
  });

  await suite.test('learner: unreadable stored bytes are backed up exactly, then play recovers', async () => {
    const garbage = '{"version":1,"lang":"en","skills":"oops'; // truncated write
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate((s) => {
        localStorage.setItem('spell_learner_en', s);
        localStorage.removeItem('spell_learner_en_unreadable');
      }, garbage);
      await page.evaluate(() => localStorage.setItem('spell_flag_learner_surfaces', 'off'));
      await page.reload();
      await page.waitForTimeout(600);
      await answerOneWord(page);
      assert((await rawOf(page, 'spell_learner_en_unreadable')) === garbage,
        'the unreadable bytes were not preserved in the backup key');
      const st = JSON.parse((await rawOf(page, 'spell_learner_en')) || 'null');
      assert(st && typeof st.version === 'number' && Array.isArray(st.log) && st.log.length === 1,
        `play did not recover to a fresh, valid state: ${JSON.stringify(st)}`);
      // A second corruption must not clobber the FIRST backup.
      await page.evaluate(() => localStorage.setItem('spell_learner_en', 'second garbage'));
      await answerOneWord(page);
      assert((await rawOf(page, 'spell_learner_en_unreadable')) === garbage,
        'a later failure overwrote the original backup');
    } finally { await ctx.close(); }
  });

  // Learner schema v1 -> v2 (FSRS difficulty reset). The unit test proves
  // migrate() on a string; only a real browser proves the stored bytes. The
  // stakes: a failed load makes load_for/note_attempt start FRESH and save
  // over the old state, so a broken migration silently wipes a player.
  await suite.test('learner: a stored v1 state migrates to v2 on the next write, losing nothing', async () => {
    const W4 = 5.1618;
    const v1 = {
      version: 1, lang: 'en', placed: true,
      skills: [
        // stuck at the 1.0 floor by the FSRS version mix
        { id: 'silent_letters', mastery: 0.7, fsrs: { stability: 12.5, difficulty: 1.0, due_day: 20400, reps: 6, lapses: 1 } },
        { id: 'doubled_consonant', mastery: 0.2, fsrs: { stability: 0.4872, difficulty: 1.0, due_day: 20391, reps: 1, lapses: 0 } },
      ],
      log: [{ day: 20390, word: 'knight', skills: ['silent_letters'], correct: false, channel: 'typed', typed: 'nite' }],
    };
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate((s) => localStorage.setItem('spell_learner_en', JSON.stringify(s)), v1);
      await surfacesOn(page); // reload with v1 in place; placed=true means no offer card
      const read = () => page.evaluate(() => JSON.parse(localStorage.getItem('spell_learner_en') || 'null'));

      // Booting and reading must not rewrite (or wipe) the stored state.
      const booted = await read();
      assert(booted && booted.version === 1 && booted.log.length === 1,
        `boot alone changed the stored v1 state: ${JSON.stringify(booted)}`);

      // One real answer through the game: the save path writes v2.
      await page.click('#orbWrap');
      await page.waitForTimeout(500);
      const w = await page.evaluate(() => window.__spelltest.currentWord());
      assert(w, 'no word served');
      await typeOnKeyboard(page, w.toLowerCase());
      await page.click('#checkBtn');
      await page.waitForFunction(() => {
        const s = JSON.parse(localStorage.getItem('spell_learner_en') || 'null');
        return s && s.log && s.log.length === 2;
      }, null, { timeout: 5000 }).catch(() => {});

      const st = await read();
      assert(st, 'learner state GONE after a write — migration failure wiped it');
      assert(st.version === 2, `saved as v${st.version}, expected v2`);
      assert(st.log.length === 2, `log should be the v1 entry + this answer, got ${st.log.length}`);
      assert(st.log[0].word === 'knight' && st.log[0].typed === 'nite', 'the v1 log entry did not survive');
      assert(st.placed === true, `placed lost: ${JSON.stringify(st.placed)}`);
      for (const id of ['silent_letters', 'doubled_consonant']) {
        const s = st.skills.find((k) => k.id === id);
        assert(s, `${id} vanished`);
        // Reset to W4; a review of this word may since have raised it (a miss)
        // or kept it (a pass reverts to exactly W4). Never back at the floor.
        assert(s.fsrs.difficulty >= W4 - 1e-9,
          `${id} difficulty ${s.fsrs.difficulty}: not reset (the floor was 1.0)`);
      }
      const sl = st.skills.find((k) => k.id === 'silent_letters');
      assert(sl.fsrs.reps >= 6 && sl.fsrs.lapses >= 1, 'review counts were reset, not just difficulty');
    } finally { await ctx.close(); }
  });
}
