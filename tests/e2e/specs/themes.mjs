// themes.spec — CC-VISUAL-ECLIPSE Phase E: F9 themes (AT-21, AT-23) and F10
// accents (AT-24 usage), behind visual.themes and visual.accents.
//
// Themes are token sets on :root[data-theme]; unlocks come from progress that
// already exists (seeded here into the stats and Daily stores) and are kept in
// their own store. AT-21's contrast is checked on the rendered page here (a
// small axe-like pass over the play screen's text) on top of the token gate
// (scripts/check-contrast.mjs, every theme x standard/Jr). AT-22 (grading is
// the same in every theme) is display_diff's all-flags-on regression test;
// AT-25 is themes::tests (ΔE2000) and scripts/flags-check.mjs.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const THEMES_ON = () => { localStorage.setItem('spell_flag_visual_themes', 'on'); localStorage.setItem('spell_flag_visual_accents', 'on'); };
// Progress that earns all three: a 9-day Daily streak, 620 correct, 130 at Hard+.
const QUALIFIED = () => {
  localStorage.setItem('spell_flag_visual_themes', 'on');
  localStorage.setItem('spell_flag_visual_accents', 'on');
  localStorage.setItem('byear_stats_v1', JSON.stringify({ en: { easy: { seen: 300, correct: 290 }, medium: { seen: 220, correct: 200 }, hard: { seen: 100, correct: 90 }, expert: { seen: 50, correct: 40 } } }));
  localStorage.setItem('spell_daily_v1', JSON.stringify({ last_completed: '2026-10-20', streak: 9, best_streak: 9, history: {} }));
};

const rows = (page) => page.evaluate(() => [...document.querySelectorAll('#themePick [data-theme-choice]')].map((r) => ({
  id: r.dataset.themeChoice, locked: r.classList.contains('locked'), text: r.textContent, active: r.classList.contains('active'),
})));
const theme = (page) => page.evaluate(() => document.documentElement.getAttribute('data-theme'));
const choose = (page, id) => page.evaluate((id) => document.querySelector(`#themePick [data-theme-choice="${id}"]`).click(), id);

/** A small axe-like contrast pass over the play screen's text. */
const contrastFailures = (page) => page.evaluate(() => {
  const rgb = (c) => { const m = c.match(/[\d.]+/g); if (!m) return null; const v = m.slice(0, 4).map(Number); if (c.startsWith('color(')) { v[0] *= 255; v[1] *= 255; v[2] *= 255; } return v; };
  const lum = ([r, g, b]) => [r, g, b].map((x) => { x /= 255; return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; }).reduce((a, x, i) => a + x * [0.2126, 0.7152, 0.0722][i], 0);
  // The effective background: translucent layers composited, top down, over
  // the page's own background colour (--bg-color), as axe does.
  const hex = (h) => { h = h.trim().replace('#', ''); return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)); };
  const bgOf = (el) => {
    const layers = [];
    for (let e = el; e && e !== document.body && e !== document.documentElement; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.backgroundImage && cs.backgroundImage !== 'none') return null; // a gradient under the text: skip
      const c = rgb(cs.backgroundColor);
      if (!c) continue;
      const a = c[3] === undefined ? 1 : c[3];
      if (a > 0) layers.push([c[0], c[1], c[2], a]);
      if (a >= 1) break;
    }
    const page = getComputedStyle(document.documentElement).getPropertyValue('--bg-color');
    let out = /^\s*#[0-9a-f]{6}/i.test(page) ? hex(page) : [0, 0, 0];
    for (const [r, g, b, a] of layers.reverse()) out = [out[0] * (1 - a) + r * a, out[1] * (1 - a) + g * a, out[2] * (1 - a) + b * a];
    return out;
  };
  const out = [];
  for (const sel of ['.setup-chip-text', '#gameKeyboard .kb-key', '.placeholder', '.rd-label', '#checkBtn', '.kb-back']) {
    for (const el of document.querySelectorAll(sel)) {
      if (!el.getClientRects().length) continue;
      const fg = rgb(getComputedStyle(el).color), bg = bgOf(el);
      if (!fg || !bg) continue;
      const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x);
      const r = (a + 0.05) / (b + 0.05);
      const cs = getComputedStyle(el);
      const large = parseFloat(cs.fontSize) >= 24 || (parseFloat(cs.fontSize) >= 18.66 && Number(cs.fontWeight) >= 700);
      if (r < (large ? 3 : 4.5)) out.push(`${sel} ${r.toFixed(2)}:1 (${getComputedStyle(el).color} on ${bg.join(',')})`);
      break;
    }
  }
  return out;
});

