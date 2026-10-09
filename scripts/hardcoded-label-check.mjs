#!/usr/bin/env node
// Visible text in index.html that no i18n key covers.
//
// WHY THIS IS NOT ALREADY CAUGHT. i18n-translated-check compares each KEY's
// translations and fails when a locale still holds English. A label written
// straight into the markup has no key, so there is nothing for it to
// compare — the string simply ships in English to all fifteen languages and
// nothing complains. That is how `vsExit` ("✕ Exit", with an aria-label
// hardcoded in English) and `climbClose` ("Close", no key at all) survived.
// Presence-vs-translation has a third case: ABSENCE.
//
// WHAT COUNTS. Text under an element carrying data-i18n is covered, and so
// is text under an ancestor that carries it. An element whose id the Rust
// side writes with set_text/set_html holds a placeholder, not a label.
// Digits, punctuation and single letters are not labels.
//
// TWO LISTS, BOTH DELIBERATE:
//   EXEMPT  — brand names and dev-only surfaces. Never translated, by
//             intent, so they are not debt.
//   AWAITING_AUDIT — real player-facing labels with no key yet. Adding one
//             means fifteen audited translations, which is an auditor's
//             job, not a checker's. They are listed BY TEXT so the list
//             cannot quietly absorb a new offender: a different string
//             fails even though the count is unchanged.
//
//   node scripts/hardcoded-label-check.mjs
//   node scripts/hardcoded-label-check.mjs --selftest

import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const SKIP_TAGS = new Set(['script', 'style', 'title', 'option', 'svg', 'path', 'head']);
const VOID_TAGS = new Set(['br', 'img', 'input', 'meta', 'link', 'hr', 'source', 'use']);
const WORD = /[A-Za-z]{2,}/;

// Brand, and surfaces a player never reaches.
const EXEMPT_TEXT = [
  'SPELL', 'Spell', 'spell', 'Game',
  'Dev preview', '🏁 Spell Racing', '✍️ CJK ink probe (F1)', 'ع Arabic (العربية)',
  '🎟️ Test entitlements (Complete)',
  'Native status (debug)', 'checking…', 'Audio source (debug)', 'nothing played yet',
  // The CJK ink probe, reached only through devOpenInk in src/dev.rs.
  'Undo', 'Clear', 'Skip', 'Read it',
];

// Player-facing and genuinely untranslated. Each needs an audited string in
// fifteen languages before it can leave this list. Eric's, 2026-10-09.
const AWAITING_AUDIT = [
  'vs',
  'Save to board',
  'Chain broken!',
  'No chains yet — be the first to start one.',
  'Spell a few words to start tracking your accuracy by difficulty.',
  '🏔 The Climb',
];

// Ids the Rust side writes through a VARIABLE rather than a literal, so the
// scan below cannot see the write. Kept short and explicit on purpose.
const WRITTEN_VIA_VARIABLE = new Set(['vsP1Name', 'vsP2Name']);

function rustWrittenIds(root) {
  const ids = new Set(WRITTEN_VIA_VARIABLE);
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (p.endsWith('.rs')) {
        const t = readFileSync(p, 'utf8');
        for (const m of t.matchAll(/set_(?:text|html)\(\s*"([A-Za-z0-9_-]+)"/g)) ids.add(m[1]);
      }
    }
  };
  const src = join(root, 'src');
  if (existsSync(src)) walk(src);
  return ids;
}

