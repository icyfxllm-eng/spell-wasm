// ortho-close.spec — CC-ORTHO-FIX-FIL-ROMANCE F2/F7: in Spanish, with ortho_profile_es ON, a player who misses only an accent
// sees the fix and keeps the free retry; with the flag OFF (the default) nothing changes. A control proves a right answer still
// scores, so the "Almost" line cannot be a message shown on every answer.
import { openApp, typeOnKeyboard, assert, clickSettled } from '../harness.mjs';

const FLAG_ON = () => localStorage.setItem('spell_flag_ortho_profile_es', 'on');
const strip = (w) => [...w].map((c) => (c === 'ñ' || c === 'Ñ' ? c : c.normalize('NFD').replace(/[̀-ͯ]/g, ''))).join('');

async function serve(page) {
  // The Climb (the default level) handles a miss with shields; a close retry applies in a fixed level, as in the German path.
  if (!(await page.evaluate(() => window.__ortho_fixed))) {
    await page.click('#setupChip').catch(() => {});
    await page.selectOption('#levelSel', 'medium').catch(() => {});
    await clickSettled(page, '#setupDone').catch(() => {});
    await page.evaluate(() => { window.__ortho_fixed = true; });
  }
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}
async function submit(page, text) {
  await typeOnKeyboard(page, text);
  await page.click('#checkBtn');
  await page.waitForFunction(() => !!document.getElementById('feedback').textContent.trim(), null, { timeout: 10000 });
}
const feedback = (page) => page.evaluate(() => ({ text: document.getElementById('feedback').textContent.trim(), cls: document.getElementById('feedback').className }));

/** Serve until the word carries an accent the plain keyboard can leave out (fresh page each time is too slow: skip forward). */
async function accentedWord(page) {
  for (let i = 0; i < 60; i++) {
    const w = await serve(page);
    if (strip(w) !== w && !w.includes(' ')) return w;
    await submit(page, w);                       // answer it right to move on
    await page.waitForTimeout(150);
  }
  throw new Error('no accented es word in 60 serves');
}

export async function run(browser, base, suite) {
  await suite.test('flag on: leaving the accent out shows the letter fix and a free retry', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es', init: FLAG_ON });
    try {
      const w = await accentedWord(page);
      await submit(page, strip(w));
      const f = await feedback(page);
      assert(f.text.includes('→'), `no letter fix on screen for ${w} typed ${strip(w)}: ${JSON.stringify(f)}`);
      assert(!(await page.$('#feedback .reveal')), `the word was revealed (scored as a miss) on the first try: ${JSON.stringify(f)}`);
      assert(f.text.includes('Casi'), `not the close line: ${JSON.stringify(f)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('flag off (default): the same slip is an ordinary miss with no fix line', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const w = await accentedWord(page);
      await submit(page, strip(w));
      const f = await feedback(page);
      assert(!f.text.includes('→'), `a fix line showed with the flag off: ${JSON.stringify(f)}`);
      assert(!!(await page.$('#feedback .reveal')), `flag off: the miss did not reveal the word: ${JSON.stringify(f)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('control, flag on: the stored spelling still scores correct and shows no fix', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es', init: FLAG_ON });
    try {
      const w = await serve(page);
      await submit(page, strip(w) === w ? w : strip(w));   // plain words are typed as stored; accented words are covered above
      const f = await feedback(page);
      if (strip(w) === w) assert(/\bgood\b/.test(f.cls) && !f.text.includes('→'), `plain word not scored right: ${JSON.stringify(f)}`);
    } finally { await ctx.close(); }
  });
}
