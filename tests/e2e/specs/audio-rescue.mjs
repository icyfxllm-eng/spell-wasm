// audio-rescue.spec — CC-AUDIO-CLARITY v1.1 F6a on the real screen.
//
// A player who cannot hear the word must neither lose a turn over it nor get a
// free answer from it (§1's rescue principle). These are A-12 to A-16.
import { openApp, assert, assertEq } from '../harness.mjs';

/// Everything a void must leave exactly as it found it (I10).
const SNAPSHOT = () => ({
  streak: window.__spelltest.streak ? window.__spelltest.streak() : null,
  misses: localStorage.getItem('byear_misses_v1'),
  stats: localStorage.getItem('byear_stats_v1'),
  learner: localStorage.getItem('spell_learner_v2') || localStorage.getItem('spell_learner_v1'),
  shields: localStorage.getItem('spell_shields_v1'),
  climb: localStorage.getItem('byear_climb_v1'),
});

async function startRound(page) {
  await page.click('#orbWrap');
  await page.waitForTimeout(600);
  return page.evaluate(() => window.__spelltest.currentWord());
}

export async function run(browser, base, suite) {
  // A-16: only the steps that exist are shown. Sentence (F5) and the second
  // voice (F8) are later phases, so today the rescue offers slow replay and
  // the reveal, and nothing else.
  await suite.test('rescue: offers only the steps that exist', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startRound(page);
      assert(!(await page.$eval('#cantHearBtn', (e) => e.disabled)), 'the rescue is offered once audio is');
      await page.click('#cantHearBtn');
      assert(await page.$eval('#rescuePanel', (e) => !e.hasAttribute('hidden')), 'the panel opens');
      // Scoped to THIS panel: every mode has a rescue now, and an unscoped
      // selector matches all four.
      const steps = await page.$$eval('#rescuePanel .rescue-steps button', (b) => b.map((x) => x.id));
      assertEq(JSON.stringify(steps), JSON.stringify(['rescueSlow', 'rescueShow']), 'exactly the built steps');
    } finally { await ctx.close(); }
  });

  // A-13, the one that matters: a void leaves no trace.
  // A-14: shown, never entered.
  await suite.test('rescue: showing the word voids the round and leaves nothing behind', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const word = await startRound(page);
      const before = await page.evaluate(SNAPSHOT);
      await page.click('#cantHearBtn');
      await page.click('#rescueShow');
      await page.waitForTimeout(300);
      // A-14: the word is shown, and the answer field is empty and locked.
      const shown = await page.$eval('#feedback', (e) => e.textContent);
      assert(shown.includes(word), `the word is shown (got ${JSON.stringify(shown)})`);
      // The typed answer renders per character into #letters, and an empty box
      // shows its placeholder -- so I11's claim is not "the box is blank", it
      // is "the word is not in it and it cannot be sent".
      const box = await page.$eval('#letters', (e) => e.textContent);
      assert(!box.includes(word), `the revealed word is never placed in the answer box (I11) -- box held ${JSON.stringify(box)}`);
      assert(await page.$eval('#checkBtn', (e) => e.disabled), 'it cannot be submitted');
      // A-13: byte-identical state.
      const after = await page.evaluate(SNAPSHOT);
      for (const k of Object.keys(before)) {
        assertEq(after[k], before[k], `${k} must be untouched by a void (I10)`);
      }
    } finally { await ctx.close(); }
  });

  // A-15: the rescue is local. Nothing it does reaches the network.
  await suite.test('rescue: sends nothing', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startRound(page);
      const seen = [];
      page.on('request', (r) => seen.push(r.url()));
      await page.click('#cantHearBtn');
      await page.click('#rescueSlow');
      await page.waitForTimeout(200);
      await page.click('#cantHearBtn');
      await page.click('#rescueShow');
      await page.waitForTimeout(400);
      // Audio playback itself may fetch a clip; nothing ELSE may leave.
      const nonAudio = seen.filter((u) => !/\/api\/speak|\.mp3|\.wav/.test(u));
      assertEq(JSON.stringify(nonAudio), '[]', `the rescue sent ${JSON.stringify(nonAudio)}`);
    } finally { await ctx.close(); }
  });

  // A-12: the exploit. A player who reveals every word scores nothing, and in
  // a capped mode the fourth step turns into a skip.
  await suite.test('rescue: revealing everything scores nothing and hits its cap', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await startRound(page);
      const before = await page.evaluate(SNAPSHOT);
      for (let i = 0; i < 3; i++) {
        if (await page.$eval('#cantHearBtn', (e) => e.disabled)) break;
        await page.click('#cantHearBtn');
        await page.click('#rescueShow');
        await page.waitForTimeout(500);
      }
      const after = await page.evaluate(SNAPSHOT);
      assertEq(after.stats, before.stats, 'A-12: revealing every word scores nothing');
      assertEq(after.misses, before.misses, 'and records no misses');
    } finally { await ctx.close(); }
  });
}
