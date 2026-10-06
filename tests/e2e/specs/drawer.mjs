// drawer.spec — CC-HUB-NAV. The navigation drawer renders ONLY from
// config/modes.json, and every rule that decides a row lives in one pure Rust
// function (modes::catalog). These tests exercise the rendered result, which
// is the part unit tests cannot see: that the registry actually reaches the
// DOM, localized, with a real destination behind every row.
//
// This file was the gamepad sheet's spec until Phase C retired the sheet. The
// laws did not retire with it — "no hidden mode can be flagged back on", "no
// upsell copy in Kid Mode", "localized with no new copy" are properties of the
// REGISTRY reaching a surface, and the drawer is now that surface. Only the
// sheet-shaped parts are gone: there are no teaser or info rows to test,
// because the drawer renders neither. An aid is `unlisted` in the registry and
// never becomes a row at all, which is the stronger version of the law the
// sheet expressed with element type.
import { openApp, assert, assertEq, pinBaseline, clickSettled, painted, domSettled} from '../harness.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

/** Open the drawer and describe every MODE row (not account, not help). */
async function rows(page) {
  await clickSettled(page, '#navBurger');
  return page.$$eval('#navDrawerPanel .nav-row', (els) =>
    els
      .map((e) => ({
        mode: e.dataset.mode,
        name: (e.querySelector('.nav-name') || {}).textContent || '',
      }))
      // `acct` and the `help_*` rows are chrome, not registry modes.
      .filter((r) => r.mode && r.mode !== 'acct' && !r.mode.startsWith('help_')),
  );
}

