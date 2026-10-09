// boardgame-stretch.spec -- CC-BOARD-GAME-POLISH v1, Phase C (Feature 1: Stretch words).
//   A-P2 (no audio, hint or word before the choice), the two results, never offered on Expert or in
//   Spell Jr, the star chip, the podium counter, and the flag-off game that has none of it.
import { openApp, assert, assertEq, domSettled } from '../harness.mjs';

const KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const FLAG_ON = () => { localStorage.setItem('spell_flag_boardgame', 'on'); localStorage.setItem('spell_flag_boardStretch', 'on'); };
const FLAG_OFF = () => { localStorage.setItem('spell_flag_boardgame', 'on'); };

const state = (page) => page.evaluate(() => JSON.parse(window.__spelltest.boardgameState() || 'null'));
const cite = (w) => w.split('|')[0];
const TIERS = { easy: 'Easy', medium: 'Medium', hard: 'Hard', expert: 'Expert' };

async function typeIt(page, typed) {
  for (const ch of typed) await page.click(`#bgKeys [data-k="u:${ch}"]`);
}

async function startPass(page, { count = 2, size = null } = {}) {
  await page.evaluate(() => document.getElementById('bgOpenBtn').click());
  await page.waitForSelector('#bgScreen.show', { timeout: 8000 });
  await page.click('[data-bg="mode:pass"]');
  await page.click(`[data-bg="count:${count}"]`);
  if (size) await page.click(`[data-bg="size:${size}"]`);
  await page.click('#bgStart');
  await page.waitForFunction(() => window.__spelltest.boardgameState() !== '', null, { timeout: 8000 });
}

/// Count every route a word's audio could take.
async function instrumentAudio(page) {
  await page.evaluate(() => {
    window.__aud = 0;
    const bump = () => { window.__aud++; };
    const proto = HTMLMediaElement.prototype, play = proto.play;
    proto.play = function () { bump(); (window.__audLog ||= []).push('media:' + (this.currentSrc || this.src || '').slice(0, 80)); return Promise.resolve(); };
    try { window.speechSynthesis.speak = () => { bump(); }; } catch (_) {}
  });
  page.__reqs = 0;
  page.__log = [];
  page.on('request', (r) => { if (/\/api\/speak|\.(mp3|m4a|ogg|wav)(\?|$)/.test(r.url())) { page.__reqs++; page.__log.push(r.url().slice(0, 100)); } });
}
const audioCount = (page) => page.evaluate(() => window.__aud).then((n) => n + page.__reqs);
const audioLog = (page) => page.evaluate(() => window.__audLog || []).then((l) => JSON.stringify([...l, ...page.__log]));

const raw = (page) => page.evaluate(() => window.__spelltest.boardgameState());

/// One step of a game being played with correct answers and the normal move. Returns the state
/// when `want(state)` holds (checked before acting), or null if the game ended first.
async function advanceTo(page, want, steps = 600) {
  for (let i = 0; i < steps; i++) {
    if (await page.isVisible('#bgOver')) return null;
    if (await page.isVisible('#bgMiss')) { await page.click('#bgMissGo'); continue; }
    if (await page.isVisible('#bgHand')) { await page.click('#bgHandGo'); continue; }
    const before = await raw(page);
    const s = JSON.parse(before);
    if (s.hand) {
      await page.waitForFunction(() => !document.getElementById('bgHand').hidden || !document.getElementById('bgMiss').hidden || !document.getElementById('bgOver').hidden, null, { timeout: 8000 }).catch(() => {});
      continue;
    }
    if (want(s)) return s;
    const changed = () => page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 4000 }).catch(() => {});
    if (s.phase === 'AwaitRoll') {
      // A-P2 is about what happens between the roll and the choice: count from here.
      await page.evaluate(() => { window.__aud = 0; window.__audLog = []; });
      page.__reqs = 0; page.__log = [];
      await page.click('#bgOrb'); await changed(); continue;
    }
    if (s.phase === 'AwaitStretch') { await page.click('#bgStretch [data-st="0"]'); await changed(); continue; }
    if (s.phase === 'AwaitSpelling') { await typeIt(page, cite(s.word)); await page.click('#bgGo'); await changed(); continue; }
    if (s.phase === 'AwaitSwitchTarget') { await page.click('[data-sw="none"]'); await changed(); continue; }
    await page.waitForFunction(() => document.getElementById('bgHand')?.hidden === false || document.getElementById('bgOver')?.hidden === false || JSON.parse(window.__spelltest.boardgameState()).phase === 'AwaitRoll', null, { timeout: 4000 }).catch(() => {});
  }
  return null;
}
const atStretch = (s) => s.phase === 'AwaitStretch';

