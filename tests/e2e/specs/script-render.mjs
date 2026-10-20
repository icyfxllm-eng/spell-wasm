// script-render.spec — CC-VISUAL-ECLIPSE AT-6: scripts with stacked marks,
// conjuncts and joining render in their own faces and are never clipped.
//
// For Arabic, Hindi and Vietnamese it plays a real round, misses on purpose so
// the reveal shows, and then checks, through the browser's own record of which
// font drew each text node (CDP CSS.getPlatformFontsForNode):
//   - the revealed word is drawn by the script's face (Noto Naskh Arabic, Noto
//     Sans Devanagari; Vietnamese by Space Mono, which carries the subset),
//   - the English "It was" around it is NOT drawn by a serif fallback (the
//     script rules used to end in `serif`, so it came out in Times),
//   - no ancestor of the answer, the result line or the meaning clips its
//     content (overflow with scrollHeight beyond clientHeight).
// It saves a screenshot of each for review in $TMPDIR/spell-script-render (outside
// the repo, so a test run leaves nothing to commit).
import { mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { openApp, assert } from '../harness.mjs';

const CASES = [
  ['ar', 'Noto Naskh Arabic'],
  ['hi', 'Noto Sans Devanagari'],
  ['vi', 'Space Mono'],
];
const SERIF = /^(Times|Times New Roman|Georgia|Noto Serif|DejaVu Serif|Liberation Serif)/;

async function fontsOf(page, selector) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('DOM.enable');
  await cdp.send('CSS.enable');
  const { root } = await cdp.send('DOM.getDocument', { depth: -1 });
  const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector });
  if (!nodeId) return [];
  const { fonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
  await cdp.detach();
  return fonts.map((f) => f.familyName);
}

export async function run(browser, base, suite) {
  const SHOTS = join(tmpdir(), 'spell-script-render');
  mkdirSync(SHOTS, { recursive: true });
  for (const [lang, face] of CASES) {
    await suite.test(`AT-6 ${lang}: the reveal is drawn by ${face}, "It was" is not serif, nothing clips`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang, viewport: { width: 390, height: 844 } });
      try {
        assert(await page.evaluate((l) => window.__spelltest.currentLang() === l || document.getElementById('langSel').value === l, lang),
          `the study language did not switch to ${lang}`);
        await page.evaluate(() => document.querySelector('#orbWrap').click());
        await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
        // A key that is not the word's first letter, so the answer is a miss.
        const ok = await page.evaluate(() => {
          const first = [...window.__spelltest.currentWord()][0];
          const key = [...document.querySelectorAll('#gameKeyboard .kb-key[data-k]')]
            .find((k) => k.offsetParent !== null && k.dataset.k && k.dataset.k !== first && k.dataset.k.length === 1);
          if (!key) return false;
          key.click();
          document.querySelector('#checkBtn').click();
          return true;
        });
        assert(ok, 'no keyboard key to type a miss with');
        await page.waitForSelector('#feedback .reveal', { timeout: 5000 });
        await page.evaluate(() => document.fonts.ready);
        const reveal = await fontsOf(page, '#feedback .reveal');
        const around = await fontsOf(page, '#feedback');
        assert(reveal.some((f) => f.startsWith(face)), `the ${lang} reveal was drawn by ${JSON.stringify(reveal)}, not ${face}`);
        assert(!around.some((f) => SERIF.test(f)), `"It was" fell back to a serif face: ${JSON.stringify(around)}`);
        const clipped = await page.evaluate(() => {
          const out = [];
          for (const id of ['letters', 'feedback', 'meaning']) {
            for (let a = document.getElementById(id); a && a !== document.body; a = a.parentElement) {
              const cs = getComputedStyle(a);
              if (/(hidden|clip|auto|scroll)/.test(cs.overflowY) && a.scrollHeight > a.clientHeight + 1) out.push(a.id || a.className);
            }
          }
          return out;
        });
        assert(!clipped.length, `clipped by: ${clipped.join(', ')}`);
        const box = await page.evaluate(() => {
          const a = document.getElementById('spellbox').getBoundingClientRect();
          const b = document.getElementById('feedback').getBoundingClientRect();
          return { x: 0, y: Math.max(0, a.top - 8), width: innerWidth, height: b.bottom - a.top + 16 };
        });
        await page.screenshot({ path: join(SHOTS, `${lang}.png`), clip: box });
      } finally { await ctx.close(); }
    });
  }
}
