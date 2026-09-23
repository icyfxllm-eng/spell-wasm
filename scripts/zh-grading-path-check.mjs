#!/usr/bin/env node
// CC-ZH-TONE F2 — one zh grading path, and only one.
//
// The Tone Law says Little Speller's tone-blindness is a FLAG on the matcher,
// never a second code path. That is easy to state and easy to violate: any mode
// that grades a typed answer can quietly reach for norm::fold_strict and get
// something that looks right for English and silently drops tone for Mandarin.
// Bee did exactly that, so this lint exists because the violation was real.
//
// The scan keys on the pipe. Mandarin is the only language whose bank entries
// are "pinyin|hanzi", so a site that splits a bank entry on '|' is a site that
// handles zh. If such a site also compares against typed input, it must route
// through pinyin:: -- otherwise it is grading Mandarin without canonicalizing
// tone.
//
// EXEMPTIONS LIVE IN THE SOURCE, NOT HERE (changed 2026-09-23). This file used
// to carry a per-FILE ALLOW map. Two things were wrong with it:
//
//   * Per file is far too coarse. The only way to exempt one line in norm.rs --
//     which a new test needed -- was to exempt the entire grading file, which
//     is the worst possible thing to blind.
//   * It rotted silently. When it was replaced it held 15 entries for 10 files
//     that actually had sites; a third of it had been dead for some time and
//     nothing said so.
//
// A site is now exempted by a comment AT the site:
//
//     // zh-ok(grading): builds the sorted bank index; never sees typed input
//
// which is visible in review, dies when the code dies, and -- the part the old
// list could not do -- FAILS THE BUILD IF IT STOPS MATCHING A SITE. An
// exemption that no longer applies to anything is an error, not a silence.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SRC = `${ROOT}/src`;

