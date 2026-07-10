// Layout regression test for the on-screen keyboard (Task 5).
// Asserts, across languages / modes / viewport widths:
//   1. every .kb-key has an identical bounding-box width and height,
//   2. the Delete key sits to the RIGHT of the Enter key and near the right edge,
//   3. no key overflows the keyboard container.
// Presentation-only: does not touch key event payloads.
//
// Usage: (cd dist && python3 -m http.server 8099 &) ; node scripts/kb-layout-check.mjs
import { chromium } from 'playwright';

const BASE = process.env.SHOT_BASE || 'http://localhost:8099/';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: Math.floor(Date.now() / 1000) });
const langs = ['en', 'fr', 'de', 'sv']; // QWERTY(10), AZERTY(10), QWERTZ(11), Swedish(11)
const modes = ['standard', 'kid', 'big-text'];
const widths = [320, 390, 430];

const browser = await chromium.launch();
let failures = 0;
const fail = (m) => { failures++; console.error('  ✗ ' + m); };

for (const w of widths) {
  for (const lang of langs) {
    for (const mode of modes) {
      const ctx = await browser.newContext({ viewport: { width: w, height: 900 }, deviceScaleFactor: 2, isMobile: true });
      await ctx.addInitScript(([age, code]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', code);
      }, [AGE, lang]);
      const page = await ctx.newPage();
      await page.goto(BASE, { waitUntil: 'load' });
      await page.waitForFunction(() => document.querySelectorAll('#langSel option').length > 0, null, { timeout: 30000 }).catch(() => {});
      await page.selectOption('#langSel', lang).catch(() => {});
      if (mode !== 'standard') await page.evaluate((m) => document.body.classList.add(m), mode);
      await page.waitForTimeout(300);

      const r = await page.evaluate(() => {
        const keys = [...document.querySelectorAll('#gameKeyboard .kb-key')].filter((k) => k.offsetParent !== null);
        const rects = keys.map((k) => k.getBoundingClientRect());
        const kb = document.getElementById('gameKeyboard').getBoundingClientRect();
        const back = document.getElementById('kbBackspace').getBoundingClientRect();
        const enter = document.getElementById('kbSubmit').getBoundingClientRect();
        return {
          n: rects.length,
          minW: Math.min(...rects.map((x) => x.width)), maxW: Math.max(...rects.map((x) => x.width)),
          minH: Math.min(...rects.map((x) => x.height)), maxH: Math.max(...rects.map((x) => x.height)),
          overflowL: Math.min(...rects.map((x) => x.left)), overflowR: Math.max(...rects.map((x) => x.right)),
          kbL: kb.left, kbR: kb.right, backL: back.left, enterR: enter.right, backR: back.right,
        };
      });
      const tag = `${w}px ${lang}/${mode}`;
      if (r.maxW - r.minW > 0.6) fail(`${tag}: key widths differ (${r.minW.toFixed(1)}–${r.maxW.toFixed(1)})`);
      if (r.maxH - r.minH > 0.6) fail(`${tag}: key heights differ (${r.minH.toFixed(1)}–${r.maxH.toFixed(1)})`);
      if (r.backL < r.enterR - 1) fail(`${tag}: Delete not right of Enter (backL=${r.backL.toFixed(0)} enterR=${r.enterR.toFixed(0)})`);
      if (r.backR > r.kbR + 1) fail(`${tag}: Delete overflows right edge`);
      if (r.overflowL < r.kbL - 1 || r.overflowR > r.kbR + 1) fail(`${tag}: a key overflows the keyboard container`);
      await ctx.close();
    }
  }
}
await browser.close();
if (failures) { console.error(`\nkb-layout-check: ${failures} failure(s).`); process.exit(1); }
console.log(`kb-layout-check: OK — uniform keys + right-side Delete across ${langs.length} langs × ${modes.length} modes × ${widths.length} widths.`);
