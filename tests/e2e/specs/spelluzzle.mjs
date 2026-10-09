// spelluzzle.spec -- CC-SPELLUZZLE Phase B. English Spell Jr and Easy, in the browser:
// the exit, the runes, decoding, clashes, the layout floor, the finale, resume, audio in hand.
// The suite types through the real keys and reads the answers from the observation-only
// seam (__spelltest.szWords), the way the other specs read currentWord.
import { openApp, assert, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
// One self-contained init script (Playwright serialises it, so it cannot call other functions):
// the flag on, and a record of every audio play so the finale can be counted.
const BOTH = () => {
  localStorage.setItem('spell_flag_spelluzzle', 'on');
  window.__plays = [];
  const orig = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () { try { window.__plays.push(this.src || this.currentSrc || ''); } catch (_) {} return orig.apply(this, arguments); };
};

export async function run(browser, base, suite) {
  const open = async ({ viewport = { width: 390, height: 844 }, age, start = true } = {}) => {
    const o = await openApp(browser, base, { lang: 'en', viewport, init: BOTH, ...(age ? { age } : {}) });
    await o.page.evaluate(() => document.getElementById('szOpenBtn').click());
    await o.page.waitForSelector('#szScreen.show', { timeout: 8000 });
    if (start) {
      await o.page.click('#szStart');
      await o.page.waitForSelector('#szBoard .sz-row', { timeout: 8000 });
      // F13: the one-time explainer shows on the first board. Dismiss it, as a player does.
      if (await o.page.evaluate(() => !document.getElementById('szHow').hidden)) await o.page.click('#szHowOk');
    }
    return o;
  };
  const words = (page) => page.evaluate(() => JSON.parse(window.__spelltest.szWords()));
  const typeWord = async (page, w) => {
    for (const c of w) await page.click(`#szKb [data-k="u:${c}"]`);
  };
  const select = async (page, i) => { await page.click(`#szBoard .sz-row[data-slot="${i}"] .sz-cells`); };

  await suite.test('spelluzzle_exit_is_top_left_and_leaves', async () => {
    const { ctx, page } = await open({ start: false });
    try {
      const r = await page.evaluate(() => { const b = document.getElementById('szExit').getBoundingClientRect(); return b.toJSON(); });
      assert(r.left < 40 && r.top < 120 && r.width >= 44 && r.height >= 44, `exit not a top-left target ${JSON.stringify(r)}`);
      const label = await page.evaluate(() => document.getElementById('szExit').textContent.trim());
      assert(label.length > 1 && !/[✕×x]/i.test(label.slice(0, 1) === 'E' ? '' : label), `exit must be a word, not a glyph: ${label}`);
      await page.click('#szExit');
      assert(!(await page.evaluate(() => document.getElementById('szScreen').classList.contains('show'))), 'exit did not leave');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_first_board_shows_the_explainer_once', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: BOTH });
    const { ctx, page } = o;
    try {
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      assert(await page.evaluate(() => !document.getElementById('szHow').hidden), 'no explainer on the first board');
      await page.click('#szHowOk');
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      assert(await page.evaluate(() => document.getElementById('szHow').hidden), 'the explainer came back');
      await page.click('#szHowBtn');
      assert(await page.evaluate(() => !document.getElementById('szHow').hidden), 'the ? button does not reopen it');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_untouched_board_shows_runes_never_letters', async () => {
    const { ctx, page } = await open();
    try {
      const m = await page.evaluate(() => {
        const cells = [...document.querySelectorAll('#szBoard .sz-cell')];
        return {
          n: cells.length,
          text: cells.map((c) => c.textContent.trim()).join(''),
          labels: cells.map((c) => c.getAttribute('aria-label')),
          rows: document.querySelectorAll('#szBoard .sz-row').length,
        };
      });
      assert(m.rows === 6, `Easy board has ${m.rows} rows`);
      assert(m.n > 0 && m.text === '', `an untouched board shows text: ${JSON.stringify(m.text)}`);
      assert(m.labels.every((l) => /^Rune \d+$/.test(l)), `labels leak or are off: ${JSON.stringify(m.labels.slice(0, 4))}`);
      const ws = await words(page);
      const html = await page.evaluate(() => document.getElementById('szScreen').innerHTML.toLowerCase());
      for (const w of ws) assert(!html.includes(`>${w}<`) && !html.includes(`"${w}"`), `answer ${w} is in the markup`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_typing_a_word_decodes_the_others_and_a_clash_is_amber', async () => {
    const { ctx, page } = await open();
    try {
      const ws = await words(page);
      await select(page, 0);
      await typeWord(page, ws[0]);
      const decoded = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-cell.decoded').length);
      assert(decoded > 0, 'committing a word decoded nothing elsewhere');
      // A second word typed wrong on a shared rune: amber, with no slot marked wrong.
      const shared = await page.evaluate(() => {
        const rows = [...document.querySelectorAll('#szBoard .sz-row')];
        const first = new Set([...rows[0].querySelectorAll('.sz-cell')].map((c) => c.dataset.rune));
        const r1 = [...rows[1].querySelectorAll('.sz-cell')];
        return r1.findIndex((c) => first.has(c.dataset.rune));
      });
      if (shared >= 0) {
        const bad = [...ws[1]];
        bad[shared] = bad[shared] === 'q' ? 'x' : 'q';
        await select(page, 1);
        await typeWord(page, bad.join(''));
        const m = await page.evaluate(() => ({
          amber: document.querySelectorAll('#szBoard .sz-cell.amber').length,
          off: document.querySelectorAll('#szBoard .sz-row.off').length,
          msg: document.getElementById('szMsg').textContent.trim(),
        }));
        assert(m.amber >= 2, `a clash made ${m.amber} amber cells`);
        assert(m.off === 0 && m.msg === '', 'a clash marked a slot or printed a verdict');
      }
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_solves_and_the_secret_is_said_once_at_the_end', async () => {
    const { ctx, page } = await open();
    try {
      const ws = await words(page);
      const secret = ws[ws.length - 1];
      const heard = () => page.evaluate((s) => window.__plays.filter((u) => u.includes('word=' + s)).length, secret);
      const before = await heard();
      for (let i = 0; i < ws.length - 1; i++) { await select(page, i); await typeWord(page, ws[i]); }
      assert((await heard()) === before, 'the secret word was played before the board was solved');
      await select(page, ws.length - 1);
      assert((await heard()) === before, 'tapping the secret slot played it');
      await typeWord(page, secret);
      await domSettled(page);
      const m = await page.evaluate(() => ({ stars: document.querySelectorAll('#szResult .sz-star').length, on: document.querySelectorAll('#szResult .sz-star.on').length }));
      assert(m.stars === 3 && m.on >= 1, `result ${JSON.stringify(m)}`);
      assert((await heard()) === before + 1, `the finale played the secret ${await heard() - before} times`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_layout_floor_at_375x667', async () => {
    const { ctx, page } = await open({ viewport: { width: 375, height: 667 } });
    try {
      await select(page, 0);
      const m = await page.evaluate(() => {
        const H = innerHeight;
        const rows = [...document.querySelectorAll('#szBoard .sz-row')];
        const cells = [...document.querySelectorAll('#szBoard .sz-cell')];
        const minW = Math.min(...cells.map((c) => c.getBoundingClientRect().width));
        const board = document.getElementById('szBoard');
        const kb = document.getElementById('szKb').getBoundingClientRect();
        return { rows: rows.length, minW, scrolls: board.scrollHeight - board.clientHeight, kbTop: kb.top, kbBottom: kb.bottom, H, over: document.documentElement.scrollWidth - innerWidth };
      });
      assert(m.minW >= 32, `cells shrank to ${m.minW}px`);
      assert(m.scrolls <= 1, `the board scrolls by ${m.scrolls}px while composing`);
      assert(m.kbBottom <= m.H + 0.5 && m.over <= 0, `keyboard ${JSON.stringify(m)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_no_red_anywhere', async () => {
    const { ctx, page } = await open({ age: KID });
    try {
      const bad = await page.evaluate(() => {
        const out = [];
        const redish = (c) => { const m = c.match(/rgba?\((\d+), (\d+), (\d+)/); if (!m) return false; const [r, g, b] = [+m[1], +m[2], +m[3]]; return r > 170 && g < 90 && b < 90; };
        for (const el of document.querySelectorAll('#szScreen *')) {
          const s = getComputedStyle(el);
          for (const p of ['color', 'backgroundColor', 'borderTopColor', 'outlineColor', 'boxShadow']) if (redish(s[p] || '')) out.push(el.className + ':' + p);
        }
        return out;
      });
      assert(bad.length === 0, `red found: ${bad.slice(0, 5).join(', ')}`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_jr_gets_jr_boards_only', async () => {
    const { ctx, page } = await open({ age: KID, start: false });
    try {
      const picker = await page.evaluate(() => document.querySelectorAll('#szTiers [data-tier]').length);
      assert(picker === 0, 'a Jr profile saw a tier picker');
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row', { timeout: 8000 });
      const rows = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-row').length);
      assert(rows === 5, `a Spell Jr board has ${rows} rows`);
      const silent = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-row[data-kind="silent"]').length);
      assert(silent === 0, 'Spell Jr has a silent slot');
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_leaving_saves_and_playing_again_resumes', async () => {
    const { ctx, page } = await open();
    try {
      const ws = await words(page);
      await select(page, 0);
      await typeWord(page, ws[0]);
      await page.click('#szExit');
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show');
      await page.click('#szStart');
      await page.waitForSelector('#szBoard .sz-row');
      const again = await words(page);
      assert(JSON.stringify(again) === JSON.stringify(ws), 'resume drew a different board');
      const typed = await page.evaluate(() => document.querySelectorAll('#szBoard .sz-row[data-slot="0"] .sz-cell.typed').length);
      assert(typed === ws[0].length, `the committed word was not put back (${typed})`);
    } finally { await ctx.close(); }
  });

  await suite.test('spelluzzle_a_clip_that_will_not_load_replaces_the_board_then_says_unavailable', async () => {
    const o = await openApp(browser, base, { lang: 'en', viewport: { width: 390, height: 844 }, init: BOTH });
    const { ctx, page } = o;
    try {
      await page.route('**/api/speak**', (r) => r.fulfill({ status: 404, body: '' }));
      await page.evaluate(() => document.getElementById('szOpenBtn').click());
      await page.waitForSelector('#szScreen.show');
      await page.click('#szStart');
      await page.waitForFunction(() => document.getElementById('szMsg').textContent.trim().length > 0, null, { timeout: 8000 });
      const m = await page.evaluate(() => ({ rows: document.querySelectorAll('#szBoard .sz-row').length, msg: document.getElementById('szMsg').textContent.trim() }));
      assert(m.rows === 0, `a board showed with its audio failing (${m.rows} rows)`);
      assert(/audio/i.test(m.msg), `no unavailable state: ${m.msg}`);
    } finally { await ctx.close(); }
  });
}
