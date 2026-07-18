// Verify the BUNDLED, subsetted fonts/nastaliq-urdu.woff2 shapes Nastaliq
// correctly — subsetting can silently drop the layout features the cascade needs.
// Loads it via @font-face over HTTP (file:// blocks @font-face) and compares its
// rendered width to the system Noto Nastaliq Urdu: a match means the shaping
// survived. Then colours per letter, in WebKit (production).
import { createServer } from 'node:http';
import { readFileSync, existsSync } from 'node:fs';
import { extname, join } from 'node:path';
import { chromium, webkit } from 'playwright';

const ROOT = new URL('../../', import.meta.url).pathname; // repo root
const MIME = { '.html': 'text/html; charset=utf-8', '.woff2': 'font/woff2' };
const server = createServer((req, res) => {
  if (req.url === '/') {
    res.writeHead(200, { 'Content-Type': MIME['.html'] });
    res.end(`<!doctype html><meta charset=utf-8><style>
      @font-face{font-family:'Bundled Nastaliq';src:url('/fonts/nastaliq-urdu.woff2') format('woff2');}
      /* width:fit-content so getBoundingClientRect measures the TEXT, not the
         block's container width (which would make every row falsely "match"). */
      #bundled,#system,#colored{width:fit-content;display:inline-block}
      #bundled{font-family:'Bundled Nastaliq';font-size:64px}
      #system{font-family:'Noto Nastaliq Urdu';font-size:64px}
      #colored{font-family:'Bundled Nastaliq';font-size:64px}
      span.c0{color:#2e7d32}span.c1{color:#c62828}
    </style>
    <div id="system" dir="rtl">پاکستان</div>
    <div id="bundled" dir="rtl">پاکستان</div>
    <div id="colored" dir="rtl"></div>
    <script>
      const w='پاکستان', h=document.getElementById('colored');
      [...w].forEach((ch,i)=>{const s=document.createElement('span');s.textContent=ch;s.className='c'+(i%2);h.appendChild(s);});
      window.__m=()=>({
        system:document.getElementById('system').getBoundingClientRect().width,
        bundled:document.getElementById('bundled').getBoundingClientRect().width,
        colored:document.getElementById('colored').getBoundingClientRect().width,
        bundledLoaded:document.fonts.check("64px 'Bundled Nastaliq'"),
      });
    </script>`);
    return;
  }
  const f = join(ROOT, req.url);
  if (!f.startsWith(ROOT) || !existsSync(f)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': MIME[extname(f)] || 'application/octet-stream' });
  res.end(readFileSync(f));
});
await new Promise((r) => server.listen(0, r));
const base = `http://localhost:${server.address().port}/`;

for (const [name, browser] of [['chromium', chromium], ['webkit', webkit]]) {
  const br = await browser.launch();
  const page = await br.newPage({ viewport: { width: 760, height: 300 }, deviceScaleFactor: 2 });
  await page.goto(base, { waitUntil: 'load' });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  const m = await page.evaluate(() => window.__m());
  const ratio = m.bundled / m.system;
  console.log(`\n=== ${name} ===`);
  console.log(`  bundled @font-face loaded: ${m.bundledLoaded}`);
  console.log(`  system width ${m.system.toFixed(1)}px  bundled ${m.bundled.toFixed(1)}px  -> ${Math.abs(ratio - 1) < 0.02 ? 'MATCH — shaping survived subsetting' : 'DIFFERS ('+(ratio*100).toFixed(0)+'%) — subsetting changed shaping'}`);
  console.log(`  coloured width ${m.colored.toFixed(1)}px  -> ${Math.abs(m.colored / m.bundled - 1) < 0.02 ? 'join intact under colouring' : 'colouring shattered it'}`);
  await page.screenshot({ path: new URL(`./bundled-${name}.png`, import.meta.url).pathname });
  await br.close();
}
server.close();