export function findings(root = '.') {
  const htmlPath = join(root, 'index.html');
  if (!existsSync(htmlPath)) return [{ id: '-', text: `${htmlPath} is missing` }];
  let html = readFileSync(htmlPath, 'utf8');
  html = html.replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<script\b[\s\S]*?<\/script>/gi, '')
    .replace(/<style\b[\s\S]*?<\/style>/gi, '');
  const written = rustWrittenIds(root);

  const out = [];
  const stack = [];
  const TAG = /<(\/?)([a-zA-Z][\w-]*)([^>]*)>/g;
  let pos = 0;
  let m;
  while ((m = TAG.exec(html))) {
    const raw = html.slice(pos, m.index).trim();
    pos = TAG.lastIndex;
    if (raw && stack.length) {
      const top = stack[stack.length - 1];
      const covered = stack.some((f) => f.i18n);
      const skipped = stack.some((f) => SKIP_TAGS.has(f.tag));
      const text = raw.replace(/\s+/g, ' ')
        .replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>')
        .replace(/&nbsp;/g, ' ').replace(/&#39;/g, "'").replace(/&quot;/g, '"');
      if (!covered && !skipped && WORD.test(text) && !written.has(top.id)) {
        out.push({ id: top.id || '-', tag: top.tag, text });
      }
    }
    const [, closing, tagRaw, attrs] = m;
    const tag = tagRaw.toLowerCase();
    if (closing) {
      for (let i = stack.length - 1; i >= 0; i -= 1) {
        if (stack[i].tag === tag) { stack.length = i; break; }
      }
    } else if (!attrs.trimEnd().endsWith('/') && !VOID_TAGS.has(tag)) {
      // data-i18n and data-i18n-html replace the VISIBLE text, so they
      // cover it. data-i18n-aria does NOT -- it sets only the accessible
      // name, which is precisely how `vsExit` shipped "✕ Exit" in English
      // while looking annotated. Matching it as coverage would reintroduce
      // the exact bug this check exists for.
      stack.push({ tag, i18n: /data-i18n(?:-html)?=/.test(attrs), id: (/id="([^"]+)"/.exec(attrs) || [])[1] });
    }
  }
  return out;
}

export function problems(root = '.') {
  const allowed = new Set([...EXEMPT_TEXT, ...AWAITING_AUDIT]);
  const seen = new Set();
  const out = [];
  for (const f of findings(root)) {
    if (allowed.has(f.text)) { seen.add(f.text); continue; }
    out.push(`<${f.tag} id=${f.id}> has the hardcoded label ${JSON.stringify(f.text)} `
      + '— no i18n key covers it, so it ships English to all 15 languages');
  }
  // A list that outlives its entries stops being a record and starts being
  // a place to hide things.
  for (const t of AWAITING_AUDIT) {
    if (!seen.has(t)) {
      out.push(`AWAITING_AUDIT still lists ${JSON.stringify(t)} but it is no longer in `
        + 'index.html — if it was translated or removed, drop it from the list');
    }
  }
  return out;
}

function selftest() {
  const { mkdtempSync, writeFileSync, mkdirSync } = require_fs();
  const dir = mkdtempSync(join(tmp(), 'hardcoded-'));
  mkdirSync(join(dir, 'src'), { recursive: true });
  const write = (body, rs = '') => {
    writeFileSync(join(dir, 'index.html'), body);
    writeFileSync(join(dir, 'src', 'a.rs'), rs);
    return findings(dir);
  };
  const cases = [
    ['a keyed label is clean', '<div data-i18n="a.b">Hello</div>', '', 0],
    ['a bare label is found', '<div>Hello there</div>', '', 1],
    ['a child of a keyed element is clean', '<div data-i18n="a.b"><span>Hi there</span></div>', '', 0],
    ['a Rust-written placeholder is clean', '<div id="x">Hello</div>', 'set_text("x", &v);', 0],
    ['digits are not a label', '<div>0</div>', '', 0],
    ['script contents are not a label', '<script>var hello = 1;</script>', '', 0],
    ['data-i18n-html covers visible text', '<div data-i18n-html="a.b"><b>Hi there</b></div>', '', 0],
    ['data-i18n-ARIA does not cover visible text',
      '<button data-i18n-aria="a.b" aria-label="Exit">Exit now</button>', '', 1],
  ];
  let bad = 0;
  for (const [name, body, rs, want] of cases) {
    const got = write(body, rs).length;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  if (bad) { console.error('hardcoded-label-check selftest: FAILED'); process.exit(1); }
  console.log('hardcoded-label-check selftest: OK');
}
function require_fs() { return fsmod; }
function tmp() { return osmod.tmpdir(); }
import * as fsmod from 'node:fs';
import * as osmod from 'node:os';

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = problems();
  if (bad.length) {
    console.error('hardcoded-label-check: FAILED');
    for (const b of bad) console.error(`  ${b}`);
    console.error('\n  A label with no key is invisible to i18n-translated-check.');
    console.error('  Reuse an audited key if one fits, or add it to AWAITING_AUDIT');
    console.error('  so the debt is named rather than lost.');
    process.exit(1);
  }
  console.log(`hardcoded-label-check: OK — ${AWAITING_AUDIT.length} label(s) awaiting an auditor, all still present`);
}
