// submit-advance.spec — two features under one spec:
//   Feature 1 (A1–A3): exactly one submit control per answer form at every
//     viewport width, fully in-viewport and not overlapping the input, and the
//     single submission path fires exactly once per button-click / Enter.
//   Feature 2 (A4–A11): Daily Challenge auto-advances after a validated-correct
//     answer (reusing CORRECT_DELAY_MS), the orb is a live instant-skip that
//     can't double-advance, wrong answers never auto-advance, the final word
//     routes to results after the same delay, an open IME composition blocks
//     validation, and the deterministic daily sequence is unchanged.
//
// Types via real key clicks on the anti-dictation on-screen keyboard (the thing
// under test), reads expected words + daily cursor from the OBSERVE-only
// __spelltest seam, and never bypasses validation.
import { openApp, assert, assertEq, IS_WEB_BUILD } from '../harness.mjs';

// Mirrors src/consts.rs `CORRECT_DELAY_MS` — the delay Daily reuses for its
// auto-advance (same beat solo already uses). Kept in sync by hand; a drift
// would surface as an A4/A5 timing failure.
const DELAY = 2200;

const currentWord = (page) => page.evaluate(() => window.__spelltest.currentWord());
const dailyIdx = (page) => page.evaluate(() => window.__spelltest.dailyIdx());
const dailyCorrect = (page) => page.evaluate(() => window.__spelltest.dailyCorrect());
const dailyActive = (page) => page.evaluate(() => window.__spelltest.dailyActive());
const feedbackClass = (page) => page.$eval('#feedback', (e) => e.className).catch(() => '');
const scrimShown = (page) => page.$eval('#dailyResScrim', (e) => e.classList.contains('show')).catch(() => false);
const typeable = (w) => /^[a-z]+$/.test((w || '').toLowerCase());

async function typeWord(page, w) {
  for (const ch of w.toLowerCase()) {
    const key = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
    if (key) await key.click();
  }
}

async function waitAnswered(page, timeout = 3500) {
  await page.waitForFunction(() => {
    const c = document.getElementById('feedback').className;
    return c.includes('good') || c.includes('bad');
  }, null, { timeout }).catch(() => {});
}

// Enter Daily and serve the first word (orb tap). Returns nothing.
async function startDaily(page) {
  await page.click('#dailyBtn'); await page.waitForTimeout(300);
  await page.click('#orbWrap'); await page.waitForTimeout(350);
}

// Advance past the current word by answering it wrong, then tapping the orb
// (wrong never auto-advances, so the orb tap is the manual advance).
async function skipWrong(page) {
  const w = await currentWord(page);
  await typeWord(page, w.toLowerCase() === 'zzzz' ? 'xxxx' : 'zzzz');
  await page.click('#checkBtn');
  await waitAnswered(page);
  await page.click('#orbWrap'); await page.waitForTimeout(300);
}

// Serve daily words until the current one is base-typeable on the keyboard.
async function serveTypeable(page) {
  for (let i = 0; i < 12; i++) {
    const w = await currentWord(page);
    if (typeable(w)) return w;
    await skipWrong(page);
  }
  throw new Error('no base-typeable daily word found in first 12');
}

