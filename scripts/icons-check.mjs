#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F8 / AT-20: no emoji used as a UI icon, and every mode
// names an icon from the set.
//
// "UI icon" is decided by place, with every exception named here and why:
//   - config/modes.json: every mode's `icon` must be a name in design/icons.
//   - index.html markup (scripts, styles and comments aside): no
//     Extended_Pictographic character outside an icon slot's emoji fallback.
//   - src/i18n/locales/en.json: none, except the keys below.
//   - Rust string literals: none, except the files below and any literal whose
//     line (or the comment line above it) carries `icon-exempt: <reason>`.
//
//   node scripts/icons-check.mjs
//   node scripts/icons-check.mjs --selftest
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const EP = /\p{Extended_Pictographic}/u;

/** Markup ids that may keep a pictograph, and why. */
const ALLOW_IDS = {
  devMenu: 'the dev door: not reachable by players (5 taps on the logo, dev builds)',
  trTranslitToggle: '"Aa ↔ 拼" is typography in a label, not an icon',
};
/** i18n keys that may keep a pictograph, and why. */
const ALLOW_KEYS = {
  'share.chain': 'share message text (F8.4: emoji in share text may stay)',
  'share.daily': 'share message text',
  'share.dailyStreak': 'share message text',
  'practice.shareText': 'share message text',
  'sd.spellWord': 'an instruction quoting the play button ("Tap ▶ ...")',
  'ws.pickSlot': 'an instruction quoting the play button',
  'tr.translit': 'typography in a label',
};
/** Rust files whose string literals may keep a pictograph, and why. */
const ALLOW_FILES = {
  'src/boardgame_screen.rs': 'Board Game scene art: pieces, boosts, medals, the finish flag (mode-owned, F3.6/E-Q7 rule)',
  'src/randospell_screen.rs': 'Randospell scene art: the seat pieces (mode-owned)',
  'src/consts.rs': 'achievement badges (content of the achievements list)',
  'src/dom.rs': 'the achievement toast trophy (achievement content)',
  'src/calendar_ui.rs': 'the Spell Calendar cheer line (content)',
  'src/lists_screen.rs': 'the kid-list lock mark (content of the word list)',
  'src/spellpic_export.rs': 'the exported picture image (content; F11 replaces cards)',
  'src/wordpic.rs': 'Spell Picture subjects (picture content)',
  'src/wordpic_screen.rs': 'Spell Picture subjects (picture content)',
  'src/icons_data.rs': 'the fallback emoji table itself',
  'src/icons.rs': 'tests of the fallback table',
};

function stripMarkup(html) {
  html = html.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<style[\s\S]*?<\/style>/g, '').replace(/<!--[\s\S]*?-->/g, '');
  html = html.replace(/<span class="ic-emoji">[^<]*<\/span>/g, '');
  for (const id of Object.keys(ALLOW_IDS)) {
    const at = html.search(new RegExp(`<(\\w+)[^>]*\\bid="${id}"`));
    if (at < 0) continue;
    const tag = html.slice(at).match(/^<(\w+)/)[1];
    // Cut the element: match its closing tag by depth.
    const re = new RegExp(`<${tag}\\b|</${tag}>`, 'g');
    re.lastIndex = at;
    let depth = 0, end = html.length, m;
    while ((m = re.exec(html))) {
      depth += m[0].startsWith('</') ? -1 : 1;
      if (depth === 0) { end = m.index + m[0].length; break; }
    }
    html = html.slice(0, at) + html.slice(end);
  }
  return html;
}

export function check({ html, en, modes, iconNames, rust }) {
  const bad = [];
  for (const m of modes) {
    if (!iconNames.includes(m.icon)) bad.push(`modes.json: ${m.id} has icon "${m.icon}", not a name in design/icons`);
  }
  stripMarkup(html).split('\n').forEach((l) => {
    if (EP.test(l)) bad.push(`index.html: an emoji icon in "${l.trim().slice(0, 90)}"`);
  });
  for (const [k, v] of Object.entries(en)) {
    if (EP.test(v) && !(k in ALLOW_KEYS)) bad.push(`en.json: ${k} carries an emoji ("${v.slice(0, 40)}")`);
  }
  for (const [file, src] of rust) {
    if (file in ALLOW_FILES) continue;
    let inTest = false;
    const lines = src.split('\n');
    lines.forEach((line, i) => {
      if (/#\[cfg\(test\)\]/.test(line)) inTest = true;
      if (inTest || /^\s*\/\//.test(line)) return;
      // A literal may stay when its line, or the comment line above it, says why.
      if (/icon-exempt:/.test(line) || /icon-exempt:/.test(lines[i - 1] || '')) return;
      for (const m of line.matchAll(/"((?:[^"\\]|\\.)*)"/g)) {
        const s = m[1].replace(/\\u\{([0-9a-fA-F]+)\}/g, (_, h) => String.fromCodePoint(parseInt(h, 16)));
        if (EP.test(s)) bad.push(`${file}:${i + 1}: an emoji in a string literal`);
      }
    });
  }
  return bad;
}

function load() {
  const walk = (d) => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
  const modesRaw = JSON.parse(readFileSync(join(ROOT, 'config/modes.json'), 'utf8'));
  return {
    html: readFileSync(join(ROOT, 'index.html'), 'utf8'),
    en: JSON.parse(readFileSync(join(ROOT, 'src/i18n/locales/en.json'), 'utf8')),
    modes: Array.isArray(modesRaw) ? modesRaw : modesRaw.modes,
    iconNames: Object.keys(JSON.parse(readFileSync(join(ROOT, 'design/icons/index.json'), 'utf8')).icons),
    rust: walk(join(ROOT, 'src')).filter((f) => f.endsWith('.rs') && !/\/tests?\//.test(f) && !/tests?\.rs$/.test(f))
      .map((f) => [relative(ROOT, f).replace(/\\/g, '/'), readFileSync(f, 'utf8')]),
  };
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const input = load();
  if (process.argv.includes('--selftest')) {
    const plants = [
      { ...input, html: input.html.replace('</body>', '<button id="x">🔥 Planted</button></body>') },
      { ...input, en: { ...input.en, 'x.planted': '⭐ Planted' } },
      { ...input, modes: [...input.modes, { id: 'planted', icon: '🎯' }] },
      { ...input, rust: [...input.rust, ['src/planted.rs', 'let s = "\\u{1F525} x";']] },
    ];
    const caught = plants.map((p) => check(p).length > 0);
    const clean = check(input).length === 0;
    if (caught.every(Boolean) && clean) {
      console.log('icons-check selftest: OK (an emoji planted in markup, a string, the registry or Rust is caught; the tree is clean)');
      process.exit(0);
    }
    console.error(`icons-check selftest: FAILED (caught ${caught}, tree clean ${clean})`);
    process.exit(1);
  }
  const bad = check(input);
  if (bad.length) {
    console.error(`icons-check: FAILED — emoji used as UI icons (AT-20):\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`icons-check: OK — ${input.modes.length} modes name an icon, no emoji UI icons (${Object.keys(ALLOW_KEYS).length} strings and ${Object.keys(ALLOW_FILES).length} files exempt by name)`);
}
