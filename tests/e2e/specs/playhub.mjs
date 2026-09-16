// playhub.spec — CC-MODE-HUB F2. The Play hub renders ONLY from
// config/modes.json, and every rule that decides a tile lives in one pure Rust
// function (modes::visible). These tests exercise the rendered result, which is
// the part unit tests cannot see: that the registry actually reaches the DOM,
// localized, with the right element type per tile kind.
//
// Tile kind is carried by ELEMENT TYPE, deliberately:
//   <button class="mode-tile">        a real destination (routes to the mode's
//                                     own entry point)
//   <div class="mode-tile info">      an in-round aid — no destination
//   <div class="mode-tile teaser">    coming_soon (no notify-me hook, D7)
// A tappable tile that goes nowhere is a lie the markup cannot tell.
import { openApp, assert, assertEq, pinBaseline } from '../harness.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

/** Open the hub and describe every tile. */
async function tiles(page) {
  await page.click('#playHubBtn');
  await page.waitForTimeout(300);
  return page.$$eval('#playHubGrid .mode-tile', (els) =>
    els.map((e) => ({
      mode: e.dataset.mode,
      tag: e.tagName.toLowerCase(),
      kind: e.classList.contains('teaser') ? 'teaser' : e.classList.contains('info') ? 'info' : 'launcher',
      name: (e.querySelector('.mt-name') || {}).textContent || '',
    })),
  );
}

