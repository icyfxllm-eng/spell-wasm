// spell-keyboard.spec — CC-SPELL-KEYBOARD Phase A, with keyboard.core ON.
//
// The rest of the suite proves the shared keyboard changes nothing a test can
// see (run it with SPELL_FLAGS=keyboard_core=on). This spec checks what the
// shared keyboard ADDS:
//   * every key on every surface is drawn by it and has a VoiceOver label (KF10);
//   * the five Vietnamese tone keys are told apart by name;
//   * labels are lowercase until Shift is armed (K-D19);
//   * Bee, which had no long-press, now offers a language's alternates.
import { openApp, assert, assertEq } from '../harness.mjs';

const ON = () => { localStorage.setItem('spell_flag_keyboard_core', 'on'); };
const openMode = (page, id) => page.evaluate((id) => document.getElementById(id).click(), id);

/** Every key in `host` is a shared key with a non-empty label. */
async function labelled(page, host) {
  return page.evaluate((h) => {
    const keys = [...document.querySelectorAll(`#${h} .skb-key`)];
    return { n: keys.length, unlabelled: keys.filter((k) => !(k.getAttribute('aria-label') || '').trim()).map((k) => k.outerHTML.slice(0, 80)) };
  }, host);
}

export async function run(browser, base, suite) {
  await suite.test('spell-keyboard: the main keyboard is the shared one, labelled, lowercase', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: ON });
    try {
      assert(await page.evaluate(() => document.body.classList.contains('kb-core')), 'body carries kb-core');
      const r = await labelled(page, 'kbLetters');
      assert(r.n >= 26, `the letters are shared keys (${r.n})`);
      assertEq(r.unlabelled.length, 0, `unlabelled keys: ${r.unlabelled.join(' | ')}`);
      const faces = await page.$$eval('#kbLetters .skb-key[data-k]', (ks) => ks.map((k) => getComputedStyle(k).textTransform + ':' + k.textContent));
      assert(faces.every((f) => !/[A-Z]/.test(f.split(':')[1]) && !f.startsWith('uppercase')), `labels show the case a key types (${faces.slice(0, 4)})`);
    } finally { await ctx.close(); }
  });

  await suite.test('spell-keyboard: the five Vietnamese tone keys are named apart', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'vi', init: ON });
    try {
      const names = await page.$$eval('#kbToneRow .skb-key', (ks) => ks.map((k) => k.getAttribute('aria-label')));
      assertEq(names.length, 5, 'five tone keys');
      assertEq(new Set(names).size, 5, `each tone has its own name (${names})`);
      assert(!names.includes('tone mark'), 'no generic "tone mark" label left');
    } finally { await ctx.close(); }
  });

  await suite.test('spell-keyboard: Bee draws shared keys and offers alternates', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'es', init: ON });
    try {
      await openMode(page, 'beeOpen');
      await page.waitForSelector('#bee.show', { timeout: 5000 });
      const r = await labelled(page, 'beeKeys');
      assert(r.n > 0, 'Bee keys are shared keys');
      assertEq(r.unlabelled.length, 0, `unlabelled Bee keys: ${r.unlabelled.join(' | ')}`);
      const acc = await page.$eval('#beeKeys .skb-key[data-k="a"]', (k) => k.dataset.acc || '');
      assert(acc.includes('á'), `á is a long-press on a in Bee (got "${acc}")`);
    } finally { await ctx.close(); }
  });
}
