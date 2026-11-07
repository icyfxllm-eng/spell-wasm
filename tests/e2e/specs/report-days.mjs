// report-days.spec — CC-GUARDIAN-REPORT v1.1 Phase B (flag guardian_report_days).
//
// What unit tests cannot see: that the flag really gates the controls, that
// Save is disabled while the weekday set is empty (A12, the UI half), that a
// saved schedule lands in storage as an edit, and that the on-demand "Last N
// days" range leaves the stored schedule byte for byte alone (A10).
import { openApp, assert, domSettled } from '../harness.mjs';

const KEY = 'spell_report_days_v1';
const shown = (page, id) => page.evaluate((i) => document.getElementById(i).classList.contains('show'), id);
const visible = (page, id) => page.evaluate((i) => !document.getElementById(i).classList.contains('btn-hide'), id);
const stored = (page) => page.evaluate((k) => localStorage.getItem(k), KEY);

async function passGate(page) {
  const q = await page.evaluate(() => document.getElementById('parentQ').textContent);
  const n = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
  const [a, b] = (q.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => n[w]);
  assert(a && b, `could not read the gate question: ${q}`);
  await page.fill('#parentAnswer', String(a * b));
  await page.evaluate(() => document.getElementById('parentSubmit').click());
  await domSettled(page);
}

async function openReport(page) {
  await page.evaluate(() => document.getElementById('rshareOpenBtn').click());
  await domSettled(page);
  await passGate(page);
  assert(await shown(page, 'rshareScrim'), 'the report did not open after the gate');
}

const click = async (page, id) => {
  await page.evaluate((i) => document.getElementById(i).click(), id);
  await domSettled(page);
};

export async function run(browser, base, suite) {
  await suite.test('report days: flag off, the controls are absent and nothing is stored', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: "localStorage.setItem('spell_dev_entitlements', 'on');" });
    try {
      await openReport(page);
      assert(!(await visible(page, 'rdaysBox')), 'Report days must be absent with the flag off');
      assert(!(await visible(page, 'rdaysLastN')), 'the Last N days tab must be absent with the flag off');
      assert((await stored(page)) === null, 'flag off must write no schedule');
    } finally { await ctx.close(); }
  });

  await suite.test('report days: Save is disabled on an empty set; a saved schedule is one edit; Last N days changes nothing (A10, A12)', async () => {
    const init = "localStorage.setItem('spell_dev_entitlements', 'on'); localStorage.setItem('spell_flag_guardian_report_days', 'on');";
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init });
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    try {
      await openReport(page);
      assert(await visible(page, 'rdaysBox'), 'Report days must show with the flag on');
      assert(await page.evaluate(() => document.getElementById('rdaysWeekly').getAttribute('aria-pressed')) === 'true', 'the default is Weekly');
      // Custom opens the chips; clear the one set day (Mon) and Save must disable.
      await click(page, 'rdaysCustom');
      assert(await visible(page, 'rdaysChips'), 'Custom must show the weekday chips');
      await click(page, 'rdaysD0');
      assert(await page.evaluate(() => document.getElementById('rdaysSave').disabled), 'Save must be disabled while no weekday is chosen');
      const untouched = await stored(page);
      await click(page, 'rdaysSave');
      assert((await stored(page)) === untouched, 'a disabled Save must not write');
      // Twice a week -> Save: one edit stored, Thu and Mon.
      await click(page, 'rdaysTwice');
      assert(!(await page.evaluate(() => document.getElementById('rdaysSave').disabled)), 'a chosen set enables Save');
      await click(page, 'rdaysSave');
      const saved = await stored(page);
      assert(saved && saved.startsWith('v1;') && saved.split(';')[2].split(',').length === 2, `one edit expected, got ${saved}`);
      assert(saved.split(';')[2].split(',')[1].startsWith('9@'), `Mon+Thu is bits 9, got ${saved}`);
      // The on-demand range: the stored schedule is byte for byte unchanged.
      const before = await stored(page);
      await click(page, 'rdaysLastN');
      await page.evaluate(() => { const r = document.getElementById('rdaysN'); r.value = '3'; r.dispatchEvent(new Event('input', { bubbles: true })); });
      await domSettled(page);
      const label = await page.evaluate(() => document.getElementById('rdaysLastN').textContent);
      assert(label === 'Last 3 days', `range label: ${label}`);
      assert((await stored(page)) === before, 'the on-demand range must not touch the stored schedule');
      assert(errors.length === 0, `page errors: ${errors.join(' | ')}`);
    } finally { await ctx.close(); }
  });
}
