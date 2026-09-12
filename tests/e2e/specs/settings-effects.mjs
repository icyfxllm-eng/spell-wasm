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

  // CC-ONBOARD-JR Done 8 — the level selector follows the experience at once,
  // not at the next launch. The inventory found the toggle rebuilt nothing, so
  // a player who switched Spell Jr on could still pick Expert.
  // Done 3 asks for English plus one non-Latin language.
  for (const lang of ['en', 'ko']) await suite.test(`settings_effect_kid_levels_${lang}`, async () => {
    const { ctx, page } = await openApp(browser, base, { lang });
    try {
      await openSettings(page);
      const levels = () => page.evaluate(() =>
        [...document.querySelectorAll('#levelSel option')].map((o) => o.value));
      const before = await levels();
      assert(before.includes('hard') && before.includes('expert'),
        `standard offers Hard and Expert, got ${JSON.stringify(before)}`);
      await page.selectOption('#levelSel', 'expert');
      await page.waitForTimeout(150);
      await flip(page, 'kidToggle', true);
      const on = await levels();
      assert(!on.includes('hard') && !on.includes('expert'),
        `Spell Jr offers no Hard or Expert, got ${JSON.stringify(on)}`);
      assert(['climb', 'easy', 'medium'].every((v) => on.includes(v)),
        `Spell Jr keeps Climb, Easy, Medium, got ${JSON.stringify(on)}`);
      const value = await page.evaluate(() => document.getElementById('levelSel').value);
      assert(value === 'medium', `an Expert selection falls back to Medium, got ${JSON.stringify(value)}`);
      await flip(page, 'kidToggle', false);
      const off = await levels();
      assert(off.includes('hard') && off.includes('expert'),
        `switching Spell Jr off brings Hard and Expert back, got ${JSON.stringify(off)}`);
    } finally { await ctx.close(); }
  });

  // CC-ONBOARD-JR Done 9 — a locked Spell Jr player finds no switch and no
  // birthday re-entry; only a grown-up passing the existing parent gate
  // reaches the birthday prompt, and an adult date reaches the standard game.
  await suite.test('settings_effect_kid_locked', async () => {
    const kidVerdict = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: kidVerdict });
    try {
      // REAL visibility. An element inside a closed panel still reports its
      // own display, so the first draft of this check passed while the
      // settings sheet was shut and the click then timed out. offsetParent
      // plus a box is what a player can actually see and press.
      const shown = (id) => page.evaluate((i) => {
        const el = document.getElementById(i);
        if (!el) return false;
        const r = el.getBoundingClientRect();
        return !!el.offsetParent && r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
      }, id);
      const scrimUp = (id) => page.evaluate((i) => document.getElementById(i).classList.contains('show'), id);
      const levels = () => page.evaluate(() =>
        [...document.querySelectorAll('#levelSel option')].map((o) => o.value));
      // The Spell Jr row lives in the SETTINGS sheet (#setScrim), not the
      // round-setup sheet openSettings opens. Every other test in this file
      // dispatches change events, so no test had ever needed the sheet open.
      await page.click('#setBtn');
      await page.waitForSelector('#setScrim.show', { timeout: 4000 });
      assert(await bodyHas(page, 'kid'), 'an under-13 verdict boots into Spell Jr');
      assert(!(await shown('kidToggle')), 'a locked Spell Jr player sees no Spell Jr switch');
      assert(await shown('kidGrownups'), 'the row offers the grown-up gate instead');
      assert(!(await scrimUp('ageScrim')), 'no birthday re-entry without the gate');
      const jr = await levels();
      assert(!jr.includes('hard') && !jr.includes('expert'), `locked Spell Jr offers no Hard or Expert, got ${JSON.stringify(jr)}`);

      await page.click('#kidGrownups');
      await page.waitForSelector('#parentScrim.show', { timeout: 4000 });
      await page.fill('#parentAnswer', '0');
      await page.click('#parentSubmit');
      await page.waitForTimeout(150);
      assert(!(await scrimUp('ageScrim')), 'a wrong answer does not open the birthday prompt');

      const q = (await page.textContent('#parentQ')) || '';
      const W = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
      const nums = (q.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => W[w]);
      assert(nums.length === 2, `could not read the parent question ${JSON.stringify(q)}`);
      await page.fill('#parentAnswer', String(nums[0] * nums[1]));
      await page.click('#parentSubmit');
      await page.waitForSelector('#ageScrim.show', { timeout: 4000 });
      await page.selectOption('#ageYear', '1990');
      await page.selectOption('#ageMonth', '1');
      await page.selectOption('#ageDay', '1');
      await page.click('#ageSubmit');
      await page.waitForTimeout(250);
      assert(!(await bodyHas(page, 'kid')), 'a grown-up birthday leaves Spell Jr');
      assert(await shown('kidToggle'), 'the switch returns for a standard player');
      assert(!(await shown('kidGrownups')), 'and the grown-up button leaves');
      const std = await levels();
      assert(std.includes('hard') && std.includes('expert'), `standard is reachable again, got ${JSON.stringify(std)}`);
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

  await suite.test('btn_hide_actually_hides', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // AUDITPASS F16-adjacent. `.btn-hide` existed only in SCOPED forms
      // (.launch.btn-hide, .wp-screen .btn-hide, …), so a bare use was
      // not hidden at all — three 4px entry-point buttons stacked at the
      // bottom of the base game as Eric's "stray white line", plus a
      // 156x40 share button and the shield HUD. A second fault hid
      // behind the first: shareBtn's INLINE display:block beat every
      // stylesheet rule. Both are invisible to static reading, so this
      // has to be measured in a browser.
      const bad = await page.evaluate(() =>
        [...document.querySelectorAll('.btn-hide')]
          .filter((el) => {
            const b = el.getBoundingClientRect();
            return getComputedStyle(el).display !== 'none' && (b.height > 0 || b.width > 0);
          })
          .map((el) => el.id || el.className));
      assert(bad.length === 0, `btn-hide failed to hide: ${JSON.stringify(bad)}`);
      // ...and hiding must not become permanent: dropping the class reveals.
      const shown = await page.evaluate(() => {
        const el = document.getElementById('shareBtn');
        el.classList.remove('btn-hide');
        return getComputedStyle(el).display;
      });
      assert(shown !== 'none', 'removing btn-hide must reveal the element');
    } finally { await ctx.close(); }
  });

  await suite.test('photo_split_proposes_and_never_auto_applies', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // F14. The photo flow is native-gated, so the review sheet is
      // unreachable in a browser through the camera. The seam fabricates
      // the OCR RESULT only — real classification, real renderer.
      await page.evaluate(() => localStorage.setItem('spell_flag_photo_list', 'on'));
      await page.reload();
      await page.waitForTimeout(700);
      await page.evaluate(() =>
        window.__spelltest.photoReview(['Thisisanexample', 'window', 'Sundeep']));
      await page.waitForSelector('#photoScrim.show', { timeout: 4000 });

      const before = await page.evaluate(() => ({
        rows: document.querySelectorAll('#photoChips .pchip-row').length,
        // the proposal must SHOW THE PIECES, not a count (Eric's call)
        splits: [...document.querySelectorAll('.pchip-split')].map((b) => b.textContent.trim()),
        words: [...document.querySelectorAll('#photoChips .pchip')].map((i) => i.value),
      }));
      assert(before.rows === 3, `three chips rendered, got ${before.rows}`);
      assert(before.words.includes('Thisisanexample'), 'the smash-up is a chip');
      const smash = before.splits.find((t) => t.includes('example'));
      assert(smash, `a split is proposed showing its pieces: ${JSON.stringify(before.splits)}`);
      assert(smash.includes('·'), 'pieces are shown separated, not counted');
      // a real word is never offered a split
      assert(!before.splits.some((t) => t === 'Split: window'), 'a bank word gets no proposal');
      // NOTHING has split yet — the proposal is inert until tapped
      assert(!before.words.includes('this'), 'no auto-split before the tap');

      await page.evaluate(() => {
        const b = [...document.querySelectorAll('.pchip-split')]
          .find((x) => x.textContent.includes('example'));
        b.click();
      });
      await page.waitForTimeout(300);
      const after = await page.evaluate(() =>
        [...document.querySelectorAll('#photoChips .pchip')].map((i) => i.value));
      for (const piece of ['this', 'is', 'an', 'example']) {
        assert(after.includes(piece), `tap produced "${piece}" — got ${JSON.stringify(after)}`);
      }
      assert(!after.includes('Thisisanexample'), 'the smashed chip was replaced');
      assert(after.includes('window'), 'other chips are untouched');
    } finally { await ctx.close(); }
  });

  await suite.test('spell_jr_shows_no_prices', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // AUDITPASS F12. The settings row promises "Bigger text, friendly
      // words, no prices". Big text and the friendly bank were real; NO
      // PRICES was not. Calendar is kidSafe:true, and its planner rendered
      // "Unlocks with Complete" for any week past the current one — an
      // upsell on a surface a child reaches. modes.rs states the Little
      // Speller zero-purchase-surface doctrine and playhub enforces it for
      // TILES; nothing checked copy rendered INSIDE a surface.
      await openSettings(page);
      await flip(page, 'kidToggle', true);
      await page.evaluate(() => document.getElementById('setupDone')?.click());
      await page.waitForTimeout(200);
      await page.evaluate(() => document.getElementById('calOpenBtn')?.click());
      await page.waitForTimeout(600);
      const text = await page.evaluate(() => {
        const el = document.getElementById('calScrim');
        return el ? (el.textContent || '') : '';
      });
      assert(text.length > 0, 'the calendar surface rendered');
      // Match the UPSELL, not the English word. A first version banned
      // "Complete" outright and flagged the goal card "Complete 3 Daily
      // Challenges", where it is a verb — a test that cannot tell an
      // upsell from ordinary copy would force the product to avoid a
      // common word. The upsell is the yb.locked chip: "Unlocks with
      // Complete", rendered as .gd-chip.
      assert(!/Unlocks? with/i.test(text),
        `Spell Jr saw an "Unlocks with" upsell in the calendar: ${text.slice(0, 120)}`);
      const chips = await page.evaluate(() =>
        document.querySelectorAll('#calScrim .gd-chip').length);
      assert(chips === 0, `Spell Jr saw ${chips} lock chip(s) — absence, never locks`);
    } finally { await ctx.close(); }
  });

  await suite.test('spellit_guide_shows_once_and_can_be_replayed', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // AUDITPASS F13. Eric's yellow-circled mic annotation: the flow was
      // not discoverable. ONCE, with a way back (his call) — a returning
      // child should not re-read four steps, but showing it once must not
      // make it unfindable, so the mic surface can replay it.
      const shown = await page.evaluate(() => {
        document.getElementById('sayItBtn')?.click();
        return document.getElementById('spellItGuide')?.classList.contains('show');
      });
      assert(shown, 'the guide appears on first entry');
      const steps = await page.evaluate(() =>
        [...document.querySelectorAll('#spellItGuide .sig-steps li')].map((l) => l.textContent.trim()));
      assert(steps.length === 4, `four steps, got ${steps.length}`);
      assert(/letter/i.test(steps.join(' ')), 'the letters rule is taught, not just implied');

      await page.evaluate(() => document.getElementById('spellItGuideGo')?.click());
      const again = await page.evaluate(() => {
        document.getElementById('sayItBtn')?.click();
        return document.getElementById('spellItGuide')?.classList.contains('show');
      });
      assert(!again, 'a second entry goes straight to play — shown ONCE');

      const replayed = await page.evaluate(() => {
        document.getElementById('spellItGuideAgain')?.click();
        return document.getElementById('spellItGuide')?.classList.contains('show');
      });
      assert(replayed, 'the way back works');
      // ...and replaying must NOT forget: asking for a reminder is not the
      // same as never having read it, and clearing would re-show it unbidden.
      const stillSeen = await page.evaluate(() =>
        localStorage.getItem('spell_spellit_guide_seen'));
      assert(stillSeen === '1', 'replaying does not reset the seen flag');
    } finally { await ctx.close(); }
  });
}
