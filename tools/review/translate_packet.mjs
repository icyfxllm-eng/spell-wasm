// Translate Phase A review packet (CC-TRANSLATE-SCREEN §8, scoped by Eric
// 2026-09-14: "light-only packet").
//
// Screenshots every screen state that can occur with today's data, at the
// default and the largest text size, from the test build (bash
// scripts/build-web-test.sh). The app has one theme, so one theme is shot. Nothing answers in production until a gloss file is signed,
// so each state marks a language's REAL rows audited through the test-build-only
// seam, exactly as tests/e2e/specs/translate-screen.mjs does. The seam flips a
// flag; it cannot author a row, so every word pictured is a real bank word.
//
// Run:  node tools/review/translate_packet.mjs
// Out:  docs/review/translate-phase-a/*.png and README.md
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startServer, launch, openApp } from '../../tests/e2e/harness.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = join(ROOT, 'docs', 'review', 'translate-phase-a');

async function openTranslate(page, audit) {
  await page.evaluate((langs) => {
    localStorage.setItem('spell_flag_translate', 'on');
    for (const l of langs) window.__spelltest.translateAudit(l, true);
  }, audit);
  await page.evaluate(() => document.getElementById('trOpenBtn').click());
  await page.waitForSelector('#trScreen.show', { timeout: 5000 });
}

async function commit(page, queries) {
  for (const q of queries) {
    await page.fill('#trSrcInput', q);
    await page.waitForTimeout(120);
    const hit = await page.$('#trSuggest [data-tr-sugg]');
    if (hit) {
      await hit.click();
      await page.waitForFunction(() => document.getElementById('trTgtWord').textContent.length > 0, null, { timeout: 4000 });
      return q;
    }
  }
  throw new Error(`no suggestion for any of ${JSON.stringify(queries)}`);
}

const pickTarget = async (page, lang) => {
  await page.click('#trTgtLang');
  await page.click(`#trPicker [data-tr-lang="${lang}"]`);
  await page.waitForTimeout(150);
};

// Each state returns a one-line caption of what it shows.
const STATES = [
  ['translate-empty', 'The screen as it opens: English to Spanish, nothing asked yet.', async (page) => {
    await openTranslate(page, ['es']);
  }],
  ['translate-single-sense', 'An English word committed from a suggestion; its Spanish answer and chosen sense.', async (page) => {
    await openTranslate(page, ['es']);
    const q = await commit(page, ['cat', 'dog', 'water', 'house', 'a']);
    return `searched "${q}"`;
  }],
  ['translate-no-target-answer', 'A concept Spanish has and Japanese lacks: the target card declines with a way out.', async (page) => {
    await openTranslate(page, ['es', 'ja']);
    const gap = await page.evaluate(() => window.__spelltest.translateGap('en', 'es', 'ja'));
    await commit(page, [gap]);
    await pickTarget(page, 'ja');
    return `concept "${gap}"`;
  }],
  ['translate-audio-unavailable', 'Every audio source fails: the control and the note say so inline (D12).', async (page, ctx) => {
    await ctx.route('**/api/speak**', (r) => r.fulfill({ status: 503, contentType: 'text/plain', body: '' }));
    await page.evaluate(() => localStorage.setItem('spell_audio_src', 'server-only'));
    await openTranslate(page, ['es']);
    await commit(page, ['cat', 'dog', 'water', 'house', 'a']);
    await page.click('#trTgtAudio');
    await page.waitForFunction(() => document.getElementById('trTgtAudio').dataset.state === 'unavailable', null, { timeout: 6000 });
  }],
  ['translate-unentitled-target', 'A target language the free tier does not own, after pressing Spell it (D3).', async (page) => {
    await openTranslate(page, ['es', 'ja']);
    await commit(page, ['cat', 'dog', 'water', 'house', 'a']);
    await pickTarget(page, 'ja');
    const word = await page.$eval('#trTgtWord', (e) => e.textContent);
    await page.click('#trSpell');
    await page.waitForTimeout(400);
    const stayed = await page.$eval('#trScreen', (e) => e.classList.contains('show'));
    if (stayed) return `Spell it declined "${word}" inline`;
    // Served at the preview tier: the screen closed into the game. Show the
    // pair as it stood, and say what happened.
    await page.evaluate(() => document.getElementById('trOpenBtn').click());
    await page.waitForSelector('#trScreen.show', { timeout: 5000 });
    return `Spell it served "${word}" at the preview tier; pictured is the pair on reopening`;
  }],
  ['translate-rtl-ar', 'An Arabic target: its own card mirrors, the spine does not.', async (page) => {
    await openTranslate(page, ['ar']);
    await pickTarget(page, 'ar');
    await commit(page, ['cat', 'dog', 'water', 'house', 'a']);
  }],
];

