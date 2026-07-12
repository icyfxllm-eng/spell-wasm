// Focused screenshot of the orb (play button) for before/after comparison.
//   node tests/e2e/orb-shot.mjs <label>
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { startServer, launch } from './harness.mjs';

const OUT = 'tests/e2e/shots/orb';
mkdirSync(OUT, { recursive: true });
const label = process.argv[2] || 'run';
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });

const { server, base } = await startServer(8137);
const browser = await launch();
const ctx = await browser.newContext({ viewport: { width: 390, height: 700 }, deviceScaleFactor: 3, isMobile: true });
await ctx.addInitScript((age) => localStorage.setItem('byear_agegate_v1', age), AGE);
await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
const page = await ctx.newPage();
await page.goto(base, { waitUntil: 'load' });
await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
await page.waitForTimeout(400);
const orb = await page.$('#orbWrap');
await orb.screenshot({ path: join(OUT, `${label}.png`) });
await browser.close();
server.close();
console.log(`wrote ${join(OUT, label + '.png')}`);
