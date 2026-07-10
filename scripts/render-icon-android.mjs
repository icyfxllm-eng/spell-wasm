import { chromium } from 'playwright';
const b = await chromium.launch();

// Adaptive-icon foreground: orb + S on a TRANSPARENT background, sized for the
// Android safe zone so the circle/squircle mask never clips the orb.
async function foreground(size, outPath) {
  const p = await b.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  const scale = 0.62;
  const html = `<!doctype html><html><head><meta charset="utf8">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,800&display=block" rel="stylesheet">
  <style>
   html,body{margin:0;padding:0;background:transparent}
   .wrap{width:${size}px;height:${size}px;background:transparent;display:flex;align-items:center;justify-content:center}
   .orb{width:${scale * 100}%;aspect-ratio:1;border-radius:50%;
     background:radial-gradient(circle at 40% 36%, #fffdea 0%, #ffe63a 32%, #ffc400 70%, #f2a000 100%);
     box-shadow:0 0 ${size * 0.04}px ${size * 0.012}px rgba(255,214,60,.55);
     display:flex;align-items:center;justify-content:center}
   .s{font-family:'Bricolage Grotesque',sans-serif;font-weight:800;color:#000;
      font-size:${scale * size * 0.86}px;line-height:1;letter-spacing:-0.03em;transform:translateY(-0.015em)}
  </style></head>
  <body><div class="wrap"><div class="orb"><div class="s">S</div></div></div></body></html>`;
  await p.setContent(html, { waitUntil: 'networkidle' });
  await p.evaluate(async () => { await document.fonts.ready; });
  await p.waitForTimeout(150);
  await p.screenshot({ path: outPath, omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } });
  await p.close();
  console.log('wrote', outPath);
}

async function solid(size, color, outPath) {
  const p = await b.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  await p.setContent(`<div style="width:${size}px;height:${size}px;background:${color}"></div>`);
  await p.screenshot({ path: outPath, clip: { x: 0, y: 0, width: size, height: size } });
  await p.close();
  console.log('wrote', outPath);
}

await foreground(1024, 'assets/icon-foreground.png');
await solid(1024, '#000000', 'assets/icon-background.png');
await b.close();
