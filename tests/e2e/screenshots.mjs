// Home-screen screenshot pass for CC-HOME-REGROUP review.
// Captures the home (topbar + play zone) for a matrix of
//   lang x device x mode(standard|kid)
// against the seam-enabled dist-test/. Output dir is argv[2] (e.g. before/ after/).
//   node tests/e2e/screenshots.mjs <outDir>
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { startServer, launch } from './harness.mjs';

// Smallest iPhone, largest phone, and a tablet — the three deliverable widths.
const SHOT_DEVICES = {
  se: { width: 375, height: 667 },
  large: { width: 430, height: 932 },
  tablet: { width: 820, height: 1180 },
};

const OUT = join('tests/e2e/shots', process.argv[2] || 'run');
mkdirSync(OUT, { recursive: true });

// Little Speller is engaged via the age-gate verdict ("kid" locks Kid Mode on),
// not a partial prefs blob — Prefs' bool fields have no serde(default), so a
// partial prefs JSON fails to deserialize and silently falls back to kid:false.
const AGE_FULL = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const AGE_KID = JSON.stringify({ verdict: 'kid', checkedAt: 1700000000 });
const LANGS = ['en', 'de', 'th', 'ja'];
const MODES = [
  { key: 'std', age: AGE_FULL },
  { key: 'kid', age: AGE_KID },
];

const { server, base } = await startServer(8131);
const browser = await launch();
let n = 0;
for (const device of Object.keys(SHOT_DEVICES)) {
  const d = SHOT_DEVICES[device];
  for (const lang of LANGS) {
    for (const mode of MODES) {
      const ctx = await browser.newContext({
        viewport: { width: d.width, height: d.height }, deviceScaleFactor: 2, isMobile: true,
      });
      await ctx.addInitScript(([age, l]) => {
        localStorage.setItem('byear_agegate_v1', age);
        localStorage.setItem('spellgame.locale', l);
      }, [mode.age, lang]);
      await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
      const page = await ctx.newPage();
      await page.goto(base, { waitUntil: 'load' });
      await page.waitForFunction(() => window.__spelltest && window.__spelltest.build() === 'testseam', null, { timeout: 30000 });
      await page.waitForTimeout(350);
      const name = `${device}_${lang}_${mode.key}.png`;
      // Clip to the top of the page (header + control groups + play zone) so the
      // grouping is the focus, not the keyboard/footer.
      await page.screenshot({ path: join(OUT, name), clip: { x: 0, y: 0, width: d.width, height: Math.min(d.height, 620) } });
      await ctx.close();
      n++;
    }
  }
}
await browser.close();
server.close();
console.log(`wrote ${n} screenshots to ${OUT}`);
