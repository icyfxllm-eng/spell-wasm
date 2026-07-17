// attempts-shields.spec — CC-ATTEMPTS-SHIELDS at the UI boundary.
//
// build-54: the flag `spell_flag_attempts_shields` defaults ON (the legacy 3-try
// mechanic is retired, so the extra-attempts toggle + Climb shields are the live
// safety net). The flag is toggled the FP2 way — real `localStorage` set through
// each context's addInitScript (mirrors sayit / spellaloud / ghost), NOT a
// harness param and NOT the cfg(test) override (which the wasm seam build lacks).
//
// Verified here:
//   * flag OFF (explicit) -> Feature 1 settings row + Feature 2 HUD hidden
//     (byte-for-byte the pre-shields build).
//   * flag ON (default)   -> row revealed, toggle still defaults OFF per player.
//   * 3-try RETIRED       -> one wrong submission is immediately the miss reveal.
//   * extra-attempts ON   -> one wrong submission grants exactly one clean retry.
//   * language matrix (en / es / ja) reruns identical assertions — zero
//     per-language behavior.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const FLAG = 'spell_flag_attempts_shields';
const hidden = (page, sel) => page.$eval(sel, (e) => e.classList.contains('btn-hide'));
const exists = (page, sel) => page.$(sel).then((h) => !!h);

/** Open the app with the dark flag pinned to `value` ('on'|'off') before boot,
 *  mirroring the FP2 spec pattern (real localStorage via addInitScript). */
async function openWithFlag(browser, base, value, lang = null) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
  await ctx.addInitScript(([age, l, flag, v]) => {
    localStorage.setItem('byear_agegate_v1', age);
    if (l) localStorage.setItem('spellgame.locale', l);
    localStorage.setItem(flag, v);
  }, [AGE, lang, FLAG, value]);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  if (lang) {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#langSel', lang).catch(() => {});
    await page.click('#setupDone').catch(() => {});
    await page.waitForTimeout(200);
  }
  return { ctx, page };
}

async function missOnce(page) {
  await page.click('#orbWrap');
  await page.waitForTimeout(500);
  const word = await page.evaluate(() => window.__spelltest.currentWord());
  const wrong = word === 'zzzz' ? 'xxxx' : 'zzzz';
  await typeOnKeyboard(page, wrong);
  await page.click('#checkBtn');
  await page.waitForTimeout(500);
}


// ---------------------------------------------------------------------------
// CC-CLIMB-SHIELDS — the FORGE at the UI boundary.
//
// The forge's maths is unit-tested in attempts.rs. What only a browser can prove
// is that the HUD RENDERS it: `update_shield_hud_ex` calls set_html("shieldForge"),
// and dom::el PANICS on a missing id. The markup shipped without #shieldForge, so
// a Climb run took the wasm module down on the first correct answer — with no
// failing test, because nothing drove The Climb. These do.
//
// State is read from data-* (not localized text), exactly as the HUD documents.
// ---------------------------------------------------------------------------

/** Enter The Climb the way a player does: setup chip -> level -> done. */
async function enterClimb(page) {
  await page.click('#setupChip').catch(() => {});
  await page.selectOption('#levelSel', 'climb').catch(() => {});
  await page.click('#setupDone').catch(() => {});
  await page.waitForTimeout(250);
}

/**
 * The Climb interrupts: a 5-word chain fires the `chain5` achievement, and its
 * modal intercepts pointer events, so a run of 6 answers stalls on the keyboard.
 * Dismiss whatever is open before reaching for a key. (My first probe hid this by
 * swallowing every click failure with .catch() — it looked like it worked.)
 */
async function dismissOverlays(page) {
  for (const sel of ['#scrim.show', '.scrim.show']) {
    const open = await page.$(sel);
    if (!open) continue;
    await page.keyboard.press('Escape').catch(() => {});
    await page.waitForTimeout(150);
    for (const btn of ['#achDone', '#setDone', '#setupDone', '#climbDone']) {
      const b = await page.$(`${btn}:visible`);
      if (b) { await b.click().catch(() => {}); await page.waitForTimeout(150); }
    }
  }
}

