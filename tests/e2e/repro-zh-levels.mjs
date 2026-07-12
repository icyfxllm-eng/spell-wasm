// Repro: does switching fixed difficulty change the served zh word?
import { startServer, launch } from './harness.mjs';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

const { server, base } = await startServer(8133);
const browser = await launch();
const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2, isMobile: true });
await ctx.addInitScript((age) => localStorage.setItem('byear_agegate_v1', age), AGE);
await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
const page = await ctx.newPage();
await page.goto(base, { waitUntil: 'load' });
await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });

// select Chinese via the setup sheet
await page.click('#setupChip').catch(() => {});
await page.selectOption('#langSel', 'zh').catch(() => {});
await page.click('#setupDone').catch(() => {});
await page.waitForTimeout(200);

const seen = [];
for (const tier of ['easy', 'medium', 'hard', 'expert']) {
  await page.click('#setupChip').catch(() => {});
  await page.selectOption('#levelSel', tier).catch(() => {});
  await page.click('#setupDone').catch(() => {});
  await page.waitForTimeout(250);
  const info = await page.evaluate(() => ({
    word: window.__spelltest.currentWord(),
    spoken: window.__spelltest.currentSpoken(),
    tier: window.__spelltest.currentTier(),
    lang: window.__spelltest.currentLang(),
    poolFirst3: window.__spelltest.pool('zh', window.__spelltest.currentTier()).slice(0, 3),
  }));
  seen.push({ selected: tier, ...info });
  console.log(`selected=${tier}  currentTier=${info.tier}  word=${info.word}  spoken=${info.spoken}`);
}
const words = seen.map((s) => s.word);
console.log('\ndistinct words served:', new Set(words).size, 'of', words.length);
console.log('tiers actually used:', seen.map((s) => s.tier).join(', '));
await browser.close();
server.close();
