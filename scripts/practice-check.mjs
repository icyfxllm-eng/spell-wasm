#!/usr/bin/env node
// CC-PRACTICE I3 — curriculum schema gate (CI-blocking).
// Per live language config/practice/<lang>.json must hold: exactly 20 words +
// 5 difficult, every ID present in that language's source-of-record bank
// (NFC-compared, same normalization as gameplay — I6), every trap class in
// the 20 has exactly ONE intro card, every intro ≤ the 90-char hard cap,
// each trap's firstWord index in range and ordered.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const TIERS = ["easy", "medium", "hard", "expert"];
const CAP = 90;

function bank(lang) {
  const words = new Set();
  if (lang === "zh") {
    const src = fs.readFileSync(`${ROOT}/src/words.rs`, "utf8");
    for (const tier of TIERS) {
      const m = src.match(new RegExp(`pub const ZH_${tier.toUpperCase()}: &\\[&str\\] = &\\[([\\s\\S]*?)\\];`));
      for (const e of m[1].matchAll(/"([^"]+)"/g)) words.add(e[1].normalize("NFC"));
    }
    return words;
  }
  for (const tier of TIERS) {
    const p = `${ROOT}/assets/words/${lang}/${tier}.txt`;
    for (const w of fs.readFileSync(p, "utf8").split("\n")) {
      if (w.trim()) words.add(w.trim().normalize("NFC"));
    }
  }
  return words;
}

const dir = `${ROOT}/config/practice`;
const files = fs.readdirSync(dir).filter((f) => f.endsWith(".json"));
let problems = [];
for (const f of files) {
  const lang = f.replace(".json", "");
  const c = JSON.parse(fs.readFileSync(`${dir}/${f}`, "utf8"));
  const b = bank(lang);
  const where = (msg) => problems.push(`${lang}: ${msg}`);
  if (c.words?.length !== 20) where(`words must be exactly 20 (got ${c.words?.length})`);
  if (c.difficult?.length !== 5) where(`difficult must be exactly 5 (got ${c.difficult?.length})`);
  for (const w of [...(c.words ?? []), ...(c.difficult ?? [])]) {
    if (!b.has(w.normalize("NFC"))) where(`word not in bank: ${JSON.stringify(w)}`);
  }
  const seen = new Set([...(c.words ?? []), ...(c.difficult ?? [])].map((w) => w.normalize("NFC")));
  if (seen.size !== 25) where(`duplicate words across the 25`);
  const traps = c.traps ?? [];
  const ids = new Set(traps.map((t) => t.id));
  if (ids.size !== traps.length) where(`duplicate trap ids`);
  if (traps.length !== 5) where(`expected 5 trap classes (got ${traps.length})`);
  let prev = -1;
  for (const t of traps) {
    if (!t.intro || typeof t.intro !== "string") where(`trap ${t.id}: missing intro`);
    else if ([...t.intro].length > CAP) where(`trap ${t.id}: intro over ${CAP} chars (${[...t.intro].length})`);
    if (!(Number.isInteger(t.firstWord) && t.firstWord >= 0 && t.firstWord < 20))
      where(`trap ${t.id}: firstWord out of range (${t.firstWord})`);
    if (t.firstWord <= prev && prev !== -1) where(`trap ${t.id}: firstWord not strictly increasing`);
    prev = t.firstWord;
  }
}
if (problems.length) {
  console.error(`practice-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(`practice-check: OK — ${files.length} curricula, schema green.`);
