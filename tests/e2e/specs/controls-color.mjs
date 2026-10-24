// controls-color.spec — Settings > Controls (Eric, build 289 review: "any way
// those areas can be color customizable as well").
//
// One pick tints the menu button, the setup bar, the answer box and Delete
// (their panel and edge are mixed from --controls-color). The default is
// Eclipse's neutral ramp (CC-ECLIPSE-HOTFIX S1: neutral default, picks still tint); a pick survives a reload; the text on those panels stays
// light, so it stays readable whatever the pick.
import { openApp, assert } from '../harness.mjs';

const look = (page) => page.evaluate(() => {
  const cs = (id) => getComputedStyle(document.getElementById(id));
  const burger = document.querySelector('.burger');
  return {
    box: cs('spellbox').borderTopColor,
    boxBg: cs('spellbox').backgroundColor,
    chip: cs('setupChip').borderTopColor,
    del: cs('kbBackspace').borderTopColor,
    burger: burger ? getComputedStyle(burger).borderTopColor : null,
    chipText: getComputedStyle(document.querySelector('.setup-chip-text')).color,
  };
});
// color-mix() computes to color(srgb r g b) on a 0-1 scale.
const rgb = (v) => (v.match(/[\d.]+/g) || []).slice(0, 3).map(Number).map((n) => (v.startsWith('color(') ? n * 255 : n));
const near = (a, b, tol = 3) => rgb(a).every((x, i) => Math.abs(x - b[i]) <= tol);

export async function run(browser, base, suite) {
  await suite.test('Controls colour: the default is the neutral ramp (raised panel #1C1C1F)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const r = await look(page);
      // 70% #52525b over #0a0a0b = (60, 60, 67); 25% = (28, 28, 31), F5's #1C1C1F.
      assert(near(r.box, [60, 60, 67]), `answer box edge ${r.box}`);
      assert(near(r.boxBg, [28, 28, 31]), `answer box panel ${r.boxBg}`);
      assert(r.box === r.chip && r.chip === r.del, `the controls disagree: ${JSON.stringify(r)}`);
    } finally { await ctx.close(); }
  });

  await suite.test('Controls colour: a swatch tints all four controls and survives a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => document.querySelector('#ctlPick .theme-swatch[data-c="#2fae95"]').click());
      // The answer box eases its border colour; mid-transition it computes to
      // an oklab() blend. Wait for the transition to land.
      await page.waitForFunction(() => !/oklab/.test(getComputedStyle(document.getElementById('spellbox')).borderTopColor), null, { timeout: 3000 });
      const r = await look(page);
      // 70% teal #2fae95 over #0a0a0b = (36, 125, 108).
      assert(near(r.box, [36, 125, 108]), `answer box edge after teal ${r.box}`);
      for (const k of ['chip', 'del', 'burger']) if (r[k]) assert(r[k] === r.box, `${k} did not follow: ${r[k]} vs ${r.box}`);
      assert(await page.evaluate(() => document.querySelector('#ctlPick .theme-swatch[data-c="#2fae95"]').classList.contains('active')), 'the teal swatch is not marked active');
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForFunction(() => !/oklab/.test(getComputedStyle(document.getElementById('spellbox')).borderTopColor), null, { timeout: 3000 });
      const after = await look(page);
      assert(after.box === r.box, `the pick did not survive a reload: ${after.box}`);
    } finally { await ctx.close(); }
  });

  await suite.test('Controls colour: a custom white pick keeps the setup bar text readable', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => { const i = document.getElementById('ctlColorInput'); i.value = '#ffffff'; i.dispatchEvent(new Event('input')); });
      const r = await look(page);
      const lum = (c) => { const [R, G, B] = rgb(c).map((x) => { x /= 255; return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; }); return 0.2126 * R + 0.7152 * G + 0.0722 * B; };
      const bgL = lum(await page.$eval('#setupChip', (e) => getComputedStyle(e).backgroundColor));
      const fgL = lum(r.chipText);
      const ratio = (Math.max(bgL, fgL) + 0.05) / (Math.min(bgL, fgL) + 0.05);
      assert(ratio >= 4.5, `setup bar text contrast ${ratio.toFixed(2)} on a white pick`);
    } finally { await ctx.close(); }
  });

  // --- Settings > Buttons ---
  const btn = (page) => page.evaluate(() => {
    const cs = (id) => getComputedStyle(document.getElementById(id));
    return { check: cs('checkBtn').backgroundColor, done: cs('setDone').backgroundColor, ink: cs('checkBtn').color,
      checkSize: cs('checkBtn').fontSize, doneSize: cs('setDone').fontSize };
  });

  await suite.test('Buttons colour: unset, primary buttons are the amber accent with dark text', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const r = await btn(page);
      assert(near(r.check, [255, 177, 77]) && r.done === r.check, `default fill ${JSON.stringify(r)}`);
      assert(rgb(r.ink).every((x) => x < 80), `default text is not dark: ${r.ink}`);
      assert(r.checkSize === '16.5px' && r.doneSize !== '16.5px', `only Check gets the big label: ${r.checkSize} / ${r.doneSize}`);
    } finally { await ctx.close(); }
  });

  await suite.test('Buttons colour: a swatch fills Check and Done, a dark pick flips the text light, and it survives a reload', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(() => document.querySelector('#btnPick .theme-swatch[data-c="#46d6b8"]').click());
      let r = await btn(page);
      assert(near(r.check, [70, 214, 184]) && r.done === r.check, `teal did not reach both: ${JSON.stringify(r)}`);
      await page.evaluate(() => { const i = document.getElementById('btnColorInput'); i.value = '#1a2a6c'; i.dispatchEvent(new Event('input')); });
      r = await btn(page);
      assert(near(r.check, [26, 42, 108]), `custom fill ${r.check}`);
      assert(rgb(r.ink).every((x) => x > 200), `text on a dark fill is not light: ${r.ink}`);
      await page.reload({ waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      assert(near((await btn(page)).check, [26, 42, 108]), 'the pick did not survive a reload');
    } finally { await ctx.close(); }
  });

  await suite.test('Buttons colour: unset, the Japanese accent still reaches the buttons', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'ja' });
    try {
      const r = await btn(page);
      // ja overrides color.accent to #ff8fab.
      assert(near(r.check, [255, 143, 171]), `ja buttons are ${r.check}, not the ja accent`);
    } finally { await ctx.close(); }
  });
}

