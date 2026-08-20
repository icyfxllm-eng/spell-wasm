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
// Sites that split the pipe for reasons other than grading (speaking it,
// indexing it, building a pool) are legitimate and listed in ALLOW with the
// reason, so the exemption is a decision on the record rather than a silence.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SRC = `${ROOT}/src`;

// Comparison primitives that decide correctness. Reaching for one of these
// near a pipe split is the shape this lint is looking for.
const COMPARISONS = [
  /\bnorm::fold_strict\b/,
  /\bfold_strict\s*\(/,
  /\bnorm::answer_matches\b/,
  /\banswer_matches\s*\(/,
  /\bfold_lenient\s*\(/,
];
// The canonicalizer. A grading site that calls one of these is compliant.
const CANONICAL = /\bpinyin::(grade|matches|matches_with|canonicalize_\w+)\b/;
// How far from the split a comparison still counts as the same logic.
const WINDOW = 12;

// Pipe splits that are NOT grading. Each needs a reason; an unexplained
// entry here would defeat the lint.
const ALLOW = {
  "pinyin.rs": "the canonicalizer itself",
  "word_index.rs": "builds the sorted bank index; never sees typed input",
  "forge.rs": "composes the letter pool for Forge; not a spelling verdict",
  "forge_screen.rs": "renders that pool",
  "spellpic.rs": "chooses pictures by render width",
  "wordpic_layout.rs": "lays words onto a picture",
  "chains_screen.rs": "displays the chain; hooks are validity, not spelling",
  "kid_filter.rs": "reads the hanzi half for the kid-safety list",
  "photo_import.rs": "dictionary membership for an imported photo, not grading",
  "game.rs": "the canonical submission path; verified separately below",
  "bee_screen.rs": "graded via pinyin::matches_with — verified separately below",
  "chains.rs": "chain identity and hooks; a link is not a spelling verdict",
  "impostor.rs": "builds the wrong-answer cards; Impostor is a choice, not typing",
  "keyboard.rs": "a test asserting every answer char is reachable on the keyboard",
  "translate.rs": "camera OCR lookup against the word list, not a graded answer",
};

// The exemption is per FILE, which is coarser than it looks: a genuine grading
// path added to an allowlisted file would inherit the pass. The two real
// grading surfaces are therefore also checked by name below, and any NEW file
// that grades zh trips the scan on its first commit.

const problems = [];
const files = fs.readdirSync(SRC).filter((f) => f.endsWith(".rs"));

for (const f of files) {
  const lines = fs.readFileSync(`${SRC}/${f}`, "utf8").split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (!/split(_once)?\s*\(\s*['"]\|['"]\s*\)/.test(lines[i])) continue;
    const from = Math.max(0, i - WINDOW);
    const to = Math.min(lines.length, i + WINDOW + 1);
    const window = lines.slice(from, to).join("\n");
    if (!COMPARISONS.some((re) => re.test(window))) continue;
    if (CANONICAL.test(window)) continue;
    if (ALLOW[f]) continue;
    problems.push(
      `${f}:${i + 1} compares a zh bank entry without the canonicalizer — ` +
        `tone is silently dropped here (F2, Invariant 5)`,
    );
  }
}

// The two real grading sites are checked by name rather than by heuristic, so
// that renaming or gutting one is a failure instead of a quiet pass.
const game = fs.readFileSync(`${SRC}/game.rs`, "utf8");
// F3 moved the submission path from matches_with to grade, which returns the
// per-syllable verdict. Either is the canonical path; neither being present
// means zh grading has left the canonicalizer.
if (!/crate::pinyin::(grade|matches_with)\s*\(/.test(game))
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
  `zh-grading-path-check: OK — ${files.length} sources scanned, one zh grading ` +
    `path (pinyin::grade / matches_with), tone-blindness is a flag.`,
);

// Selftest: the scan has to actually catch the thing it exists to catch. Feed
// it the exact shape Bee had before F2 -- a pipe split graded by fold_strict,
// in a file nobody exempted -- and confirm it fires.
{
  const VIOLATION = `
fn submit(target: &str, typed: &str) -> bool {
    let want = crate::norm::fold_strict(target.split('|').next().unwrap_or(target));
    let got = crate::norm::fold_strict(typed);
    got == want
}`;
  const lines = VIOLATION.split("\n");
  let caught = false;
  for (let i = 0; i < lines.length; i++) {
    if (!/split(_once)?\s*\(\s*['"]\|['"]\s*\)/.test(lines[i])) continue;
    const w = lines.slice(Math.max(0, i - WINDOW), i + WINDOW + 1).join("\n");
    if (COMPARISONS.some((re) => re.test(w)) && !CANONICAL.test(w)) caught = true;
  }
  if (!caught) {
    console.error("zh-grading-path-check: SELFTEST FAILED — the scan no longer detects");
    console.error("  a fold_strict-graded pipe split. The lint is not protecting anything.");
    process.exit(1);
  }
  console.log("zh-grading-path-check: selftest OK — a second grading path fails the build.");
}
