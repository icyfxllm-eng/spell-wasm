// metadata-audit.spec — CC-FINALE Done #4: the exported FILES inspected
// by test. The seam returns the real PNG bytes (same plan → svg →
// rasterize path Save/Share use); the spec walks the chunk table:
//
//   - no eXIf, no tEXt/iTXt/zTXt, no tIME — no location, no identifiers,
//     no timestamps, nothing to strip later;
//   - no learner data: the raw bytes never contain the storage prefix;
//   - keepsake and card both audited (they are different pipelines: the
//     card frames the piece; the keepsake IS the piece).
//
// The wordmark/attribution SVG-level guarantees are pinned by unit tests
// in spellpic_export.rs (the_wordmark_lives_on_the_card_and_never_the_
// keepsake, masterpiece_cards_carry_the_attribution_and_others_do_not).
import { openApp, assert } from '../harness.mjs';

async function openPicture(page, pic) {
  await page.evaluate(() => document.getElementById('wordPicOpen').click());
  await page.waitForSelector('#wpPicker.show', { timeout: 5000 });
  await page.click(`[data-pic="${pic}"]`);
  await page.waitForSelector('#wpPlay.show', { timeout: 5000 });
  await page.waitForTimeout(500);
  for (let i = 0; i < 6 && (await page.$('#wpHow.show')); i++) {
    await page.click('#wpHowNext');
    await page.waitForTimeout(150);
  }
}

/** Chunk type names of a PNG, in file order. */
function pngChunks(buf) {
  assert(buf.length > 8 && buf[0] === 0x89 && buf[1] === 0x50, 'not a PNG');
  const types = [];
  let off = 8;
  while (off + 8 <= buf.length) {
    const len = buf.readUInt32BE(off);
    types.push(buf.toString('ascii', off + 4, off + 8));
    off += 12 + len;
  }
  return types;
}

const FORBIDDEN = ['eXIf', 'tEXt', 'iTXt', 'zTXt', 'tIME'];

export async function run(browser, base, suite) {
  for (const product of ['keepsake', 'card']) {
    await suite.test(`metadata: the ${product} carries no EXIF, no text, no learner data`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        await openPicture(page, 'star');
        const dataUrl = await page.evaluate(
          (p) => window.__spelltest.picExportPng(p), product);
        assert(dataUrl.startsWith('data:image/png;base64,'), `no PNG came back for ${product}`);
        const buf = Buffer.from(dataUrl.slice('data:image/png;base64,'.length), 'base64');
        const chunks = pngChunks(buf);
        assert(chunks[0] === 'IHDR' && chunks[chunks.length - 1] === 'IEND',
          `malformed chunk table: ${chunks}`);
        for (const bad of FORBIDDEN) {
          assert(!chunks.includes(bad), `${product} carries a ${bad} chunk: ${chunks}`);
        }
        assert(!buf.includes('spell_learner_'), `${product} leaked learner storage bytes`);
        assert(!buf.includes('spell_flag_'), `${product} leaked flag storage bytes`);
      } finally { await ctx.close(); }
    });
  }
}
