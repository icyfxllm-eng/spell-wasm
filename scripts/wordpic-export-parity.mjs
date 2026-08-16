#!/usr/bin/env node
// CC-PICTURE-COLOR — the export/play parity gate.
//
// A Spell Picture is drawn twice. On screen it takes its neutrals from
// index.html; exported, it carries them inline, because an <img>-rasterized
// SVG loads no stylesheet. Two copies of the same values, and for a long time
// a comment in wordpic_screen.rs claimed THIS SCRIPT enforced they matched.
// The script did not exist. The comment was written in good faith and the
// guard was never built, so the two drifted freely.
//
// They did drift. Build 178 introduced a light gallery ground for masterpieces
// and, refactoring the inline styles onto shared tokens, collapsed the pinned
// stroke (.92) and the feature stroke (.95) into one — dropping exported
// features to .92 while the screen kept .95. Nobody would have noticed from a
// PNG. This gate exists so the next one is caught at build time.
//
// The Rust `Ground` constants are the source of truth; index.html must agree.
// Checked BOTH ways round, for both grounds:
//
//   DARK   .wp-pinned / .wp-feature / .wp-outline / .wp-guide / --wp-ink
//   LIGHT  the same, under .wp-light                            (overrides)
//
// The guide joined the Ground on 2026-08-16 and it is the reason this file
// was not enough. It was a loose CSS rule outside the token set, painted
// near-white on EVERY ground -- so when ship 171 moved the masterpieces to a
// cream canvas, the traced artwork underneath the words went invisible on six
// of seven masters, and this gate could not see it because the biggest layer
// in the picture was not one of the things it checked.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const rust = readFileSync(join(ROOT, 'src/wordpic.rs'), 'utf8');
const html = readFileSync(join(ROOT, 'index.html'), 'utf8');
const screen = readFileSync(join(ROOT, 'src/wordpic_screen.rs'), 'utf8');

const problems = [];

/** Pull one `Ground { ... }` constant out of the Rust source. */
function ground(name) {
  const m = rust.match(new RegExp(`pub const ${name}: Ground = Ground \\{([^}]*)\\}`));
  if (!m) {
    problems.push(`src/wordpic.rs: cannot find "pub const ${name}: Ground"`);
    return null;
  }
  const out = {};
  for (const f of m[1].matchAll(/(\w+)\s*:\s*"([^"]+)"/g)) out[f[1]] = f[2];
  for (const k of ['bg', 'ink', 'stroke', 'feature', 'outline', 'guide']) {
    if (!out[k]) problems.push(`${name} is missing the "${k}" token`);
  }
  return out;
}

/** The body of a CSS rule in index.html, or null. */
function rule(selector) {
  const esc = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const m = html.match(new RegExp(`(^|[}\\n])\\s*${esc}\\s*\\{([^}]*)\\}`, 'm'));
  return m ? m[2] : null;
}

/** Assert a CSS rule declares `prop: expected`. */
function declares(selector, prop, expected, label) {
  const body = rule(selector);
  if (body === null) {
    problems.push(`index.html: no rule for "${selector}" (${label})`);
    return;
  }
  // First declaration of the property wins for our purposes; these rules set
  // each property once.
  const m = body.match(new RegExp(`(?:^|;)\\s*${prop}\\s*:\\s*([^;]+)`));
  if (!m) {
    problems.push(`index.html ${selector}: no "${prop}" declaration (${label})`);
    return;
  }
  const got = m[1].trim();
  if (got !== expected) {
    problems.push(
      `${label}: index.html has ${selector} { ${prop}: ${got} } ` +
      `but Rust says ${expected} — the screen and the export would disagree`
    );
  }
}

const DARK = ground('DARK');
const LIGHT = ground('LIGHT');

if (DARK) {
  declares('.wp-pinned', 'stroke', DARK.stroke, 'DARK pinned stroke');
  declares('.wp-feature', 'fill', DARK.feature, 'DARK feature fill');
  declares('.wp-feature', 'stroke', DARK.feature, 'DARK feature stroke');
  declares('.wp-outline', 'stroke', DARK.outline, 'DARK outline');
  declares('.wp-guide', 'stroke', DARK.guide, 'DARK guide layer');
  // The word ink reaches Play through a custom property with a fallback.
  declares('.wp-word', 'fill', `var(--wp-ink,${DARK.ink})`, 'DARK word ink fallback');
}

if (LIGHT) {
  declares('.wp-light', '--wp-ink', LIGHT.ink, 'LIGHT word ink');
  declares('.wp-light', 'background', LIGHT.bg, 'LIGHT ground');
  declares('.wp-light .wp-pinned', 'stroke', LIGHT.stroke, 'LIGHT pinned stroke');
  declares('.wp-light .wp-feature', 'fill', LIGHT.feature, 'LIGHT feature fill');
  declares('.wp-light .wp-feature', 'stroke', LIGHT.feature, 'LIGHT feature stroke');
  declares('.wp-light .wp-outline', 'stroke', LIGHT.outline, 'LIGHT outline');
  declares('.wp-light .wp-guide', 'stroke', LIGHT.guide, 'LIGHT guide layer');
}

// THE CLASS SET, not only the values. index.html and the inline export block
// must paint the SAME classes: a rule that is missing from the export is not
// a drift in a number, it is a layer that does not render at all. .wp-guide
// was absent from the export block for the guide layer's whole life, so every
// shared picture lost the artwork while the screen kept it, and a
// value-only check could never see it.
{
  const inline = screen.match(/<style>@font-face[\s\S]*?<\/style>/);
  if (!inline) {
    problems.push('src/wordpic_screen.rs: cannot find the inline export style block');
  } else {
    const classesIn = (src) => new Set(
      [...src.matchAll(/\.(wp-[a-z-]+)\s*\{/g)].map((m) => m[1]));
    const painted = classesIn(inline[0]);
    for (const cls of ['wp-pinned', 'wp-feature', 'wp-outline', 'wp-guide', 'wp-word']) {
      if (!painted.has(cls)) {
        problems.push(
          `export block does not paint .${cls} — an <img> loads no stylesheet, ` +
          `so that layer renders with SVG defaults (an unstyled stroke is none)`);
      }
    }
  }
}

// The two grounds must actually differ, or the light one is decoration.
if (DARK && LIGHT && DARK.bg === LIGHT.bg) {
  problems.push('DARK and LIGHT share a background — the gallery ground does nothing');
}

// --selftest: prove the gate bites, by the project's standing discipline. A
// parity check that cannot fail is how this drift got here in the first place.
if (process.argv.includes('--selftest')) {
  const before = problems.length;
  const fake = { ...DARK, feature: 'rgba(1,2,3,.99)' };
  const saved = problems.length;
  declares('.wp-feature', 'fill', fake.feature, 'selftest');
  const bit = problems.length > saved;
  problems.length = before;
  if (!bit) {
    console.error('FAIL wordpic-export-parity selftest: a deliberate mismatch went unnoticed');
    process.exit(1);
  }
  console.log('wordpic-export-parity: selftest OK — a deliberate mismatch fails the build');
  process.exit(0);
}

if (problems.length) {
  console.error('wordpic-export-parity FAILED — screen and export neutrals disagree:');
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}
console.log(
  'wordpic-export-parity: OK — index.html matches wordpic::DARK and wordpic::LIGHT ' +
  '(12 token bindings across both grounds)'
);
