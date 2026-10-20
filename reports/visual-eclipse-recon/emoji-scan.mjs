// R9 emoji-as-UI scanner. Usage: node emoji-scan.mjs <repo-root> <out.tsv>
// Scans src/**/*.rs, index.html (HTML + CSS content:), top-level *.js, and
// src/i18n/locales/en.json for Extended_Pictographic code points PLUS icon-ish
// symbol blocks that are NOT Extended_Pictographic (check marks, crosses,
// stars, arrows, geometric shapes, misc technical) since those are used as icons too.
// Each hit is classified by context into: comment | share-text | word-content |
// test | ui-icon(candidate).
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = process.argv[2];
const out = process.argv[3];
const EP = /\p{Extended_Pictographic}/u;
// supplemental symbol ranges used as icons (not EP): arrows, misc technical,
// geometric shapes, misc symbols, dingbats, misc symbols & arrows
const SUP = /[←-⇿⌀-⏿■-◿☀-⛿✀-➿⬀-⯿✕×]/u;
const SKIN_VS = /[️︎‍\u{1F3FB}-\u{1F3FF}]/u;

function walk(d, acc = []) {
  for (const n of readdirSync(d)) {
    const p = join(d, n);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, acc); else acc.push(p);
  }
  return acc;
}

const files = [
  ...walk(join(root, 'src')).filter((f) => f.endsWith('.rs')),
  join(root, 'index.html'),
  ...readdirSync(root).filter((f) => f.endsWith('.js')).map((f) => join(root, f)),
  join(root, 'src/i18n/locales/en.json'),
];

const rows = [];
for (const f of files) {
  const rel = relative(root, f);
  const lines = readFileSync(f, 'utf8').split('\n');
  let inTestMod = false;
  let inCssBlockComment = false;
  lines.forEach((raw, i) => {
    // decode Rust \u{XXXX}, JS \uXXXX (incl. surrogate pairs) and HTML numeric entities so escaped icons are seen
    const line = raw
      .replace(/\\u\{([0-9a-fA-F]{2,6})\}/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
      .replace(/\\u(d[89ab][0-9a-f]{2})\\u(d[c-f][0-9a-f]{2})/gi, (_, a, b) => String.fromCharCode(parseInt(a, 16), parseInt(b, 16)))
      .replace(/\\u([0-9a-fA-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))
      .replace(/&#x([0-9a-fA-F]+);/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
      .replace(/&#([0-9]+);/g, (_, d) => String.fromCodePoint(parseInt(d, 10)));
    if (/#\[cfg\(test\)\]/.test(line)) inTestMod = true;
    const chars = [...line];
    const hits = [];
    for (const c of chars) {
      if (SKIN_VS.test(c)) continue;
      if (EP.test(c)) hits.push([c, 'EP']);
      else if (SUP.test(c)) hits.push([c, 'SYM']);
    }
    if (!hits.length) return;
    const t = line.trim();
    let cls;
    // Comment detection: whole-line comments and trailing comments that hold all hits.
    const codePart = (() => {
      let s = line;
      const ci = s.indexOf('//');
      if (ci >= 0 && !/https?:\/\//.test(s.slice(Math.max(0, ci - 6), ci + 2))) s = s.slice(0, ci);
      s = s.replace(/<!--.*?-->/g, '').replace(/\/\*.*?\*\//g, '');
      return s;
    })();
    const codeHits = hits.filter(([c]) => codePart.includes(c));
    if (/^(\/\/|\/\*|\*|<!--|#)/.test(t) || !codeHits.length) cls = 'comment';
    else if (inTestMod || /tests?\//.test(rel) || /_tests?\.rs$/.test(rel)) cls = 'test';
    else if (/share|clipboard|navigator\.share|copy_text|grid_text|result_text|emoji_grid|square/i.test(line)) cls = 'share-text';
    else if (/words?_?(list|bank)|lexicon|homophone|glyph_for_word|WORDS|example|sentence/i.test(line)) cls = 'word-content?';
    else if (/console\.|log::|eprintln|println|panic!|debug|assert/i.test(line)) cls = 'log/debug';
    else cls = 'ui-icon?';
    const uniq = [...new Set(codeHits.length ? codeHits.map((h) => h[0] + (h[1] === 'SYM' ? '*' : '')) : hits.map((h) => h[0] + (h[1] === 'SYM' ? '*' : '')))];
    rows.push({ loc: `${rel}:${i + 1}`, emoji: uniq.join(' '), cls, ctx: t.slice(0, 160).replace(/\t/g, ' ') });
  });
}
writeFileSync(out, 'loc\temoji\tclass\tcontext\n' + rows.map((r) => `${r.loc}\t${r.emoji}\t${r.cls}\t${r.ctx}`).join('\n') + '\n');
const by = {};
for (const r of rows) by[r.cls] = (by[r.cls] || 0) + 1;
console.log(JSON.stringify(by), 'rows', rows.length);
