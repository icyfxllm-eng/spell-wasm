// BD-G4 — the tier partition law.
//
// A word belongs to exactly ONE tier of its language. That held cleanly across
// all fourteen banks for months, guarded by nothing, and I broke it in a single
// pass: the easy-tier rebuild COPIED core words into EASY instead of promoting
// them, leaving 1,426 duplicate copies at two difficulties at once. It shipped
// in builds 157 and 158 before the count was noticed.
//
// Nothing caught it because nothing was looking. An invariant that survives on
// good intentions is one nobody has tested.
//
// Checked against assets/words/{code}/{tier}.txt — the SOURCE. The Rust banks
// are generated from these, so checking the artifact would test the generator
// rather than the data.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const WORDS = `${ROOT}/assets/words`;
const TIERS = ["easy", "medium", "hard", "expert"];

let failed = false;
let langs = 0;
let words = 0;

for (const lang of fs.readdirSync(WORDS).sort()) {
  const dir = `${WORDS}/${lang}`;
  if (!fs.statSync(dir).isDirectory()) continue;
  if (!fs.existsSync(`${dir}/easy.txt`)) continue;  // exclusions/, profanity/, ...
  langs++;

  const home = new Map();      // word -> first tier that claimed it
  const clashes = [];
  for (const tier of TIERS) {
    const p = `${dir}/${tier}.txt`;
    if (!fs.existsSync(p)) continue;
    const seen = new Set();
    for (const w of fs.readFileSync(p, "utf8").split("\n").map((s) => s.trim()).filter(Boolean)) {
      words++;
      // duplicated inside one tier is also illegal — a pool that lists a word
      // twice weights it twice
      if (seen.has(w)) { clashes.push([w, tier, tier]); continue; }
      seen.add(w);
      if (home.has(w)) clashes.push([w, home.get(w), tier]);
      else home.set(w, tier);
    }
  }
  if (clashes.length) {
    failed = true;
    const shown = clashes.slice(0, 5)
      .map(([w, a, b]) => `${w} (${a}+${b})`).join(", ");
    console.error(`FAIL ${lang}: ${clashes.length} word(s) in more than one tier — ${shown}${clashes.length > 5 ? " …" : ""}`);
  }
}

if (!langs || !words) {
  console.error("FAIL: no banks found — this check is vacuous");
  failed = true;
} else if (!failed) {
  console.log(`tier-partition-check: OK — ${words} words across ${langs} banks, every word in exactly one tier`);
}
process.exit(failed ? 1 : 0);
