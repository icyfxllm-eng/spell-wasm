#!/usr/bin/env node
// CC-WORDLIST-SOURCES addendum, Feature 4 — render the license manifest as a
// markdown table for proposal appendices.
//
//   node scripts/emit-license-table.mjs            # to stdout
//   node scripts/emit-license-table.mjs --check    # round-trip: table matches manifest
//
// A FORMATTING PASS ONLY. It holds no data of its own and derives nothing: every
// cell is a field read straight from sources/registry.json. If a verdict looks
// wrong in the table, the manifest is wrong — there is nowhere else for it to
// have come from. That is the point of --check, which re-parses the rendered
// table and asserts it round-trips to the same values, so a table pasted into a
// proposal cannot drift from the gate that enforces it.
//
// The sentence this exists to support, from the addendum: "every word list traces
// to a documented source whose license permits this use, and the build fails
// otherwise." The table shows the tracing; scripts/license-gate.mjs is the
// "build fails otherwise".
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const reg = JSON.parse(readFileSync(join(ROOT, "sources", "registry.json"), "utf8"));

const cell = (v) => (v === null || v === undefined || v === "" ? "—" : String(v).replace(/\|/g, "\\|"));
const rows = Object.entries(reg.sources || {}).sort(([a], [b]) => a.localeCompare(b));

function render() {
  const out = [];
  out.push("# Word-list sources — license manifest");
  out.push("");
  out.push("Generated from `sources/registry.json` by `scripts/emit-license-table.mjs`. Do not hand-edit.");
  out.push("");
  out.push("| Lang | Source | License (SPDX) | Tier | Permitted use | Verified by | Verified |");
  out.push("|------|--------|----------------|------|---------------|-------------|----------|");
  for (const [lang, s] of rows) {
    out.push(
      `| ${cell(lang)} | ${cell(s.name)} | ${cell(s.license_spdx)} | ${cell(s.tier)} | ` +
        `${cell(s.permitted_use)} | ${cell(s.verified_by)} | ${cell(s.verified_date)} |`,
    );
  }
  out.push("");
  const unknown = rows.filter(([, s]) => s.permitted_use === "UNKNOWN").map(([l]) => l);
  if (unknown.length) {
    out.push(`> **${unknown.length} of ${rows.length} sources are UNKNOWN** — no human verdict yet: ` +
             `${unknown.join(", ")}. An UNKNOWN blocks shipping for any ACTIVE language ` +
             `(scripts/license-gate.mjs). This table is not yet a claim that can be made in a proposal.`);
    out.push("");
  }
  return out.join("\n");
}

if (process.argv.includes("--check")) {
  // Round-trip: re-parse the rendered table and assert every cell matches the
  // manifest it came from. Guards the failure that would actually hurt — a table
  // in a proposal saying PERMITTED while the manifest says UNKNOWN.
  const table = render();
  const problems = [];
  const parsed = new Map();
  for (const line of table.split("\n")) {
    const m = line.match(/^\| (\S+) \| (.*?) \| (.*?) \| (.*?) \| (.*?) \| (.*?) \| (.*?) \|$/);
    if (!m || m[1] === "Lang" || m[1].startsWith("---")) continue;
    parsed.set(m[1], { name: m[2], spdx: m[3], tier: m[4], use: m[5], by: m[6], date: m[7] });
  }
  if (parsed.size !== rows.length) problems.push(`rendered ${parsed.size} rows, manifest has ${rows.length}`);
  for (const [lang, s] of rows) {
    const p = parsed.get(lang);
    if (!p) { problems.push(`${lang}: missing from the rendered table`); continue; }
    const want = {
      name: cell(s.name), spdx: cell(s.license_spdx), tier: cell(s.tier),
      use: cell(s.permitted_use), by: cell(s.verified_by), date: cell(s.verified_date),
    };
    for (const k of Object.keys(want)) {
      if (p[k] !== want[k]) problems.push(`${lang}.${k}: table has ${JSON.stringify(p[k])}, manifest has ${JSON.stringify(want[k])}`);
    }
  }
  if (problems.length) {
    console.error(`emit-license-table --check: FAILED — ${problems.length} mismatch(es):`);
    for (const p of problems) console.error("  ✗ " + p);
    process.exit(1);
  }
  console.log(`emit-license-table --check: OK — ${rows.length} rows round-trip to the manifest exactly.`);
  process.exit(0);
}

console.log(render());
