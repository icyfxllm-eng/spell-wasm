// Render the coloring-feedback demo with the BUNDLED font and screenshot it in
// WebKit (production engine). Also asserts, per case, that the coloured reveal's
// width equals an uncoloured render of the same word — i.e. the join survives the
// colouring in the real flow, not just the isolated test.
import { createServer } from 'node:http';
import { readFileSync, existsSync } from 'node:fs';
import { extname, join } from 'node:path';
import { webkit } from 'playwright';

const ROOT = new URL('../../', import.meta.url).pathname;
const demo = readFileSync(new URL('./demo.html', import.meta.url), 'utf8');
const MIME = { '.woff2': 'font/woff2' };
const server = createServer((req, res) => {
  if (req.url === '/' || req.url.startsWith('/?')) { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(demo); return; }
  const f = join(ROOT, req.url.split('?')[0]);
  if (!f.startsWith(ROOT) || !existsSync(f)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': MIME[extname(f)] || 'application/octet-stream' });
  res.end(readFileSync(f));
});
await new Promise((r) => server.listen(0, r));
const base = `http://localhost:${server.address().port}/`;

const br = await webkit.launch();
const page = await br.newPage({ viewport: { width: 720, height: 1100 }, deviceScaleFactor: 2 });
await page.goto(base, { waitUntil: 'load' });
await page.evaluate(() => document.fonts.ready);
await page.waitForFunction(() => window.__ready === true);
await page.waitForTimeout(300);

// Join integrity in the real flow: each coloured reveal must match an uncoloured
// render of the same word, or the colouring shattered it.
const check = await page.evaluate(() => {
  const seg = new Intl.Segmenter('ur', { granularity: 'grapheme' });
  const probe = document.createElement('span');
  probe.style.cssText = "font-family:'Noto Nastaliq Urdu',serif;font-size:56px;position:absolute;visibility:hidden;white-space:nowrap";
  document.body.appendChild(probe);
  const results = [];
  for (const rev of document.querySelectorAll('.reveal')) {
    const word = [...rev.querySelectorAll('span')].map((s) => s.textContent).join('');
    probe.textContent = word;
    const plain = probe.getBoundingClientRect().width;
    const colored = rev.getBoundingClientRect().width;
    results.push({ word, plain, colored, intact: Math.abs(colored - plain) < 1 });
  }
  probe.remove();
  return results;
});
for (const r of check) console.log(`  "${r.word}": coloured ${r.colored.toFixed(1)}px vs plain ${r.plain.toFixed(1)}px -> ${r.intact ? 'JOIN INTACT' : 'SHATTERED'}`);
console.log(`  all intact: ${check.every((r) => r.intact)}`);

await page.screenshot({ path: new URL('./demo-webkit.png', import.meta.url).pathname, fullPage: true });
await br.close();
server.close();
