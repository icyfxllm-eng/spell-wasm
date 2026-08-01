// finale-pixels.spec — CC-FINALE Done #1, the literal pixel diff.
//
// The export renderer's output at 1× must match the in-play final frame's
// piece region exactly (HUD excluded) for the dog and Eiffel goldens —
// proof of "same solver output, no layout drift" at the RASTER level, in
// the real browser rasterizer, through the real export path (fonts
// fetched with fetch-and-refuse, mode Export, the run's own seed).
//
// Terms of the comparison, stated plainly: the two sides legitimately
// differ in exactly three mode-specific ways — the export carries its own
// <style> + backdrop rect (the play frame styles via page CSS), and the
// play frame carries live-play animation state (entry class, reveal
// stagger delays). We graft the export's OWN style block and backdrop
// into the play markup and strip the animation state, then rasterize
// both through one identical path. After that, "exactly" means exactly:
// zero differing bytes across all 512×512 RGBA pixels. The Rust-side
// geometry invariant proves the markup; this proves the pixels.
import { openApp, assert, assertEq } from '../harness.mjs';
import { completePicture } from './finale.mjs';

async function pixelDiff(page) {
  return page.evaluate(async () => {
    const exportSvg = await window.__spelltest.picExportSvg();
    const style = (exportSvg.match(/<style>[\s\S]*?<\/style>/) || [null])[0];
    if (!style) return { error: 'export svg carries no style block' };
    const backdrop = '<rect width="512" height="512" fill="#0e1420"/>';
    if (!exportSvg.includes(backdrop)) return { error: 'export svg carries no backdrop' };
    const playRaw = document.querySelector('#wpRevealStage svg').outerHTML;
    const settle = (s) =>
      s.replace(/ style="animation-delay:[^"]*"/g, '').replace(/class="wp-word new"/g, 'class="wp-word"');
    const play = settle(playRaw).replace(/<svg([^>]*)>/, (m, a) => `<svg${a}>${style}${backdrop}`);
    const exp = settle(exportSvg);
    const raster = async (svg) => {
      const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }));
      try {
        const img = new Image();
        img.src = url;
        await img.decode();
        const c = document.createElement('canvas');
        c.width = 512; c.height = 512;
        const g = c.getContext('2d', { willReadFrequently: true });
        g.drawImage(img, 0, 0, 512, 512);
        return g.getImageData(0, 0, 512, 512).data;
      } finally { URL.revokeObjectURL(url); }
    };
    const [a, b] = [await raster(play), await raster(exp)];
    let diff = 0;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) diff++;
    return { diff, bytes: a.length };
  });
}

export async function run(browser, base, suite) {
  for (const pic of ['dog', 'eiffel']) {
    await suite.test(`finale-pixels: export raster == play frame for ${pic}`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        const typed = await completePicture(page, pic);
        assert(typed > 0, 'never typed a word');
        await page.waitForSelector('#wpReveal.show', { timeout: 8000 });
        await page.click('#wpRevealStage'); // settle to rest before sampling
        await page.waitForTimeout(200);
        const r = await pixelDiff(page);
        assert(!r.error, r.error || '');
        assert(r.bytes === 512 * 512 * 4, `rasterized the full 1x frame (${r.bytes})`);
        assertEq(r.diff, 0, `${pic}: ${r.diff} of ${r.bytes} bytes drifted between play and export`);
      } finally { await ctx.close(); }
    });
  }
}
