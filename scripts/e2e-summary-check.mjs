#!/usr/bin/env node
// The e2e finished, and finished green — asserted from its own summary rather
// than inferred from the absence of failures.
//
// WHY THIS EXISTS. gate.sh used to decide the e2e's fate by counting "✗" marks
// in its log: zero reds meant "clean board" and the gate continued. But an e2e
// that dies BEFORE RUNNING A SINGLE TEST also has zero reds. On 2026-09-23 that
// happened for real -- a peer session held port 8129, the harness threw
// EADDRINUSE, and the gate printed "e2e: clean board (zero reds)". The only
// thing that failed the build was the NEXT line's `grep -E "E2E:"` finding
// nothing and tripping set -e. Incidental, not designed: reorder those two
// lines and a crashed e2e ships as a pass.
//
// Absence of evidence was being read as evidence of absence. So the rule is now
// positive: each suite must SAY it ran and say everything passed.
//
// Port conflicts are the common trigger and two sessions gate at once often, so
// this is not a hypothetical.
import fs from "node:fs";

// `npm run e2e` runs two suites and ORs their exit codes, so one can crash
// while the other reports. Both must be present and both must be green.
const SUITES = ["TEST-REPORT-app.md", "TEST-REPORT-site.md"];
const SUMMARY = /E2E:\s*(\d+)\/(\d+)\s+passed\.\s*Report → tests\/e2e\/(TEST-REPORT-[\w-]+\.md)/g;

export function check(log) {
  const problems = [];
  const seen = new Map();
  for (const m of log.matchAll(SUMMARY)) seen.set(m[3], { passed: +m[1], total: +m[2] });

  for (const suite of SUITES) {
    const r = seen.get(suite);
    if (!r) {
      problems.push(
        `${suite}: no summary line — the suite did not finish, so "no failures" ` +
          `means nothing. It crashed, hung, or never started (a port conflict is ` +
          `the usual cause).`,
      );
      continue;
    }
    if (r.total === 0) problems.push(`${suite}: reported 0 tests — an empty run is not a pass`);
    else if (r.passed !== r.total) problems.push(`${suite}: ${r.total - r.passed} of ${r.total} failed`);
  }
  const reds = (log.match(/✗/g) || []).length;
  if (reds) problems.push(`${reds} red test(s) in the log`);
  return { problems, seen };
}

if (process.argv.includes("--selftest")) {
  const green =
    "✅ E2E: 222/222 passed. Report → tests/e2e/TEST-REPORT-app.md\n" +
    "✅ E2E: 15/15 passed. Report → tests/e2e/TEST-REPORT-site.md\n";
  const cases = [
    ["both suites green", green, false],
    ["the EADDRINUSE crash this check exists for — zero reds, zero tests",
      "Error: listen EADDRINUSE: address already in use :::8129\n  code: 'EADDRINUSE'\n", true],
    ["only the app suite reported", green.split("\n")[0] + "\n", true],
    ["a suite that ran nothing", "✅ E2E: 0/0 passed. Report → tests/e2e/TEST-REPORT-app.md\n" + green.split("\n")[1], true],
    ["a real red", green.replace("222/222", "221/222") + "  ✗ spell-search golden\n", true],
    ["silence", "", true],
  ];
  const bad = [];
  for (const [label, log, shouldFail] of cases) {
    const got = check(log).problems.length > 0;
    if (got !== shouldFail) bad.push(`${label}: expected ${shouldFail ? "a failure" : "a pass"}, got ${got ? "a failure" : "a pass"}`);
  }
  if (bad.length) {
    console.error("e2e-summary-check: SELFTEST FAILED");
    for (const b of bad) console.error("  ✗ " + b);
    process.exit(1);
  }
  console.log(`e2e-summary-check: selftest OK — ${cases.length} fixtures, including the EADDRINUSE crash.`);
  process.exit(0);
}

const path = process.argv[2];
if (!path) { console.error("e2e-summary-check: needs the e2e log path"); process.exit(1); }
const log = fs.existsSync(path) ? fs.readFileSync(path, "utf8") : "";
const { problems, seen } = check(log);
if (problems.length) {
  console.error(`e2e-summary-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  console.error("  --- last 15 lines of the e2e log ---");
  for (const l of log.split("\n").slice(-15)) console.error("  " + l);
  process.exit(1);
}
const totals = [...seen].map(([s, r]) => `${s.replace(/TEST-REPORT-|\.md/g, "")} ${r.passed}/${r.total}`).join(", ");
console.log(`e2e-summary-check: OK — both suites finished and reported green (${totals}).`);
