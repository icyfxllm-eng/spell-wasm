// hotfix-contrast.spec — CC-ECLIPSE-HOTFIX F1, I1 and I2: every word and icon
// reads on every theme, on every screen.
//
// Screens come from the registry (every drawer row this player can open, so a
// mode added later is covered without anyone listing it) plus the home
// screen's interactive states: a miss with its reveal, and the voice
// spelling's retry hint. Each runs on Eclipse and Daybreak (Light), and on
// each with the Background picker's furthest swatch, because a pick swaps
// only the page and the ink on it: that is the case that put white text on
// white cards in 298.
//
// Three layers, no thresholds moved:
//   1. axe-core's color-contrast rule, zero violations (I1).
//   2. What axe marks "incomplete" (one-letter keyboard keys, icons, text on a
//      gradient) is measured against the pixels actually painted behind it,
//      at the same 4.5:1 / 3:1 (icons 3:1). Emoji are full-colour pictures,
//      and text under an open modal is not what the player is reading; both
//      are listed, never silently dropped.
//   3. I2: a field's boundary and an unselected chip's outline reach 3:1
//      against what surrounds them, and no unselected state is opacity-only.
import { readFileSync } from 'node:fs';
import AxeBuilder from '@axe-core/playwright';
import { typeOnKeyboard, domSettled, assert } from '../harness.mjs';
import { THEMES, BACKGROUNDS, openThemed, pickBackground, drawerRows, openRow, residualContrast, controlBoundaries } from '../hotfix-lib.mjs';

const EN = JSON.parse(readFileSync(new URL('../../../src/i18n/locales/en.json', import.meta.url), 'utf8'));

/** The states on the home screen that are not reachable from the drawer. */
const STATES = {
  // A word served: the keyboard is live (it is locked, and exempt, before).
  async served(page) {
    await page.click('#orbWrap');
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
  },
  async miss(page) {
    await page.click('#orbWrap');
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
    const word = await page.evaluate(() => window.__spelltest.currentWord());
    await typeOnKeyboard(page, word + 'q');
    await page.click('#checkBtn');
    await page.waitForFunction(() => /\bbad\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
  },
  async hit(page) {
    await page.click('#orbWrap');
    await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
    await typeOnKeyboard(page, await page.evaluate(() => window.__spelltest.currentWord()));
    await page.click('#checkBtn');
    await page.waitForFunction(() => /\bgood\b/.test(document.getElementById('feedback').className), null, { timeout: 10000 });
  },
  // The retry hint, as spell_aloud::set_status writes it (the mic itself has
  // no recognizer in Chromium).
  async retryHint(page) {
    await page.evaluate((t) => { document.getElementById('voiceSpellStatus').textContent = t; }, EN['voiceSpell.didntCatch']);
  },
};

async function audit(page, label, failures) {
  await domSettled(page);
  const r = await new AxeBuilder({ page }).withRules(['color-contrast']).analyze();
  for (const v of r.violations) for (const n of v.nodes) {
    const d = (n.any[0] || {}).data || {};
    failures.push(`${label}: ${n.target.join(' ')} ${d.contrastRatio}:1 < ${d.expectedContrastRatio} (${d.fgColor} on ${d.bgColor})`);
  }
  const skipped = [];
  for (const b of await residualContrast(page, r.incomplete, skipped)) failures.push(`${label}: ${b}`);
  for (const b of await controlBoundaries(page)) failures.push(`${label}: ${b}`);
  return skipped.length;
}

export async function run(browser, base, suite) {
  // HOTFIX_THEMES=daybreak narrows a local run; CI runs every theme.
  const only = (process.env.HOTFIX_THEMES || '').split(',').filter(Boolean);
  for (const theme of THEMES.filter((x) => !only.length || only.includes(x))) {
    for (const bg of BACKGROUNDS[theme]) {
      const name = `${theme}${bg ? ` + Background ${bg}` : ''}`;
      await suite.test(`I1/I2 ${name}: every registry screen and home state reads (axe + measured residue + boundaries)`, async () => {
        const failures = [];
        let rows;
        {
          const { ctx, page } = await openThemed(browser, base, theme);
          try { rows = await drawerRows(page); } finally { await ctx.close(); }
        }
        assert(rows.length >= 20, `only ${rows.length} drawer rows: the registry walk is not reaching the modes`);
        const visits = [['home', null], ['lived-in home', null, true], ...Object.keys(STATES).map((s) => [s, STATES[s]]), ...rows.map((r) => [r, (p) => openRow(p, r)])];
        let skipped = 0;
        for (const [label, go, lived] of visits) {
          const { ctx, page } = await openThemed(browser, base, theme, { lived: !!lived });
          try {
            await pickBackground(page, bg);
            if (go) await go(page);
            skipped += await audit(page, label, failures);
          } catch (e) {
            failures.push(`${label}: could not be audited (${e.message.split('\n')[0]})`);
          } finally { await ctx.close(); }
        }
        assert(!failures.length, `${failures.length} contrast failure(s) on ${name} (${skipped} unmeasurable nodes listed by the probe):\n    ${failures.slice(0, 40).join('\n    ')}`);
      });
    }
  }
}
