#!/usr/bin/env node
// CC-ZH-TONE F1a — the pinned syllable inventory must match its pin.
//
// The pin is recomputed here from the list actually shipped in
// src/pinyin_inventory.rs, not read back from the generator. Editing the list
// by hand without regenerating fails; regenerating puts both the list and the
// pin in the same diff, which is the point of pinning it.
//
// Also asserts the two properties the canonicalizer relies on and cannot check
// cheaply at runtime: the list is sorted (lookup binary-searches it) and every
// syllable the shipped zh bank can ask for is present (a missing one makes that
// word unanswerable).
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const problems = [];

const src = fs.readFileSync(`${ROOT}/src/pinyin_inventory.rs`, "utf8");

const pin = src.match(/INVENTORY_PIN: &str = "([0-9a-f]{64})"/)?.[1];
if (!pin) problems.push("INVENTORY_PIN missing or malformed");

const block = src.match(/pub const SYLLABLES: \[&str; (\d+)\] = \[([\s\S]*?)\n\];/);
if (!block) problems.push("SYLLABLES array not found");

let syllables = [];
if (block) {
  const declared = Number(block[1]);
  syllables = [...block[2].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  if (syllables.length !== declared)
    problems.push(`SYLLABLES declares ${declared} but holds ${syllables.length}`);

  const actual = crypto.createHash("sha256").update(syllables.join("\n")).digest("hex");
  if (pin && actual !== pin)
    problems.push(
      `inventory does not match its pin\n      pinned ${pin}\n      actual ${actual}\n` +
        `      regenerate: python3 tools/build-pinyin-inventory.py`,
    );

  const sorted = [...syllables].sort();
  if (sorted.join("\u0000") !== syllables.join("\u0000"))
    problems.push("inventory is not sorted — the canonicalizer binary-searches it");
  if (new Set(syllables).size !== syllables.length)
    problems.push("inventory contains duplicates");
}

// Bank coverage. The bank stores the v encoding (lv3xing2) alongside a handful
// of ü spellings (lü3ke4); D2 folds both, so fold before checking.
const words = fs.readFileSync(`${ROOT}/src/words.rs`, "utf8");
const have = new Set(syllables);
const missing = new Set();
let checked = 0;
for (const tier of ["ZH_EASY", "ZH_MEDIUM", "ZH_HARD", "ZH_EXPERT"]) {
  const m = words.match(new RegExp(`pub const ${tier}: &\\[&str\\] = &\\[([\\s\\S]*?)\\n\\];`));
  if (!m) {
    problems.push(`${tier} not found in src/words.rs`);
    continue;
  }
  for (const [, entry] of m[1].matchAll(/"([^"]+)"/g)) {
    for (const [, syl] of entry.split("|")[0].matchAll(/([a-zü:]+)[0-5]?/g)) {
      checked++;
      const folded = syl.replace(/u:/g, "ü").replace(/v/g, "ü");
      if (!have.has(folded)) missing.add(`${syl} (folded ${folded})`);
    }
  }
}
if (missing.size)
  problems.push(`bank syllables absent from the inventory — unanswerable words: ${[...missing].join(", ")}`);

if (problems.length) {
  console.error(`pinyin-inventory-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(
  `pinyin-inventory-check: OK — ${syllables.length} syllables, pin verified, sorted, ` +
    `${checked} bank syllable uses all covered.`,
);
