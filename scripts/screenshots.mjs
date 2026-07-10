// App Store / marketing screenshots via Playwright (Chromium), driving the
// built web app (dist/) at exact App Store device pixel dimensions — one set
// per App Store locale, each in that language.
//
// The age gate is bypassed by seeding a "full" verdict into localStorage before
// boot. For each locale we seed spellgame.locale AND drive the in-app language
// selector, so the whole UI (menus, orb prompt, settings, and the per-language
// keyboard layout — AZERTY for fr, QWERTZ for de …) renders in that language.
//
// Usage:
//   npm run build
//   npm i -D playwright && npx playwright install chromium   # one-time
//   (cd dist && python3 -m http.server 8099 &)
//   node scripts/screenshots.mjs                             # -> fastlane/screenshots/<locale>/
import { chromium } from 'playwright';
import fs from 'fs';

const BASE = process.env.SHOT_BASE || 'http://localhost:8099/';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: Math.floor(Date.now() / 1000) });

// App-locale code -> App Store Connect localization directory.
const locales = [
  { code: 'en', dir: 'en-US' },
  { code: 'es', dir: 'es-ES' },
  { code: 'fr', dir: 'fr-FR' },
  { code: 'de', dir: 'de-DE' },
  { code: 'pt', dir: 'pt-BR' },
  { code: 'it', dir: 'it' },
  { code: 'nl', dir: 'nl-NL' },
  { code: 'pl', dir: 'pl' },
  { code: 'sv', dir: 'sv' },
];

const devices = [
  { name: 'iphone69', w: 430, h: 932, dpr: 3, mobile: true },   // -> 1290 x 2796
  { name: 'ipad13', w: 1032, h: 1376, dpr: 2, mobile: false },  // -> 2064 x 2752
];

const browser = await chromium.launch();
for (const loc of locales) {
  const outDir = `fastlane/screenshots/${loc.dir}`;
  fs.mkdirSync(outDir, { recursive: true });
  for (const d of devices) {
    const ctx = await browser.newContext({
      viewport: { width: d.w, height: d.h },
      deviceScaleFactor: d.dpr,
      isMobile: d.mobile,
    });
    await ctx.addInitScript(
      ([age, code]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', code); // boot UI in this language
      },
      [AGE, loc.code],
    );
    const page = await ctx.newPage();
    await page.goto(BASE, { waitUntil: 'load' });
    // Wait for wasm boot: the language selector gets populated once the app is up.
    await page
      .waitForFunction(() => document.querySelectorAll('#langSel option').length > 0, null, { timeout: 30000 })
      .catch(() => {});
    // Drive the real language switch (localizes UI + swaps the keyboard layout +
    // sets the word language) exactly as a user would.
    await page.selectOption('#langSel', loc.code).catch(() => {});
    await page.waitForTimeout(1200);
    await page.screenshot({ path: `${outDir}/${d.name}-1-play.png` });
    await page.click('#setBtn').catch(() => {});
    await page.waitForTimeout(700);
    await page.screenshot({ path: `${outDir}/${d.name}-2-settings.png` });
    await ctx.close();
  }
  console.log(`captured ${loc.dir}`);
}
await browser.close();
