#!/usr/bin/env node
// Generate credits.json from the provenance registry (CC-WORDLIST-SOURCES, D4).
//
//   node scripts/gen-credits.mjs            # write credits.json
//   node scripts/gen-credits.mjs --check    # verify credits.json is up to date (CI)
//
// credits.json is the SINGLE attribution surface (D4): the app's About screen
// renders it, and adding a registry entry auto-adds its attribution. Only
// sources that actually back an emitted wordlists/<lang>.txt are credited, so a
// registry entry with no shipped list produces no phantom credit.
//
// Deterministic: sources are sorted by language code; no timestamps.

import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const registry = JSON.parse(readFileSync(join(ROOT, "sources", "registry.json"), "utf8"));

const wlDir = join(ROOT, "wordlists");
const langsWithLists = new Set(
  existsSync(wlDir)
    ? readdirSync(wlDir).filter((f) => f.endsWith(".txt")).map((f) => f.slice(0, -4))
    : []
);

const sources = Object.entries(registry.sources)
  .filter(([lang]) => langsWithLists.has(lang))
  .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
  .map(([lang, s]) => {
    const c = s.credits || {};
    return {
      lang,
      name: c.name || s.name,
      url: c.url || s.url,
      license: c.license || s.license_spdx,
      attribution: c.attribution ||
        `Word data derived from ${s.name} (${s.url}), ${s.license_spdx}.`,
    };
  });

const credits = {
  title: "Word-list data sources",
  note: "Spelling word lists in these languages are derived from the open " +
        "language resources below. Full license texts live in sources/<lang>/.",
  sources,
};

const out = JSON.stringify(credits, null, 2) + "\n";
const outPath = join(ROOT, "credits.json");

if (process.argv.includes("--check")) {
  const cur = existsSync(outPath) ? readFileSync(outPath, "utf8") : "";
  if (cur !== out) {
    console.error("gen-credits: FAIL — credits.json is stale. Run `node scripts/gen-credits.mjs`.");
    process.exit(1);
  }
  console.log("gen-credits: credits.json is up to date.");
} else {
  writeFileSync(outPath, out);
  console.log(`gen-credits: wrote credits.json (${sources.length} source(s)).`);
}
