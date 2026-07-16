#!/usr/bin/env node
// Provenance validator for the shipped curated Spanish lists (CC-WORDLIST-SOURCES).
//
//   node scripts/provenance-validate.mjs            # validate es, write report
//   node scripts/provenance-validate.mjs --lang es  # explicit language
//   node scripts/provenance-validate.mjs --check    # validate, no write (CI)
//
// Provenance-BACKS (option 1) the EXISTING curated list against the licensed
// source — it does NOT replace it. For every word shipped in
//   assets/words/<lang>/{easy,medium,hard,expert}.txt
// it asserts the word EXISTS in the raw open-licensed source index
//   sources/<lang>/surface-index.txt
// (NFC + lowercased; hyphen/space compounds split so every token must be backed),
// or is an explicitly reviewed entry in
//   sources/<lang>/curated-exceptions.txt.
//
// It emits wordlists/<lang>.provenance.json: backed count, backed %, and the exact
// list of any GENUINE misses (curated words truly absent from the raw source AND
// not on the exceptions allowlist). GENUINE misses make this exit non-zero.
//
// NO silent fallback: a missing surface index is a HARD ERROR (we never assume a
// word is backed). Nothing here writes to assets/words/ or src/.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const rel = (p) => p.replace(ROOT + "/", "");
const TIERS = ["easy", "medium", "hard", "expert"];

// --- args -------------------------------------------------------------------
const argv = process.argv.slice(2);
const checkOnly = argv.includes("--check");
let lang = "es";
const li = argv.indexOf("--lang");
if (li !== -1 && argv[li + 1]) lang = argv[li + 1];

// --- helpers ----------------------------------------------------------------
// NFC + lowercase, matching sources/es/surface_index.py so membership is exact.
const norm = (s) => s.normalize("NFC").toLowerCase();

function readLines(path) {
  return readFileSync(path, "utf8")
    .split("\n")
    .map((l) => l.replace(/\r$/, ""));
}

// A curated cell may be a hyphen/space compound (e.g. "e-mail", "buenos aires").
// Provenance requires EVERY token to be source-backed, so split and test each.
function tokens(word) {
  return norm(word)
    .split(/[\s-]+/)
    .map((t) => t.trim())
    .filter((t) => t.length > 0);
}

// --- load the raw source surface index (HARD ERROR if missing) --------------
const indexPath = join(ROOT, "sources", lang, "surface-index.txt");
if (!existsSync(indexPath)) {
  console.error(
    `provenance-validate: FAIL — ${rel(indexPath)} is missing.\n` +
      `  The raw source index is required; there is no silent fallback.\n` +
      `  Build it first:  scripts/surface-index.sh ${lang}   (needs unmunch).`
  );
  process.exit(1);
}
const surface = new Set();
for (const line of readLines(indexPath)) {
  if (line !== "") surface.add(line); // index is already NFC + lowercased
}
if (surface.size === 0) {
  console.error(`provenance-validate: FAIL — ${rel(indexPath)} is empty.`);
  process.exit(1);
}

// --- load the reviewed exceptions allowlist (optional) ----------------------
const exceptionsPath = join(ROOT, "sources", lang, "curated-exceptions.txt");
const exceptions = new Set();
if (existsSync(exceptionsPath)) {
  for (let line of readLines(exceptionsPath)) {
    const hash = line.indexOf("#");
    if (hash !== -1) line = line.slice(0, hash);
    line = line.trim();
    if (line !== "") exceptions.add(norm(line));
  }
}

// --- validate every curated word in every tier ------------------------------
const perTier = {};
let total = 0;
let backedBySource = 0;
let backedByException = 0;
const genuineMisses = [];
const usedExceptions = new Set();

