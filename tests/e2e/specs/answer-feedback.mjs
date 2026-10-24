// answer-feedback.spec — CC-VISUAL-ECLIPSE Phase B F4 (AT-13, AT-14, and the
// on-screen half of AT-11/AT-12), behind visual.feedback.
//
// Off: today's reveal, byte for byte — no rows, coral, the orb shakes.
// On: a miss shows "You wrote" / "Correct" with the letters marked, in caution
// amber, and the orb dims instead of shaking; a correct answer lifts the
// letters in a stagger that ends inside 300 ms. Neither row ever holds more
// than the attempt and the spelling the base game already reveals (I6).
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const FLAG_ON = () => localStorage.setItem('spell_flag_visual_feedback', 'on');

async function serve(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}

async function answer(page, text) {
  await typeOnKeyboard(page, text);
  await page.click('#checkBtn');
  await page.waitForFunction(() => /\b(good|bad)\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
}

const colourOf = (page, sel) => page.$eval(sel, (e) => getComputedStyle(e).color);
const cssVar = (page, name) => page.evaluate((n) => {
  const probe = document.createElement('span');
  probe.style.color = `var(${n})`;
  document.body.appendChild(probe);
  const c = getComputedStyle(probe).color;
  probe.remove();
  return c;
}, name);

export async function run(browser, base, suite) {
  await suite.test('F4 off (default): the reveal is unchanged — no rows, coral, the orb shakes', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const word = await serve(page);
      await answer(page, word + 'q');
      assert(!(await page.$('#feedback .reveal-diff')), 'rows appeared with the flag off');
      assert(!(await page.evaluate(() => document.body.classList.contains('visual-feedback'))), 'the body class is set with the flag off');
      assert((await colourOf(page, '#feedback')) === (await cssVar(page, '--coral')), 'the off path changed colour');
      const anim = await page.$eval('#orbWrap .orb', (e) => getComputedStyle(e).animationName);
      assert(anim === 'shake', `the off path orb plays ${anim}`);
    } finally { await ctx.close(); }
  });

  await suite.test('F4 on: a miss shows You wrote / Correct, marked, in amber, and the orb dims', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const word = await serve(page);
      const typed = word + 'q';
      await answer(page, typed);
      const r = await page.evaluate(() => {
        const rows = [...document.querySelectorAll('#feedback .rd-row')];
        const caption = document.querySelector('#letters .rd-caption')?.textContent;
        const text = (sel) => document.querySelector(sel)?.textContent;
        return {
          rows: rows.length,
          you: text('#letters .reveal-attempt'),
          ok: text('#feedback .rd-ok .reveal'),
          extra: [...document.querySelectorAll('#letters .reveal-attempt s.rd-extra')].map((e) => e.textContent),
          labels: [caption, ...rows.map((r) => r.querySelector('.rd-label').textContent)],
          orb: getComputedStyle(document.querySelector('#orbWrap .orb')).animationName,
          wordColour: getComputedStyle(document.querySelector('#letters .reveal-attempt')).color,
        };
      });
      assert(r.rows === 1, `${r.rows} rows on the result line (the attempt lives in the answer box)`);
      assert(r.labels[0] === 'You wrote' && r.labels[1] === 'Correct', `labels ${r.labels}`);
      assert(r.you === typed, `the attempt row reads "${r.you}", typed "${typed}"`);
      assert(r.ok.toLowerCase() === word.toLowerCase(), `the correct row reads "${r.ok}", the word is "${word}"`);
      assert(r.extra.join('') === 'q', `the extra letter is not struck: ${r.extra}`);
      assert(r.orb === 'orbDim', `the orb plays ${r.orb}, not the dim`);
      const warn = await cssVar(page, '--warn');
      assert((await colourOf(page, '#feedback')) === warn, 'the miss is not caution amber');
      assert(r.wordColour === warn, 'the attempt row is not caution amber');
      const coral = await cssVar(page, '--coral');
      const red = await page.evaluate((c) => [...document.querySelectorAll('#feedback, #feedback *, #orbWrap .orb')].filter((e) => getComputedStyle(e).color === c).length, coral);
      assert(red === 0, `${red} elements still coral`);
    } finally { await ctx.close(); }
  });

  await suite.test('F4 on: a correct answer lifts the letters, 25 ms apart, all inside 300 ms', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const word = await serve(page);
      await answer(page, word);
      const r = await page.evaluate(() => {
        const ms = (v) => (v.endsWith('ms') ? parseFloat(v) : parseFloat(v) * 1000);
        const ls = [...document.querySelectorAll('#letters .ltr')];
        return {
          n: ls.length,
          lifted: ls.filter((e) => e.classList.contains('lift')).length,
          delays: ls.map((e) => ms(getComputedStyle(e).animationDelay)),
          dur: ls.length ? ms(getComputedStyle(ls[0]).animationDuration) : 0,
          cls: document.getElementById('feedback').className,
        };
      });
      assert(/good/.test(r.cls), `the answer did not grade correct (${r.cls})`);
      assert(r.n > 0 && r.lifted === r.n, `${r.lifted} of ${r.n} letters lifted`);
      if (r.n > 1) assert(Math.abs(r.delays[1] - r.delays[0] - Math.min(25, 120 / (r.n - 1))) < 0.5, `stagger ${r.delays[1] - r.delays[0]}ms for ${r.n} letters`);
      const total = Math.max(...r.delays) + r.dur;
      assert(total <= 300.5, `the wave takes ${total}ms`);
    } finally { await ctx.close(); }
  });

  await suite.test('F4 on, Reduce Motion: no letter moves, and the orb only dims', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      const word = await serve(page);
      await answer(page, word);
      const lift = await page.$$eval('#letters .ltr.lift', (ls) => ls.map((e) => getComputedStyle(e).animationName));
      assert(lift.length && lift.every((a) => a === 'none'), `lift under Reduce Motion: ${lift}`);
    } finally { await ctx.close(); }
    const second = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const page = second.page;
      await page.emulateMedia({ reducedMotion: 'reduce' });
      const word = await serve(page);
      await answer(page, word + 'q');
      const orb = await page.$eval('#orbWrap .orb', (e) => getComputedStyle(e).animationName);
      assert(orb === 'orbDim', `the orb plays ${orb} under Reduce Motion, not the dim`);
    } finally { await second.ctx.close(); }
  });

  await suite.test('AT-14: the play screen draws no letter slot before input, flag on or off', async () => {
    for (const init of [null, FLAG_ON]) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', init });
      try {
        await serve(page);
        const r = await page.evaluate(() => ({
          ltrs: document.querySelectorAll('#letters .ltr').length,
          slots: document.querySelectorAll('#letters .slot, #letters [data-slot]').length,
          placeholder: !!document.querySelector('#letters .placeholder'),
        }));
        assert(r.ltrs === 0 && r.slots === 0 && r.placeholder, `before input: ${JSON.stringify(r)} (flag ${init ? 'on' : 'off'})`);
      } finally { await ctx.close(); }
    }
  });

  await suite.test('AT-13: before an answer, nothing of the spelling is on screen, flag on', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const word = await serve(page);
      await typeOnKeyboard(page, word.slice(0, 1));
      const r = await page.evaluate(() => ({
        fb: document.getElementById('feedback').textContent,
        rows: document.querySelectorAll('.reveal-diff, .reveal').length,
      }));
      assert(r.rows === 0, 'a reveal is on screen before the answer');
      assert(!r.fb.toLowerCase().includes(word.toLowerCase()), 'the word is in the feedback line before the answer');
    } finally { await ctx.close(); }
  });

  await suite.test('E-Q7: DefMatch, Spelldoku, Spell Cross and Impostor mark a wrong answer amber, not red, flag on', async () => {
    for (const [init, want] of [[FLAG_ON, 'amber'], [null, 'red']]) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', init });
      try {
        const r = await page.evaluate(() => {
          const probe = (cls, prop) => {
            const host = document.createElement('div');
            host.innerHTML = `<div class="${cls}">x</div>`;
            document.body.appendChild(host);
            const v = getComputedStyle(host.firstChild)[prop];
            host.remove();
            return v;
          };
          // color-mix() computes to color(srgb r g b / a) on a 0-1 scale.
          const rgb = (v) => (v.match(/[\d.]+/g) || []).slice(0, 3).map(Number).map((n) => (v.startsWith('color(') ? n * 255 : n));
          const warmth = (v) => { const [r, g, b] = rgb(v); return { r, g, b }; };
          return {
            dm: warmth(probe('dm-card wrong', 'backgroundColor')),
            sd: warmth(probe('sd-cell wrong', 'color')),
            xw: warmth(probe('xw-cell wrong', 'color')),
            imp: warmth(probe('imp-card wrong', 'backgroundColor')),
          };
        });
        // Amber has green well above blue and a strong red; the shipped reds
        // (coral, flag red, the maroon card) have green close to blue.
        for (const [k, c] of Object.entries(r)) {
          const amber = c.g - c.b > 25 && c.r > c.g;
          assert(amber === (want === 'amber'), `${k} is ${JSON.stringify(c)}, expected ${want}`);
        }
      } finally { await ctx.close(); }
    }
  });
}
