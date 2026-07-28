#!/usr/bin/env node
// CC-PRACTICE I1 — the failure-proof purity gate (CI-blocking).
// Practice modules must import no timer/score/lives/leaderboard machinery and
// write to no store but their own record. Symbol grep over src/practice*.rs.
import fs from "node:fs";
const FORBIDDEN = [
  /\btimer\b/i, /\bscore\b/i, /\blives\b/i, /leaderboard/i,
  /\bboard::/, /\bstats::/, /\bclimb::/, /\bdaily::/,
  /misses::add_miss/, /attempts::/, /set_json\((?!&key)/,
];
let problems = [];
for (const f of ["src/practice.rs", "src/practice_screen.rs"]) {
  // Comments may NAME the forbidden things (that's documentation); only CODE
  // may not use them. Strip comment lines before the symbol grep.
  const src = fs.readFileSync(f, "utf8")
    .split("\n").filter((l) => !l.trim().startsWith("//")).join("\n");
  for (const rx of FORBIDDEN) {
    const m = src.match(rx);
    if (m) problems.push(`${f}: forbidden symbol ${JSON.stringify(m[0])}`);
  }
}
if (problems.length) {
  console.error("practice-purity: FAILED");
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log("practice-purity: OK — no timer/score/lives/leaderboard symbols, own store only.");
