// F2 gate: does the logical-property sweep move ANY element in LTR?
// PNG diffing can't answer this — the round screen has a blinking caret, a
// pulsing orb and a live timer, so it differs against ITSELF run to run.
// Element geometry is deterministic and names the offender.
// CC-RTL F2 evidence tool — did the logical-property sweep move anything in LTR?
//
// F2 asks for "visual regression screenshots proving LTR output is pixel-identical".
// Screenshots CANNOT prove that here, and I tried before writing this: the home
// and round screens differ against THEMSELVES between two runs of identical
// code, because the orb pulses (a transform, which moves getBoundingClientRect),
// the caret blinks and the timer ticks. A PNG diff of this app answers "was the
// frame captured at the same moment", not "did the layout change".
//
// So this measures every element's box instead. It is deterministic (animations
// are frozen below), and when it fails it names the element and the pixel delta
// rather than showing a coloured smudge.
//
// Two things this tool learned the hard way, both of which made it report a
// meaningless pass at first:
//
//   1. The harness serves dist-test/, NOT the repo root. Editing index.html and
//      measuring proves nothing until scripts/build-web-test.sh has run — the
//      first "998/998 identical" was one stale build compared against itself.
//   2. The first three screens never render ANY of the four inline margin edits,
//      which live in dialogs. The screens below open them on purpose.
//
// Usage — ALWAYS rebuild between sides, and ALWAYS run the typo control:
//   git show HEAD:index.html > index.html && bash scripts/build-web-test.sh
//   node tests/e2e/css-logical-geometry.mjs /tmp/before.json
//   git checkout index.html && bash scripts/build-web-test.sh
//   node tests/e2e/css-logical-geometry.mjs /tmp/after.json
//   diff <(jq -S . /tmp/before.json) <(jq -S . /tmp/after.json)
//
// A pass means nothing unless the gate can fail: introduce a one-character typo
// (margin-inline-strt) and confirm it reports a moved box. It should show
// #cancelImport shifting 145px.
import { startServer, launch, typeOnKeyboard } from './harness.mjs';
const out = process.argv[2];
const { server, base } = await startServer(8212);
const browser = await launch();
const all = {};
for (const [name, fn] of [
  ['home', async (p) => {}],
  ['round', async (p) => { await p.click('#orbWrap').catch(()=>{}); await p.waitForTimeout(400); await typeOnKeyboard(p, 'cat'); }],
  ['settings', async (p) => { await p.click('#setBtn').catch(()=>{}); await p.waitForTimeout(300); }],
  // The dialogs. Added after a negative control proved the first three screens
  // never render ANY of the four inline margin edits — the sweep's least
  // certain sites were being graded by a gate that couldn't see them.
  ['import',  async (p) => { await p.click('#importBtn').catch(()=>{}); await p.waitForTimeout(400); }],
  ['account', async (p) => { await p.click('#accountBtn').catch(()=>{}); await p.waitForTimeout(400); }],
]) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, deviceScaleFactor: 2 });
  await ctx.addInitScript(() => {
    localStorage.setItem('byear_agegate_v1', JSON.stringify({ verdict: 'full', checkedAt: 1700000000 }));
    localStorage.setItem('spellgame.locale', 'en');
  });
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const p = await ctx.newPage();
  // The orb pulses (transform: scale), and transforms DO move
  // getBoundingClientRect — that alone made the measurement differ against
  // itself. Freeze all animation so a mid-frame capture can't masquerade as a
  // layout change. Applied identically to before and after, so it cannot hide
  // a real regression.
  await p.goto(base);
  await p.addStyleTag({ content: '*,*::before,*::after{animation:none!important;transition:none!important}' });
  await p.waitForFunction(() => window.__spelltest?.build() === 'testseam', null, { timeout: 30000 });
  await p.waitForTimeout(600);
  await fn(p);
  await p.waitForTimeout(500);
  all[name] = await p.evaluate(() => {
    const path = (e) => { const b=[]; for(let n=e;n&&n.nodeType===1;n=n.parentElement){ b.unshift(n.id?`#${n.id}`:`${n.tagName.toLowerCase()}${n.className&&typeof n.className==='string'?'.'+n.className.trim().split(/\s+/).join('.'):''}:nth-child(${[...n.parentElement?.children||[]].indexOf(n)+1})`); if(n.id) break; } return b.join('>'); };
    const r = {};
    for (const e of document.querySelectorAll('*')) {
      const b = e.getBoundingClientRect();
      if (!b.width && !b.height) continue;   // invisible: nothing to move
      const k = path(e);
      // round to 2dp: sub-hundredth float noise is not a layout change
      r[k] = [b.x, b.y, b.width, b.height].map(v => Math.round(v * 100) / 100);
    }
    return r;
  });
  await ctx.close();
}
await browser.close(); server.close();
const fs = await import('node:fs');
fs.writeFileSync(out, JSON.stringify(all, null, 1));
console.log('  measured:', Object.entries(all).map(([k,v]) => `${k}=${Object.keys(v).length} boxes`).join(', '));
