#!/usr/bin/env node
// CC-VISUAL-ECLIPSE F8: design/icons -> the inline sprite in index.html and
// src/icons_data.rs. Like tokens-build, it only checks by default.
//
// Each icon is drawn on a 24 px grid with rounded strokes. The sprite keeps
// only the shapes: stroke width (the --icon-stroke token), colour
// (currentColor) and caps come from the .icon rule, so the theme drives them.
//
//   node scripts/icons-build.mjs            # check the sprite and the Rust table are current
//   node scripts/icons-build.mjs --write    # regenerate both
//   node scripts/icons-build.mjs --selftest
import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const DIR = join(ROOT, 'design/icons');
const BEGIN = '<!-- icons:begin';
const END = '<!-- icons:end -->';

export function load(dir = DIR) {
  const index = JSON.parse(readFileSync(join(dir, 'index.json'), 'utf8')).icons;
  const files = readdirSync(dir).filter((f) => f.endsWith('.svg')).map((f) => f.slice(0, -4)).sort();
  const problems = [];
  for (const n of files) if (!(n in index)) problems.push(`${n}.svg has no entry in index.json`);
  for (const n of Object.keys(index)) if (!files.includes(n)) problems.push(`index.json names ${n} but there is no ${n}.svg`);
  const icons = files.map((name) => {
    const svg = readFileSync(join(dir, `${name}.svg`), 'utf8');
    if (!/viewBox="0 0 24 24"/.test(svg)) problems.push(`${name}.svg is not on the 24 px grid`);
    const body = svg.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '').trim();
    return { name, body, emoji: index[name] };
  });
  return { icons, problems };
}

export function sprite(icons) {
  return [
    `${BEGIN}: GENERATED from design/icons by scripts/icons-build.mjs. Never edit by hand. -->`,
    '<svg xmlns="http://www.w3.org/2000/svg" style="position:absolute;width:0;height:0;overflow:hidden" aria-hidden="true">',
    ...icons.map((i) => `<symbol id="i-${i.name}" viewBox="0 0 24 24">${i.body}</symbol>`),
    '</svg>',
    END,
  ].join('\n');
}

export function rust(icons) {
  const esc = (s) => s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return [
    '//! GENERATED from design/icons by scripts/icons-build.mjs. Do not edit; the',
    '//! pre-push hook checks it is current. (name, the emoji it replaced)',
    '',
    `pub const ICONS: &[(&str, &str)] = &[`,
    ...icons.map((i) => `    ("${esc(i.name)}", "${esc(i.emoji)}"),`),
    '];',
    '',
  ].join('\n');
}

function splice(html, block) {
  const a = html.indexOf(BEGIN);
  if (a >= 0) {
    const b = html.indexOf(END, a) + END.length;
    return html.slice(0, a) + block + html.slice(b);
  }
  // First run: right after the <body> TAG -- at the start of a line, so a
  // "<body>" mentioned in a CSS comment cannot catch it (the first version did).
  const m = html.match(/^<body[^>]*>/m);
  return html.slice(0, m.index + m[0].length) + '\n' + block + html.slice(m.index + m[0].length);
}

const MAIN = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (MAIN) {
  const args = process.argv.slice(2);
  if (args.includes('--selftest')) {
    const { icons } = load();
    const s = sprite(icons);
    const every = icons.every((i) => s.includes(`<symbol id="i-${i.name}"`));
    // A stroke width baked into a symbol would override the token.
    const tokenOnly = !/<symbol[^>]*>[^\n]*stroke-width/.test(s);
    // A "<body>" inside a CSS comment must not catch the sprite; the real tag does.
    const html = '<html><style>/* lives on <body> */</style>\n<body class="x">\n<p>hi</p></body></html>';
    const once = splice(html, s), twice = splice(once, s);
    const placed = once.indexOf(BEGIN) > once.indexOf('<body class="x">');
    if (once === twice && every && tokenOnly && placed) {
      console.log('icons-build selftest: OK (splice is idempotent, every icon becomes a symbol)');
      process.exit(0);
    }
    console.error('icons-build selftest: FAILED');
    process.exit(1);
  }
  const { icons, problems } = load();
  if (problems.length) {
    console.error(`icons-build: FAILED\n  ${problems.join('\n  ')}`);
    process.exit(1);
  }
  const htmlPath = join(ROOT, 'index.html');
  const rsPath = join(ROOT, 'src/icons_data.rs');
  const html = readFileSync(htmlPath, 'utf8');
  const wantHtml = splice(html, sprite(icons));
  const wantRs = rust(icons);
  let rsNow = '';
  try { rsNow = readFileSync(rsPath, 'utf8'); } catch {}
  if (args.includes('--write')) {
    writeFileSync(htmlPath, wantHtml);
    writeFileSync(rsPath, wantRs);
    console.log(`icons-build: wrote ${icons.length} icons to index.html and src/icons_data.rs`);
  } else if (wantHtml !== html || wantRs !== rsNow) {
    console.error('icons-build: FAILED — the sprite or src/icons_data.rs is stale. Run: node scripts/icons-build.mjs --write');
    process.exit(1);
  } else {
    console.log(`icons-build: OK, ${icons.length} icons, sprite and src/icons_data.rs are current.`);
  }
}