export async function run(browser, base, suite) {
  // ---- Feature 1: exactly one submit control per width -------------------
  const WIDTHS = [[320, 568], [375, 667], [390, 844], [428, 926], [1280, 800]];

  await suite.test('A1: exactly one visible submit control at each width (320→1280)', async () => {
    for (const [width, height] of WIDTHS) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width, height } });
      try {
        await page.click('#orbWrap'); await page.waitForTimeout(300); // serve a word → form active
        const n = await page.evaluate(() => {
          // A "submit control" = an element wired to submit the answer. The two
          // in the DOM are #checkBtn and #kbSubmit; count only the visible ones.
          return ['checkBtn', 'kbSubmit'].filter((id) => {
            const el = document.getElementById(id);
            if (!el) return false;
            const st = getComputedStyle(el);
            if (st.display === 'none' || st.visibility === 'hidden') return false;
            const r = el.getBoundingClientRect();
            return r.width > 0 && r.height > 0;
          }).length;
        });
        assertEq(n, 1, `submit controls at ${width}x${height}`);
      } finally { await ctx.close(); }
    }
  });

  await suite.test('A2: the submit control sits in-viewport and never overlaps the input', async () => {
    for (const [width, height] of WIDTHS) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width, height } });
      try {
        await page.click('#orbWrap'); await page.waitForTimeout(300);
        const r = await page.evaluate(() => {
          const btn = document.getElementById('checkBtn');
          btn.scrollIntoView({ block: 'center' });
          const b = btn.getBoundingClientRect();
          const inp = document.getElementById('spellbox').getBoundingClientRect();
          const noOverlap = b.right <= inp.left || b.left >= inp.right || b.bottom <= inp.top || b.top >= inp.bottom;
          const inView = b.left >= -1 && b.top >= -1 && b.right <= window.innerWidth + 1 && b.bottom <= window.innerHeight + 1;
          return { noOverlap, inView, b: { x: b.x, y: b.y, w: b.width, h: b.height } };
        });
        assert(r.b.w > 0 && r.b.h > 0, `submit control has no box at ${width}x${height}`);
        assert(r.inView, `submit control clipped out of viewport at ${width}x${height}: ${JSON.stringify(r.b)}`);
        assert(r.noOverlap, `submit control overlaps the input at ${width}x${height}`);
      } finally { await ctx.close(); }
    }
  });

  await suite.test('A3: submit fires exactly once — via button click and via Enter', async () => {
    // Solo mode: a correct answer bumps the chain by exactly 1. A double-fire
    // would either double-bump or be swallowed by the single-submission guard;
    // asserting +1 proves one net validation per action (I1).
    for (const via of ['button', 'enter']) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await page.click('#orbWrap'); await page.waitForTimeout(400);
        const w = await currentWord(page);
        assert(typeable(w), `A3 needs a base-typeable word, got "${w}"`);
        const before = await page.$eval('#streakNum', (e) => parseInt(e.textContent, 10) || 0);
        await typeWord(page, w);
        if (via === 'button') await page.click('#checkBtn');
        else await page.evaluate(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })));
        await page.waitForFunction((b) => (parseInt(document.getElementById('streakNum').textContent, 10) || 0) !== b, before, { timeout: 3000 });
        await page.waitForTimeout(300);
        const after = await page.$eval('#streakNum', (e) => parseInt(e.textContent, 10) || 0);
        assertEq(after, before + 1, `chain delta for submit via ${via}`);
      } finally { await ctx.close(); }
    }
  });

  // ---- Feature 2: Daily auto-advance -------------------------------------
  await suite.test('A4: Daily(en) correct → next word auto-loads within DELAY+500ms', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startDaily(page);
      await serveTypeable(page);
      const idx0 = await dailyIdx(page);
      const w = await currentWord(page);
      await typeWord(page, w);
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 3500 });
      // Not advanced yet, immediately after the success feedback.
      assertEq(await dailyIdx(page), idx0, 'advanced before the delay elapsed');
      // Auto-advances after the delay, with no further input.
      await page.waitForFunction((i) => window.__spelltest.dailyIdx() === i + 1, idx0, { timeout: DELAY + 500 });
      assertEq(await dailyIdx(page), idx0 + 1, 'did not auto-advance within DELAY+500ms');
    } finally { await ctx.close(); }
  });

  await suite.test('A5: orb-click ~300ms in advances immediately by exactly 1', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startDaily(page);
      await serveTypeable(page);
      const idx0 = await dailyIdx(page);
      const w = await currentWord(page);
      await typeWord(page, w);
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 3500 });
      await page.waitForTimeout(300);
      await page.click('#orbWrap'); // skip
      // Immediate: advanced well before the full delay would have.
      await page.waitForFunction((i) => window.__spelltest.dailyIdx() === i + 1, idx0, { timeout: 800 });
      assertEq(await dailyIdx(page), idx0 + 1, 'orb skip did not advance immediately');
      // And the pending auto-advance timer is a no-op — still exactly +1.
      await page.waitForTimeout(DELAY + 400);
      assertEq(await dailyIdx(page), idx0 + 1, 'skip + timer double-advanced');
    } finally { await ctx.close(); }
  });

  await suite.test('A6: 5 rapid orb-clicks advance by exactly 1 (no double-skip)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startDaily(page);
      await serveTypeable(page);
      const idx0 = await dailyIdx(page);
      const w = await currentWord(page);
      await typeWord(page, w);
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 3500 });
      for (let i = 0; i < 5; i++) { await page.click('#orbWrap'); }
      await page.waitForTimeout(DELAY + 400);
      assertEq(await dailyIdx(page), idx0 + 1, 'rapid orb-clicks skipped more than one word');
    } finally { await ctx.close(); }
  });

  await suite.test('A7: incorrect answer never auto-advances (wait 3×DELAY)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startDaily(page);
      await serveTypeable(page);
      const idx0 = await dailyIdx(page);
      const w = await currentWord(page);
      await typeWord(page, w.toLowerCase() === 'zzzz' ? 'xxxx' : 'zzzz'); // wrong
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('bad'), null, { timeout: 3500 });
      await page.waitForTimeout(DELAY * 3);
      assertEq(await dailyIdx(page), idx0, 'wrong answer auto-advanced');
    } finally { await ctx.close(); }
  });

  await suite.test('A8: final word → results after DELAY; score/streak == a skip-advance run', async () => {
    // Play the full deterministic set two ways — letting auto-advance run, and
    // skipping via the orb — answering each word identically (correct iff
    // base-typeable). Same date ⇒ same words ⇒ identical recorded score/streak,
    // proving the advance mechanism is presentational and the final word is not
    // special-cased.
    async function playDaily(mode) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await startDaily(page);
        for (let guard = 0; guard < 40; guard++) {
          if (await scrimShown(page)) break;
          if (!(await dailyActive(page))) break;
          const w = await currentWord(page);
          if (!w) break;
          const correct = typeable(w);
          await typeWord(page, correct ? w : (w.toLowerCase() === 'zzzz' ? 'xxxx' : 'zzzz'));
          await page.click('#checkBtn');
          await waitAnswered(page);
          const good = (await feedbackClass(page)).includes('good');
          if (good && mode === 'auto') {
            await page.waitForTimeout(DELAY + 400); // let auto-advance fire
          } else {
            await page.click('#orbWrap'); await page.waitForTimeout(250); // wrong or skip → manual
          }
        }
        assert(await scrimShown(page), `${mode}: results screen never appeared`);
        const score = await page.$eval('#dailyResScore', (e) => e.textContent.trim());
        const streak = await page.$eval('#dailyResStreak', (e) => e.textContent.trim());
        return { score, streak };
      } finally { await ctx.close(); }
    }
    const auto = await playDaily('auto');
    const skip = await playDaily('skip');
    assertEq(auto.score, skip.score, 'recorded score differs between auto-advance and skip runs');
    assertEq(auto.streak, skip.streak, 'recorded streak differs between auto-advance and skip runs');
  });

  await suite.test('A9: language matrix — es is coming-soon-gated; en async path proves agnosticism', async () => {
    // Half one is a PLATFORM claim and it now cuts both ways: Spanish is
    // coming-soon-gated on the site and playable in the app (Eric,
    // 2026-07-31). Asserting the build's own truth makes this stronger than
    // the old hardcoded "gated" -- it pins the split from the UI in whichever
    // config is running, instead of being right in one and red in the other.
    //
    // Half two is config-independent: en Daily grades via the ASYNC /api/check
    // path and es/others via the SYNC norm path, yet both funnel into the one
    // on_correct daily hook that schedules the advance (verified below for
    // en); and the I4 grep audit shows ZERO per-language conditionals in the
    // advance path.
    {
      const { ctx, page } = await openApp(browser, base, { lang: 'es' });
      try {
        const gated = await page.evaluate(() => document.body.classList.contains('coming-soon'));
        assertEq(gated, IS_WEB_BUILD,
          IS_WEB_BUILD ? 'Spanish must be gated on the site build'
                       : 'Spanish must be playable in the app build');
      } finally { await ctx.close(); }
    }
    {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await startDaily(page);
        await serveTypeable(page);
        const idx0 = await dailyIdx(page);
        const w = await currentWord(page);
        await typeWord(page, w);
        await page.click('#checkBtn'); // en → async backend_verify → on_correct
        // Wait for the GRADE to land before timing the advance -- exactly what
        // A4 does. Starting the clock at the click made this the only timing
        // assertion in the file that also had to cover a live /api/check round
        // trip to the word server, leaving 800ms for a call measured at 232ms
        // mean and 577ms worst while the rest of the gate saturates the CPU.
        // It went red at HEAD with no code change: the test was timing the
        // network, not the auto-advance it exists to prove.
        await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'),
          null, { timeout: 3500 });
        await page.waitForFunction((i) => window.__spelltest.dailyIdx() === i + 1, idx0, { timeout: DELAY + 800 });
        assertEq(await dailyIdx(page), idx0 + 1, 'async-path correct answer did not auto-advance');
      } finally { await ctx.close(); }
    }
  });

  await suite.test('A10: open IME composition blocks validation until compositionend', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#orbWrap'); await page.waitForTimeout(400);
      const w = await currentWord(page);
      assert(typeable(w), `A10 needs a base-typeable word, got "${w}"`);
      await typeWord(page, w);
      // Open a composition, then try to submit — must NOT validate.
      await page.evaluate(() => window.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true })));
      await page.click('#checkBtn');
      await page.waitForTimeout(800);
      assert(!(await feedbackClass(page)).includes('good'), 'validated while a composition was open');
      // Close the composition — now the same submit validates.
      await page.evaluate(() => window.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })));
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 3000 });
      assert((await feedbackClass(page)).includes('good'), 'submit did not validate after compositionend');
    } finally { await ctx.close(); }
  });

  await suite.test('A11: daily sequence is deterministic for a fixed date+language', async () => {
    async function firstWords(n) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await startDaily(page);
        const seq = [];
        for (let i = 0; i < n; i++) {
          seq.push(await currentWord(page));
          await skipWrong(page);
        }
        return seq;
      } finally { await ctx.close(); }
    }
    const a = await firstWords(4);
    const b = await firstWords(4);
    assert(a.length === 4 && a.every((w) => w && w.length), 'daily produced empty words');
    assertEq(JSON.stringify(a), JSON.stringify(b), 'daily word sequence not deterministic across runs');
  });

  // ---- Solo: the orb skip and the pending auto-advance --------------------
  await suite.test('A12: solo orb-skip — the next word is not replaced when the old timer fires', async () => {
    // The solo correct answer sets the same DELAY timer Daily does. Skip ahead
    // with the orb, start typing the next word, and outlast the old timer: the
    // word on screen and the letters typed into it must both survive.
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 4000 });
      const w1 = await currentWord(page);
      assert(typeable(w1), `A12 needs a base-typeable word, got "${w1}"`);
      await typeWord(page, w1);
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('feedback').className.includes('good'), null, { timeout: 3500 });
      await page.click('#orbWrap'); // skip
      await page.waitForFunction((p) => window.__spelltest.currentWord() !== p, w1, { timeout: 800 });
      const w2 = await currentWord(page);
      await page.waitForFunction(() => !document.getElementById('gameKeyboard').classList.contains('locked'), null, { timeout: 3000 });
      const first = w2.toLowerCase()[0];
      await page.click(`#gameKeyboard .kb-key[data-k="${first}"]`);
      await page.waitForTimeout(DELAY + 600);
      assertEq(await currentWord(page), w2, 'the old timer replaced the word skipped to');
      const box = await page.$eval('#spellbox', (e) => e.textContent.trim());
      assertEq(box, first, 'the letter typed into the skipped-to word was wiped');
    } finally { await ctx.close(); }
  });
}
