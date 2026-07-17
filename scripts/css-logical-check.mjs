#!/usr/bin/env node
// CC-RTL F2 gate: physical left/right CSS properties must not come back.
//
// Logical properties (margin-inline-start, inset-inline-end, text-align:start)
// follow READING ORDER, so they flip automatically when dir="rtl". Physical
// ones don't. In LTR the two are identical, which is exactly why a physical
// property can sit in the stylesheet looking perfectly fine for years and then
// break the moment RTL_SUPPORTED flips on. This check is the only thing
// standing between "we did the sweep once" and "it silently eroded".
//
// A physical property is not automatically a bug. Three kinds are CORRECT and
// are allowlisted individually below — each needs a reason, because "left"
// meaning "the left of the screen" is legitimate when the thing being
// positioned is not text:
//   1. the centering idiom (left:50% + translateX(-50%)) — geometry, not order
//   2. env(safe-area-inset-*)                            — the notch is physical
//   3. a specular highlight                              — light doesn't flip
//
// Run: node scripts/css-logical-check.mjs [--selftest]
import { readFileSync } from 'node:fs';

const FILES = ['index.html', 'privacy.html'];

// Each entry must name the exact declaration AND why it is physical on purpose.
// Anything not listed here fails. Adding an entry is a deliberate, reviewed act.
const ALLOW = [
  {
    why: 'centering idiom: `left:50%` + `translateX(-50%)` centres an element. Centre is\n' +
         '       geometric, not directional — converting it would MIS-centre in RTL, not fix it.\n' +
         '       Matched structurally (the pair anywhere in the same rule), because the two\n' +
         '       declarations are not always adjacent.',
    test: (decl, rule) => /^(left|right)\s*:\s*50%/.test(decl) && /translateX\(\s*-50%\s*\)/.test(rule),
  },
  {
    why: 'specular highlight on the orb: a light source does not flip with reading order',
    test: (decl, rule) => /^left\s*:\s*15%/.test(decl) && /\.orb::before/.test(rule),
  },
  {
    why: 'the notch is at the physical left/right edge in every locale, RTL included',
    test: (decl) => /^padding-(left|right)\s*:\s*env\(safe-area-inset-/.test(decl),
  },
];

const PHYSICAL = /(?:^|[;{"\s])((?:margin|padding|border|scroll-margin|scroll-padding)-(?:left|right)\s*:|text-align\s*:\s*(?:left|right)\b|(?<![\w-])(?:left|right)\s*:)/g;

function scan(text, file) {
  const out = [];
  text.split('\n').forEach((line, i) => {
    // Skip comment-only lines: prose may legitimately say "left".
    if (/^\s*(\/\*|\*|<!--)/.test(line)) return;
    for (const m of line.matchAll(PHYSICAL)) {
      const decl = line.slice(m.index).replace(/^[;{"\s]+/, '');
      // `line` is the enclosing rule: this stylesheet keeps one rule per line,
      // so it is the right scope for a structural test like the centering pair.
      if (ALLOW.some((a) => a.test(decl, line))) continue;
      out.push({ file, line: i + 1, decl: decl.split(/[;}]/)[0].trim() });
    }
  });
  return out;
}

if (process.argv.includes('--selftest')) {
  // A check that cannot fail is worse than no check: it reports success forever.
  // Prove this one detects each shape it claims to catch.
  const cases = [
    ['.x{margin-left:4px}', 'margin-left'],
    ['.x{text-align:left}', 'text-align:left'],
    ['.x{right:12px}', 'bare right'],
    ['.x{border-right:1px solid red}', 'border-right'],
  ];
  let bad = 0;
  for (const [css, name] of cases) {
    const hits = scan(css, 'selftest');
    console.log(`  ${hits.length ? 'caught' : 'MISSED'}  ${name}`);
    if (!hits.length) bad++;
  }
  // And prove the allowlist actually suppresses, or every run would fail.
  for (const [css, name] of [
    ['.t{position:absolute;left:50%;transform:translateX(-50%)}', 'centering idiom'],
    ['#toast{position:fixed;left:50%;bottom:24px;transform:translateX(-50%) translateY(20px)}', 'centering idiom, non-adjacent + extra translate'],
    ['.orb::before{top:8%;left:15%}', 'specular highlight'],
    ['.pad{padding-left:env(safe-area-inset-left, 0px)}', 'safe-area inset'],
  ]) {
    const alw = scan(css, 'selftest');
    console.log(`  ${alw.length ? 'FALSE POSITIVE' : 'allowed'}  ${name}`);
    if (alw.length) bad++;
  }
  // The allowlist must not be a blanket pass: a plain left: must still fail
  // even in a rule that also centres something.
  const strict = scan('.t{left:50%;transform:translateX(-50%);margin-left:4px}', 'selftest');
  console.log(`  ${strict.length ? 'caught' : 'MISSED'}  margin-left inside an allowlisted centering rule`);
  if (!strict.length) bad++;
  process.exit(bad ? 1 : 0);
}

const found = FILES.flatMap((f) => scan(readFileSync(f, 'utf8'), f));
if (found.length) {
  console.error('Physical CSS properties found. Use logical equivalents so RTL flips them:\n');
  for (const f of found) console.error(`  ${f.file}:${f.line}  ${f.decl}`);
  console.error('\n  margin-left -> margin-inline-start      text-align:left -> text-align:start');
  console.error('  right:      -> inset-inline-end         border-left     -> border-inline-start');
  console.error('\nIf a use is genuinely physical (centering, safe-area, lighting), add it to');
  console.error('ALLOW in this script with a reason.');
  process.exit(1);
}
console.log(`css-logical-check: clean (${FILES.length} files, ${ALLOW.length} documented physical uses)`);
