// progress-report.spec — CC-REPORTS-SHARE v1 F5–F7, through CC-HUB-NAV AE-1,
// extended by CC-GUARDIAN-REPORT v1 F4–F7 (the Guardian Report).
//
// What unit tests cannot see: that the drawer row really is absent for a
// child, for an unanswered age gate and without Complete; that the
// #progress-report deep link is refused in exactly those cases and lands on
// the parent gate otherwise; and that a real learner store, written in the
// CR-1 shape, reaches the report text through the real LearnerQuery.
import { openApp, assert, clickSettled, domSettled } from '../harness.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

/** CR-1 attempts `daysAgo` days back (7: always inside last Mon–Sun week;
 * 0: this week). "because" and "rabbit" are English Hard bank rows. */
function learnerStore(daysAgo = 7, words = [['because', 'misspelled', false], ['rabbit', 'correct', true]]) {
  const ts = Math.floor(Date.now() / 1000) - daysAgo * 86400;
  const attempt = (word, outcome, correct) => ({
    day: Math.floor(ts / 86400), word, skills: [], correct, channel: 'typed',
    typed: correct ? null : 'becuase', ts, outcome, mode: 'standard',
  });
  return JSON.stringify({
    version: 2, lang: 'en', skills: [], placed: true,
    log: words.map((w) => attempt(...w)),
  });
}

