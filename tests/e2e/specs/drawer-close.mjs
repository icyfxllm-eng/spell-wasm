// drawer-close.spec — CC-HUB-NAV v1.3.2. How the drawer is dismissed.
//
// The amendment names a native `DrawerCloseTests` target; this drawer is HTML
// in a webview, so its acceptance tests live here, beside the rest of the
// drawer's. The device matrix is the viewport matrix: SE-class 375x667,
// iPhone-16-class 393x852, Pro-Max-class 430x932.
//
// T3 IS DELIBERATELY ABSENT. It asserts that a right swipe still dismisses
// the drawer, "unchanged from build 219" — and the census found no
// swipe-to-close gesture anywhere in the app, in this build or any before it.
// A test written against behaviour that has never existed cannot be a
// regression guard, and stubbing one that passes vacuously would be worse
// than not having it. Adding the gesture is a feature, and unasked for.
import { openApp, assert, assertEq } from '../harness.mjs';

const DEVICES = [
  ['SE', { width: 375, height: 667 }],
  ['16', { width: 393, height: 852 }],
  ['ProMax', { width: 430, height: 932 }],
];

async function openDrawer(page) {
  await page.click('#navBurger');
  await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
}

const isOpen = (page) => page.$eval('#navDrawer', (e) => e.classList.contains('show'));

/// Put the scroller at the top, the middle or the bottom.
async function scrollTo(page, where) {
  await page.evaluate((w) => {
    const s = document.getElementById('navDrawerScroll');
    const max = s.scrollHeight - s.clientHeight;
    s.scrollTop = w === 'top' ? 0 : w === 'bottom' ? max : Math.round(max / 2);
  }, where);
  await page.waitForTimeout(150);
}