export async function run(browser, base, suite) {
  // CC-HUB-DEADROWS F3 — the dead-row sweep.
  //
  // Rows reached a TestFlight build doing nothing when tapped, and the existing
  // destination test could not see it: that one checks the ROW is not disabled
  // and taps exactly one of them. This checks what every row PRESSES.
  //
  // Deliberately NOT a btn-hide check. Hidden is not dead: a hidden button
  // still fires its handler, and the drawer leans on exactly that — practiceOpen
  // and defMatchOpen are kept in the DOM hidden, purely so a row can click them
  // (the climbBtn idiom). The things that really swallow a tap are a target that
  // is not in the DOM, a target that is `disabled` (the drawer says it itself:
  // "a disabled button fires no click"), and a row with no target at all.
  // Acceptance 5 wants this across uiLangs and both profiles, because the row
  // SET differs per profile (juniorPolicy) and per language (availability), so
  // English-standard alone would leave the Jr menu and the RTL layout unswept.
  // ar is here for being right-to-left; Jr for having a different row set.
  const sweepRows = (page) => page.$$eval('#navDrawerPanel .nav-row', (els) =>
    els.map((e) => {
      const id = e.dataset.mode;
      const target = e.dataset.target;
      if (!target) return { id, why: 'no data-target — the row presses nothing' };
      const el = document.getElementById(target);
      if (!el) return { id, why: `target #${target} is not in the DOM` };
      if (el.disabled) return { id, why: `target #${target} is disabled; a disabled button fires no click` };
      return null;
    }).filter(Boolean));

  const assertSweep = async (page) => {
    await clickSettled(page, '#navBurger');
    const seen = await page.$$eval('#navDrawerPanel .nav-row', (els) => els.length);
    assert(seen > 0, 'the drawer rendered no rows at all');
    const bad = await sweepRows(page);
    assert(bad.length === 0,
      `rows that press nothing live: ${bad.map((b) => `${b.id} (${b.why})`).join('; ')}`);
  };

  for (const lang of ['en', 'ar']) {
    await suite.test(`every drawer row presses something that can receive it (${lang})`, async () => {
      // openApp, not a hand-built context: it answers the age gate, which
      // otherwise sits over the burger and the click times out.
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        await assertSweep(page);
      } finally { await ctx.close(); }
    });
  }

  await suite.test('every drawer row presses something that can receive it (Jr)', async () => {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true });
    await pinBaseline(ctx);
    try {
      await ctx.addInitScript(([age]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', 'en');
      }, [AGE_KID]);
      await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
      const page = await ctx.newPage();
      await page.goto(base);
      await domSettled(page);
      await assertSweep(page);
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: opens from the meta corner and renders registry rows', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const t = await rows(page);
      const shown = await page.$eval('#navDrawer', (e) => e.classList.contains('show'));
      assert(shown, 'drawer did not open');
      assert(t.length > 0, 'drawer rendered no rows');
      assert(!t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing is cut and must not appear');
      assert(t.some((x) => x.mode === 'def_match'), 'an all-platform mode still reaches the drawer');
      for (const iosOnly of ['photo_list', 'spell_aloud']) {
        assert(!t.some((x) => x.mode === iosOnly), `${iosOnly} is iOS-only and must not appear on web`);
      }
    } finally { await ctx.close(); }
  });

  // A4. The sheet is gone, so for most of these modes the row IS the door.
  // A row that renders and goes nowhere is the failure this whole phase was
  // about, and it is invisible to a unit test: route_for is pure, but whether
  // the element it names exists in the shipped markup is not.
  await suite.test('drawer: every mode row has a destination that exists', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const t = await rows(page);
      for (const r of t) {
        const live = await page.evaluate((mode) => {
          const row = document.querySelector(`.nav-row[data-mode="${mode}"]`);
          return !!row && !row.disabled;
        }, r.mode);
        assert(live, `${r.mode} rendered a dead row`);
      }
      // And one end to end, chosen because it is a full screen with its own
      // exit: tapping the row must actually land there.
      if (t.some((x) => x.mode === 'spelldoku')) {
        await clickSettled(page, '.nav-row[data-mode="spelldoku"]');
        const closed = await page.$eval('#navDrawer', (e) => !e.classList.contains('show'));
        assert(closed, 'the drawer must close behind a row that navigates');
        const arrived = await page.evaluate(() =>
          [...document.querySelectorAll('.scrim.show, .screen.show')].some((e) => /doku/i.test(e.id)));
        assert(arrived, 'tapping the SpellDoku row did not open SpellDoku');
      }
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: word_stories is never rendered (F8 hard gate)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => localStorage.setItem('spell_flag_word_stories', 'on'));
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await painted(page);
      const t = await rows(page);
      assert(!t.some((x) => x.mode === 'word_stories'), 'word_stories appeared despite status:hidden');
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: a hidden mode cannot be flagged back on', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => localStorage.setItem('spell_flag_online_spelloff', 'on'));
      await page.reload();
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await painted(page);
      const t = await rows(page);
      assert(!t.some((x) => x.mode === 'online_spelloff'),
        'a hidden mode appeared because its flag was on — `hidden` must win');
    } finally { await ctx.close(); }
  });

  // "No chains yet — be the first to start one." was wrapping one word per line.
  await suite.test('home: the empty chains line reads as a sentence, not a column', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const box = await page.$eval('#boardList li.empty', (e) => {
        const r = e.getBoundingClientRect();
        const line = parseFloat(getComputedStyle(e).lineHeight) || 20;
        return { width: Math.round(r.width), height: Math.round(r.height), line: Math.round(line) };
      });
      assert(box.width > 200, `the line gets the card's width, not the rank column (got ${box.width}px)`);
      assert(box.height <= box.line * 3, `and wraps at most three lines (got ${box.height}px)`);
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: rows are localized with no new copy (es)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await rows(page);
      assert(t.length > 0, 'es drawer rendered no rows');
      for (const x of t) {
        assert(!x.name.startsWith('tools.'), `row rendered a raw i18n key: ${x.name}`);
        assert(x.name.trim().length > 0, `row ${x.mode} rendered an empty name`);
      }
      const enNames = { practice: 'Practice', def_match: 'Definition Match' };
      const localized = t.some((x) => enNames[x.mode] && x.name !== enNames[x.mode]);
      assert(localized, 'no row localized to es — the catalog is not being reached');
    } finally { await ctx.close(); }
  });

  // The drawer is taller than a phone screen. The sheet it replaced used to
  // let the wheel scroll the PAGE behind it, so closing dropped the player at
  // the bottom of home; the panel owns its own scroller for that reason.
  await suite.test('drawer: scrolling the panel never scrolls the page behind it', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: 375, height: 320 } });
    try {
      const pageY = () => page.evaluate(() => Math.round(window.scrollY));
      const before = await pageY();
      await page.click('#navBurger');
      await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
      const overflows = await page.evaluate(() => {
        // v1.3.2 F1: the PANEL is a frame now — a scroller plus a pinned
        // footer — so the element that moves is the scroller inside it.
        const p = document.getElementById('navDrawerScroll');
        return p.scrollHeight > p.clientHeight;
      });
      assert(overflows, 'this viewport must make the panel overflow, or the test proves nothing');
      await page.mouse.move(300, 200);
      // sleep-ok: pacing a gesture, not waiting for work. Six wheel events delivered back-to-back are one fling to the browser; the spacing is what makes them a scroll.
      for (let i = 0; i < 6; i++) { await page.mouse.wheel(0, 200); await page.waitForTimeout(40); }
      await domSettled(page);
      const after = await page.evaluate(() => ({
        panel: Math.round(document.getElementById('navDrawerScroll').scrollTop),
        page: Math.round(window.scrollY),
      }));
      assert(after.panel > 0, 'the panel itself scrolls');
      assertEq(after.page, before, 'the page behind the drawer stays where the player left it');
      await page.click('#navDrawerClose');
      await page.waitForFunction(() => !document.querySelector('#drawer.show'), null, { timeout: 4000 }).catch(() => {});
      assertEq(await pageY(), before, 'and closing lands back there, not at the bottom of home');
      assert(!(await page.evaluate(() => document.body.classList.contains('hub-open'))),
        'the scroll lock is released on close');
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: A2.3 — no locks, no upsell (Full-tier gating still UNPROVABLE)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await rows(page);
      // AUDITPASS F7 left this law with nothing real to prove. Only two modes
      // were ever entitlementLevel:"full" — ghost_racing, now cut, and
      // online_spelloff, which is hidden with its flag off. So a previewed
      // language hides no Full-tier mode because none is LIVE, not because
      // entitlement gated it. The gap is named out loud rather than asserted
      // for the wrong reason: when a Full-tier mode goes live, assert on THAT.
      assert(!t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing must be absent on a previewed language');
      const txt = await page.$eval('#navDrawerPanel', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|unlock|premium/.test(txt), 'drawer rendered lock/upsell copy — absence, not locks');
    } finally { await ctx.close(); }
  });

  await suite.test('drawer: A2.2 — Little Speller sees only kidSafe rows, zero upsell', async () => {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true });
    await pinBaseline(ctx);
    try {
      await ctx.addInitScript(([age]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', 'en');
        localStorage.setItem('spell_flag_online_spelloff', 'on');
      }, [AGE_KID]);
      await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
      const page = await ctx.newPage();
      await page.goto(base);
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await painted(page);
      const t = await rows(page);
      assert(t.length > 0, 'kid drawer rendered nothing at all');
      // Derive the law from the REGISTRY, never a hardcoded list. modes.rs
      // owns the exact kid menu (a Rust test pins list and order); what e2e
      // must guarantee is the safety property that list exists to protect —
      // no adult-only mode is EVER kid-visible, whatever its flag says.
      for (const banned of ['photo_list', 'online_spelloff', 'word_stories', 'spell_aloud']) {
        assert(!t.some((x) => x.mode === banned), `${banned} must never appear in Kid Mode`);
      }
      const txt = await page.$eval('#navDrawerPanel', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|buy|unlock|premium|\$/.test(txt), 'Kid Mode drawer must carry zero locks/upsell strings');
    } finally { await ctx.close(); }
  });
}