export async function run(browser, base, suite) {
  await suite.test('F9 off (default): no data-theme, no Appearance rows', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      assert((await theme(page)) === null, 'data-theme set with the flag off');
      assert(await page.evaluate(() => getComputedStyle(document.getElementById('themePick')).display === 'none'), 'the rows show with the flag off');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-23: a new player sees System and five themes; locked ones show how, never a price or a store link', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: THEMES_ON });
    try {
      const r = await rows(page);
      assert(r.map((x) => x.id).join() === 'system,eclipse,daybreak,chalkboard,typewriter,beestage', `rows: ${r.map((x) => x.id)}`);
      assert(r.find((x) => x.id === 'system').active, 'System is not the default');
      const locked = r.filter((x) => x.locked).map((x) => x.id);
      assert(locked.join() === 'chalkboard,typewriter,beestage', `locked: ${locked}`);
      for (const x of r) assert(!/[$€£¥]|price|buy|store|purchase|complete/i.test(x.text), `${x.id} mentions a price or store: ${x.text}`);
      const links = await page.evaluate(() => document.querySelectorAll('#themePick a, #themePick [href]').length);
      assert(links === 0, 'a theme row links somewhere');
      await choose(page, 'typewriter');
      assert((await theme(page)) !== 'typewriter', 'a locked theme could be chosen');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-23: a player who already qualifies gets all three themes the first time, and they stay after a reset', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: QUALIFIED });
    try {
      const r = await rows(page);
      assert(!r.some((x) => x.locked), `still locked: ${r.filter((x) => x.locked).map((x) => x.id)}`);
      await page.evaluate(() => localStorage.removeItem('byear_stats_v1'));
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(!(await rows(page)).some((x) => x.locked), 'a theme was revoked when the stats were wiped');
    } finally { await ctx.close(); }
  });

  await suite.test('F9: System follows the OS; a chosen theme applies its tokens and survives a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: QUALIFIED });
    try {
      await page.emulateMedia({ colorScheme: 'light' });
      await page.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'daybreak', null, { timeout: 3000 });
      await page.emulateMedia({ colorScheme: 'dark' });
      await page.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'eclipse', null, { timeout: 3000 });
      await choose(page, 'chalkboard');
      assert((await theme(page)) === 'chalkboard', 'choosing Chalkboard did not apply it');
      const bg = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--bg').trim());
      assert(bg === '#16291f', `Chalkboard's --bg is ${bg}`);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert((await theme(page)) === 'chalkboard', 'the choice did not survive a reload');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-21: every theme, standard and Jr, keeps the play screen\'s text readable', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: QUALIFIED });
    try {
      // Measure during a word: before one is served the keyboard is dimmed as disabled.
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      const bad = [];
      for (const kid of [false, true]) {
        await page.evaluate((k) => document.body.classList.toggle('kid', k), kid);
        for (const t of ['eclipse', 'daybreak', 'chalkboard', 'typewriter', 'beestage']) {
          await choose(page, t);
          await page.waitForTimeout(350); // sleep-ok: keys ease their background (80 ms); measure the settled theme, not the transition
          const f = await contrastFailures(page);
          if (f.length) bad.push(`${t}${kid ? '-jr' : ''}: ${f.join(', ')}`);
        }
      }
      assert(!bad.length, bad.join('\n  '));
    } finally { await ctx.close(); }
  });

  await suite.test('F9: the Typewriter theme gives every key a keystrike sound; other themes do not', async () => {
    // Init scripts run in the page, so this cannot call QUALIFIED(): it repeats it.
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: () => {
      for (const f of ['visual_themes', 'visual_accents', 'visual_sound']) localStorage.setItem(`spell_flag_${f}`, 'on');
      localStorage.setItem('byear_stats_v1', JSON.stringify({ en: { easy: { seen: 700, correct: 650 }, hard: { seen: 150, correct: 130 } } }));
      localStorage.setItem('spell_daily_v1', JSON.stringify({ last_completed: '2026-10-20', streak: 9, best_streak: 9, history: {} }));
      // A silent browser voice that FINISHES: a keystrike is (rightly) dropped while the voice speaks.
      Object.defineProperty(window, 'speechSynthesis', { configurable: true, get: () => ({
        speak(u) { setTimeout(() => { u.onstart && u.onstart(); setTimeout(() => u.onend && u.onend(), 150); }, 10); }, // sleep-ok: a fake voice's own duration
        cancel() {}, pause() {}, resume() {}, getVoices: () => [], speaking: false, pending: false, paused: false, addEventListener() {}, removeEventListener() {},
      }) });
    } });
    try {
      await page.click('#orbWrap');
      await page.waitForTimeout(700); // sleep-ok: the fake voice's 160 ms must have ended
      const played = () => page.evaluate(() => window.__spelltest.sfxCounts()[0]);
      const a = await played();
      await typeOnKeyboard(page, 'ab');
      assert((await played()) === a, 'a key made a sound outside Typewriter');
      await choose(page, 'typewriter');
      const b = await played();
      await typeOnKeyboard(page, 'cd');
      assert((await played()) - b >= 2, 'no keystrike sound in Typewriter');
    } finally { await ctx.close(); }
  });

  await suite.test('F10: the language chip, progress fill and orb carry the study language\'s accent, and switching language changes it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: THEMES_ON });
    try {
      const read = () => page.evaluate(() => ({
        study: document.documentElement.getAttribute('data-study'),
        accent: getComputedStyle(document.documentElement).getPropertyValue('--lang-accent').trim(),
        dot: getComputedStyle(document.querySelector('.setup-chip-text'), '::before').backgroundColor,
        orb: getComputedStyle(document.querySelector('#orbWrap .orb')).backgroundImage,
      }));
      const en = await read();
      assert(en.study === 'en', `en: ${JSON.stringify(en)}`);
      assert(en.dot !== 'rgba(0, 0, 0, 0)', 'the chip has no accent dot');
      await page.click('#setupChip').catch(() => {});
      await page.selectOption('#langSel', 'es').catch(() => {});
      await page.click('#setupDone').catch(() => {});
      await page.waitForFunction(() => document.documentElement.getAttribute('data-study') === 'es', null, { timeout: 5000 });
      const es = await read();
      assert(es.dot !== en.dot, `the accent did not change with the language (${en.dot} -> ${es.dot})`);
      assert(es.orb !== en.orb, 'the orb tint did not change with the language');
    } finally { await ctx.close(); }
  });
}
