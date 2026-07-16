#!/usr/bin/env node
// License gate for the word-list provenance registry (CC-WORDLIST-SOURCES).
//
//   node scripts/license-gate.mjs
//
// FAILS the build (exit 1) if any of the following is true:
//   (a) a wordlists/<lang>.txt exists without a COMPLETE sources/<lang>/ entry
//       (registry entry + fetch.sh + LICENSE + PROVENANCE.md);
//   (b) a registry license tier is not A/B, or a Tier C source is used to
//       produce a shipped/derived list (Tier C is unmodified/server-side only);
//   (c) the generated credits file (credits.json) is missing a required
//       attribution for any Tier B (attribution) source that has a wordlist.
//
// PASSES (exit 0) only when every wordlist is fully backed. There is no silent
// fallback: a missing piece is a hard error.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const rel = (p) => p.replace(ROOT + "/", "");

const errors = [];
const notes = [];
const fail = (m) => errors.push(m);

// --- load registry ----------------------------------------------------------
const regPath = join(ROOT, "sources", "registry.json");
if (!existsSync(regPath)) {
  console.error("license-gate: FAIL — sources/registry.json is missing.");
  process.exit(1);
}
const registry = JSON.parse(readFileSync(regPath, "utf8"));
const sources = registry.sources || {};
const shipTiers = new Set(registry.allowed_tiers_for_ship || ["A", "B"]);
const knownTiers = new Set(Object.keys(registry.tiers || { A: 1, B: 1, C: 1 }));

// --- credits (attribution surface) ------------------------------------------
const creditsPath = join(ROOT, "credits.json");
let creditsSources = [];
if (existsSync(creditsPath)) {
  try {
    creditsSources = JSON.parse(readFileSync(creditsPath, "utf8")).sources || [];
  } catch (e) {
    fail(`credits.json is present but not valid JSON: ${e.message}`);
  }
} else {
  notes.push("credits.json not found (run `node scripts/gen-credits.mjs`).");
}
const creditedUrls = new Set(creditsSources.map((s) => s.url));

// --- enumerate emitted wordlists --------------------------------------------
const wlDir = join(ROOT, "wordlists");
const wordlistLangs = existsSync(wlDir)
  ? readdirSync(wlDir)
      .filter((f) => f.endsWith(".txt"))
      .map((f) => f.slice(0, -4))
  : [];

if (wordlistLangs.length === 0) {
  notes.push("No wordlists/*.txt present — nothing to gate.");
}

for (const lang of wordlistLangs) {
  const src = sources[lang];

  // (a) complete registry + filesystem entry
  if (!src) {
    fail(`wordlists/${lang}.txt exists but sources/registry.json has no "${lang}" entry.`);
    continue;
  }
  const dir = join(ROOT, "sources", lang);
  for (const req of ["fetch.sh", "LICENSE", "PROVENANCE.md"]) {
    const p = join(dir, req);
    if (!existsSync(p)) {
      fail(`wordlists/${lang}.txt: incomplete registry — missing ${rel(p)}.`);
    }
  }
  for (const field of ["name", "url", "license_spdx", "tier"]) {
    if (!src[field]) fail(`registry "${lang}": missing required field "${field}".`);
  }

  // (b) tier validity
  const tier = src.tier;
  if (!knownTiers.has(tier)) {
    fail(`registry "${lang}": unknown license tier "${tier}" (expected A/B/C).`);
  } else if (!shipTiers.has(tier)) {
    // Tier C: only allowed unmodified/server-side. A generated wordlist is a
    // DERIVED, modified artifact, so a Tier C source backing one is a failure.
    fail(`registry "${lang}": tier ${tier} source may not back a derived wordlist ` +
         `(Tier C is unmodified/server-side use only). STOP AND ASK.`);
  }

  // (c) attribution present for attribution-required tiers (B)
  if (tier === "B") {
    if (!creditedUrls.has(src.url)) {
      fail(`wordlists/${lang}.txt: Tier B source "${src.name}" requires attribution, ` +
           `but credits.json has no entry for ${src.url}. Regenerate credits.json.`);
    }
  }
}

// --- also sanity-check every registered source declares a known tier --------
for (const [lang, src] of Object.entries(sources)) {
  if (!knownTiers.has(src.tier)) {
    fail(`registry "${lang}": tier "${src.tier}" is not one of A/B/C ` +
         `(STOP AND ASK — see D1).`);
  }
}

// --- report -----------------------------------------------------------------
for (const n of notes) console.log(`license-gate: note — ${n}`);
if (errors.length) {
  console.error("\nlicense-gate: FAIL");
  for (const e of errors) console.error("  ✗ " + e);
  process.exit(1);
}
console.log(`license-gate: PASS — ${wordlistLangs.length} wordlist(s) fully backed ` +
            `(${wordlistLangs.join(", ") || "none"}).`);