for (const tier of TIERS) {
  const tierPath = join(ROOT, "assets", "words", lang, `${tier}.txt`);
  if (!existsSync(tierPath)) {
    console.error(`provenance-validate: FAIL — missing curated tier ${rel(tierPath)}.`);
    process.exit(1);
  }
  const words = readLines(tierPath).map((w) => w.trim()).filter((w) => w !== "");
  perTier[tier] = { count: words.length, backed: 0, exception: 0, misses: [] };

  for (const raw of words) {
    total++;
    const n = norm(raw);
    const toks = tokens(raw);
    const allTokensBacked =
      toks.length > 0 && toks.every((t) => surface.has(t));

    if (allTokensBacked) {
      backedBySource++;
      perTier[tier].backed++;
    } else if (exceptions.has(n)) {
      backedByException++;
      perTier[tier].exception++;
      usedExceptions.add(n);
    } else {
      genuineMisses.push({ word: n, tier });
      perTier[tier].misses.push(n);
    }
  }
}

const backedTotal = backedBySource + backedByException;
const pct = (num) => Math.round((num / total) * 10000) / 100;

// Exceptions listed but no longer present in any curated tier — stale allowlist.
const staleExceptions = [...exceptions].filter((e) => !usedExceptions.has(e)).sort();

// --- report artifact --------------------------------------------------------
const report = {
  language: lang,
  generator: "scripts/provenance-validate.mjs",
  note:
    "Provenance validation of the SHIPPED curated list against the raw " +
    "open-licensed source surface index. Membership = EXISTENCE in " +
    `sources/${lang}/surface-index.txt (NFC + lowercased; hyphen/space ` +
    "compounds require every token backed), OR a reviewed entry in " +
    `sources/${lang}/curated-exceptions.txt. The curated list is UNCHANGED.`,
  source_index: {
    path: rel(indexPath),
    surface_forms: surface.size,
  },
  curated_tiers: TIERS.map((t) => `assets/words/${lang}/${t}.txt`),
  counts: {
    curated_total: total,
    backed_total: backedTotal,
    backed_by_source: backedBySource,
    backed_by_exception: backedByException,
    genuine_misses: genuineMisses.length,
  },
  percentages: {
    backed_total_pct: pct(backedTotal),
    backed_by_source_pct: pct(backedBySource),
  },
  per_tier: perTier,
  exceptions_allowlist: {
    path: rel(exceptionsPath),
    size: exceptions.size,
    used: [...usedExceptions].sort(),
    stale: staleExceptions,
  },
  // GENUINE misses = curated words absent from the raw source AND not on the
  // reviewed allowlist. This list MUST be empty for a clean provenance pass.
  genuine_misses: genuineMisses.map((m) => m.word).sort(),
};

const outPath = join(ROOT, "wordlists", `${lang}.provenance.json`);
const out = JSON.stringify(report, null, 2) + "\n";

if (checkOnly) {
  const cur = existsSync(outPath) ? readFileSync(outPath, "utf8") : "";
  if (cur !== out) {
    console.error(
      `provenance-validate: FAIL — ${rel(outPath)} is stale. ` +
        `Run \`node scripts/provenance-validate.mjs\`.`
    );
    process.exit(1);
  }
} else {
  writeFileSync(outPath, out, "utf8");
}

// --- console summary --------------------------------------------------------
console.log(
  `provenance-validate(${lang}): ${backedTotal}/${total} curated words backed ` +
    `(${pct(backedTotal)}%) — ${backedBySource} in source, ` +
    `${backedByException} reviewed exceptions.`
);
if (staleExceptions.length) {
  console.log(
    `provenance-validate(${lang}): note — ${staleExceptions.length} stale ` +
      `exception(s) no longer in any tier: ${staleExceptions.join(", ")}.`
  );
}
if (genuineMisses.length) {
  console.error(
    `\nprovenance-validate(${lang}): FAIL — ${genuineMisses.length} GENUINE miss(es) ` +
      `(absent from source AND not on the exceptions allowlist):`
  );
  for (const m of genuineMisses) console.error(`  ✗ ${m.word}  [${m.tier}]`);
  console.error(
    `  Add a justified entry to ${rel(exceptionsPath)} (for a real RAE headword) ` +
      `or ask Eric before removing a curated word.`
  );
  process.exit(1);
}
console.log(
  `provenance-validate(${lang}): PASS — every curated word is source-backed or ` +
    `a reviewed exception. Report -> ${rel(outPath)}.`
);
