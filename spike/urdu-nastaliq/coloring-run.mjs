// Can we give per-letter Urdu feedback by COLORING letters (not positioning
// markers)? Coloring needs no geometry — the shaper places the ink, we pick the
// colour. The only risk is that splitting the text to colour it shatters the
// Nastaliq join, exactly as per-letter <span>s did in F4. Width is the tell: a
// shattered word is markedly wider than the joined baseline.
//
// Tested in Chromium AND WebKit (production is iOS WKWebView).
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { chromium, webkit } from 'playwright';

const html = readFileSync(new URL('./coloring.html', import.meta.url), 'utf8');
const server = createServer((_q, res) => { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(html); });
await new Promise((r) => server.listen(0, r));
const base = `http://localhost:${server.address().port}/`;

for (const [name, browser] of [['chromium', chromium], ['webkit', webkit]]) {
  const br = await browser.launch();
  const page = await br.newPage({ viewport: { width: 760, height: 500 }, deviceScaleFactor: 2 });
  await page.goto(base, { waitUntil: 'load' });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  const r = await page.evaluate(() => window.__probe());
  const verdict = (ratio) => ratio > 1.15 ? `SHATTERED (${(ratio * 100).toFixed(0)}%)` : `JOIN PRESERVED (${(ratio * 100).toFixed(0)}%)`;
  console.log(`\n=== ${name} ===  baseline ${r.baselineWidth.toFixed(1)}px`);
  console.log(`  span + color:          ${verdict(r.htmlSpans)}`);
  console.log(`  span + background:     ${verdict(r.withBackground)}`);
  console.log(`  span + transform(pop): ${verdict(r.withTransform)}`);
  console.log(`  span + inline-block:   ${verdict(r.withInlineBlock)}`);
  console.log(`  SVG <tspan> + fill:    ${verdict(r.svgTspan)}`);
  await page.screenshot({ path: new URL(`./coloring-${name}.png`, import.meta.url).pathname });
  await br.close();
}
server.close();
