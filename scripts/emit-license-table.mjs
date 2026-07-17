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
// have come from.
//
// WHAT --check ACTUALLY PROVES, precisely, because the name oversells it:
// it renders from the manifest, re-parses its own output, and asserts the values
// survive. So it catches a RENDERER THAT LIES — a dropped column, a bad escape, a
// cell that says "—" while the manifest says "eric". Verified by probe: breaking
// the renderer that way does fail it.
//
// What it CANNOT catch: staleness. It regenerates the table from the manifest on
// every run, so both sides always move together — a manifest edit can never make
// it fail. A table already pasted into a proposal going stale is invisible to it,
// and that is the failure that would actually hurt. Fixing that means committing
// the rendered table and diffing against THAT; until then, regenerate before you
// paste, and do not read a green --check as "the proposal is current".
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
// Displayed content is not a word list, but it IS text we show a child — and a
// proposal table that lists 16 word-list sources while omitting the definitions
// shipping in every round would be exactly the kind of true-but-misleading
// artifact this manifest exists to prevent.
const shown = Object.entries(reg.displayed_content || {}).sort(([a], [b]) => a.localeCompare(b));

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
  if (shown.length) {
    out.push("## Displayed content (not word lists)");
    out.push("");
    out.push("Text this app renders to a user that does not come from a word list.");
    out.push("");
    out.push("| What | Source | Reachable | Permitted use | Verified by |");
    out.push("|------|--------|-----------|---------------|-------------|");
    for (const [id, d] of shown) {
      out.push(`| ${cell(id)} | ${cell(d.name)} | ${d.reachable ? "**shipping**" : "gated"} | ${cell(d.permitted_use)} | ${cell(d.verified_by)} |`);
    }
    out.push("");
  }
  const unknown = rows.filter(([, s]) => s.permitted_use === "UNKNOWN").map(([l]) => l);
  const unknownShown = shown.filter(([, d]) => d.permitted_use === "UNKNOWN" && d.reachable).map(([i]) => i);
  if (unknownShown.length) {
    out.push(`> **${unknownShown.length} REACHABLE displayed-content source(s) are UNKNOWN** — ` +
             `${unknownShown.join(", ")}. This is text on screen NOW with no recorded licence.`);
    out.push("");
  }
  if (unknown.length) {
    out.push(`> **${unknown.length} of ${rows.length} word-list sources are UNKNOWN** — no human verdict yet: ` +
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
  // The displayed-content table is 5 columns, so the 7-group regex above skips it
  // — which would leave the ONLY rows that are actually shipping exempt from the
  // round-trip guarantee. Parse them too.
  const parsedShown = new Map();
  for (const line of table.split("\n")) {
    const m = line.match(/^\| (\S+) \| (.*?) \| (\*\*shipping\*\*|gated) \| (.*?) \| (.*?) \|$/);
    if (!m) continue;
    parsedShown.set(m[1], { name: m[2], reachable: m[3] === "**shipping**", use: m[4], by: m[5] });
  }
  if (parsedShown.size !== shown.length) problems.push(`rendered ${parsedShown.size} displayed-content rows, manifest has ${shown.length}`);
  for (const [id, d] of shown) {
    const p = parsedShown.get(id);
    if (!p) { problems.push(`displayed_content ${id}: missing from the rendered table`); continue; }
    if (p.name !== cell(d.name)) problems.push(`displayed_content ${id}.name: table ${JSON.stringify(p.name)} vs manifest ${JSON.stringify(cell(d.name))}`);
    if (p.reachable !== !!d.reachable) problems.push(`displayed_content ${id}.reachable: table ${p.reachable} vs manifest ${!!d.reachable}`);
    if (p.use !== cell(d.permitted_use)) problems.push(`displayed_content ${id}.permitted_use: table ${JSON.stringify(p.use)} vs manifest ${JSON.stringify(cell(d.permitted_use))}`);
    if (p.by !== cell(d.verified_by)) problems.push(`displayed_content ${id}.verified_by: table ${JSON.stringify(p.by)} vs manifest ${JSON.stringify(cell(d.verified_by))}`);
  }

  if (problems.length) {
    console.error(`emit-license-table --check: FAILED — ${problems.length} mismatch(es):`);
    for (const p of problems) console.error("  ✗ " + p);
    process.exit(1);
  }
  console.log(`emit-license-table --check: OK — ${rows.length} word-list + ${shown.length} displayed-content rows round-trip exactly.`);
  process.exit(0);
}

console.log(render());
