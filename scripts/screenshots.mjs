// App Store / marketing screenshots via Playwright (Chromium), driving the
// built web app (dist/) at exact App Store device pixel dimensions.
//
// The age gate is bypassed by seeding a "full" verdict into localStorage before
// the app boots, so shots show gameplay rather than the first-launch DOB prompt.
//
// Usage:
//   npm run build                       # produce dist/
//   npm i -D playwright && npx playwright install chromium   # one-time
//   (cd dist && python3 -m http.server 8099 &)               # serve dist/
//   node scripts/screenshots.mjs                             # -> fastlane/screenshots/en-US
//
// App Store required sizes: 6.9"/6.7" iPhone (1290x2796) and 13" iPad (2064x2752).
import { chromium } from 'playwright';
import fs from 'fs';

const BASE = process.env.SHOT_BASE || 'http://localhost:8099/';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: Math.floor(Date.now() / 1000) });
const outDir = 'fastlane/screenshots/en-US';
fs.mkdirSync(outDir, { recursive: true });

const devices = [
  { name: 'iphone69', w: 430, h: 932, dpr: 3, mobile: true },   // -> 1290 x 2796
  { name: 'ipad13', w: 1032, h: 1376, dpr: 2, mobile: false },  // -> 2064 x 2752
];

const browser = await chromium.launch();
for (const d of devices) {
  const ctx = await browser.newContext({
    viewport: { width: d.w, height: d.h },
    deviceScaleFactor: d.dpr,
    isMobile: d.mobile,
  });
  await ctx.addInitScript(a => localStorage.setItem('byear_agegate_v1', a), AGE);
  const page = await ctx.newPage();
  await page.goto(BASE, { waitUntil: 'load' });
  // Wait for the wasm to boot (orb glyph shows "tap to hear a word").
  await page
    .waitForFunction(() => {
      const g = document.getElementById('orbGlyph');
      return g && /tap/i.test(g.textContent || '');
    }, null, { timeout: 30000 })
    .catch(() => {});
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${outDir}/${d.name}-1-play.png` });
  await page.click('#setBtn').catch(() => {});
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${outDir}/${d.name}-2-settings.png` });
  await ctx.close();
  console.log(`captured ${d.name}`);
}
await browser.close();
