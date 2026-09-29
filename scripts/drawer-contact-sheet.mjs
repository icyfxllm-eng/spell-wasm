#!/usr/bin/env node
// CC-HUB-GROUP-L10N Done #7 and #11 — the contact sheet.
//
// One drawer per interface language, plus The Climb's name on its start
// surfaces for es, ru and zh. Writes PNGs and an index.html that lays them
// out side by side, so the whole set is one thing to look at rather than
// fifteen files to open.
//
//   node scripts/build-web-dev.sh   (or any served build)
//   node scripts/drawer-contact-sheet.mjs
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync, readdirSync } from 'node:fs';

const BASE = process.env.SHOT_BASE || 'http://127.0.0.1:8141/';
const OUT = 'audio_clarity/../contact-sheet';   // repo-root/contact-sheet
const AGE = JSON.stringify({ verdict: 'full', checkedAt: 1700000000 });
const LANGS = readdirSync('src/i18n/locales').filter((f) => f.endsWith('.json')).map((f) => f.replace('.json', '')).sort();

mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch();
const rows = [];

for (const lang of LANGS) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 812 }, deviceScaleFactor: 2, isMobile: true });
  await ctx.addInitScript(([age, l]) => {
    localStorage.setItem('byear_agegate_v1', age);
    localStorage.setItem('spellgame.locale', l);
  }, [AGE, lang]);
  await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 200, contentType: 'audio/mpeg', body: Buffer.from([]) }));
  const page = await ctx.newPage();
  await page.goto(BASE, { waitUntil: 'load' });
  await page.waitForTimeout(1600);
  // Dismiss whatever the front door put up, without caring what it says.
  await page.evaluate(() => {
    for (const e of document.querySelectorAll('.scrim.show')) e.classList.remove('show');
  });
  await page.waitForTimeout(300);
  await page.click('#navBurger');
  await page.waitForSelector('#navDrawer.show', { timeout: 4000 });
  await page.waitForTimeout(350);
  const shot = `${OUT}/drawer-${lang}.png`;
  await page.screenshot({ path: shot });
  const text = await page.evaluate(() => ({
    headers: [...document.querySelectorAll('#navDrawerScroll .nav-group')].map((e) => e.textContent),
    climb: (document.querySelector('.nav-row[data-mode="climb"] .nav-name') || {}).textContent || null,
    pill: (document.getElementById('setupChipText') || {}).textContent || null,
  }));
  rows.push({ lang, shot: `drawer-${lang}.png`, ...text });
  console.log(`${lang}: ${text.headers.join(' | ')}   climb=${text.climb}`);
  await ctx.close();
}

const html = `<!doctype html><meta charset="utf-8"><title>Drawer contact sheet</title>
<style>body{font:14px system-ui;background:#14121c;color:#eee;margin:24px}
h1{font-size:18px}figure{margin:0}figcaption{padding:6px 0;font-size:12px;color:#aaa}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:18px}
img{width:100%;border:1px solid #333;border-radius:8px}
b{color:#f5c36b}</style>
<h1>Drawer, one per interface language — CC-HUB-GROUP-L10N</h1>
<p style="max-width:70ch;color:#bbb;line-height:1.5">Two of the three Play
headers appear here. <b>Word puzzles</b> does not, and that is correct rather
than missing: every mode in that group — SpellDoku, Spell Search, Spell Cross,
Letter Forge, Word Chains — is <code>platforms: ["ios"]</code>, so in a browser
the group has no rows and an empty group renders no header. It appears on
device. Its string is held by the same gate as the other two
(<code>i18n-translated-check</code>: present, non-empty, and not equal to the
English), and the longest of the fifteen, "Словесные головоломки", was measured
at 375pt and fits on one line without shrinking.</p>
<div class="grid">
${rows.map((r) => `<figure><img src="${r.shot}" alt="${r.lang}">
<figcaption><b>${r.lang}</b><br>${r.headers.join(' · ')}<br>Climb: ${r.climb ?? '—'}<br>${r.pill ?? ''}</figcaption></figure>`).join('\n')}
</div>`;
writeFileSync(`${OUT}/index.html`, html);
console.log(`\ncontact sheet -> ${OUT}/index.html (${rows.length} languages)`);
await browser.close();
