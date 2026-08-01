// picture-wall.spec — CC-PICTURE-PLATFORM feature 5, the crawl half of the
// wall. Runs ONLY against the site build: it proves the *rendered* app shows
// no trace of Spell Picture, which the byte scan cannot — a string can be
// absent from the bundle yet a tile can still render from a stale registry,
// and a byte can be present yet never reach the DOM. Two different claims;
// the wall needs both green.
//
// D3 discipline carried through: the strings swept for are GENERATED from the
// repo's own locale tables (all 15 languages), never hand-listed, so a new
// translation cannot dodge the sweep.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { openApp, assert, assertEq, IS_WEB_BUILD } from '../harness.mjs';

const LOCALES = join(process.cwd(), 'src', 'i18n', 'locales');

/** Every localized Spell Picture string long enough to be distinctive. */
function pictureStrings() {
  const out = [];
  for (const f of readdirSync(LOCALES).filter((n) => n.endsWith('.json'))) {
    const t = JSON.parse(readFileSync(join(LOCALES, f), 'utf8'));
    for (const [k, v] of Object.entries(t)) {
      if (!/^(wordpic|finale|tools\.wordpic)\./.test(k)) continue;
      const s = String(v).replace(/\{\w+\}/g, '').trim();
      if (s.length >= 10) out.push({ lang: f.replace('.json', ''), key: k, s });
    }
  }
  return out;
}

export async function run(browser, base, suite) {
  if (!IS_WEB_BUILD) return; // registered web-only in run.mjs; belt and braces

  const STRINGS = pictureStrings();

  await suite.test('wall: no picture DOM, tile, or asset request in any of 15 languages', async () => {
    assert(STRINGS.length > 50, `only ${STRINGS.length} generated strings — the sweep would be toothless`);
    const langs = [...new Set(STRINGS.map((x) => x.lang))];
    assertEq(langs.length, 15, 'sweep must cover all fifteen locale tables');

    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const offenders = [];
      page.on('request', (r) => {
        if (/wordpic|spellpic|scans\.json|manifests/.test(r.url())) offenders.push(r.url());
      });
      for (const ui of langs) {
        // Drive the app's own locale mechanism, then let it re-translate.
        await page.evaluate((code) => {
          localStorage.setItem('spellgame.locale', code);
          location.reload();
        }, ui);
        await page.waitForLoadState('load');
        await page.waitForTimeout(700);

        const found = await page.evaluate((needles) => {
          const text = document.body.innerText;
          const html = document.body.innerHTML;
          const hits = [];
          for (const n of needles) if (text.includes(n.s)) hits.push(`${n.lang}:${n.key}`);
          // Structure, not just copy: no picture node may exist at all.
          for (const sel of ['#wpPlay', '#wpPicker', '.wp-screen', '[data-pic]',
                             '#wordPicOpen', '#wpReveal', '#wpGallery']) {
            if (document.querySelector(sel)) hits.push(`node ${sel}`);
          }
          if (/wp-outline|wp-pinned|scanlock/.test(html)) hits.push('picture markup in innerHTML');
          return hits;
        }, STRINGS.filter((x) => x.lang === ui || x.lang === 'en'));
        assertEq(found.length, 0, `[ui=${ui}] picture traces rendered: ${found.slice(0, 4).join(', ')}`);
      }
      assertEq(offenders.length, 0, `picture asset requests: ${offenders.slice(0, 3).join(', ')}`);
    } finally { await ctx.close(); }
  });

  await suite.test('wall: direct picture routes are 404 by absence, not by guard', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      for (const route of ['picture', 'picture/gallery', 'config/wordpic/scans.json',
                           'content-pipeline/wordpic/manifests/dog.json']) {
        const status = await page.evaluate(async (r) => (await fetch(r)).status, route);
        assertEq(status, 404, `${route} should not exist on the site`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('wall: the hub registry itself carries no app-only mode', async () => {
    // The tile list is built from the registry, so an empty DOM could mean
    // "filtered at render time" — which D1 forbids. Ask the bundle: the
    // stripped registry must not even know the mode's id.
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const leak = await page.evaluate(async () => {
        const r = await fetch('pkg/spell_wasm_bg.wasm');
        const buf = new Uint8Array(await r.arrayBuffer());
        const hay = new TextDecoder('latin1').decode(buf);
        return ['word_picture', 'wordPicOpen'].filter((n) => hay.includes(n));
      });
      assertEq(leak.length, 0, `wasm still names: ${leak.join(', ')}`);
    } finally { await ctx.close(); }
  });
}
