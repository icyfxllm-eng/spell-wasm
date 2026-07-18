// Urdu Nastaliq per-glyph geometry spike.
//
// QUESTION: can SVG <text>.getExtentOfChar(i) place a per-letter feedback marker
// under Nastaliq Urdu, where the canvas advance-width approach failed (11–29px
// error)? Nastaliq cascades each letter down-and-left within a ligature, so a
// usable answer must (a) return a real box per character, (b) show that cascade
// as a vertical stagger, and (c) do so in WEBKIT, since production is iOS WKWebView.
//
// Served over HTTP: file:// blocks font resolution and the whole thing measures
// a fallback. Both engines tested; a result that only holds in Chromium is
// useless for this app.
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { chromium, webkit } from 'playwright';

const HTML = new URL('./probe.html', import.meta.url);
const html = readFileSync(HTML, 'utf8');

const server = createServer((_req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(html);
});
await new Promise((r) => server.listen(0, r));
const base = `http://localhost:${server.address().port}/`;

function analyze(engine, data) {
  const lines = [];
  const f = data.font;
  const rendered = f.fontsCheckNastaliq === true || f.differsFromMono;
  lines.push(`\n=== ${engine} ===`);
  lines.push(`  font: document.fonts.check("Noto Nastaliq Urdu")=${f.fontsCheckNastaliq}, differsFromMono=${f.differsFromMono} (w=${f.nastaliqWidth?.toFixed?.(1)} vs mono ${f.monoWidth?.toFixed?.(1)})`);
  lines.push(`  => Nastaliq actually rendered: ${rendered ? 'YES' : 'NO — results below are a FALLBACK, discard'}`);

  for (const w of data.svg) {
    lines.push(`  word "${w.word}"  chars(js)=${w.jsChars} numChars(svg)=${w.numChars}`);
    const boxes = w.perChar.map((c) => c.ext).filter((e) => e && !e.error);
    if (boxes.length !== w.perChar.length) {
      lines.push(`    getExtentOfChar: ${w.perChar.length - boxes.length}/${w.perChar.length} threw`);
    }
    // Per-char summary.
    for (const c of w.perChar) {
      if (c.ext && !c.ext.error) {
        lines.push(`    [${c.i}] ${c.cp}  ext x=${c.ext.x.toFixed(1)} y=${c.ext.y.toFixed(1)} w=${c.ext.w.toFixed(1)} h=${c.ext.h.toFixed(1)}  rot=${c.rot}`);
      } else {
        lines.push(`    [${c.i}] ${c.cp}  ext=ERROR ${c.ext?.error || ''}`);
      }
    }
    // Cascade signatures.
    if (boxes.length >= 2) {
      const ys = boxes.map((b) => b.y);
      const yStagger = Math.max(...ys) - Math.min(...ys);
      const xs = boxes.map((b) => b.x);
      // Do boxes overlap horizontally (later letter tucks under earlier)?
      let overlaps = 0;
      const sorted = [...boxes].sort((a, b) => b.x - a.x); // RTL: right first
      for (let k = 1; k < sorted.length; k++) {
        const prev = sorted[k - 1], cur = sorted[k];
        if (cur.x + cur.w > prev.x + 0.5) overlaps++;
      }
      const distinctX = new Set(xs.map((x) => x.toFixed(1))).size;
      lines.push(`    cascade: y-stagger=${yStagger.toFixed(1)}px  distinctX=${distinctX}/${boxes.length}  h-overlaps=${overlaps}  allZeroW=${boxes.every((b) => b.w < 0.5)}`);
    }
  }

  // DOM/Range comparison for the first word.
  const d0 = data.dom[0];
  lines.push(`  DOM Range (word "${d0.word}"):`);
  for (const c of d0.perChar) {
    if (c.error) lines.push(`    [${c.ci}] ${c.ch} ERROR`);
    else lines.push(`    [${c.ci}] ${c.ch}  x=${c.x.toFixed(1)} y=${c.y.toFixed(1)} w=${c.w.toFixed(1)} h=${c.h.toFixed(1)}`);
  }
  const dys = d0.perChar.filter((c) => !c.error).map((c) => c.y);
  if (dys.length) lines.push(`    Range y-stagger=${(Math.max(...dys) - Math.min(...dys)).toFixed(1)}px (0 => line boxes, cannot see cascade)`);

  return { text: lines.join('\n'), rendered };
}

const results = {};
for (const [name, browser] of [['chromium', chromium], ['webkit', webkit]]) {
  const br = await browser.launch();
  const page = await br.newPage({ viewport: { width: 1200, height: 700 }, deviceScaleFactor: 2 });
  await page.goto(base, { waitUntil: 'load' });
  // Give the font a beat to resolve, then wait on document.fonts.ready.
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  const data = await page.evaluate(() => window.__probe());
  results[name] = analyze(name, data);
  // Draw each getExtentOfChar box over the real ink, so the numbers are visible:
  // if the boxes tracked glyphs, each rect would hug one letter.
  await page.evaluate(() => {
    const SVGNS = 'http://www.w3.org/2000/svg';
    const svg = document.getElementById('svg');
    const texts = [...svg.querySelectorAll('text')];
    for (const t of texts) {
      const n = t.getNumberOfChars();
      for (let i = 0; i < n; i++) {
        let e; try { e = t.getExtentOfChar(i); } catch (_) { continue; }
        const r = document.createElementNS(SVGNS, 'rect');
        r.setAttribute('x', e.x); r.setAttribute('y', e.y);
        r.setAttribute('width', e.width); r.setAttribute('height', e.height);
        r.setAttribute('fill', 'none');
        r.setAttribute('stroke', ['red', 'blue', 'green', 'orange', 'purple', 'teal', 'magenta'][i % 7]);
        r.setAttribute('stroke-width', '1');
        svg.appendChild(r);
      }
    }
  });
  await page.locator('#svg').screenshot({ path: new URL(`./boxes-${name}.png`, import.meta.url).pathname });
  await br.close();
}
server.close();

console.log(results.chromium.text);
console.log(results.webkit.text);
console.log('\n=== headline ===');
for (const eng of ['chromium', 'webkit']) {
  console.log(`  ${eng}: Nastaliq rendered=${results[eng].rendered}`);
}