export async function run(browser, base, suite) {
  await suite.test('hub: opens from the meta corner and renders registry tiles', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const t = await tiles(page);
      const shown = await page.$eval('#playHub', (e) => e.classList.contains('show'));
      assert(shown, 'hub did not open');
      assert(t.length > 0, 'hub rendered no tiles');
      // AUDITPASS F7 / D2: ghost_racing is CUT — status:hidden in the
      // registry, so it tiles nowhere for anyone. It used to be this
      // spec's proof that an all-platform aid reaches the hub; def_match
      // carries that now. say_it / photo_list / spell_aloud are iOS-only,
      // syllable_replay is es-only, word_stories is hidden, and
      // online_spelloff's flag is off.
      assert(!t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing is cut and must not tile');
      assert(t.some((x) => x.mode === 'def_match'), 'an all-platform mode still reaches the hub');
      for (const iosOnly of ['say_it', 'photo_list', 'spell_aloud']) {
        assert(!t.some((x) => x.mode === iosOnly), `${iosOnly} is iOS-only and must not tile on web`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('hub: word_stories is never rendered (F8 hard gate)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // Even with its flag forced ON, `hidden` must win: the spec proposal in
      // docs/word-stories-review.md is not approved.
      await page.evaluate(() => localStorage.setItem('spell_flag_word_stories', 'on'));
      await page.reload(); await page.waitForTimeout(600);
      const t = await tiles(page);
      assert(!t.some((x) => x.mode === 'word_stories'), 'word_stories tiled despite status:hidden');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: coming_soon renders as a non-tappable teaser', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // CC-HUB-CLEANUP D5 RETIRED the hub teaser: Spell-Off keeps its
      // own entry point, and `hidden` beats a flag. The teaser
      // SEMANTICS stay tested in modes.rs for whenever the hub wants
      // them back; what the hub must guarantee now is that no hidden
      // mode can be flagged back onto it.
      await page.evaluate(() => localStorage.setItem('spell_flag_online_spelloff', 'on'));
      await page.reload(); await page.waitForTimeout(600);
      const t = await tiles(page);
      assert(!t.some((x) => x.mode === 'online_spelloff'),
        'a hidden mode tiled because its flag was on — `hidden` must win');
      // and nothing that DOES tile is a dead teaser
      assert(t.every((x) => x.kind !== 'teaser'),
        'a non-tappable teaser reached the hub after D5 retired them');
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

  await suite.test('hub: tiles are localized with no new copy (es)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await tiles(page);
      // The law is "tiles are localized with no NEW copy" — every tile
      // renders from the shipped tools.* catalog. (This used to lean on
      // syllable_replay, which the registry has since marked hidden;
      // pinning a law to one mode is how it rotted.)
      assert(t.length > 0, 'es hub rendered no tiles');
      for (const x of t) {
        assert(!x.name.startsWith('tools.'), `tile rendered a raw i18n key: ${x.name}`);
        assert(x.name.trim().length > 0, `tile ${x.mode} rendered an empty name`);
      }
      // at least one tile differs from its English string — proof the
      // es catalog is actually reached, not silently falling back
      const enNames = { practice: 'Practice', def_match: 'Definition Match' };
      const localized = t.some((x) => enNames[x.mode] && x.name !== enNames[x.mode]);
      assert(localized, 'no tile localized to es — the catalog is not being reached');
    } finally { await ctx.close(); }
  });

  // The hub is taller than a phone screen. It used to let the wheel scroll the
  // PAGE behind it, so closing dropped the player at the bottom of home.
  await suite.test('hub: scrolling the hub never scrolls the page behind it', async () => {
    // Short viewport so the hub's own content overflows, as it does on a phone.
    const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: 375, height: 320 } });
    try {
      const pageY = () => page.evaluate(() => Math.round(window.scrollY));
      const before = await pageY();
      await page.click('#playHubBtn');
      await page.waitForSelector('#playHub.show', { timeout: 4000 });
      const overflows = await page.evaluate(() => {
        const h = document.getElementById('playHub');
        return h.scrollHeight > h.clientHeight;
      });
      assert(overflows, 'this viewport must make the hub overflow, or the test proves nothing');
      await page.mouse.move(180, 200);
      for (let i = 0; i < 6; i++) { await page.mouse.wheel(0, 200); await page.waitForTimeout(40); }
      await page.waitForTimeout(250);
      const after = await page.evaluate(() => ({
        hub: Math.round(document.getElementById('playHub').scrollTop),
        page: Math.round(window.scrollY),
      }));
      assert(after.hub > 0, 'the hub itself scrolls');
      assertEq(after.page, before, 'the page behind the hub stays where the player left it');
      await page.click('#playHubClose');
      await page.waitForTimeout(250);
      assertEq(await pageY(), before, 'and closing lands back there, not at the bottom of home');
      assert(!(await page.evaluate(() => document.body.classList.contains('hub-open'))),
        'the scroll lock is released on close');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A2.3 — Full-only gating (UNPROVABLE while no live mode is Full-tier)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await tiles(page);
      // AUDITPASS F7 left this spec with nothing real to prove. Only two
      // modes were ever entitlementLevel:"full" — ghost_racing, now cut,
      // and online_spelloff, which is status:hidden with its flag off.
      // So a previewed language hides no Full-tier mode because none is
      // LIVE, not because entitlement gated it. Asserting ghost's absence
      // here would pass for the wrong reason, which is the exact failure
      // this audit kept finding. The gap is named out loud instead: when
      // a Full-tier mode goes live again, assert on THAT and delete this.
      assert(!t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing must be absent on a previewed language');
      const txt = await page.$eval('#playHubGrid', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|unlock|premium/.test(txt), 'hub rendered lock/upsell copy — absence, not locks');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A2.2 — Little Speller sees only kidSafe tiles, zero upsell', async () => {
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
      await page.goto(base); await page.waitForTimeout(800);
      const t = await tiles(page);
      assert(t.length > 0, 'kid hub rendered nothing at all');
      // Derive the law from the REGISTRY, never a hardcoded list: a kid
      // may see exactly the live, kidSafe, web-platform modes. (The old
      // two-name allowlist predated D5's menu and quietly failed for
      // months once `practice` became kidSafe.)
      // The registry is compiled into the wasm, so the invariant is
      // asserted from the OTHER side: modes.rs owns the exact kid menu
      // (a Rust test pins it, list and order). What e2e must guarantee
      // is the safety property that list exists to protect — no
      // adult-only mode is EVER kid-visible, whatever its flag says.
      for (const banned of ['say_it', 'photo_list', 'online_spelloff', 'word_stories', 'spell_aloud']) {
        assert(!t.some((x) => x.mode === banned), `${banned} must never tile in Kid Mode`);
      }
      // and every tile the kid does see is tappable — no locked upsell
      assert(t.every((x) => x.kind !== 'teaser' && x.kind !== 'locked'),
        'Kid Mode showed a locked or teaser tile — zero upsell (A2.2)');
      const txt = await page.$eval('#playHubGrid', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|buy|unlock|premium|\$/.test(txt), 'Kid Mode hub must carry zero locks/upsell strings');
    } finally { await ctx.close(); }
  });
}