/// At the choice: everything A-P2 asks of the screen, then commit.
async function checkChoice(page, s) {
  assert(await page.isVisible('#bgStretch'), 'the two buttons are on screen');
  assertEq(s.word, null, 'the engine holds no word yet');
  assert(!(await page.isVisible('#bgKeys')), 'no answer keys while choosing');
  assertEq(await audioCount(page), 0, `no audio before the choice (A-P2): ${await audioLog(page)}`);
  const view = (await page.textContent('#bgScreen')).toLowerCase();
  const btns = await page.$$eval('#bgStretch .bg-st-opt', (b) => b.map((x) => x.textContent.replace(/\s+/g, ' ').trim()));
  assertEq(btns.length, 2, 'normal move and Stretch');
  assert(btns[0].includes(TIERS[s.offer.normal]) && btns[0].includes(String(s.offer.nd)), `normal button names tier and tile: ${btns[0]}`);
  assert(btns[1].includes(TIERS[s.offer.stretch]) && btns[1].includes(String(s.offer.sd)) && btns[1].includes('\u2605'), `Stretch button: ${btns[1]}`);
  assert(['medium', 'hard'].includes(s.offer.normal), 'never offered from Expert');
  assertEq(view.includes('undefined'), false);
}

async function choose(page, s, take) {
  const who = s.seat;
  await page.click(`#bgStretch [data-st="${take ? 1 : 0}"]`);
  await page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState()).phase === 'AwaitSpelling', null, { timeout: 4000 });
  const t = await state(page);
  assertEq(t.stretchSpell, take, 'the word is the committed kind');
  assertEq(t.tier, take ? s.offer.stretch : s.offer.normal, 'drawn from the committed tier (I-P6)');
  await domSettled(page);
  return { who, t };
}

async function answer(page, t, typed) {
  const before = await raw(page);
  const chips0 = await page.$eval('#bgResults', (e) => e.innerHTML);
  await typeIt(page, typed);
  await page.click('#bgGo');
  await page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 4000 });
  // The result chip for this answer is the signal that the turn has been drawn.
  await page.waitForFunction((c) => document.getElementById('bgResults').innerHTML !== c, chips0, { timeout: 4000 });
}

