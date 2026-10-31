#!/usr/bin/env node
// A release branch cut from an older release branch silently loses fixes that
// landed on main since (b307 and b309 both shipped without the Test
// entitlements cut). Run from the tree a build is cut from: fails unless every
// commit in fastlane/must-contain.txt is an ancestor of HEAD or was
// cherry-picked (same subject line).
//
//   node scripts/must-contain-check.mjs            # the ship lane runs this
//   node scripts/must-contain-check.mjs --selftest
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

export function parse(text) {
  return text.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('#'))
    .map(l => { const i = l.search(/\s/); return i < 0 ? { sha: l, why: '' } : { sha: l.slice(0, i), why: l.slice(i).trim() }; });
}

// has(sha) -> true when sha is an ancestor of HEAD; false when it is not
// (or the object is unknown to this clone).
export function missing(entries, has) { return entries.filter(e => !has(e.sha)); }

// A cherry-pick has a new sha, so an unreachable sha also passes when HEAD's
// history holds a commit with the same subject line.
function gitHas(sha) {
  try { execFileSync('git', ['merge-base', '--is-ancestor', sha, 'HEAD'], { cwd: root, stdio: 'ignore' }); return true; }
  catch { /* not an ancestor: try the cherry-pick route */ }
  try {
    const subject = execFileSync('git', ['log', '-1', '--format=%s', sha], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
    if (!subject) return false;
    const hits = execFileSync('git', ['log', 'HEAD', '--fixed-strings', `--grep=${subject}`, '--format=%s'], { cwd: root, encoding: 'utf8' });
    return hits.split('\n').some(l => l === subject);
  } catch { return false; }
}

if (process.argv.includes('--selftest')) {
  const e = parse('# c\n\naaa one\nbbb two words\n');
  if (e.length !== 2 || e[1].why !== 'two words') { console.error('selftest: parse'); process.exit(1); }
  if (missing(e, s => s === 'aaa').map(x => x.sha).join() !== 'bbb') { console.error('selftest: missing'); process.exit(1); }
  if (missing(e, () => true).length !== 0) { console.error('selftest: all present'); process.exit(1); }
  console.log('must-contain-check selftest ok');
  process.exit(0);
}

const entries = parse(readFileSync(join(root, 'fastlane/must-contain.txt'), 'utf8'));
const bad = missing(entries, gitHas);
if (bad.length) {
  console.error('MUST-CONTAIN FAIL: this tree is missing commits every build needs:');
  for (const b of bad) console.error(`  ${b.sha.slice(0, 8)}  ${b.why}`);
  console.error('  fix: cherry-pick them (or cut the release branch from main). If git does not know the sha, fetch first.');
  process.exit(1);
}
console.log(`must-contain-check: ${entries.length} required commit(s) present`);
