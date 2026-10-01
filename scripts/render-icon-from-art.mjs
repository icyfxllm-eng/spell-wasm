// Builds every app icon from a supplied piece of art (assets/icon-source.webp):
// an orb centred on a pure black field. The art is cropped to a square around
// the orb, then padded with black so the orb fills the same fraction of the
// canvas the CSS renderer used (0.84 full, 0.64 maskable, 0.62 Android fg).
// Afterwards run: npx capacitor-assets generate --android
import sharp from 'sharp';

const SRC = 'assets/icon-source.webp';

// Find the orb: bounding box of bright, warm pixels.
const { data, info } = await sharp(SRC).removeAlpha().raw().toBuffer({ resolveWithObject: true });
let x0 = Infinity, x1 = -1, y0 = Infinity, y1 = -1;
for (let y = 0; y < info.height; y++) {
  for (let x = 0; x < info.width; x++) {
    const i = (y * info.width + x) * 3;
    if (data[i] > 220 && data[i + 1] > 130) {
      if (x < x0) x0 = x; if (x > x1) x1 = x;
      if (y < y0) y0 = y; if (y > y1) y1 = y;
    }
  }
}
const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2, diam = Math.max(x1 - x0, y1 - y0) + 1;
console.log(`orb ${diam}px at (${cx}, ${cy}) in ${info.width}x${info.height}`);

async function render(size, orbFrac, outPath) {
  // Square side in source pixels that makes the orb occupy orbFrac of it.
  const side = Math.round(diam / orbFrac);
  const left = Math.round(cx - side / 2), top = Math.round(cy - side / 2);
  // Pad with black wherever the square overruns the source.
  const pad = {
    left: Math.max(0, -left), top: Math.max(0, -top),
    right: Math.max(0, left + side - info.width), bottom: Math.max(0, top + side - info.height),
  };
  // Two pipelines: sharp always runs extract before extend within one.
  const padded = await sharp(SRC).removeAlpha().extend({ ...pad, background: '#000' }).png().toBuffer();
  await sharp(padded)
    .extract({ left: left + pad.left, top: top + pad.top, width: side, height: side })
    .resize(size, size, { kernel: 'lanczos3' })
    .png()
    .toFile(outPath);
  console.log('wrote', outPath, size + 'px');
}

await render(1024, 0.84, 'assets/icon.png');
await render(1024, 0.84, 'ios/App/App/Assets.xcassets/AppIcon.appiconset/AppIcon-512@2x.png');
await render(1024, 0.84, 'desktop/src-tauri/icons/icon.png');
await render(512, 0.84, 'icons/icon-512.png');
await render(192, 0.84, 'icons/icon-192.png');
await render(512, 0.64, 'icons/icon-512-maskable.png');
// Android adaptive layers: the foreground is opaque black around the orb, which
// is invisible over the solid black background layer.
await render(1024, 0.62, 'assets/icon-foreground.png');
await sharp({ create: { width: 1024, height: 1024, channels: 3, background: '#000' } }).png().toFile('assets/icon-background.png');
