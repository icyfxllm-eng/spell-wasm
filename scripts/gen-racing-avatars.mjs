#!/usr/bin/env node
// Generate the 16 Spell Racing preset avatars (CC-SPELL-RACING D5, approved: two
// families — 8 stars + 8 wands). Each SVG is COLOUR-AGNOSTIC: the tinted shape uses
// `fill="currentColor"` so the app applies the chosen color_id via CSS `color`.
// Fixed accents (dark eye marks, gold crown, white shine, grey wand stick) are not
// the identity colour and stay constant across all 6 palette colours.
//
// Identity = { avatar_id: 0..15, color_id: 0..5 }. No free text — COPPA-inert.
//
// NOTE: the Spell Racing mode that consumes these is REVIEW-GATED and not built.
// This is the approved D5 art only. Re-run to regenerate:
//   node scripts/gen-racing-avatars.mjs
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'assets', 'racing', 'avatars');
mkdirSync(OUT, { recursive: true });

const D = '#1f1a33';                    // fixed dark for eye/detail marks
const CC = 'currentColor';              // the tinted identity shape

const starPts = (cx, cy, R, r, n, rot) => {
  const p = [];
  for (let i = 0; i < 2 * n; i++) {
    const a = (rot + (i * 180) / n) * Math.PI / 180, rad = i % 2 ? r : R;
    p.push((cx + rad * Math.cos(a)).toFixed(1) + ',' + (cy + rad * Math.sin(a)).toFixed(1));
  }
  return p.join(' ');
};
const star = (cx, cy, R, r, n, c = CC, rot = -90) => `<polygon points="${starPts(cx, cy, R, r, n, rot)}" fill="${c}"/>`;
const eyes = (x1, x2, y = 30) => `<circle cx="${x1}" cy="${y}" r="2.4" fill="${D}"/><circle cx="${x2}" cy="${y}" r="2.4" fill="${D}"/>`;
const smile = (y = 38) => `<path d="M27 ${y}q5 4 10 0" stroke="${D}" stroke-width="2" fill="none" stroke-linecap="round"/>`;
const spk = (x, y, s = 2.2) => star(x, y, s, s / 2.3, 4, '#ffffff', -90);

const STARS = [
  ['star-classic',  () => star(32, 32, 22, 9, 5) + eyes(26, 38) + smile(37)],
  ['star-sparkle',  () => star(32, 32, 22, 6, 4) + eyes(27, 37, 31)],
  ['star-burst',    () => star(32, 32, 22, 11, 6) + eyes(27, 37) + smile(37)],
  ['star-shooting', () => `<path d="M8 46q10 2 18-6" stroke="${CC}" stroke-width="3" fill="none" stroke-linecap="round" opacity=".5"/>` + star(38, 24, 16, 7, 5)],
  ['star-wink',     () => star(32, 32, 22, 9, 5) + `<path d="M23 30q3 3 6 0" stroke="${D}" stroke-width="2" fill="none" stroke-linecap="round"/><circle cx="39" cy="30" r="2.4" fill="${D}"/>` + smile(37)],
  ['star-cool',     () => star(32, 32, 22, 9, 5) + `<rect x="22" y="27" width="20" height="6" rx="3" fill="${D}"/>`],
  ['star-champ',    () => `<path d="M24 12l3 4 5-5 5 5 3-4v6H24z" fill="#f4b400"/>` + star(32, 34, 20, 8, 5) + eyes(27, 37, 34)],
  ['star-twinkle',  () => star(32, 32, 21, 8, 5) + spk(46, 18, 3) + spk(15, 40, 2.4) + eyes(27, 38)],
];

// Wand: fixed grey stick+handle; only the TIP charm carries the identity colour.
const wand = (tip) => `${tip}<rect x="18" y="34" width="26" height="6" rx="3" transform="rotate(-42 31 37)" fill="#8a7fb0"/><circle cx="20" cy="45" r="3.4" fill="#6f6690"/>`;
const WANDS = [
  ['wand-star',     () => wand(star(44, 20, 11, 4.5, 5) + spk(52, 12))],
  ['wand-sparkle',  () => wand(star(44, 20, 10, 3, 4) + spk(52, 13) + spk(37, 15) + spk(50, 27))],
  ['wand-crescent', () => wand(`<path d="M52 12a11 11 0 1 0 0 18 8.5 8.5 0 0 1 0-18z" fill="${CC}"/>` + spk(38, 14))],
  ['wand-crystal',  () => wand(`<polygon points="44,10 51,20 44,31 37,20" fill="${CC}"/><polygon points="44,10 44,31 37,20" fill="#000" opacity=".12"/>`)],
  ['wand-bolt',     () => wand(`<path d="M46 9l-9 12h6l-6 12 13-15h-6z" fill="${CC}"/>`)],
  ['wand-heart',    () => wand(`<path d="M44 30c-9-6-11-13-6-16 3-2 6 0 6 2 0-2 3-4 6-2 5 3 3 10-6 16z" fill="${CC}"/>`)],
  ['wand-orb',      () => wand(`<circle cx="44" cy="20" r="10" fill="${CC}"/><circle cx="41" cy="17" r="3" fill="#fff" opacity=".55"/>`)],
  ['wand-trail',    () => wand(star(45, 19, 9, 4, 5) + spk(35, 26) + spk(29, 33) + spk(23, 40))],
];

const ALL = [...STARS, ...WANDS];
const file = (inner) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64" role="img">${inner}</svg>\n`;

ALL.forEach(([name, build], i) => {
  writeFileSync(join(OUT, `${String(i).padStart(2, '0')}-${name}.svg`), file(build()));
});

// Manifest: the ordered avatar_id list + the approved colour palette (color_id).
const manifest = {
  schemaNote: 'CC-SPELL-RACING D5 — preset identity. avatar_id indexes `avatars`; color_id indexes `colors`. No free text.',
  families: { stars: STARS.map((a) => a[0]), wands: WANDS.map((a) => a[0]) },
  avatars: ALL.map(([name], i) => ({ id: i, name, file: `${String(i).padStart(2, '0')}-${name}.svg` })),
  colors: [
    { id: 0, name: 'coral',  hex: '#ff6b7a' },
    { id: 1, name: 'amber',  hex: '#ffb14d' },
    { id: 2, name: 'yellow', hex: '#ffe14d' },
    { id: 3, name: 'teal',   hex: '#46d6b8' },
    { id: 4, name: 'blue',   hex: '#7aa2ff' },
    { id: 5, name: 'purple', hex: '#c792ff' },
  ],
};
writeFileSync(join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(`wrote ${ALL.length} avatars + manifest.json to assets/racing/avatars/`);
