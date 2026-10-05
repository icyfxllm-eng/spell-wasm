#!/usr/bin/env node
// CC-AUDIO-CLARITY F3 — every row in a pronunciation lexicon actually loads.
//
// The server drops a malformed row in SILENCE. _load_lexicon skips anything
// with fewer than five tab-separated fields, and skips any row where word,
// ipa, auditor or signed_at is empty (F3 step 2: an unsigned row is a guess,
// so it must not load). Both behaviours are correct. Neither is observable:
// the request still returns 200, the clip is still served, and it is simply
// the unfixed one. An operator who has just pasted in an auditor's signed row
// would have every reason to believe the fix had shipped.
//
// That is the failure shape this project keeps meeting -- a reported TTS
// upload that had actually succeeded, a cmp against files curl never wrote, a
// voice override "verified" by comparing two non-deterministic clips. The
// common thread is a check that cannot fail. So: this one reads the file the
// way app.py reads it and fails if any row a human wrote does not survive it.
//
//   node scripts/lexicon-load-check.mjs
//   node scripts/lexicon-load-check.mjs --selftest
//
// Worklists are exempt BY NAME, not by content: <lang>.worklist.tsv is
// generated with ipa/auditor/signed_at deliberately blank, because that is the
// form an auditor is handed. A worklist that loaded would mean a guess had
// been shipped.

import { readdirSync, readFileSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';

const DIR = 'audio/lexicon';
const REQUIRED = ['word', 'ipa', 'provider', 'auditor', 'signed_at'];
// app.py keeps these four non-empty or the row does not load. provider is NOT
// among them there, so it is not among them here either -- mirroring the
// server's rule matters more than tidiness.
const MUST_BE_SET = [0, 1, 3, 4];

/** Rows that a human wrote and that app.py would silently discard. */
export function badRows(dir = DIR) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    if (!name.endsWith('.tsv') || name.endsWith('.worklist.tsv')) continue;
    const lines = readFileSync(join(dir, name), 'utf8').split('\n');
    let headerSeen = false;
    for (let i = 0; i < lines.length; i += 1) {
      const line = lines[i];
      if (!line.trim() || line.startsWith('#')) continue;
      if (line.startsWith('word\t')) {
        headerSeen = true;
        const cols = line.split('\t').map((c) => c.trim());
        if (REQUIRED.some((c, k) => cols[k] !== c)) {
          out.push({ file: name, line: i + 1, why: `header is "${cols.join(' | ')}", want "${REQUIRED.join(' | ')}"` });
        }
        continue;
      }
      const parts = line.split('\t');
      if (parts.length < 5) {
        out.push({ file: name, line: i + 1, why: `${parts.length} tab-separated field(s), need 5 — app.py skips this row` });
        continue;
      }
      const v = parts.map((p) => p.trim());
      const missing = MUST_BE_SET.filter((k) => !v[k]).map((k) => REQUIRED[k]);
      if (missing.length) {
        out.push({ file: name, line: i + 1, why: `${v[0] || '(no word)'}: empty ${missing.join(', ')} — app.py will not load it` });
      }
    }
    if (!headerSeen) out.push({ file: name, line: 1, why: 'no header row' });
  }
  return out;
}

function selftest() {
  const cases = [
    ['a signed row loads', 'word\tipa\tprovider\tauditor\tsigned_at\nchip\ttʃɪp\tgoogle\tA Name\t2026-01-01', 0],
    ['an unsigned row does not', 'word\tipa\tprovider\tauditor\tsigned_at\nchip\ttʃɪp\tgoogle\t\t', 1],
    ['a row with no ipa does not', 'word\tipa\tprovider\tauditor\tsigned_at\nchip\t\tgoogle\tA Name\t2026-01-01', 1],
    ['too few columns is caught', 'word\tipa\tprovider\tauditor\tsigned_at\nchip\ttʃɪp\tgoogle', 1],
    ['spaces-not-tabs is caught', 'word\tipa\tprovider\tauditor\tsigned_at\nchip ti google A Name 2026-01-01', 1],
    ['a header-only file is fine', 'word\tipa\tprovider\tauditor\tsigned_at', 0],
    ['a wrong header is caught', 'word\tipa\tprovider\tauditor\tsigned\nchip\ttʃɪp\tgoogle\tA Name\t2026-01-01', 1],
    ['a missing header is caught', 'chip\ttʃɪp\tgoogle\tA Name\t2026-01-01', 1],
  ];
  let bad = 0;
  for (const [name, body, want] of cases) {
    const dir = mkdtempSync(join(tmpdir(), 'lexcheck-'));
    writeFileSync(join(dir, 'en.tsv'), body);
    const got = badRows(dir).length;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  // A worklist is exempt even though every row in it is unsigned.
  const wdir = mkdtempSync(join(tmpdir(), 'lexcheck-'));
  writeFileSync(join(wdir, 'en.worklist.tsv'), 'word\tipa\tprovider\tauditor\tsigned_at\nchip\t\tgoogle\t\t');
  const wgot = badRows(wdir).length;
  console.log(`  ${wgot === 0 ? 'ok    ' : 'FAILED'} a worklist is exempt by name (want 0, got ${wgot})`);
  if (wgot !== 0) bad += 1;
  if (bad) { console.error('lexicon-load-check selftest: FAILED'); process.exit(1); }
  console.log('lexicon-load-check selftest: OK');
}

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = badRows();
  if (bad.length) {
    console.error('lexicon-load-check: FAILED');
    console.error('  These rows are in the file but would NOT load. The server does not');
    console.error('  complain; it just serves the unfixed clip, so nothing else would tell you.');
    for (const b of bad) console.error(`    ${b.file}:${b.line}  ${b.why}`);
    process.exit(1);
  }
  console.log('lexicon-load-check: OK — every lexicon row loads as app.py would read it');
}
