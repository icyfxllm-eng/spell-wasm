// jr-climb.spec — CC-ONBOARD-JR F3 / Done 4 and 5, on the real screen.
//
// The engine tests already prove the shape of a Jr Climb run. This proves it
// where a player meets it: the words served, the tier each came from, the
// word-21 message on the feedback line, and the network.
//
// The leaderboard half carries a control, or it would prove nothing. A guest
// never posts, and adaptive Climb never posts either -- submit_run ranks fixed
// medium / hard / expert chains only, by design. So a SIGNED-IN standard player
// on fixed Medium must post, and the identical signed-in setup for a Spell Jr
// player must not. /api/auth/me and /api/climb/** are stubbed, so nothing here
// reaches a server.
import { openApp, assert } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const TOKEN_KEY = 'byear_climb_token_v1';
const GHOST_KEY = 'spell_ghost_v1';
const UP = '↑';

const seam = (page, fn) => page.evaluate((f) => window.__spelltest[f](), fn);
const feedbackText = (page) => page.$eval('#feedback', (e) => e.textContent);

// The keyboard is locked between words (it takes no taps while the next word is
// being served), so a test that types the instant a word appears types into a
// locked keyboard. A player waits for it to unlock; so does this.
async function waitTypeable(page) {
  await page.waitForFunction(() => {
    const kb = document.getElementById('gameKeyboard');
    return kb && !kb.classList.contains('locked');
  }, null, { timeout: 5000 });
}

const runEnded = (page) => page.evaluate(() => document.getElementById('scrim').classList.contains('show'));

// The run is over: either the name sheet is up, or the chain has reset to 0.
const chainOver = (page) => page.evaluate(() =>
  document.getElementById('scrim').classList.contains('show')
  || document.getElementById('streakNum').textContent.trim() === '0');

async function typeWord(page, w) {
  for (const ch of w.toLowerCase()) {
    const key = await page.$(`#gameKeyboard .kb-key[data-k="${ch}"]`);
    assert(key, `no keyboard key for "${ch}" in "${w}"`);
    await key.click();
  }
}

async function waitAnswered(page, timeout = 3500) {
  await page.waitForFunction(() => {
    const c = document.getElementById('feedback').className;
    return c.includes('good') || c.includes('bad');
  }, null, { timeout });
}

// What the screen showed when a wait gave up, so a failure names its word.
const screen = (page) => page.evaluate(() => {
  const f = document.getElementById('feedback');
  return JSON.stringify({
    word: window.__spelltest.currentWord(), tier: window.__spelltest.currentTier(),
    feedback: `${f.className} "${f.textContent.trim().slice(0, 60)}"`,
    typed: (document.getElementById('spellbox') || {}).textContent?.trim().slice(0, 30),
    keyboardLocked: document.getElementById('gameKeyboard').classList.contains('locked'),
    checkDisabled: document.getElementById('checkBtn').disabled,
    sheetOpen: document.getElementById('scrim').classList.contains('show'),
  });
});

// Answer the current word correctly, then tap the orb, which skips the
// post-answer delay and serves the next word at once.
async function answerAndAdvance(page) {
  const w = await seam(page, 'currentWord');
  await waitTypeable(page);
  await typeWord(page, w);
  await page.click('#checkBtn');
  await waitAnswered(page);
  assert(await page.$eval('#feedback', (e) => e.className.includes('good')),
    `the correct answer "${w}" was not graded correct`);
  await page.click('#orbWrap');
  await page.waitForFunction((prev) => window.__spelltest.currentWord() !== prev, w, { timeout: 4000 });
}

const shieldOffered = (page) => page.evaluate(() => document.getElementById('shieldScrim').classList.contains('show'));

// End the run with a real miss. A long run earns shields, so a miss may first
// ask to spend one -- decline, as a player ending the run would. Miss again only
// if a second try kept the run alive: once the run has ended, the "Chain broken"
// name sheet covers the game, and there is nothing left to miss.
async function missToEnd(page) {
  for (let i = 0; i < 4; i++) {
    if (await chainOver(page)) break;
    if (await shieldOffered(page)) {
      await page.click('#shieldDecline');
      await page.waitForTimeout(1300); // declining ends the chain, then the sheet may open
      continue;
    }
    const w = await seam(page, 'currentWord');
    if (!w) break;
    // A keyboard that stays locked means no retry is on offer: stop, and let the
    // caller's chainOver check say what state the run was left in.
    if (!(await waitTypeable(page).then(() => true).catch(() => false))) break;
    if (await chainOver(page)) break;
    await typeWord(page, w === 'zzzz' ? 'xxxx' : 'zzzz');
    await page.click('#checkBtn');
    await waitAnswered(page).catch(() => {});
    await page.waitForTimeout(1300); // end_chain decides, then posts, then opens the sheet
  }
  await page.waitForTimeout(800);
}

