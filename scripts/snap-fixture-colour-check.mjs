#!/usr/bin/env node
// Every CC-SNAP fixture image must be tagged Display P3.
//
// WHY. On 2026-10-06 the highlight fixture was found to hold four colour
// spaces at once: 13 shots in Display P3, 3 in sRGB, 2 untagged, and one
// carrying a MONITOR profile (PHL 241V8LB). Every measurement in the review
// record reads raw pixels, which ignores the tag, so rows in different
// spaces were compared in the same tables. That is not a rounding error:
// the same file read delta_s 0.129 as P3 and 0.204 converted to sRGB, which
// is wider than the gaps the thresholds are argued over.
//
// P3 and not sRGB because converting the vivid markers to sRGB CLIPS them --
// row 10's orange and cyan both hit S 1.000, and row 12 had been sitting
// clipped at 1.000 all along. P3 holds every real highlighter measured.
// Eric's call, 2026-10-06.
//
// This reads the ICC profile's description directly out of the file. It is
// deliberately dumb: no decoding, no conversion, no opinion about the
// pixels. A shot in the wrong space is a measurement in the wrong units.
//
//   node scripts/snap-fixture-colour-check.mjs
//   node scripts/snap-fixture-colour-check.mjs --selftest

import { readdirSync, readFileSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { inflateSync } from 'node:zlib';
import { pathToFileURL } from 'node:url';

const DIRS = ['tests/fixtures/snap-highlight', 'tests/fixtures/snap-layout'];
const WANT = 'Display P3';

/** The ICC profile's 'desc' tag, or null when there is no profile at all. */
export function profileName(buf) {
  const icc = extractIcc(buf);
  if (!icc) return null;
  // ICC: 128-byte header, then a tag table. Find 'desc' and read its text.
  const count = icc.readUInt32BE(128);
  for (let i = 0; i < count; i += 1) {
    const e = 132 + i * 12;
    if (e + 12 > icc.length) break;
    if (icc.toString('ascii', e, e + 4) !== 'desc') continue;
    const off = icc.readUInt32BE(e + 4);
    const size = icc.readUInt32BE(e + 8);
    if (off + size > icc.length) break;
    const type = icc.toString('ascii', off, off + 4);
    if (type === 'desc') {           // ICC v2 textDescriptionType
      const n = icc.readUInt32BE(off + 8);
      return icc.toString('ascii', off + 12, off + 12 + n).replace(/\0+$/, '');
    }
    if (type === 'mluc') {           // ICC v4 multiLocalizedUnicodeType
      const len = icc.readUInt32BE(off + 20);
      const o2 = icc.readUInt32BE(off + 24);
      // The string is UTF-16 BIG endian; Buffer only decodes LE, so swap.
      const be = icc.subarray(off + o2, off + o2 + len);
      const le = Buffer.allocUnsafe(be.length);
      for (let k = 0; k + 1 < be.length; k += 2) { le[k] = be[k + 1]; le[k + 1] = be[k]; }
      return le.toString('utf16le').replace(/\0+$/, '');
    }
  }
  return '(profile with no description)';
}

function extractIcc(buf) {
  // PNG: an iCCP chunk, zlib-deflated after a null-terminated name.
  if (buf.length > 8 && buf.readUInt32BE(0) === 0x89504e47) {
    let p = 8;
    while (p + 8 < buf.length) {
      const len = buf.readUInt32BE(p);
      const type = buf.toString('ascii', p + 4, p + 8);
      if (type === 'iCCP') {
        const body = buf.subarray(p + 8, p + 8 + len);
        const z = body.indexOf(0);
        return inflateSync(body.subarray(z + 2));
      }
      if (type === 'IEND') break;
      p += 12 + len;
    }
    return null;
  }
  // JPEG: APP2 segments marked ICC_PROFILE, possibly in several chunks.
  if (buf.length > 3 && buf[0] === 0xff && buf[1] === 0xd8) {
    let p = 2;
    const parts = [];
    while (p + 4 < buf.length) {
      if (buf[p] !== 0xff) { p += 1; continue; }
      const marker = buf[p + 1];
      if (marker === 0xda) break;                       // start of scan
      const len = buf.readUInt16BE(p + 2);
      if (marker === 0xe2 && buf.toString('ascii', p + 4, p + 15) === 'ICC_PROFILE') {
        parts.push(buf.subarray(p + 4 + 12 + 2, p + 2 + len));
      }
      p += 2 + len;
    }
    return parts.length ? Buffer.concat(parts) : null;
  }
  return null;
}

export function problems(dirs = DIRS) {
  const out = [];
  for (const dir of dirs) {
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir).filter((n) => /\.(jpe?g|png)$/i.test(n))) {
      const p = join(dir, name);
      const got = profileName(readFileSync(p));
      if (got === null) out.push(`${p} carries no colour profile — it must be ${WANT}`);
      else if (got !== WANT) out.push(`${p} is "${got}" — it must be ${WANT}`);
    }
  }
  return out;
}

function selftest() {
  // Build from real fixtures, so this tests the actual parser and not a mock.
  const live = DIRS.filter(existsSync)
    .flatMap((d) => readdirSync(d).filter((n) => /\.(jpe?g|png)$/i.test(n)).map((n) => join(d, n)));
  const jpg = live.find((f) => /\.jpe?g$/i.test(f));
  const png = live.find((f) => /\.png$/i.test(f));
  const cases = [];
  if (jpg) cases.push(['a real P3 jpeg reads as Display P3', jpg, WANT]);
  if (png) cases.push(['a real P3 png reads as Display P3', png, WANT]);
  let bad = 0;
  for (const [name, file, want] of cases) {
    const got = profileName(readFileSync(file));
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  // A file with no profile at all must be caught, not waved through.
  const dir = mkdtempSync(join(tmpdir(), 'colourcheck-'));
  writeFileSync(join(dir, 'x.jpg'), Buffer.from([0xff, 0xd8, 0xff, 0xd9]));
  const got = problems([dir]).length;
  const ok = got === 1;
  if (!ok) bad += 1;
  console.log(`  ${ok ? 'ok    ' : 'FAILED'} an untagged jpeg fails (want 1, got ${got})`);
  if (bad) { console.error('snap-fixture-colour-check selftest: FAILED'); process.exit(1); }
  console.log('snap-fixture-colour-check selftest: OK');
}

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = problems();
  if (bad.length) {
    console.error('snap-fixture-colour-check: FAILED');
    for (const b of bad) console.error(`  ${b}`);
    console.error('\n  Saturations from different colour spaces are different units.');
    console.error('  Run: tools/normalise_fixture_colour.py --write');
    process.exit(1);
  }
  const n = DIRS.filter(existsSync)
    .reduce((a, d) => a + readdirSync(d).filter((x) => /\.(jpe?g|png)$/i.test(x)).length, 0);
  console.log(`snap-fixture-colour-check: OK — ${n} shots, all ${WANT}`);
}
