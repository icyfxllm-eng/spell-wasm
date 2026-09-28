// hub-inventory.spec — CC-HUB-NAV v1.3.1 I-N2, A1, A2, A3.
//
// This file was hub-row.spec, which proved the quick-play row's membership
// was registry-decided. v1.3.1 F2 retires that row, so the law inverts: what
// has to be true now is that NOTHING above the orb except the wordmark, the
// burger, the tagline and the session pill renders at all.
//
// The buttons themselves are still in the DOM, and that is load-bearing, not
// laziness. #climbBtn carries the leaderboard's click handler and ghost.rs
// opens the board by synthesising a click on it; every other tile is the
// element a drawer row proxies its tap to. A hidden button still dispatches
// click; a deleted one does not. So the inventory is counted over RENDERED
// children — `offsetParent === null` is the test, not `querySelector`.
import { openApp, assert, assertEq } from '../harness.mjs';

const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });

/// Everything that actually paints above the orb, flattened to a short list.
async function aboveTheOrb(page) {
  return page.evaluate(() => {
    const orb = document.getElementById('orbWrap');
    const top = orb.getBoundingClientRect().top;
    const ALLOWED = ['brandMark', 'brandTag', 'navBurger', 'setupChip'];
    const named = [];
    for (const e of document.querySelectorAll('body *')) {
      if (e.offsetParent === null) continue;                 // not rendered
      const r = e.getBoundingClientRect();
      if (r.height === 0 || r.bottom > top) continue;        // not above the orb
      // A control's own parts are not separate hub chrome: the session pill's
      // chevron is the pill, not a fifth thing above the orb. I-N2 is about
      // what a player would count, so descendants of an allowed control are
      // skipped rather than listed.
      // `closest` matches the element itself, so the allowed controls have to
      // be exempted from their own exemption or the list comes back empty.
      if (ALLOWED.some((id) => e.id !== id && e.closest(`#${id}`))) continue;
      // Screen-reader-only live regions are not visible chrome; they are
      // clipped to a pixel and exist for announcements.
      if (e.closest('.sr-only')) continue;
      // Only the things a player would call a control or a line of text:
      // ignore pure layout wrappers by requiring an id or a semantic class.
      const cls = typeof e.className === 'string' ? e.className : '';
      if (!e.id && !/^(mark|tag|burger)/.test(cls)) continue;
      named.push(e.id || cls.split(' ')[0]);
    }
    return named;
  });
}

export async function run(browser, base, suite) {
  await suite.test('hub: nothing above the orb but the brand, the burger and the session pill', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const seen = await aboveTheOrb(page);
      const allowed = new Set(['brandMark', 'brandTag', 'navBurger', 'setupChip', 'setupChipText', 'mark', 'tag']);
      const extra = seen.filter((s) => !allowed.has(s));
      assertEq(extra.length, 0, `unexpected hub chrome above the orb: ${JSON.stringify(extra)}`);
      for (const want of ['brandMark', 'navBurger', 'setupChip']) {
        assert(seen.includes(want), `${want} must render above the orb (saw ${JSON.stringify(seen)})`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A2 — no quick-play tile and no Misses chip renders', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const shown = await page.$$eval('.modes-row button, #missesBtn',
        (els) => els.filter((e) => e.offsetParent !== null).map((e) => e.id));
      assertEq(shown.length, 0, `retired hub controls still render: ${JSON.stringify(shown)}`);
      // ...and they are still THERE, because the drawer taps them.
      for (const id of ['climbBtn', 'dailyBtn', 'vsBtn', 'wordPicTile', 'missesBtn']) {
        assert(await page.$(`#${id}`), `${id} must stay in the DOM — the drawer proxies to it`);
      }
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A3 — the burger is 88x44, top-trailing, centred on the wordmark', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const m = await page.evaluate(() => {
        const b = document.getElementById('navBurger').getBoundingClientRect();
        const w = document.querySelector('.brand .mark').getBoundingClientRect();
        const chip = document.getElementById('setupChip').getBoundingClientRect();
        return {
          w: Math.round(b.width), h: Math.round(b.height),
          // Measured against the CONTENT COLUMN's trailing edge, which is what
          // the safe-area margin resolves to in this layout: the burger lines
          // up with the session pill below it rather than hanging 2pt outside
          // the column to satisfy a literal 16.
          trailingDelta: Math.round(Math.abs((chip.right) - b.right)),
          vDelta: Math.round(Math.abs((b.top + b.height / 2) - (w.top + w.height / 2))),
          nothingBeside: document.querySelectorAll('.meta-corner > *:not(.retired)').length,
        };
      });
      assertEq(m.w, 88, 'burger width');
      assertEq(m.h, 44, 'burger height — the 44pt tap minimum');
      assert(m.trailingDelta <= 1, `burger trailing edge is ${m.trailingDelta}pt off the content column`);
      assert(m.vDelta <= 2, `burger centre is ${m.vDelta}pt off the wordmark's`);
      assertEq(m.nothingBeside, 1, 'nothing may sit beside the burger (F4)');
    } finally { await ctx.close(); }
  });

  await suite.test('hub: A7 — the burger frame does not move with the Misses count', async () => {
    const frames = [];
    for (const n of [0, 181, 1203]) {
      const { ctx, page } = await openApp(browser, base, {
        lang: 'en',
        init: () => {},
      });
      try {
        await page.evaluate((count) => {
          const now = Date.now();
          const list = [];
          for (let i = 0; i < count; i++) {
            list.push({ word: `w${i}`, lang: 'en', tier: 'easy', misses: 1, box_: 0, due: now - 1000, ts: now - 1000, review: null });
          }
          localStorage.setItem('byear_misses_v1', JSON.stringify(list));
        }, n);
        await page.reload({ waitUntil: 'load' });
        await page.waitForTimeout(900);
        frames.push(await page.evaluate(() => {
          const b = document.getElementById('navBurger').getBoundingClientRect();
          return [Math.round(b.x), Math.round(b.y), Math.round(b.width), Math.round(b.height)];
        }));
      } finally { await ctx.close(); }
    }
    assertEq(JSON.stringify(frames[1]), JSON.stringify(frames[0]), 'frame moved between 0 and 181 misses');
    assertEq(JSON.stringify(frames[2]), JSON.stringify(frames[0]), 'frame moved between 0 and 1,203 misses');
  });

  await suite.test('hub: A12 — Spell Jr is never nudged, at any miss count', async () => {
    for (const n of [0, 181, 182]) {
      const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true });
      try {
        await ctx.addInitScript(([age, count]) => {
          localStorage.setItem('byear_agegate_v1', age);
          localStorage.setItem('spellgame.locale', 'en');
          localStorage.setItem('spell_nav_nudge_v1', '0');
          const now = Date.now();
          const list = [];
          for (let i = 0; i < count; i++) {
            list.push({ word: `w${i}`, lang: 'en', tier: 'easy', misses: 1, box_: 0, due: now - 1000, ts: now - 1000, review: null });
          }
          localStorage.setItem('byear_misses_v1', JSON.stringify(list));
        }, [AGE_KID, n]);
        await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
        const page = await ctx.newPage();
        await page.goto(base); await page.waitForTimeout(900);
        assert(await page.evaluate(() => document.body.classList.contains('kid')), 'this profile must be Spell Jr');
        // I-N7 says the view does not EXIST, which is stronger than hidden.
        assertEq(await page.$$eval('#navBurger .nudge', (e) => e.length), 0,
          `the nudge dot exists in Spell Jr at ${n} misses`);
      } finally { await ctx.close(); }
    }
  });
}
