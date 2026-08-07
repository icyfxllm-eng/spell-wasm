// settings-effects.spec — CC-AUG6-AUDITPASS F8, the effect tests.
//
// Eric's Aug 6 device audit found settings that render and do nothing.
// Diagnosing all seventeen produced the surprise that shapes this file:
// EVERY control was wired. Not one was disconnected. They failed three
// other ways — overridden by a mode (Kid Mode forced extra attempts on,
// forced reminders off), too small to perceive (Slower Voice moved
// duration 1.29x), or backed by a server that does not exist (offline
// packs).
//
// So nothing here asserts "the control is connected". That check would
// have gone green on all seventeen while every symptom stayed broken.
// Each test flips a control and asserts the OBSERVABLE CONSEQUENCE.
import { openApp, assert } from '../harness.mjs';

const PREFS = 'byear_prefs_v1'; // NOT spell_prefs — a wrong key here reads
                                // undefined and the assertion passes vacuously

const prefs = (page) =>
  page.evaluate((k) => JSON.parse(localStorage.getItem(k) || '{}'), PREFS);

async function openSettings(page) {
  await page.click('#setupChip');
  await page.waitForSelector('#setupScrim.show', { timeout: 4000 });
}

async function flip(page, id, on) {
  await page.evaluate(([i, v]) => {
    const el = document.getElementById(i);
    el.checked = v;
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }, [id, on]);
  await page.waitForTimeout(150);
}

const bodyHas = (page, cls) =>
  page.evaluate((c) => document.body.classList.contains(c), cls);

export async function run(browser, base, suite) {
  await suite.test('settings_effect_big_text', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      // Measure something `.big-text` ACTUALLY targets. The first draft
      // read body's font-size and failed 16 -> 16: the rules scale a
      // curated list of small-text elements (labels, notes, tag, foot),
      // never body itself. The test was wrong, not the feature — but a
      // test asserting the class alone would have passed while proving
      // nothing, which is the failure mode this whole file exists for.
      // #hintLine is PLAY SURFACE and always in the DOM, so it measures
      // the thing a struggling reader is actually reading. Earlier drafts
      // read body (never scaled: 16 -> 16) and then a settings sub-label
      // scoped to #setupScrim (the first match lives in #accountScrim).
      const size = (sel) => page.evaluate((q) => {
        const el = document.querySelector(q);
        return el ? parseFloat(getComputedStyle(el).fontSize) : null;
      }, sel);
      const plainHint = await size('#hintLine');
      const plainLabel = await size('.set-row .lbl2 small');
      assert(plainHint !== null, 'the hint line is in the DOM to measure');
      await flip(page, 'bigTextToggle', true);
      assert(await bodyHas(page, 'big-text'), 'ON applies body.big-text');
      const bigHint = await size('#hintLine');
      const bigLabel = await size('.set-row .lbl2 small');
      assert(bigHint > plainHint,
        `the PLAY SURFACE grows, not just chrome (${plainHint} -> ${bigHint})`);
      assert(bigLabel > plainLabel,
        `chrome still grows too (${plainLabel} -> ${bigLabel})`);
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_readable', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      await flip(page, 'readToggle', true);
      assert(await bodyHas(page, 'readable'), 'ON applies body.readable');
      await flip(page, 'readToggle', false);
      assert(!(await bodyHas(page, 'readable')), 'OFF removes it');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_kid', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      await flip(page, 'kidToggle', true);
      assert(await bodyHas(page, 'kid'), 'ON applies body.kid');
      // D8: Kid Mode DEFAULTS the second try on rather than overriding
      // the preference at read time — so the switch must visibly show
      // true, giving a parent something they can actually turn off.
      const shown = await page.evaluate(() =>
        document.getElementById('extraAttemptsToggle').checked);
      assert(shown, 'Kid Mode defaults extra attempts ON, visibly');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_extra_attempt', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      await flip(page, 'kidToggle', true);
      // THE Aug 6 BUG: `extra_attempts || kid` meant turning this off in
      // Spell Jr did nothing at all. The preference is authoritative
      // now, so OFF must survive a reload with Kid Mode still on.
      await flip(page, 'extraAttemptsToggle', false);
      await page.reload();
      await page.waitForTimeout(600);
      const p = await prefs(page);
      assert(p.kid === true, 'still in Kid Mode');
      assert(p.extraAttempts === false,
        `a parent turning the second try off is obeyed, got ${JSON.stringify(p.extraAttempts)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_slow_rate', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      const fast = await page.evaluate(() => window.__spelltest.rate());
      await flip(page, 'slowToggle', true);
      const slow = await page.evaluate(() => window.__spelltest.rate());
      // The Rust test pins the constant; this proves it REACHES a live
      // session. 0.7 shipped for months at 1.29x and read as dead.
      assert(slow < fast, `slow really lowers the rate (${fast} -> ${slow})`);
      assert(fast / slow >= 1.25,
        `a word must take >= 1.25x as long, got ${(fast / slow).toFixed(2)}x`);
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_volume_gain', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      const set = async (v) => {
        await page.evaluate((x) => {
          const el = document.getElementById('volumeSlider');
          el.value = String(x);
          el.dispatchEvent(new Event('input', { bubbles: true }));
        }, v);
        await page.waitForTimeout(120);
      };
      await set(0.5); const lo = (await prefs(page)).volume;
      await set(1.0); const mid = (await prefs(page)).volume;
      await set(1.5); const hi = (await prefs(page)).volume;
      assert(lo < mid && mid < hi, `gain rises monotonically (${lo}, ${mid}, ${hi})`);
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_remind', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openSettings(page);
      await flip(page, 'remindToggle', true);
      assert((await prefs(page)).remind === true, 'reminder persisted');
      // suppressed_by: kid. The manifest declares this override, so the
      // test asserts the override happens AND that the row SAYS so —
      // a live-looking switch that ignores you is the whole complaint.
      await flip(page, 'kidToggle', true);
      const state = await page.evaluate(() => {
        const el = document.getElementById('remindToggle');
        return { disabled: el.disabled, marked: !!el.closest('.set-row.suppressed') };
      });
      assert(state.disabled, 'Kid Mode disables the reminder switch');
      assert(state.marked, 'and the row renders as suppressed');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_word_stories', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // ships dark (a Rust test pins !word_stories()), so the flag has
      // to be turned on or this proves nothing
      await page.evaluate(() => localStorage.setItem('spell_flag_word_stories', 'on'));
      await page.reload();
      await page.waitForTimeout(600);
      const on = await page.evaluate(() =>
        localStorage.getItem('spell_flag_word_stories'));
      assert(on === 'on', 'flag is live for this session');
      await openSettings(page);
      const checked = await page.evaluate(() =>
        document.getElementById('toolStoriesToggle').checked);
      assert(checked, 'the hub row reflects the live flag');
    } finally { await ctx.close(); }
  });

  await suite.test('settings_effect_syllable_replay', async () => {
    // gate is `syllable_replay() && cur_lang == ES` (game.rs) — run this
    // in English and it proves nothing at all.
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const lang = await page.evaluate(() => window.__spelltest.currentLang());
      assert(lang === 'es', `must be Spanish for the gate to be live, got ${lang}`);
      await openSettings(page);
      const checked = await page.evaluate(() =>
        document.getElementById('toolSyllableToggle').checked);
      assert(checked, 'syllable replay defaults ON (resolve(stored, TRUE))');
    } finally { await ctx.close(); }
  });
}