const SPLIT = /split(_once)?\s*\(\s*['"]\|['"]\s*\)/;
const COMPARISONS = [
  /\bnorm::fold_strict\b/,
  /\bfold_strict\s*\(/,
  /\bnorm::answer_matches\b/,
  /\banswer_matches\s*\(/,
  /\bfold_lenient\s*\(/,
];
const CANONICAL = /\bpinyin::(grade\w*|matches|matches_with|canonicalize_\w+)\b/;
const WINDOW = 12;
const ANNOT = /\/\/\s*zh-ok\(grading\):\s*(\S.*?)\s*$/;
const ANNOT_ANY = /\/\/\s*zh-ok\(grading\)/;
const MIN_REASON = 12;

// Line indices inside a #[cfg(test)] module. Test code does not ship, so a test
// that splits a pipe and folds it is not a grading path the app can take --
// it is a test asserting something about the bank. Counted by BRACES rather
// than "everything after the first #[cfg(test)]", because that shortcut would
// silently stop scanning any production code that followed a test module.
function testLines(lines) {
  // Brace counting has one dangerous failure and one harmless one, so it is
  // built to fail the harmless way. Ending the region EARLY over-scans, which
  // at worst costs a visible false positive. Ending it LATE would swallow
  // production code and blind the lint silently. Hence both guards below.
  const bare = (l) => l.replace(/"(\\.|[^"\\])*"/g, '""').replace(/'(\\.|[^'\\])*'/g, "''");
  const out = new Set();
  for (let i = 0; i < lines.length; i++) {
    if (!/#\[cfg\(test\)\]/.test(lines[i])) continue;
    // Guard 1: an attribute on a brace-less item (#[cfg(test)] use foo;) never
    // opens a scope of its own. Without this the loop below would attach the
    // attribute to whatever came next and swallow it. An earlier version looked
    // ahead a fixed four lines for a brace, which just found the NEXT item's
    // brace and had the same bug -- the selftest fixture caught it. The item is
    // brace-less exactly when a ';' arrives before any '{'.
    let braceless = false;
    outer: for (let j = i; j < lines.length; j++) {
      for (const ch of bare(lines[j]).replace(/#\[[^\]]*\]/g, "")) {
        if (ch === "{") break outer;
        if (ch === ";") { braceless = true; break outer; }
      }
    }
    if (braceless) { out.add(i); continue; }
    let depth = 0, opened = false;
    for (let j = i; j < lines.length; j++) {
      for (const ch of bare(lines[j])) {   // Guard 2: braces inside literals do not count
        if (ch === "{") { depth++; opened = true; }
        else if (ch === "}") depth--;
      }
      out.add(j);
      if (opened && depth <= 0) break;
    }
  }
  return out;
}

// One scan, used for the real tree and for the selftest fixtures, so the
// selftest exercises the code that actually runs rather than a copy of it.
function scan(name, text, acc) {
  const lines = text.split("\n");
  const inTest = testLines(lines);
  lines.forEach((l, i) => { if (ANNOT_ANY.test(l)) acc.allAnnot.add(`${name}:${i + 1}`); });

  for (let i = 0; i < lines.length; i++) {
    if (!SPLIT.test(lines[i]) || inTest.has(i)) continue;
    const window = lines.slice(Math.max(0, i - WINDOW), Math.min(lines.length, i + WINDOW + 1)).join("\n");
    if (!COMPARISONS.some((re) => re.test(window))) continue;
    acc.sites++;
    if (CANONICAL.test(window)) continue; // compliant: it routes through the canonicalizer

    // An annotation applies to the next line of CODE, so the search walks up
    // through contiguous comment lines and stops at the first one that is not a
    // comment. An earlier version looked back a fixed three lines, which meant a
    // reason long enough to wrap onto a fourth silently stopped applying -- the
    // same class of magic-number bug this whole change exists to remove.
    let ok = false;
    for (let k = i; k >= 0; k--) {
      if (k !== i && !/^\s*\/\//.test(lines[k])) break;
      const m = lines[k].match(ANNOT);
      if (!m) continue;
      acc.usedAnnot.add(`${name}:${k + 1}`);
      if (m[1].length < MIN_REASON)
        acc.problems.push(`${name}:${k + 1} zh-ok(grading) needs a real reason, not ${JSON.stringify(m[1])}`);
      ok = true;
      acc.exempted++;
      break;
    }
    if (!ok)
      acc.problems.push(
        `${name}:${i + 1} compares a zh bank entry without the canonicalizer — ` +
          `tone is silently dropped here (F2, Invariant 5). If this is not a ` +
          `grading path, say so at the site with: // zh-ok(grading): <reason>`,
      );
  }
}

function newAcc() {
  return { problems: [], sites: 0, exempted: 0, usedAnnot: new Set(), allAnnot: new Set() };
}

// The anti-rot rule the old ALLOW map could not enforce: an exemption that
// stopped matching a site is an error, not a silence.
function checkStale(acc) {
  for (const a of acc.allAnnot)
    if (!acc.usedAnnot.has(a))
      acc.problems.push(`${a} zh-ok(grading) matches no site — stale exemption, delete it`);
}

const acc = newAcc();
const files = fs.readdirSync(SRC).filter((f) => f.endsWith(".rs"));
for (const f of files) scan(f, fs.readFileSync(`${SRC}/${f}`, "utf8"), acc);
checkStale(acc);
const problems = acc.problems;

// The two real grading sites are checked by name rather than by heuristic, so
// that renaming or gutting one is a failure instead of a quiet pass.
const game = fs.readFileSync(`${SRC}/game.rs`, "utf8");
// The submission path has been renamed twice as this file grew: F3 moved it
// from matches_with to grade, F5 to grade_sandhi_aware. Match the FAMILY rather
// than a fixed name -- anything in pinyin:: is the canonicalizer, and pinning
// the exact symbol just means the gate fails on correct refactors.
if (!/crate::pinyin::(grade\w*|matches_with)\s*\(/.test(game))
  problems.push("game.rs no longer grades zh through the canonicalizer (F2)");
if (!/ToneMode::for_kid\s*\(/.test(game))
  problems.push(
    "game.rs does not derive the tone mode from the kid flag — Little Speller " +
      "must be tone-blind by FLAG, not by a separate path (F2.3)",
  );

const bee = fs.readFileSync(`${SRC}/bee_screen.rs`, "utf8");
if (!/crate::pinyin::matches_with\s*\(/.test(bee))
  problems.push(
    "bee_screen.rs grades zh without the canonicalizer — this is the original " +
      "F2 violation and must not come back",
  );

if (problems.length) {
  console.error(`zh-grading-path-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(
  `zh-grading-path-check: OK — ${files.length} sources scanned, ${acc.sites} pipe site(s), ` +
    `${acc.exempted} exempted at the site, one zh grading path, tone-blindness is a flag.`,
);

// Selftest: the scan has to catch what it exists to catch, and the exemption
// mechanism has to be harder to abuse than the list it replaced. Each fixture
// names the failure it proves.
{
  const GRADE = (body) => `fn submit(t: &str, typed: &str) -> bool {\n${body}\n}`;
  const VIOLATION = GRADE(`    let want = crate::norm::fold_strict(t.split('|').next().unwrap_or(t));\n    crate::norm::fold_strict(typed) == want`);
  const cases = [
    ["an unexempted second grading path", VIOLATION, true],
    ["a site exempted with a real reason",
      VIOLATION.replace("    let want", "    // zh-ok(grading): fixture, not a real grading path at all\n    let want"), false],
    ["an exemption with no real reason",
      VIOLATION.replace("    let want", "    // zh-ok(grading): ok\n    let want"), true],
    ["a stale exemption matching no site",
      "// zh-ok(grading): this one points at nothing any more\nfn f() {}", true],
    ["the same violation inside a test module",
      `#[cfg(test)]\nmod tests {\n${VIOLATION}\n}`, false],
    ["production code after a brace-less #[cfg(test)] item",
      `#[cfg(test)]\nuse crate::fixture;\n${VIOLATION}`, true],
    ["a reason long enough to wrap over several comment lines",
      VIOLATION.replace("    let want",
        "    // zh-ok(grading): a reason long enough that it wraps, which used to push the\n" +
        "    // marker past a fixed three-line lookback and silently stop applying to the\n" +
        "    // site below it. Four continuation lines, deliberately: with three the\n" +
        "    // marker still landed inside the old window and proved nothing\n    let want"), false],
  ];
  const bad = [];
  for (const [label, text, shouldFail] of cases) {
    const a = newAcc();
    scan("fixture.rs", text, a);
    checkStale(a);
    if (a.problems.length > 0 !== shouldFail)
      bad.push(`${label}: expected ${shouldFail ? "a failure" : "a pass"}, got ${a.problems.length} problem(s)`);
  }
  if (bad.length) {
    console.error("zh-grading-path-check: SELFTEST FAILED — the lint is not protecting anything:");
    for (const b of bad) console.error("  ✗ " + b);
    process.exit(1);
  }
  console.log(`zh-grading-path-check: selftest OK — ${cases.length} fixtures, including a stale exemption and a brace-less cfg(test).`);
}
