// CC-HINDI-PHASE0 F2 — akshara render prototype, verified in Chromium AND WebKit
// (production is iOS WKWebView). Answers three things the F1 commit set up:
//   1. Does one-tile-per-akshara keep conjuncts whole (क्ष stays one shape)?
//   2. Does the browser's OWN grapheme segmentation agree with the Rust core
//      (i.e. does the platform implement GB9c Indic conjuncts)?
//   3. F4 preview: is segmentation/rendering normalization-stable (NFC == NFD)?
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { chromium, webkit } from 'playwright';

const html = readFileSync(new URL('./probe.html', import.meta.url), 'utf8');
const server = createServer((_q, res) => { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(html); });
await new Promise((r) => server.listen(0, r));
const base = `http://localhost:${server.address().port}/`;

for (const [name, browser] of [['chromium', chromium], ['webkit', webkit]]) {
  const br = await browser.launch();
  const page = await br.newPage({ viewport: { width: 780, height: 1000 }, deviceScaleFactor: 2 });
  await page.goto(base, { waitUntil: 'load' });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__ready === true);
  await page.waitForTimeout(300);
  const r = await page.evaluate(() => window.__probe());

  console.log(`\n=== ${name} ===`);
  console.log('  browser segmenter vs Rust core (GB9c Indic conjuncts):');
  let allAgree = true;
  for (const a of r.jsAgrees) {
    if (!a.agree) allAgree = false;
    console.log(`    ${a.word.padEnd(8)} core=${a.core.join('|')}  js=${a.js.join('|')}  ${a.agree ? 'AGREE' : 'DISAGREE'}`);
  }
  console.log(`  => browser agrees with core on all: ${allAgree}`);
  console.log('  normalization stability (F4 preview): NFC width == NFD width');
  for (const n of r.normStable) console.log(`    ${n.word.padEnd(8)} nfc=${n.nfcWidth.toFixed(1)} nfd=${n.nfdWidth.toFixed(1)}  ${n.sameWidth ? 'STABLE' : 'DIFFERS'}`);
  console.log(`  => all normalization-stable: ${r.normStable.every((n) => n.sameWidth)}`);

  await page.screenshot({ path: new URL(`./render-${name}.png`, import.meta.url).pathname, fullPage: true });
  await br.close();
}
server.close();