export async function run(browser, base, suite) {
  await suite.test('boardgame_stretch_choice_precedes_every_word_and_pays_two', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await instrumentAudio(page);
      await startPass(page, { count: 2, size: 'full' });
      const trapHit = async () => (await page.$eval('#bgResults', (e) => e.textContent)).includes('\u26a0');

      // 1. The normal move: unchanged by Stretch, no star on the chip.
      let s = await advanceTo(page, atStretch);
      assert(s, 'a Stretch offer appeared');
      await checkChoice(page, s);
      let c = await choose(page, s, false);
      await answer(page, c.t, cite(c.t.word));
      let after = await state(page);
      if (!(await trapHit())) assertEq(after.pos[c.who], s.offer.nd, 'the normal move lands where the button said');
      assert(!(await page.$eval('#bgResults', (e) => e.textContent)).includes('\u2605'), 'a normal move carries no star');

      // 2. A correct Stretch: roll + 2, star on the chip, word from the harder tier.
      s = await advanceTo(page, atStretch);
      assert(s, 'a second offer appeared');
      await checkChoice(page, s);
      c = await choose(page, s, true);
      await answer(page, c.t, cite(c.t.word));
      after = await state(page);
      const chips = await page.$eval('#bgResults', (e) => e.textContent);
      if (!(await trapHit())) assertEq(after.pos[c.who], s.offer.sd, 'a correct Stretch lands on roll + 2');
      assert(/\u2605 \u2713 \d+\u2192\d+/.test(chips), `the chip marks a Stretch move: ${chips}`);
      assertEq(after.sc[c.who][1], 1, 'the take is counted');
      assertEq(after.sc[c.who][2], 1, 'and the hit');

      // 3. A wrong Stretch: moves 0, chip says so.
      s = await advanceTo(page, atStretch);
      assert(s, 'a third offer appeared');
      const posBefore = s.pos[s.seat];
      await checkChoice(page, s);
      c = await choose(page, s, true);
      await answer(page, c.t, 'q');
      after = await state(page);
      assertEq(after.pos[c.who], posBefore, 'a wrong Stretch moves 0');
      assert(/\u2605 \u2717/.test(await page.$eval('#bgResults', (e) => e.textContent)), 'the chip marks a missed Stretch');
    } finally { await ctx.close(); }
  });

  await suite.test('boardgame_stretch_is_never_offered_from_expert_or_in_spell_jr', async () => {
    const a = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await startPass(a.page, { count: 2, size: 'sprint' });
      let offers = 0, expertLandings = 0;
      for (let i = 0; i < 400; i++) {
        const s = await advanceTo(a.page, (x) => x.phase === 'AwaitStretch' || (x.phase === 'AwaitSpelling' && x.kind === 'Landing'), 60);
        if (!s) break;
        if (s.phase === 'AwaitStretch') {
          offers++;
          assert(s.offer.normal !== 'expert' && s.offer.stretch !== 'easy' && s.offer.stretch !== 'medium', `offer ${JSON.stringify(s.offer)}`);
          await a.page.click('#bgStretch [data-st="0"]');
          await a.page.waitForFunction(() => JSON.parse(window.__spelltest.boardgameState()).phase !== 'AwaitStretch', null, { timeout: 4000 });
          continue;
        }
        if (s.tier === 'expert') expertLandings++;
        await typeIt(a.page, cite(s.word));
        const b = await raw(a.page);
        await a.page.click('#bgGo');
        await a.page.waitForFunction((x) => window.__spelltest.boardgameState() !== x, b, { timeout: 4000 }).catch(() => {});
        if (offers >= 6 && expertLandings >= 1) break;
      }
      assert(offers >= 3, `only ${offers} offers seen`);
      assert(expertLandings >= 1, 'an Expert tile was landed on without an offer');
    } finally { await a.ctx.close(); }

    const j = await openApp(browser, base, { lang: 'en', age: KID, init: FLAG_ON });
    try {
      await startPass(j.page, { count: 2 });
      assertEq((await state(j.page)).variant, 'Jr');
      for (let i = 0; i < 40; i++) {
        const s = await advanceTo(j.page, (x) => x.phase === 'AwaitStretch' || (x.phase === 'AwaitSpelling' && x.kind === 'Landing'), 30);
        if (!s) break;
        assert(s.phase !== 'AwaitStretch', 'Spell Jr was offered Stretch');
        assert(!(await j.page.isVisible('#bgStretch')), 'no Stretch buttons in Spell Jr');
        await typeIt(j.page, cite(s.word));
        const b = await raw(j.page);
        await j.page.click('#bgGo');
        await j.page.waitForFunction((x) => window.__spelltest.boardgameState() !== x, b, { timeout: 4000 }).catch(() => {});
      }
    } finally { await j.ctx.close(); }
  });

  await suite.test('boardgame_stretch_podium_counter_and_flag_off_has_none', async () => {
    const a = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await startPass(a.page, { count: 2, size: 'sprint' });
      // Take every Stretch, spell it right, until the podium.
      for (let i = 0; i < 600 && !(await a.page.isVisible('#bgOver')); i++) {
        const s = await advanceTo(a.page, atStretch, 40);
        if (!s) continue;
        const before = await raw(a.page);
        await a.page.click('#bgStretch [data-st="1"]');
        await a.page.waitForFunction((b) => window.__spelltest.boardgameState() !== b, before, { timeout: 4000 });
        const t = await state(a.page);
        await typeIt(a.page, cite(t.word));
        const b2 = await raw(a.page);
        await a.page.click('#bgGo');
        await a.page.waitForFunction((x) => window.__spelltest.boardgameState() !== x, b2, { timeout: 4000 }).catch(() => {});
      }
      await a.page.waitForSelector('#bgOver:not([hidden])', { timeout: 20000 });
      const podium = await a.page.$eval('#bgPodium', (e) => e.textContent);
      assert(/Stretch: \d+ of \d+ taken, \d+ right/.test(podium), `the podium shows the on-device counter: ${podium}`);
    } finally { await a.ctx.close(); }

    const o = await openApp(browser, base, { lang: 'en', init: FLAG_OFF });
    try {
      await startPass(o.page, { count: 2, size: 'sprint' });
      assertEq((await state(o.page)).stretchOn, false, 'the flag is off');
      let rolls = 0;
      for (let i = 0; i < 300 && rolls < 30 && !(await o.page.isVisible('#bgOver')); i++) {
        const s = await advanceTo(o.page, (x) => x.phase === 'AwaitSpelling' && x.kind === 'Landing', 40);
        if (!s) break;
        rolls++;
        assert(!(await o.page.isVisible('#bgStretch')), 'no Stretch choice with the flag off');
        assertEq(s.offer, null);
        await typeIt(o.page, cite(s.word));
        const b = await raw(o.page);
        await o.page.click('#bgGo');
        await o.page.waitForFunction((x) => window.__spelltest.boardgameState() !== x, b, { timeout: 4000 }).catch(() => {});
      }
      assert(rolls >= 10);
    } finally { await o.ctx.close(); }
  });
}
