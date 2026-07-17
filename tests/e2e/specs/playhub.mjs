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
import { openApp, assert } from '../harness.mjs';

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
      // On web/en: ghost racing is the all-platform aid. say_it / photo_list /
      // spell_aloud are iOS-only, syllable_replay is es-only, word_stories is
      // hidden, online_spelloff's flag is off.
      assert(t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing missing on web/en');
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
      await page.evaluate(() => localStorage.setItem('spell_flag_online_spelloff', 'on'));
      await page.reload(); await page.waitForTimeout(600);
      const t = await tiles(page);
      const so = t.find((x) => x.mode === 'online_spelloff');
      assert(so, 'online_spelloff did not tile with its flag on');
      assert(so.kind === 'teaser', `expected a teaser, got ${so.kind}`);
      assert(so.tag === 'div', `a teaser must NOT be a button (got <${so.tag}>)`);
    } finally { await ctx.close(); }
  });

  await suite.test('hub: tiles are localized with no new copy (es)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await tiles(page);
      // es-only aid appears; and it renders Spanish from the shipped tools.*
      // catalog rather than a raw key or English.
      const syl = t.find((x) => x.mode === 'syllable_replay');
      assert(syl, 'syllable_replay missing on es');
      assert(!syl.name.startsWith('tools.'), `tile rendered a raw i18n key: ${syl.name}`);
      assert(syl.name !== 'Syllable replay', 'tile did not localize to es');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A2.3 — a Full-only mode is ABSENT on a previewed language', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      const t = await tiles(page);
      // ghost_racing needs Full; es resolves to Preview under FREE_TIER, so it is
      // absent — never a locked tile.
      assert(!t.some((x) => x.mode === 'ghost_racing'), 'ghost_racing must be absent on a previewed language');
      const txt = await page.$eval('#playHubGrid', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|unlock|premium/.test(txt), 'hub rendered lock/upsell copy — absence, not locks');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A2.2 — Little Speller sees only kidSafe tiles, zero upsell', async () => {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true });
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
      for (const x of t) {
        assert(['ghost_racing', 'syllable_replay'].includes(x.mode), `${x.mode} is not kidSafe but tiled in Kid Mode`);
      }
      const txt = await page.$eval('#playHubGrid', (e) => e.textContent.toLowerCase());
      assert(!/lock|upgrade|buy|unlock|premium|\$/.test(txt), 'Kid Mode hub must carry zero locks/upsell strings');
    } finally { await ctx.close(); }
  });
}
