#!/usr/bin/env node
// CC-VISUAL-ECLIPSE I4 (A4): with Reduce Motion on, no transform, scale,
// translate or rotate animation runs anywhere. Colour and opacity may still
// change; that is how state stays perceivable (D8).
//
// Static, over the whole stylesheet, so it covers every mode, including
// animations too short-lived to catch running in a browser:
//   1. every @keyframes that moves something (transform/translate/scale/rotate)
//      is found;
//   2. every rule that plays one of them, or that transitions transform (or
//      `all`), outside a (prefers-reduced-motion: no-preference) block, must be
//      neutralised inside a (prefers-reduced-motion: reduce) block: the same
//      selector there with animation none, an animation swapped for a keyframe
//      that does not move, or transition none / a transition list without
//      transform.
//
//   node scripts/reduce-motion-check.mjs
//   node scripts/reduce-motion-check.mjs --selftest
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const MOVES = /(^|[;{\s])(transform|translate|scale|rotate)\s*:/;
const norm = (s) => s.replace(/\s+/g, ' ').replace(/\s*([>+~,])\s*/g, '$1').trim();

/** Split CSS into top-level items: rules, @media blocks, @keyframes. */
function items(css) {
  const out = [];
  let i = 0;
  while (i < css.length) {
    const open = css.indexOf('{', i);
    if (open < 0) break;
    const head = css.slice(i, open).trim();
    let depth = 1, j = open + 1;
    while (j < css.length && depth) { if (css[j] === '{') depth++; else if (css[j] === '}') depth--; j++; }
    out.push({ head, body: css.slice(open + 1, j - 1) });
    i = j;
  }
  return out;
}

function decls(body) {
  const d = {};
  for (const m of body.matchAll(/([\w-]+)\s*:\s*([^;]+)/g)) d[m[1].trim()] = m[2].trim();
  return d;
}

export function check(cssRaw) {
  const css = cssRaw.replace(/\/\*[\s\S]*?\*\//g, '');
  const moving = new Set();
  const normal = []; // [selector, decls]
  const reduced = []; // [selector, decls]
  const walk = (list, ctx) => {
    for (const it of list) {
      if (/^@keyframes/.test(it.head)) {
        if (MOVES.test(it.body)) moving.add(it.head.replace(/^@keyframes\s+/, '').trim());
      } else if (/^@media/.test(it.head)) {
        const c = /prefers-reduced-motion\s*:\s*reduce/.test(it.head) ? 'reduce'
          : /prefers-reduced-motion\s*:\s*no-preference/.test(it.head) ? 'skip' : ctx;
        walk(items(it.body), c);
      } else if (!it.head.startsWith('@')) {
        if (ctx === 'skip') continue;
        const d = decls(it.body);
        for (const sel of it.head.split(',')) (ctx === 'reduce' ? reduced : normal).push([norm(sel), d]);
      }
    }
  };
  walk(items(css), 'normal');

  const animNames = (d) => {
    const v = d['animation-name'] || d.animation || '';
    return v.split(',').map((p) => p.trim().split(/\s+/).find((w) => moving.has(w))).filter(Boolean);
  };
  const transitionsTransform = (d) => /\b(transform|all)\b/.test(d.transition || d['transition-property'] || '');
  const neutralised = (sel, kind) => reduced.some(([rs, d]) => {
    if (rs !== sel && rs !== '*' && rs !== '*::before' && rs !== '*::after') return false;
    if (kind === 'anim') {
      const v = d.animation || d['animation-name'];
      return v !== undefined && (/^none\b/.test(v) || animNames(d).length === 0);
    }
    const t = d.transition || d['transition-property'];
    return t !== undefined && (/^none\b/.test(t) || !/\b(transform|all)\b/.test(t));
  });

  const problems = [];
  for (const [sel, d] of normal) {
    const names = animNames(d);
    if (names.length && !neutralised(sel, 'anim')) problems.push(`${sel} plays ${names.join(', ')} (moves) with no reduced-motion override`);
    if (transitionsTransform(d) && !neutralised(sel, 'trans')) problems.push(`${sel} transitions transform with no reduced-motion override`);
  }
  return { problems, moving: [...moving] };
}

if (process.argv.includes('--selftest')) {
  const cases = [
    ['a moving animation without an override is caught',
      check('@keyframes s{to{transform:scale(2)}} .a{animation:s 1s}').problems.length === 1],
    ['an override with animation none clears it',
      check('@keyframes s{to{transform:scale(2)}} .a{animation:s 1s} @media (prefers-reduced-motion: reduce){.a{animation:none}}').problems.length === 0],
    ['swapping to a non-moving keyframe clears it',
      check('@keyframes s{to{transform:scale(2)}} @keyframes f{to{opacity:0}} .a{animation:s 1s} @media (prefers-reduced-motion:reduce){.a{animation:f .2s}}').problems.length === 0],
    ['an opacity-only animation is fine',
      check('@keyframes f{to{opacity:0}} .a{animation:f 1s}').problems.length === 0],
    ['a transform transition without an override is caught',
      check('.a{transition:transform .2s}').problems.length === 1],
    ['a colour transition is fine', check('.a{transition:color .2s}').problems.length === 0],
    ['rules only active without reduce are fine',
      check('@keyframes s{to{transform:scale(2)}} @media (prefers-reduced-motion:no-preference){.a{animation:s 1s}}').problems.length === 0],
    ['an override for a different selector does not count',
      check('@keyframes s{to{transform:scale(2)}} .a{animation:s 1s} @media (prefers-reduced-motion: reduce){.b{animation:none}}').problems.length === 1],
  ];
  let bad = 0;
  for (const [name, ok] of cases) { console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name}`); if (!ok) bad++; }
  if (bad) { console.error(`reduce-motion-check selftest: ${bad} failed`); process.exit(1); }
  console.log('reduce-motion-check selftest: OK');
  process.exit(0);
}

const html = readFileSync(join(ROOT, 'index.html'), 'utf8');
const css = html.slice(html.indexOf('<style>') + 7, html.indexOf('</style>'));
const { problems, moving } = check(css);
if (problems.length) {
  console.error(`reduce-motion-check: FAILED, ${problems.length} moving animation(s) or transform transition(s) run under Reduce Motion (I4):`);
  for (const p of problems) console.error('  ✗ ' + p);
  process.exit(1);
}
console.log(`reduce-motion-check: OK, ${moving.length} moving keyframes, every use neutralised under Reduce Motion.`);
