// progress-report.spec — CC-REPORTS-SHARE v1 F5–F7, through CC-HUB-NAV AE-1.
//
// What unit tests cannot see: that the drawer row really is absent for a
// child, for an unanswered age gate and without Complete; that the
// #progress-report deep link is refused in exactly those cases and lands on
// the parent gate otherwise; and that a real learner store, written in the
// CR-1 shape, reaches the report text through the real LearnerQuery.
import { openApp, assert, clickSettled, domSettled } from '../harness.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

/** One CR-1 attempt exactly a week ago: always inside last Mon–Sun week. */
function learnerStore() {
  const ts = Math.floor(Date.now() / 1000) - 7 * 86400;
  const attempt = (word, outcome, correct) => ({
    day: Math.floor(ts / 86400), word, skills: [], correct, channel: 'typed',
    typed: correct ? null : 'recieve', ts, outcome, mode: 'standard',
  });
  return JSON.stringify({
    version: 2, lang: 'en', skills: [], placed: true,
    log: [attempt('receive', 'misspelled', false), attempt('rabbit', 'correct', true)],
  });
}

/** Init script: optional Complete (the dev grant), optional seeded learner. */
const init = ({ complete, learner }) => [
  complete ? "localStorage.setItem('spell_dev_entitlements', 'on');" : '',
  learner ? `localStorage.setItem('spell_learner_en', ${JSON.stringify(learnerStore())});` : '',
].join('\n');

/** Present as iOS. The row is app-only, so without this every "absent" below
 * would be absent for the platform and prove nothing. Applied after load: the
 * app's bridge script replaces window.SpellNativeLang at boot. The drawer and
 * the router both read the platform when they run, so this is in time. */
const asNative = (page) => page.evaluate(() => {
  window.SpellNativeLang = window.SpellNativeLang || {};
  window.SpellNativeLang.nativePlatform = () => true;
});

async function drawerHas(page, id) {
  await clickSettled(page, '#navBurger');
  const ids = await page.$$eval('#navDrawerPanel .nav-row', (els) => els.map((e) => e.dataset.mode));
  await page.keyboard.press('Escape').catch(() => {});
  return ids.includes(id);
}

const shown = (page, id) => page.evaluate((i) => document.getElementById(i).classList.contains('show'), id);

async function deepLink(page) {
  await page.evaluate(() => { window.location.hash = '#progress-report'; });
  await domSettled(page);
}

async function passGate(page) {
  const q = await page.evaluate(() => document.getElementById('parentQ').textContent);
  const n = { three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8 };
  const [a, b] = (q.toLowerCase().match(/three|four|five|six|seven|eight/g) || []).map((w) => n[w]);
  assert(a && b, `could not read the gate question: ${q}`);
  await page.fill('#parentAnswer', String(a * b));
  await page.evaluate(() => document.getElementById('parentSubmit').click());
  await domSettled(page);
}

export async function run(browser, base, suite) {
  await suite.test('progress report: a grown-up with Complete gets the row, the gate first, then the week', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true }) });
    try {
      await asNative(page);
      assert(await drawerHas(page, 'progress_report'), 'the row must be in Your Words for a known adult with Complete');
      await page.evaluate(() => document.getElementById('rshareOpenBtn').click());
      await domSettled(page);
      assert(await shown(page, 'parentScrim'), 'the parent gate must open before anything renders');
      assert(!(await shown(page, 'rshareScrim')), 'the report rendered before the gate was passed');
      await passGate(page);
      assert(await shown(page, 'rshareScrim'), 'passing the gate must open the report');
      const text = await page.evaluate(() => document.getElementById('rshareBody').textContent);
      assert(text.startsWith('Spelling progress'), `unexpected report head: ${text.slice(0, 80)}`);
      assert(text.includes('Working on (1): receive'), `the seeded miss did not reach the report: ${text}`);
      assert(text.includes('New this week (1): rabbit'), `the seeded new word did not reach the report: ${text}`);
      // This week so far: the seeded attempts are last week's, so it is the short report.
      await page.evaluate(() => document.getElementById('rshareNow').click());
      await domSettled(page);
      const now = await page.evaluate(() => document.getElementById('rshareBody').textContent);
      assert(now.includes('This week so far') && now.includes('No practice this week yet.'), `partial week wrong: ${now}`);
    } finally { await ctx.close(); }
  });

  await suite.test('progress report: the deep link lands on the parent gate, never on the report', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: false }) });
    try {
      await asNative(page);
      await deepLink(page);
      assert(await shown(page, 'parentScrim'), '#progress-report must open the parent gate');
      assert(!(await shown(page, 'rshareScrim')), 'the deep link skipped the gate');
      await passGate(page);
      assert(await shown(page, 'rshareScrim'), 'the gate must lead to the report');
    } finally { await ctx.close(); }
  });

  for (const [who, age, complete] of [
    ['Spell Jr', AGE_KID, true],
    ['an unanswered age gate', null, true],
    ['no Complete', undefined, false],
  ]) {
    await suite.test(`progress report: absent, and the deep link refused, for ${who}`, async () => {
      const opts = { lang: 'en', init: init({ complete, learner: true }) };
      if (age !== undefined) opts.age = age;
      const { ctx, page } = await openApp(browser, base, opts);
      try {
        await asNative(page);
        // The control: the same page DOES list the other iOS-only Your Words
        // row, so an absent progress_report is the rule, not the platform.
        if (age !== null) {
          assert(await drawerHas(page, 'calendar'), `the iOS stub did not take for ${who}`);
          assert(!(await drawerHas(page, 'progress_report')), `the row showed for ${who}`);
        }
        await deepLink(page);
        assert(!(await shown(page, 'parentScrim')), `the deep link opened the gate for ${who}`);
        assert(!(await shown(page, 'rshareScrim')), `the deep link opened the report for ${who}`);
        assert((await page.evaluate(() => window.location.hash)) === '', 'a refused link is consumed, not left in the URL');
      } finally { await ctx.close(); }
    });
  }
}
