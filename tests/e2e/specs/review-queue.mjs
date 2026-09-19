// review-queue.spec — CC-LEARNING-ENGINE-L0 R2: the missed-words queue is
// scheduled by review.rs (FSRS-4.5, graded per D3), not Leitner boxes.
//
// Browser-only claims, because they are about stored bytes and real wiring:
// a miss enters the queue due now; a pre-R2 entry carries over keeping its due
// time; and a correct answer after replaying the audio grades Hard (comes back
// sooner) where a clean one grades Good.
import { openApp, assert } from '../harness.mjs';

const KEY = 'byear_misses_v1';
const DAY = 86400000;

async function reloadBooted(page) {
  await page.reload();
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  await page.waitForTimeout(300);
}

const queue = (page) => page.evaluate((k) => JSON.parse(localStorage.getItem(k) || '[]'), KEY);

// Type on the on-screen keyboard; every key must exist and be live.
async function type(page, text) {
  for (const ch of text) {
    const k = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
    assert(k, `no key for ${JSON.stringify(ch)}`);
    const locked = await page.evaluate(() => document.getElementById('gameKeyboard').classList.contains('locked'));
    assert(!locked, `keyboard locked while typing ${JSON.stringify(text)}`);
    await k.click({ timeout: 5000 });
  }
}

async function serveFresh(page) {
  const before = await page.evaluate(() => window.__spelltest.currentWord());
  await page.click('#orbWrap');
  await page.waitForFunction((b) => { const w = window.__spelltest.currentWord(); return w && w !== b; }, before, { timeout: 5000 });
  return page.evaluate(() => window.__spelltest.currentWord());
}

// A deliberate miss: a wrong answer the keyboard can type.
async function missCurrent(page, w) {
  await type(page, w.toLowerCase() === 'zzz' ? 'qqq' : 'zzz');
  await page.click('#checkBtn');
  await page.waitForTimeout(400);
}

export async function run(browser, base, suite) {
  await suite.test('review: a miss enters the queue on the review rule, due now', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const w = await serveFresh(page);
      const t0 = Date.now();
      await missCurrent(page, w);
      const e = (await queue(page)).find((m) => m.word === w);
      assert(e, `the missed word ${w} is not in the queue`);
      assert(e.review && e.review.step === 0, `expected a learning-step-0 review state, got ${JSON.stringify(e.review)}`);
      assert(e.due <= Date.now() && e.due >= t0 - 1000, `a fresh miss must be due now: due=${e.due}, now=${Date.now()}`);
      assert(e.box_ === 0, `new entries don't use Leitner boxes (box_=${e.box_})`);
    } finally { await ctx.close(); }
  });

  await suite.test('review: a pre-R2 queue entry carries over and keeps its due time', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const due = Date.now() + 2 * DAY + 12345;
      await page.evaluate(([k, due]) => localStorage.setItem(k, JSON.stringify([
        { word: 'rhythm', lang: 'en', tier: 'medium', misses: 2, box_: 4, due, ts: Date.now() - 5 * 86400000 },
      ])), [KEY, due]);
      await reloadBooted(page);
      // Any queue write saves the whole list, carried over at load.
      const w = await serveFresh(page);
      await missCurrent(page, w);
      const e = (await queue(page)).find((m) => m.word === 'rhythm');
      assert(e, 'the carried-over entry vanished');
      assert(e.due === due, `the due time changed in carry-over: ${e.due} vs ${due}`);
      assert(e.review && e.review.step === 2 && e.review.fsrs.stability === 3,
        `box 4 should carry over to FSRS with stability 3: ${JSON.stringify(e.review)}`);
      assert(e.misses === 2, 'miss count lost in carry-over');
    } finally { await ctx.close(); }
  });

  await suite.test('review: a correct answer after a replay grades Hard and comes back sooner than a clean one', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Two words on FSRS, identical state, both due now.
      await page.evaluate(([k]) => {
        const now = Date.now(), today = Math.floor(now / 86400000);
        const card = { step: 2, fsrs: { stability: 3, difficulty: 7.6214, due_day: today, reps: 1, lapses: 0 } };
        localStorage.setItem(k, JSON.stringify(['rabbit', 'carrot'].map((word) => (
          { word, lang: 'en', tier: 'easy', misses: 1, box_: 0, due: now - 60000, ts: now - 3 * 86400000, review: card }))));
      }, [KEY]);
      await reloadBooted(page);
      await page.evaluate(() => document.getElementById('missesBtn').click());
      await page.waitForTimeout(300);
      await page.click('#orbWrap'); // review serves on the orb, like normal play
      await page.waitForFunction(() => window.__spelltest.currentWord(), null, { timeout: 5000 });
      const answered = {};
      for (let i = 0; i < 2; i++) {
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        assert(w === 'rabbit' || w === 'carrot', `review served ${JSON.stringify(w)}, expected a queued word`);
        if (w === 'rabbit') {
          await page.evaluate(() => document.getElementById('replayBtn').click());
          await page.waitForTimeout(200);
        }
        await type(page, w);
        await page.click('#checkBtn');
        answered[w] = true;
        await page.waitForFunction((p) => window.__spelltest.currentWord() !== p, w, { timeout: 6000 }).catch(() => {});
        await page.waitForTimeout(300);
      }
      assert(answered.rabbit && answered.carrot, `review didn't serve both words: ${JSON.stringify(answered)}`);
      const q = await queue(page);
      const rabbit = q.find((m) => m.word === 'rabbit');
      const carrot = q.find((m) => m.word === 'carrot');
      const today = Math.floor(Date.now() / DAY);
      // FSRS-4.5 from S=3 at R=0.9: Good -> S~8.1 (over 7 days: graduates),
      // Hard (x w15) -> S~4.2, due in 4 days.
      assert(!carrot, `the clean (Good) answer should graduate out of the queue: ${JSON.stringify(carrot)}`);
      assert(rabbit, 'the replayed (Hard) answer should stay queued');
      assert(rabbit.review.fsrs.reps === 2, `Hard should count as a review: ${JSON.stringify(rabbit.review)}`);
      const inDays = Math.round(rabbit.due / DAY) - today;
      assert(inDays >= 3 && inDays <= 5, `Hard should come back in about 4 days, got ${inDays}`);
    } finally { await ctx.close(); }
  });
}