/** Init script: optional Complete (the dev grant), optional seeded learner. */
const init = ({ complete, learner, daysAgo = 7, words }) => [
  complete ? "localStorage.setItem('spell_dev_entitlements', 'on');" : '',
  learner ? `localStorage.setItem('spell_learner_en', ${JSON.stringify(learnerStore(daysAgo, words))});` : '',
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

const LISTS = 'byear_word_lists_v1';
const listsRaw = (page) => page.evaluate((k) => localStorage.getItem(k), LISTS);

/** The local Monday (YYYY-MM-DD) of the week holding now minus `daysAgo`. */
const mondayOf = (page, daysAgo) => page.evaluate((d) => {
  const t = new Date(Date.now() - d * 86400000);
  t.setDate(t.getDate() - ((t.getDay() + 6) % 7));
  return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')}`;
}, daysAgo);

/** A recording stand-in for @capacitor/local-notifications, installed BEFORE
 * boot so the app registers its action listener on it. It keeps the pending
 * set the OS would (schedule adds by id, cancel removes), so I11 is checkable.
 * `display` is what the permission prompt answers. */
const fakeNotifications = (display) => `
  window.Capacitor = window.Capacitor || {};
  window.Capacitor.Plugins = window.Capacitor.Plugins || {};
  window.__notif = { requested: 0, checked: 0, scheduled: [], pending: {}, maxPending: 0, listeners: {} };
  window.Capacitor.Plugins.LocalNotifications = {
    requestPermissions: async () => { window.__notif.requested++; return { display: ${JSON.stringify(display)} }; },
    checkPermissions: async () => { window.__notif.checked++; return { display: ${JSON.stringify(display)} }; },
    schedule: async (a) => {
      const n = window.__notif;
      for (const x of a.notifications) { n.pending[x.id] = JSON.parse(JSON.stringify(x)); n.scheduled.push(n.pending[x.id]); }
      n.maxPending = Math.max(n.maxPending, Object.keys(n.pending).length);
    },
    cancel: async (a) => { for (const x of a.notifications) delete window.__notif.pending[x.id]; },
    registerActionTypes: async () => {},
    addListener: async (name, fn) => { window.__notif.listeners[name] = fn; },
  };`;

const notif = (page) => page.evaluate(() => window.__notif);

async function openReport(page) {
  await page.evaluate(() => document.getElementById('rshareOpenBtn').click());
  await domSettled(page);
  await passGate(page);
  assert(await shown(page, 'rshareScrim'), 'the report did not open after the gate');
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
      assert(text.startsWith('Guardian Report'), `unexpected report head: ${text.slice(0, 80)}`);
      assert(text.includes('Working on (1): because'), `the seeded miss did not reach the report: ${text}`);
      assert(text.includes('Practice list (1): because'), `the missed bank row is the practice list (no auditor, so no top-up): ${text}`);
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

  await suite.test('guardian report: Add makes one dated list per week, and a second Add writes nothing (A17, A19)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true }) });
    try {
      await asNative(page);
      const monday = await mondayOf(page, 7);
      await openReport(page);
      await page.evaluate(() => document.getElementById('rshareAdd').click());
      await domSettled(page);
      assert(await shown(page, 'listsScreen'), 'Add must open the list it made');
      const note = await page.evaluate(() => document.getElementById('listsNote').textContent);
      assert(note === 'Added to My Words', `first Add said: ${note}`);
      const first = JSON.parse(await listsRaw(page));
      const keyed = first.lists.filter((l) => l.key === `guardian:0:${monday}:en` && !l.deletedAt);
      assert(keyed.length === 1, `want one keyed list, got ${keyed.length}`);
      assert(keyed[0].name.startsWith('Week of '), `list name: ${keyed[0].name}`);
      const entries = keyed[0].entries.map((e) => `${e.text}|${e.lang}`);
      assert(JSON.stringify(entries) === JSON.stringify(['because|en']), `entries: ${entries}`);
      assert(keyed[0].entries[0].text === 'because'.normalize('NFC'), 'the word must be stored NFC');

      // Again, through the gate again: nothing is written, the same list opens.
      await page.evaluate(() => document.getElementById('listsClose').click());
      await domSettled(page);
      await openReport(page);
      await page.evaluate(() => document.getElementById('rshareAdd').click());
      await domSettled(page);
      const again = await page.evaluate(() => document.getElementById('listsNote').textContent);
      assert(again === 'Already in My Words', `second Add said: ${again}`);
      assert(JSON.stringify(JSON.parse(await listsRaw(page))) === JSON.stringify(first), 'a second Add changed My Words');
    } finally { await ctx.close(); }
  });

  await suite.test('guardian report: a week of only own-list words still has a practice list, and Add saves it', async () => {
    // "zorvenly" is in no bank: a word from the player's own My Words list.
    const words = [['zorvenly', 'misspelled', false], ['zorvenly', 'misspelled', false]];
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true, words }) });
    try {
      await asNative(page);
      const monday = await mondayOf(page, 7);
      await openReport(page);
      const text = await page.evaluate(() => document.getElementById('rshareBody').textContent);
      assert(text.includes('Practice list (1): zorvenly'), `an own-list miss must reach the practice list: ${text}`);
      await page.evaluate(() => document.getElementById('rshareAdd').click());
      await domSettled(page);
      assert(await shown(page, 'listsScreen'), 'Add must open the list it made');
      const keyed = JSON.parse(await listsRaw(page)).lists.filter((l) => l.key === `guardian:0:${monday}:en` && !l.deletedAt);
      const entries = keyed.flatMap((l) => l.entries.map((e) => `${e.text}|${e.lang}`));
      assert(JSON.stringify(entries) === JSON.stringify(['zorvenly|en']), `entries: ${entries}`);
    } finally { await ctx.close(); }
  });

  await suite.test('guardian report: a dismissed gate writes nothing, and Add without the gate is inert (A18, I8)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true }) });
    try {
      await asNative(page);
      const before = await listsRaw(page);
      const monday = await mondayOf(page, 7);
      await page.evaluate((m) => { window.location.hash = `#guardian-report?week=${m}`; }, monday);
      await domSettled(page);
      assert(await shown(page, 'parentScrim'), 'the week link must open the parent gate');
      await page.evaluate(() => document.getElementById('parentCancel').click());
      await domSettled(page);
      assert(!(await shown(page, 'rshareScrim')), 'the report opened without the gate');
      await page.evaluate(() => document.getElementById('rshareAdd').click());
      await domSettled(page);
      assert((await listsRaw(page)) === before, 'My Words changed without a passed gate');
    } finally { await ctx.close(); }
  });

  await suite.test('guardian report: the week link opens the week it names', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true }) });
    try {
      await asNative(page);
      for (const [daysAgo, hasWord] of [[7, true], [14, false]]) {
        const monday = await mondayOf(page, daysAgo);
        await page.evaluate((m) => { window.location.hash = `#guardian-report?week=${m}`; }, monday);
        await domSettled(page);
        await passGate(page);
        const text = await page.evaluate(() => document.getElementById('rshareBody').textContent);
        assert(text.includes('because') === hasWord, `week of ${monday}: ${text}`);
        await page.evaluate(() => document.getElementById('rshareDone').click());
        await domSettled(page);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('guardian ping: no prompt before the toggle; a refusal reads off with one line (A21)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true, daysAgo: 0 }) + fakeNotifications('denied') });
    try {
      await asNative(page);
      await openReport(page);
      assert((await notif(page)).requested === 0, 'notification permission was requested before the toggle');
      assert(await page.evaluate(() => !document.getElementById('rsharePing').classList.contains('btn-hide')), 'the toggle must show where notifications exist');
      await page.evaluate(() => document.getElementById('rsharePing').click());
      await domSettled(page);
      const n = await notif(page);
      assert(n.requested === 1, `turning the toggle on must ask once, asked ${n.requested}`);
      assert(Object.keys(n.pending).length === 0, 'a refused permission left a notification pending');
      assert(await page.evaluate(() => !document.getElementById('rsharePing').classList.contains('on')), 'a refused toggle must read off');
      assert(await page.evaluate(() => !document.getElementById('rsharePingDenied').classList.contains('btn-hide')), 'the settings line must show');
    } finally { await ctx.close(); }
  });

  // On a Monday before 08:00 the next ping is today's, which reports LAST week.
  const reportedDaysAgo = () => { const d = new Date(); return d.getDay() === 1 && d.getHours() < 8 ? 7 : 0; };

  await suite.test('guardian ping: one count-only notification for next Monday 08:00, rescheduled, routed through the gate (A20, I7, I11)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true, daysAgo: reportedDaysAgo() }) + fakeNotifications('granted') });
    try {
      await asNative(page);
      await openReport(page);
      await page.evaluate(() => document.getElementById('rsharePing').click());
      await domSettled(page);
      let n = await notif(page);
      const pending = Object.values(n.pending);
      assert(pending.length === 1 && pending[0].id === 3, `want the one Guardian notification, got ${JSON.stringify(n.pending)}`);
      const p = pending[0];
      assert(p.body === 'Guardian Report ready · 1 practice word', `body: ${p.body}`);
      assert(p.title === 'Guardian Report', `title: ${p.title}`);
      const at = new Date(p.schedule.at);
      assert(at.getDay() === 1 && at.getHours() === 8 && at.getMinutes() === 0 && at > new Date(), `fires ${at}`);
      assert(p.actionTypeId === 'GUARDIAN_REPORT' && /^#guardian-report\?week=\d{4}-\d{2}-\d{2}$/.test(p.extra.route), `action: ${JSON.stringify(p)}`);
      const payload = JSON.stringify(p).toLowerCase();
      for (const w of ['because', 'rabbit', 'becuase']) assert(!payload.includes(w), `the payload carries ${w}`);
      assert(await page.evaluate(() => document.getElementById('rsharePing').classList.contains('on')), 'a granted toggle reads on');

      // Foreground / session end reschedules; still exactly one pending.
      const count = n.scheduled.length;
      await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
      await domSettled(page);
      n = await notif(page);
      assert(n.scheduled.length === count + 1 && n.maxPending === 1, `reschedule: ${n.scheduled.length} scheduled, max pending ${n.maxPending}`);

      // The action goes through the router, so through the gate.
      await page.evaluate(() => document.getElementById('rshareDone').click());
      await page.evaluate((route) => window.__notif.listeners.localNotificationActionPerformed({ actionId: 'review', notification: { extra: { route } } }), p.extra.route);
      await domSettled(page);
      assert(await shown(page, 'parentScrim') && !(await shown(page, 'rshareScrim')), 'the action must land on the parent gate');
      await page.evaluate(() => document.getElementById('parentCancel').click());

      // Turning it off leaves nothing pending.
      await openReport(page);
      await page.evaluate(() => document.getElementById('rsharePing').click());
      await domSettled(page);
      assert(Object.keys((await notif(page)).pending).length === 0, 'off must cancel the pending notification');
    } finally { await ctx.close(); }
  });

  await suite.test('guardian ping: a week with no sessions schedules nothing (A22)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: false }) + fakeNotifications('granted') });
    try {
      await asNative(page);
      await openReport(page);
      await page.evaluate(() => document.getElementById('rsharePing').click());
      await domSettled(page);
      const n = await notif(page);
      assert(n.requested === 1 && n.scheduled.length === 0, `an empty week scheduled ${JSON.stringify(n.scheduled)}`);
      assert(await page.evaluate(() => document.getElementById('rsharePing').classList.contains('on')), 'the toggle is on; there is just nothing to send');
    } finally { await ctx.close(); }
  });

  await suite.test('guardian ping: the dashboard digest is the same one notification (Phase D merge, I11)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: init({ complete: true, learner: true, daysAgo: reportedDaysAgo() }) + fakeNotifications('granted') });
    try {
      await asNative(page);
      await page.evaluate(() => document.getElementById('gdashOpen').click());
      await domSettled(page);
      await passGate(page);
      await page.evaluate(() => document.getElementById('gdDigest').click());
      await domSettled(page);
      let n = await notif(page);
      assert(n.requested === 1, 'the dashboard toggle must ask for permission when turned on');
      const pending = Object.values(n.pending);
      assert(pending.length === 1 && pending[0].id === 3 && pending[0].body === 'Guardian Report ready · 1 practice word', `dashboard digest: ${JSON.stringify(n.pending)}`);
      // The report screen shows the same toggle, already on.
      await openReport(page);
      assert(await page.evaluate(() => document.getElementById('rsharePing').classList.contains('on')), 'the report toggle must read the shared opt-in');
      n = await notif(page);
      assert(n.maxPending === 1, `more than one Guardian notification was pending (${n.maxPending})`);
    } finally { await ctx.close(); }
  });
}
