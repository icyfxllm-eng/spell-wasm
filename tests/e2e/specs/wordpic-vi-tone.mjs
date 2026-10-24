// wordpic-vi-tone.spec — a Vietnamese player can finish a Spell Picture word.
//
// Spell Picture borrows the shared game keyboard. Letters reached the picture
// through surface hooks, but the tone row did not: a tone tap went to the base
// game's own hidden answer, so no toned word could ever be completed here. And
// the letter-by-letter check called the bare vowel a miss before its tone could
// be tapped. This types the waiting word the way a player does: plain letters by
// tap, ă â ê ô ơ ư đ by a real long-press, each tone from the tone row right
// after its vowel, and expects the word to land.
import { openApp, assert, assertEq } from '../harness.mjs';
import { openPicture } from './economics.mjs';

const TONES = ['̀', '́', '̉', '̃', '̣'];

/** Long-press `base` and release on `want` in the accent popover. */
async function longPress(page, base, want) {
  const key = await page.$(`#gameKeyboard .kb-key[data-k="${base}"]`);
  assert(key, `no key for ${base}`);
  const k = await key.boundingBox();
  await page.mouse.move(k.x + k.width / 2, k.y + k.height / 2);
  await page.mouse.down();
  await page.waitForSelector('#skbPop.show', { timeout: 3000 });
  const opt = await page.evaluateHandle((w) =>
    [...document.querySelectorAll('.kb-popover.show .kb-acc')].find((b) => b.textContent.toLowerCase() === w), want);
  const o = await opt.asElement()?.boundingBox();
  assert(o, `${want} is not offered on ${base}`);
  await page.mouse.move(o.x + o.width / 2, o.y + o.height / 2, { steps: 4 });
  await page.mouse.up();
}

/** Type one Vietnamese word through the borrowed keyboard. */
async function typeVi(page, word) {
  for (const ch of word.normalize('NFC')) {
    const nfd = [...ch.normalize('NFD')];
    const tone = nfd.find((m) => TONES.includes(m));
    const letter = nfd.filter((m) => !TONES.includes(m)).join('').normalize('NFC');
    const base = letter.normalize('NFD')[0];
    if (letter === base || letter === ' ') {
      await page.click(`#gameKeyboard .kb-key[data-k="${letter}"]`);
    } else {
      await longPress(page, base === 'd' ? 'd' : base, letter);
    }
    if (tone) {
      const hex = tone.codePointAt(0).toString(16);
      await page.click(`#kbToneRow .kb-tone[data-tone="${hex}"]`);
    }
  }
}

const placed = (page) => page.$$eval('#wpStage .wp-word', (els) => els.length);

export async function run(browser, base, suite) {
  await suite.test('wordpic-vi: a toned Vietnamese word can be typed and lands', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'vi' });
    try {
      // Pictures unlock per language: let Surprise me pick one a new vi player has.
      await openPicture(page, null);
      assert(!(await page.$('#kbToneRow.btn-hide')), 'the tone row is offered in Spell Picture');
      // Find a word with a tone in it: those are the ones that could not land.
      let word = '';
      let before = 0;
      for (let i = 0; i < 6; i++) {
        word = await page.evaluate(() => window.__spelltest.picWord());
        before = await placed(page);
        if ([...word.normalize('NFD')].some((m) => TONES.includes(m))) break;
        await typeVi(page, word);
        await page.waitForFunction((n) => document.querySelectorAll('#wpStage .wp-word').length > n,
          before, { timeout: 4000 });
      }
      assert([...word.normalize('NFD')].some((m) => TONES.includes(m)), `no toned word came up (last: ${word})`);
      await typeVi(page, word);
      await page.waitForFunction((n) => document.querySelectorAll('#wpStage .wp-word').length > n,
        before, { timeout: 4000 }).catch(() => {});
      assertEq(await placed(page), before + 1, `the toned word ${word} landed`);
      assert(!(await page.$('#wpStage .miss-dim, #wpStage .miss-charged')), 'typing a vowel before its tone was not a miss');
    } finally { await ctx.close(); }
  });
}
