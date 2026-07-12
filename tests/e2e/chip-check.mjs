// Setup-chip one-line spot-check across all 17 locales at the smallest supported
// width (375px), in the WORST case per locale: native-language word source +
// Expert difficulty + timed. If the nowrap chip text fits without overflow here,
// every lighter selection fits too. Reports overflow per locale + saves a tight
// screenshot of the chip region.
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { startServer, launch } from './harness.mjs';

const OUT = join('tests/e2e/shots', 'chip');
mkdirSync(OUT, { recursive: true });
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const LOCALES = ['en', 'es', 'fr', 'de', 'pt', 'it', 'nl', 'pl', 'sv', 'nb', 'tr', 'vi', 'ko', 'ja', 'fil', 'zh', 'th'];

const { server, base } = await startServer(8132);
const browser = await launch();
const rows = [];
for (const lang of LOCALES) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
  await ctx.addInitScript(([age, l]) => {
    localStorage.setItem('byear_agegate_v1', age);
    localStorage.setItem('spellgame.locale', l);
  }, [AGE, lang]);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(base, { waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
  // Worst case: native language word-source + Expert + timed.
  await page.click('#setupChip').catch(() => {});
  await page.selectOption('#langSel', lang).catch(() => {});      // native endonym (often longer than "English")
  await page.selectOption('#levelSel', 'expert').catch(() => {}); // longest difficulty label
  await page.selectOption('#modeSel', 'on').catch(() => {});      // timed (Quick Bee label)
  await page.click('#setupDone').catch(() => {});
  await page.waitForTimeout(200);
  const info = await page.evaluate(() => {
    const chip = document.querySelector('#setupChip');
    const txt = document.querySelector('#setupChipText');
    return {
      text: txt ? txt.textContent : '(none)',
      // nowrap text overflowing its chip => not one line. +1px tolerance for rounding.
      overflow: chip ? chip.scrollWidth > chip.clientWidth + 1 : true,
      chipW: chip ? Math.round(chip.getBoundingClientRect().width) : 0,
      textW: txt ? Math.round(txt.getBoundingClientRect().width) : 0,
    };
  });
  rows.push({ lang, ...info });
  await page.screenshot({ path: join(OUT, `${lang}.png`), clip: { x: 0, y: 150, width: 375, height: 220 } });
  await ctx.close();
}
await browser.close();
server.close();

let fails = 0;
console.log('locale | overflow | chipW/textW | text');
for (const r of rows) {
  const flag = r.overflow ? 'OVERFLOW ⚠' : 'ok       ';
  if (r.overflow) fails++;
  console.log(`${r.lang.padEnd(4)} | ${flag} | ${String(r.textW).padStart(3)}/${r.chipW} | ${r.text}`);
}
console.log(`\n${fails === 0 ? '✅ all 17 fit one line (worst case)' : `⚠ ${fails} locale(s) overflow`}`);