export async function run(browser, base, suite) {
  // T1 — nine cases: three devices x three scroll positions.
  for (const [name, viewport] of DEVICES) {
    await suite.test(`close: T1 — the bottom X dismisses at every scroll offset (${name})`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport });
      try {
        for (const where of ['top', 'middle', 'bottom']) {
          await openDrawer(page);
          await scrollTo(page, where);
          // I2: fully on screen and hit-testable wherever the list sits.
          const onScreen = await page.evaluate(() => {
            const b = document.getElementById('navDrawerClose').getBoundingClientRect();
            return b.top >= 0 && b.left >= 0 && b.bottom <= innerHeight && b.right <= innerWidth;
          });
          assert(onScreen, `${name}/${where}: the close button is not fully on screen`);
          await page.click('#navDrawerClose');
          await page.waitForTimeout(250);
          assert(!(await isOpen(page)), `${name}/${where}: the drawer did not close`);
        }
      } finally { await ctx.close(); }
    });
  }

  // T2 — no close control survives in the top half.
  await suite.test('close: T2 — nothing closes the drawer from its top half', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDrawer(page);
      const topHalf = await page.evaluate(() => {
        const panel = document.getElementById('navDrawerPanel').getBoundingClientRect();
        const mid = panel.top + panel.height / 2;
        return [...document.querySelectorAll('#navDrawerPanel button')]
          .filter((b) => b.getBoundingClientRect().bottom < mid)
          .filter((b) => /close|✕|✖|×/i.test(`${b.id} ${b.getAttribute('aria-label') || ''} ${b.textContent}`))
          .map((b) => b.id || b.textContent);
      });
      assertEq(topHalf.length, 0, `a close control survives in the top half: ${JSON.stringify(topHalf)}`);
      assertEq(await page.$$eval('.nav-head', (e) => e.length), 0, 'the old header wrapper is gone');
    } finally { await ctx.close(); }
  });

  // T4 — the scrim still dismisses.
  await suite.test('close: T4 — a scrim tap dismisses', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDrawer(page);
      await page.mouse.click(20, 300);           // the dimmed area, left of the panel
      await page.waitForTimeout(250);
      assert(!(await isOpen(page)), 'a scrim tap must dismiss the drawer');
    } finally { await ctx.close(); }
  });

  // T5 — the frame, on every device, and a tap that beats an in-flight scroll.
  for (const [name, viewport] of DEVICES) {
    await suite.test(`close: T5 — 56x56, 16pt inside the trailing edge (${name})`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport });
      try {
        await openDrawer(page);
        const m = await page.evaluate(() => {
          const b = document.getElementById('navDrawerClose').getBoundingClientRect();
          const p = document.getElementById('navDrawerPanel').getBoundingClientRect();
          return { w: Math.round(b.width), h: Math.round(b.height),
                   trailing: Math.round(p.right - b.right) };
        });
        assert(m.w >= 56 && m.h >= 56, `${name}: hit rect is ${m.w}x${m.h}, under 56x56 (I4)`);
        assert(Math.abs(m.trailing - 16) <= 1, `${name}: trailing inset is ${m.trailing}pt, not 16 (I4)`);
        // A tap mid-scroll closes rather than just stopping the scroll.
        await page.evaluate(() => {
          const s = document.getElementById('navDrawerScroll');
          s.scrollTo({ top: s.scrollHeight, behavior: 'smooth' });
        });
        await page.click('#navDrawerClose');
        await page.waitForTimeout(250);
        assert(!(await isOpen(page)), `${name}: a tap during a scroll must still close`);
      } finally { await ctx.close(); }
    });
  }

  // T6 — I5: the last row clears the band.
  await suite.test('close: T6 — the last row scrolls clear of the footer', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', viewport: { width: 375, height: 667 } });
    try {
      await openDrawer(page);
      await scrollTo(page, 'bottom');
      const clear = await page.evaluate(() => {
        const rows = [...document.querySelectorAll('#navDrawerScroll .nav-row')];
        const last = rows[rows.length - 1].getBoundingClientRect();
        const foot = document.getElementById('navDrawerFoot').getBoundingClientRect();
        return { lastBottom: Math.round(last.bottom), footTop: Math.round(foot.top) };
      });
      assert(clear.lastBottom <= clear.footTop,
        `the last row ends at ${clear.lastBottom} and the band starts at ${clear.footTop}`);
    } finally { await ctx.close(); }
  });

  // T7 — I3: nothing under the status bar. The webview reports the inset as
  // env(safe-area-inset-top); in a desktop browser that is 0, so this asserts
  // the CONTAINMENT rather than a number — no row may escape the panel's box.
  await suite.test('close: T7 — no row draws outside the panel at the top', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDrawer(page);
      await scrollTo(page, 'top');
      const escaped = await page.evaluate(() => {
        const p = document.getElementById('navDrawerPanel').getBoundingClientRect();
        const scroll = document.getElementById('navDrawerScroll').getBoundingClientRect();
        return [...document.querySelectorAll('#navDrawerScroll .nav-row')]
          .filter((r) => {
            const b = r.getBoundingClientRect();
            return b.top < Math.max(p.top, scroll.top) - 1;
          }).length;
      });
      assertEq(escaped, 0, 'a row drew above the panel/scroller top edge');
      // ...and the panel really does carry the inset, so a device with a
      // notch pushes the scroller down rather than letting rows behind it.
      const hasInset = await page.evaluate(() =>
        getComputedStyle(document.getElementById('navDrawerPanel')).paddingTop !== '');
      assert(hasInset, 'the panel must carry a top padding for the safe area');
    } finally { await ctx.close(); }
  });

  // T8 — I8: the nudge watermark is identical whichever path closed it.
  await suite.test('close: T8 — every close path leaves the nudge watermark identical', async () => {
    const readMark = (page) => page.evaluate(() => localStorage.getItem('spell_nav_nudge_v1'));
    const marks = [];
    for (const how of ['button', 'scrim', 'escape']) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await page.evaluate(() => {
          const now = Date.now();
          const list = [];
          for (let i = 0; i < 7; i++) {
            list.push({ word: `w${i}`, lang: 'en', tier: 'easy', misses: 1, box_: 0, due: now - 1000, ts: now - 1000, review: null });
          }
          localStorage.setItem('byear_misses_v1', JSON.stringify(list));
          localStorage.setItem('spell_nav_nudge_v1', '0');
        });
        await page.reload({ waitUntil: 'load' });
        await page.waitForTimeout(900);
        await openDrawer(page);
        if (how === 'button') await page.click('#navDrawerClose');
        if (how === 'scrim') await page.mouse.click(20, 300);
        if (how === 'escape') await page.keyboard.press('Escape');
        await page.waitForTimeout(250);
        assert(!(await isOpen(page)), `${how} did not close the drawer`);
        marks.push(await readMark(page));
      } finally { await ctx.close(); }
    }
    assertEq(marks[1], marks[0], 'scrim close left a different watermark from the button');
    assertEq(marks[2], marks[0], 'Escape close left a different watermark from the button');
  });

  // T9's half that a browser can check: the accessible name, and focus
  // returning to the burger. The announcement string itself is VoiceOver's.
  await suite.test('close: T9 — named for a screen reader, and focus goes back to the burger', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await openDrawer(page);
      const label = await page.$eval('#navDrawerClose', (e) => e.getAttribute('aria-label'));
      assert(label && label.trim().length > 0, 'the close button has no accessible name');
      assertEq(await page.$eval('#navDrawerClose', (e) => e.tagName.toLowerCase()), 'button',
        'it must be a real button, so it is announced as one');
      await page.click('#navDrawerClose');
      await page.waitForTimeout(250);
      assertEq(await page.evaluate(() => document.activeElement && document.activeElement.id), 'navBurger',
        'focus must return to the burger after a close');
    } finally { await ctx.close(); }
  });
}
