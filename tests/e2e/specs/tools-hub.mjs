// tools-hub.spec — Pillar 3, the "Tools & Features" hub in Settings.
//
// Verifies at the UI boundary:
//   * the Tools section renders (header + all eight rows) with populated,
//     localized availability hints;
//   * a switch flips the real localStorage['spell_flag_<name>'] and that state
//     survives a reload (word_stories: default OFF -> ON);
//   * a flipped flag changes the tool's actual visibility — ghost racing turned
//     OFF via the hub hides the live #ghostPace marker in a Climb run; turned
//     back ON, the marker reappears (deterministic seeded best run);
//   * Kid Mode simplifies the list (owner/dark rows hidden, play aids kept).
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

const GHOST_KEY = 'spell_ghost_v1';
// A slow ghost (first correct at 8s) so a quick real answer lands clearly, and
// the marker is deterministically shown whenever the flag is on.
const GHOST_SEED = { en: { events: [{ t: 8000, c: true }, { t: 16000, c: true }, { t: 24000, c: true }] } };

const ls = (page, k) => page.evaluate((key) => localStorage.getItem(key), k);
const hidden = (page, sel) => page.$eval(sel, (e) => e.classList.contains('btn-hide'));
const boot = (page) => page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });

const ROWS = ['toolGhostRow', 'toolSyllableRow', 'toolSayItRow', 'toolPhotoRow', 'toolSpellAloudRow', 'toolStoriesRow', 'toolSpelloffRow', 'toolShieldsRow'];

async function playOneCorrect(page) {
  await page.click('#orbWrap');
  await page.waitForTimeout(500);
  const word = await page.evaluate(() => window.__spelltest.currentWord());
  if (!word) return false;
  const baseOnly = [...word.toLowerCase()].every((c) => /[a-z'-]/.test(c));
  if (!baseOnly) return false;
  await typeOnKeyboard(page, word.toLowerCase());
  await page.click('#checkBtn');
  await page.waitForTimeout(400);
  return true;
}

export async function run(browser, base, suite) {
  await suite.test('renders: Tools section + all eight rows + populated hints', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      assert(!(await hidden(page, '#toolsHub')), 'tools hub should be visible');
      for (const row of ROWS) {
        assert(!!(await page.$('#' + row)), `row ${row} present in DOM`);
      }
      // The availability hints are filled by Rust on settings open (not empty).
      const ghostHint = await page.$eval('#toolGhostHint', (e) => e.textContent.trim());
      assert(ghostHint.length > 0, 'ghost availability hint should be populated');
      // The native-gated Say It hint reflects "not on this device" off-device (web).
      const sayHint = await page.$eval('#toolSayItHint', (e) => e.textContent.trim());
      assert(sayHint.length > 0, 'say-it availability hint should be populated');
      // Switches reflect the compiled flag defaults: ghost ON, word_stories OFF.
      assert(await page.$eval('#toolGhostToggle', (e) => e.checked), 'ghost defaults ON');
      assert(!(await page.$eval('#toolStoriesToggle', (e) => e.checked)), 'word_stories defaults OFF');
    } finally { await ctx.close(); }
  });

  await suite.test('toggle flips localStorage flag and persists across reload', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      assert((await ls(page, 'spell_flag_word_stories')) === null, 'flag unset before toggle');
      await page.click('#toolStoriesToggle');
      assert((await ls(page, 'spell_flag_word_stories')) === 'on', 'flipping the row writes the on flag');
      await page.reload({ waitUntil: 'load' });
      await boot(page);
      assert((await ls(page, 'spell_flag_word_stories')) === 'on', 'flag persists across reload');
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      assert(await page.$eval('#toolStoriesToggle', (e) => e.checked), 'switch reflects the persisted flag');
      // Flip back off and confirm the write.
      await page.click('#toolStoriesToggle');
      assert((await ls(page, 'spell_flag_word_stories')) === 'off', 'flipping back writes the off flag');
    } finally { await ctx.close(); }
  });

  await suite.test('flipping ghost off/on changes the tool visibility in a Climb run', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      await page.evaluate(([k, v]) => localStorage.setItem(k, JSON.stringify(v)), [GHOST_KEY, GHOST_SEED]);

      // Turn ghost OFF via the hub, reload so the flag takes effect at run start.
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      await page.click('#toolGhostToggle'); // default ON -> OFF
      assert((await ls(page, 'spell_flag_ghost_racing')) === 'off', 'ghost flag written off');
      await page.reload({ waitUntil: 'load' });
      await boot(page);
      const played = await playOneCorrect(page);
      if (played) {
        assert(await hidden(page, '#ghostPace'), 'ghost pace marker must stay hidden while the tool is off');
      }

      // Turn ghost back ON via the hub, reload, and confirm the marker returns.
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      await page.click('#toolGhostToggle'); // OFF -> ON
      assert((await ls(page, 'spell_flag_ghost_racing')) === 'on', 'ghost flag written on');
      await page.reload({ waitUntil: 'load' });
      await boot(page);
      const played2 = await playOneCorrect(page);
      if (played2) {
        assert(!(await hidden(page, '#ghostPace')), 'ghost pace marker must appear once the tool is on');
      }
    } finally { await ctx.close(); }
  });

  await suite.test('Kid Mode simplifies the hub: dark/owner rows hidden, play aids kept', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      await page.click('#setBtn');
      await page.waitForTimeout(150);
      await page.click('#kidToggle'); // enter Kid Mode
      await page.waitForTimeout(150);
      assert(!(await hidden(page, '#toolGhostRow')), 'ghost racing stays for little spellers');
      assert(!(await hidden(page, '#toolSyllableRow')), 'syllable replay stays for little spellers');
      assert(await hidden(page, '#toolSayItRow'), 'Say It hidden in Kid Mode');
      assert(await hidden(page, '#toolStoriesRow'), 'word stories hidden in Kid Mode');
      assert(await hidden(page, '#toolSpelloffRow'), 'online spell-off hidden in Kid Mode');
    } finally { await ctx.close(); }
  });
}
