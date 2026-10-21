// frame-log.spec — CC-VISUAL-ECLIPSE A5: the AT-19 frame-time logger works end
// to end, and only from the dev door.
//
// Five quick taps on the logo open the dev menu; the logger switch starts a
// requestAnimationFrame sampler; typing and a checked answer are marked as
// moments; the report lists them with frame counts and a verdict. Off by
// default: with the switch untouched, the report holds no frames at all.
// These numbers come from headless Chromium and mean nothing for AT-19 itself,
// which is measured on the iPhone SE (2nd gen); this proves the instrument.
import { openApp, assert } from '../harness.mjs';

const openDoor = (page) => page.evaluate(() => {
  const logo = document.getElementById('brandMark');
  for (let i = 0; i < 5; i++) logo.click();
});
const report = (page) => page.evaluate(() => {
  document.getElementById('devFrameReport').click();
  return document.getElementById('devFrameReportText').textContent;
});

export async function run(browser, base, suite) {
  await suite.test('A5: the frame logger is off by default and records nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      assert(await page.evaluate(() => document.getElementById('devFrameReportBox').hidden), 'the report panel shows before anyone asked for it');
      await openDoor(page);
      await page.waitForSelector('#devMenu.show', { timeout: 4000 });
      assert(!(await page.evaluate(() => document.getElementById('devFrameLog').checked)), 'the logger switch starts on');
      const r = await report(page);
      assert(!/\n(idle|typing|result|speaking)\s/.test(r), `the logger recorded frames while off:\n${r}`);
    } finally { await ctx.close(); }
  });

  await suite.test('A5: switched on in the dev door, it records typing and the result, and survives a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDoor(page);
      await page.waitForSelector('#devMenu.show', { timeout: 4000 });
      await page.evaluate(() => { const c = document.getElementById('devFrameLog'); c.checked = true; c.dispatchEvent(new Event('change')); });
      await page.evaluate(() => document.getElementById('devMenuClose').click());
      await page.evaluate(() => document.querySelector('#orbWrap').click());
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      for (const k of ['q', 'z']) await page.evaluate((k) => document.querySelector(`#gameKeyboard .kb-key[data-k="${k}"]`).click(), k);
      await page.waitForTimeout(300); // sleep-ok: frames have to elapse inside the typing window to be counted
      await page.evaluate(() => document.querySelector('#checkBtn').click());
      await page.waitForTimeout(400); // sleep-ok: frames inside the result window
      await openDoor(page);
      const r = await report(page);
      const row = (name) => (r.match(new RegExp(`\\n${name}\\s+(\\d+)`)) || [])[1];
      assert(Number(row('typing')) > 0, `no typing frames in the report:\n${r}`);
      assert(Number(row('result')) > 0, `no result frames in the report:\n${r}`);
      assert(/AT-19/.test(r) && /(PASS|FAIL)/.test(r), `the report carries no verdict:\n${r}`);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(await page.evaluate(() => document.getElementById('devFrameLog').checked), 'the logger switch did not survive a reload');
    } finally { await ctx.close(); }
  });
}
