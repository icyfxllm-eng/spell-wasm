// Regenerate App Store screenshots showing the NEW grouped home UI, at Apple's
// exact required pixel sizes, matching the existing set's device classes,
// per-locale layout, and filenames. Outputs to a staging dir for review.
//   node tests/e2e/appstore-shots.mjs
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { startServer, launch } from './harness.mjs';

const OUT = 'tests/e2e/shots/appstore';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

// spellgame.locale -> fastlane screenshots/ dir
const LOCALES = { de: 'de-DE', en: 'en-US', es: 'es-ES', fr: 'fr-FR', it: 'it', nl: 'nl-NL', pl: 'pl', pt: 'pt-BR', sv: 'sv' };

// device class -> viewport (CSS px) + dpr so width*dpr × height*dpr = required px
const DEVICES = {
  iphone69: { width: 430, height: 932, dpr: 3 },   // 1290 × 2796
  ipad13: { width: 1032, height: 1376, dpr: 2 },   // 2064 × 2752
};

const { server, base } = await startServer(8134);
const browser = await launch();
let n = 0;
for (const [loc, dir] of Object.entries(LOCALES)) {
  mkdirSync(join(OUT, dir), { recursive: true });
  for (const [dev, d] of Object.entries(DEVICES)) {
    for (const [idx, screen] of [[1, 'play'], [2, 'settings']]) {
      const ctx = await browser.newContext({
        viewport: { width: d.width, height: d.height }, deviceScaleFactor: d.dpr,
        isMobile: dev.startsWith('iphone'),
      });
      await ctx.addInitScript(([age, l]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', l);
      }, [AGE, loc]);
      await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
      const page = await ctx.newPage();
      await page.goto(base, { waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(300);
      if (screen === 'settings') {
        await page.click('#setBtn').catch(() => {});
        await page.waitForTimeout(350); // scrim fade
      }
      await page.screenshot({ path: join(OUT, dir, `${dev}-${idx}-${screen}.png`) }); // viewport = exact required px
      await ctx.close();
      n++;
    }
  }
  console.log(`  ${dir}: 4 shots`);
}
await browser.close();
server.close();
console.log(`\nwrote ${n} screenshots to ${OUT}`);