async function setLevel(page, level) {
  await page.evaluate((v) => {
    const s = document.getElementById('levelSel');
    s.value = v;
    s.dispatchEvent(new Event('change', { bubbles: true }));
  }, level);
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 4000 });
}

// A session with a stored token and a stubbed /api/auth/me, so the app restores
// a signed-in player at boot. Every /api/climb/ request is recorded.
async function signedIn(browser, base, age) {
  const { ctx, page } = await openApp(browser, base, { lang: 'en', age });
  const requests = [];
  page.on('request', (r) => { if (r.url().includes('/api/climb/')) requests.push(`${r.method()} ${r.url()}`); });
  await ctx.route('**/api/auth/me', (r) => r.fulfill({
    status: 200, contentType: 'application/json', body: JSON.stringify({ user: { id: 7, username: 'probe' } }),
  }));
  await ctx.route('**/api/climb/**', (r) => r.fulfill({
    status: 200, contentType: 'application/json', body: JSON.stringify({ record: false }),
  }));
  await page.evaluate((k) => localStorage.setItem(k, 'probe-token'), TOKEN_KEY);
  await page.reload({ waitUntil: 'load' });
  await page.evaluate((b) => { window.SPELL_API_BASE = b.replace(/\/$/, ''); }, base);
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  await page.waitForTimeout(500); // /api/auth/me resolves and the session is restored
  return { ctx, page, requests };
}

export async function run(browser, base, suite) {
  // Done 4 — 23 words of adaptive Climb for a Spell Jr player.
  await suite.test('jr_climb_run_steps_to_medium_at_word_21_once', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: KID });
    const requests = [];
    page.on('request', (r) => { if (r.url().includes('/api/climb/')) requests.push(r.url()); });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'a Spell Jr player');
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 4000 });

      const tiers = [];
      const beats = [];
      let clearedInTime = null;
      for (let n = 1; n <= 23; n++) {
        tiers.push(await seam(page, 'currentTier'));
        if (n === 21) {
          const shown = await page.waitForFunction((up) => document.getElementById('feedback').textContent.includes(up),
            UP, { timeout: 1200 }).then(() => true).catch(() => false);
          if (shown) {
            beats.push(n);
            await page.waitForTimeout(1700);
            clearedInTime = !(await feedbackText(page)).includes(UP);
          }
        } else if ((await feedbackText(page)).includes(UP)) {
          beats.push(n);
        }
        if (n < 23) {
          try { await answerAndAdvance(page); } catch (e) {
            throw new Error(`word ${n}: ${e.message.split('\n')[0]} -- screen ${await screen(page)}`);
          }
        }
      }

      const expected = [...Array(20).fill('easy'), 'medium', 'medium', 'medium'];
      assert(JSON.stringify(tiers) === JSON.stringify(expected),
        `words 1-20 are Easy and 21-23 Medium, got ${JSON.stringify(tiers)}`);
      assert(JSON.stringify(beats) === JSON.stringify([21]),
        `the level-up message shows once, at word 21 (got ${JSON.stringify(beats)})`);
      assert(clearedInTime === true, 'and clears itself within 1.5 s');

      await missToEnd(page);
      assert(await chainOver(page), 'the closing miss ended the run, so the no-post check below means something');
      assert(requests.length === 0, `a Jr Climb run makes no leaderboard request (saw ${JSON.stringify(requests)})`);
      assert(!(await page.evaluate((k) => localStorage.getItem(k), GHOST_KEY)),
        'and records no Spell Racing ghost');
    } finally { await ctx.close(); }
  });

  // Done 5 / I6 — the leaderboard guard, proven against a control that does post.
  await suite.test('jr_climb_leaderboard_guard_has_a_control', async () => {
    {
      const { ctx, page, requests } = await signedIn(browser, base);
      try {
        await setLevel(page, 'medium');
        await answerAndAdvance(page);
        await missToEnd(page);
        assert(requests.some((u) => u.includes('/api/climb/submit-chain')),
          `control: a signed-in standard player's Medium chain posts (saw ${JSON.stringify(requests)})`);
      } finally { await ctx.close(); }
    }
    {
      const { ctx, page, requests } = await signedIn(browser, base, KID);
      try {
        await setLevel(page, 'medium');
        await answerAndAdvance(page);
        await missToEnd(page);
        assert(await chainOver(page), 'the Spell Jr run ended, as the control run did');
        assert(requests.length === 0,
          `the identical setup for a Spell Jr player posts nothing (saw ${JSON.stringify(requests)})`);
      } finally { await ctx.close(); }
    }
  });
}
