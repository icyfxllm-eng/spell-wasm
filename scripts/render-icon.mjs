import { chromium } from 'playwright';

const b = await chromium.launch();

async function render(size, maskable, outPath) {
  const p = await b.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  // Maskable icons must keep content inside the ~80% safe circle, so shrink the orb.
  const scale = maskable ? 0.64 : 0.84;
  const html = `<!doctype html><html><head><meta charset="utf8">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,800&display=block" rel="stylesheet">
  <style>
   html,body{margin:0;padding:0}
   .wrap{width:${size}px;height:${size}px;background:#000;display:flex;align-items:center;justify-content:center;overflow:hidden}
   .orb{position:relative;width:${scale * 100}%;aspect-ratio:1;border-radius:50%;
     background:radial-gradient(circle at 40% 36%, #fffdea 0%, #ffe63a 32%, #ffc400 70%, #f2a000 100%);
     box-shadow:0 0 ${size * 0.05}px ${size * 0.015}px rgba(255,214,60,.6),
                0 0 ${size * 0.12}px ${size * 0.045}px rgba(255,190,30,.38);
     display:flex;align-items:center;justify-content:center}
   .s{font-family:'Bricolage Grotesque',sans-serif;font-weight:800;color:#000;
      font-size:${scale * size * 0.86}px;line-height:1;letter-spacing:-0.03em;transform:translateY(-0.015em)}
  </style></head>
  <body><div class="wrap"><div class="orb"><div class="s">S</div></div></div></body></html>`;
  await p.setContent(html, { waitUntil: 'networkidle' });
  await p.evaluate(async () => { await document.fonts.ready; });
  await p.waitForTimeout(150);
  await p.screenshot({ path: outPath, clip: { x: 0, y: 0, width: size, height: size } });
  await p.close();
  console.log('wrote', outPath, size + 'px', maskable ? '(maskable)' : '');
}

await render(1024, false, 'assets/icon.png');
await render(1024, false, 'ios/App/App/Assets.xcassets/AppIcon.appiconset/AppIcon-512@2x.png');
await render(512, false, 'icons/icon-512.png');
await render(192, false, 'icons/icon-192.png');
await render(512, true, 'icons/icon-512-maskable.png');
await b.close();