const NOT_SHOT = [
  ['translate-multi-sense', 'Cannot occur with today\'s data: every gloss file maps each word to one concept, so no word has a second sense to pick. The four-then-more rule (D9) is unit-tested; picturing it would need an invented row, which I7 forbids.'],
  ['translate-jr-band-decline', 'Cannot be reached through the screen: the Spell Jr filter keeps too-hard words out of the suggestions and answers, so Spell it is never offered one. The verdict is unit-tested.'],
  ['translate-cap-reached', 'Cannot occur: My Words has no word cap (the free limit is two custom lists). There is no state to picture.'],
];

mkdirSync(OUT, { recursive: true });
const { server, base } = await startServer(8130);
const browser = await launch();
const rows = [];
try {
  for (const [name, caption, setup] of STATES) {
    for (const size of ['default', 'largest']) {
      const { ctx, page } = await openApp(browser, base, { lang: 'en' });
      try {
        if (size === 'largest') await page.evaluate(() => document.body.classList.add('big-text'));
        const detail = await setup(page, ctx);
        await page.waitForTimeout(250);
        const file = `${name}--${size}.png`;
        await page.screenshot({ path: join(OUT, file) });
        rows.push({ name, size, file, caption, detail: detail || '' });
        process.stdout.write(`  shot ${file}${detail ? ` (${detail})` : ''}\n`);
      } finally { await ctx.close(); }
    }
  }
} finally { await browser.close(); server.close(); }

const md = [
  '# Translate Phase A — review packet',
  '',
  'One theme (Eric, 2026-09-14: "light-only packet"). The app has a single look, and it is the dark',
  'one pictured; there is no light or dark switch to shoot both sides of. iPhone SE size (375×667 at 2×),',
  'default text and the largest text size. Generated by `node tools/review/translate_packet.mjs`',
  'from the test build; each state marks the named languages\' real rows audited through the',
  'test-only seam, since no gloss file is signed yet. Every word pictured is a real bank word.',
  '',
  '| State | Default text | Largest text | What it shows |',
  '|---|---|---|---|',
  ...STATES.map(([name, caption]) => {
    const d = rows.find((r) => r.name === name && r.size === 'default');
    const l = rows.find((r) => r.name === name && r.size === 'largest');
    const detail = d && d.detail ? ` (${d.detail})` : '';
    return `| \`${name}\` | ![](${d.file}) | ![](${l.file}) | ${caption}${detail} |`;
  }),
  '',
  '## Named fixtures that are not pictured',
  '',
  ...NOT_SHOT.map(([name, why]) => `- \`${name}\`: ${why}`),
  '',
  'The "door-taken instrumentation schema" the spec asked for is not included: instrumentation (F14) was cut on 2026-09-12 under the zero-telemetry posture.',
  '',
  '## Seen in these screenshots, for review (2026-09-14)',
  '',
  '- **Spell it\'s decline borrows the Calendar\'s words.** `translate-unentitled-target` reads "That word isn\'t ready for planning yet": the screen reuses `cal.declined`. Translate needs its own line, authored in all 15 locales.',
  '- **An English sense repeats the word.** Under an English source, the sense line shows the concept, which for English is the word itself ("cat" under "cat").',
  '- **Word of the day and Passport look unstyled**, a bare word and a count below the action row; at the largest text size they fall below the bottom of an SE screen.',
  '- **At the largest text size, "Now spell it" wraps** to two lines. The action row still holds still (I1, tested).',
  '',
];
writeFileSync(join(OUT, 'README.md'), md.join('\n'));
process.stdout.write(`PACKET ${rows.length} screenshots -> ${OUT}\n`);