/** Answer the current word correctly on the real on-screen keyboard. */
async function answerCorrectly(page) {
  await dismissOverlays(page);
  await page.click('#orbWrap').catch(() => {});
  await page.waitForTimeout(400);
  const word = await page.evaluate(() => window.__spelltest.currentWord());
  if (!word) return null;
  await dismissOverlays(page);
  await typeOnKeyboard(page, word);
  await page.click('#checkBtn').catch(() => {});
  await page.waitForTimeout(600);
  return word;
}

const forgeState = (page) => page.evaluate(() => {
  const h = document.getElementById('shieldHud');
  return {
    forge: h.dataset.forge,
    segments: +h.dataset.segments,
    shields: +h.dataset.shields,
    gain: +h.dataset.gain,
    pips: h.querySelectorAll('.shield-forge .seg').length,
    lit: h.querySelectorAll('.shield-forge .seg.on').length,
  };
});

export async function run(browser, base, suite) {
  await suite.test('flag OFF (explicit): extra-attempts row + shield HUD hidden', async () => {
    const { ctx, page } = await openWithFlag(browser, base, 'off');
    try {
      assert(await exists(page, '#extraAttemptsRow'), 'row element present in DOM');
      assert(await hidden(page, '#extraAttemptsRow'), 'extra-attempts row must be hidden when flag OFF');
      assert(await hidden(page, '#shieldHud'), 'shield HUD must be hidden when flag OFF');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON (build-54 default): extra-attempts row revealed + toggle defaults OFF', async () => {
    // No flag set -> compiled default ON.
    const { ctx, page } = await openApp(browser, base, {});
    try {
      assert(!(await hidden(page, '#extraAttemptsRow')), 'row must be shown when flag ON (default)');
      const checked = await page.$eval('#extraAttemptsToggle', (e) => e.checked);
      assert(checked === false, 'the per-player toggle still defaults OFF even though the flag is on');
    } finally { await ctx.close(); }
  });

  await suite.test('3-try RETIRED: one wrong submission is immediately the miss (word revealed)', async () => {
    // Normal mode, extra-attempts toggle OFF -> the base path is now one-shot:
    // a single wrong answer reveals the word (no 2nd/3rd try) and reads as a miss.
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await missOnce(page);
      const cls = await page.$eval('#feedback', (e) => e.className);
      assert(!cls.includes('good'), 'wrong answer must not read as correct');
      assert(cls.includes('bad'), 'one wrong answer must land on the miss consequence');
      const revealed = await page.$('#feedback .reveal');
      assert(!!revealed, 'the word must be revealed after a single miss (one-shot, no retry pips)');
      const still = await page.evaluate(() => window.__spelltest.build());
      assert(still === 'testseam', 'wasm crashed on the one-shot miss path');
    } finally { await ctx.close(); }
  });

  await suite.test('extra-attempts ON: a wrong submission grants exactly one clean retry', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // The extra-attempts toggle applies only in a FIXED level (not the adaptive
      // Climb, where shields own retries). Pick Medium via the setup chip first.
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#levelSel', 'medium').catch(() => {});
      await page.click('#setupDone').catch(() => {});
      await page.waitForTimeout(150);
      // Enable the per-player toggle, then miss once.
      await page.click('#setBtn').catch(() => {});
      await page.click('#extraAttemptsToggle');
      await page.click('#setDone').catch(() => {});
      await missOnce(page);
      // A retry was granted: the word is NOT yet revealed and the check control is
      // live again for a second swing. (finalize would show a `.reveal` span.)
      const revealed = await page.$('#feedback .reveal');
      assert(!revealed, 'the first miss under the toggle must grant a retry, not reveal the word');
      const canRetry = await page.$eval('#checkBtn', (e) => !e.disabled);
      assert(canRetry, 'controls must be re-enabled for the granted retry');
      const still = await page.evaluate(() => window.__spelltest.build());
      assert(still === 'testseam', 'wasm crashed on the extra-attempt retry path');
    } finally { await ctx.close(); }
  });

  await suite.test('flag ON: extra-attempts toggle persists across reload (single-source pref)', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await page.click('#setBtn').catch(() => {});
      await page.click('#extraAttemptsToggle');
      assert(await page.$eval('#extraAttemptsToggle', (e) => e.checked), 'toggle should be on after click');
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(await page.$eval('#extraAttemptsToggle', (e) => e.checked), 'toggle state must survive reload');
    } finally { await ctx.close(); }
  });

  // Language matrix — identical assertions, three languages incl. a
  // composition-input one (ja). Same code path, no per-language branching.
  for (const lang of ['en', 'es', 'ja']) {
    await suite.test(`flag ON in ${lang}: toggle revealed (identical test code)`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        assert(!(await hidden(page, '#extraAttemptsRow')), `${lang}: row shown`);
        assert(await exists(page, '#extraAttemptsToggle'), `${lang}: toggle present`);
      } finally { await ctx.close(); }
    });
  }

  // ----- CC-CLIMB-SHIELDS: the forge -----

  await suite.test('climb: the forge HUD renders without panicking (regression)', async () => {
    const { ctx, page } = await openWithFlag(browser, base, 'on');
    const errs = [];
    // A wasm panic surfaces as a pageerror. The audio stub's NotSupportedError is
    // the harness's empty MP3 body, not the app.
    page.on('pageerror', (e) => { const m = String(e).split('\n')[0]; if (!/NotSupportedError/.test(m)) errs.push(m); });
    try {
      await enterClimb(page);
      const word = await answerCorrectly(page);
      assert(word, 'no word was served in The Climb');
      assert(!(await hidden(page, '#shieldHud')), 'shield HUD must be visible in The Climb');
      const st = await forgeState(page);
      assert(st.pips === 5, `forge track must render 5 pips (FORGE.segments_per_shield), got ${st.pips}`);
      assert(errs.length === 0, `wasm panicked: ${errs.join(' | ')}`);
    } finally { await ctx.close(); }
  });

  await suite.test('climb: a perfect word grants 2 segments and lights 2 pips', async () => {
    const { ctx, page } = await openWithFlag(browser, base, 'on');
    try {
      await enterClimb(page);
      await answerCorrectly(page);
      const st = await forgeState(page);
      // First-attempt correct with no syllable replay == perfect == 2 segments.
      assert(st.gain === 2, `a perfect word grants 2 segments, got ${st.gain}`);
      assert(st.segments === 2 && st.lit === 2, `track must show 2/5, got ${st.lit}/${st.pips}`);
      assert(st.forge === 'forging', `state must be forging, got ${st.forge}`);
    } finally { await ctx.close(); }
  });

  await suite.test('climb: forging FREEZES at the cap (2 shields, gain 0)', async () => {
    const { ctx, page } = await openWithFlag(browser, base, 'on');
    try {
      await enterClimb(page);
      // 5 perfect words = 10 segments = 2 shields = the cap.
      let st = null;
      for (let i = 0; i < 6; i++) { if (!(await answerCorrectly(page))) break; st = await forgeState(page); }
      assert(st.shields === 2, `cap is 2 shields (CHANGED from 3), got ${st.shields}`);
      assert(st.forge === 'full', `at the cap the state is full, got ${st.forge}`);
      assert(st.gain === 0, `at the cap a correct word grants NOTHING, got ${st.gain}`);
      assert(st.segments === 0, `no partial progress is held while full, got ${st.segments}`);
    } finally { await ctx.close(); }
  });
}
