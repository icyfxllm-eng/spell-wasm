#!/usr/bin/env node
// CC-PRACTICE v2 I3 — curriculum schema gate (CI-blocking).
// Per live language config/practice/<lang>.json must hold: exactly 20 words +
// 5 difficult, every ID present in that language's source-of-record bank
// (NFC-compared, same normalization as gameplay), every trap class has
// exactly ONE intro card ≤90 chars, firstWord in range and ordered.
// v2 adds: template refs name a real template with required params; decoys
// are single typing units; choice beats carry exactly two emoji, sit inside
// the 20, and their alt is a bank word NOT among the sequenced 25; coach
// pools total ≤40 lines of ≤90 chars.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const TIERS = ["easy", "medium", "hard", "expert"];
const CAP = 90;
const COACH_CAP = 40;
const TEMPLATES = { TAP_SILENT_UNIT: ["unit"], HEAR_PICK: ["foil"], TAP_STACK: ["unit", "pair"] };
const COACH_SLOTS = ["wordDone", "wordSetup", "phase", "reveal", "ceremony"];

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
    // v2: template refs (D5 — fixed set, required params present).
    if (t.template != null) {
      const req = TEMPLATES[t.template.id];
      if (!req) where(`trap ${t.id}: unknown template ${JSON.stringify(t.template.id)}`);
      else
        for (const k of req) {
          if (typeof t.template.params?.[k] !== "string" || !t.template.params[k])
            where(`trap ${t.id}: template ${t.template.id} missing param ${k}`);
        }
    }
    // v2: decoys are single typing units (D3). NFC length 1 covers every
    // lineup script (jamo decoys come from the runtime inventory instead).
    for (const d of t.decoys ?? []) {
      if ([...d.normalize("NFC")].length !== 1) where(`trap ${t.id}: decoy not a single unit: ${JSON.stringify(d)}`);
    }
  }
  // v2: choice beats (D8).
  for (const beat of c.choiceBeats ?? []) {
    if (!(Number.isInteger(beat.at) && beat.at > 0 && beat.at < 20)) where(`beat at ${beat.at}: out of range`);
    if (beat.emoji?.length !== 2) where(`beat at ${beat.at}: needs exactly two emoji`);
    if (!beat.alt || !b.has(beat.alt.normalize("NFC"))) where(`beat at ${beat.at}: alt not in bank: ${JSON.stringify(beat.alt)}`);
    if (seen.has(beat.alt?.normalize("NFC"))) where(`beat at ${beat.at}: alt duplicates a sequenced word`);
  }
  // v2: coach pools (D4).
  const coach = c.coach ?? {};
  for (const k of Object.keys(coach)) {
    if (!COACH_SLOTS.includes(k)) where(`coach: unknown slot ${JSON.stringify(k)}`);
  }
  let total = 0;
  for (const k of COACH_SLOTS) {
    for (const line of coach[k] ?? []) {
      total += 1;
      if (typeof line !== "string" || !line) where(`coach.${k}: empty line`);
      else if ([...line].length > CAP) where(`coach.${k}: line over ${CAP} chars: ${JSON.stringify(line)}`);
    }
  }
  if (total > COACH_CAP) where(`coach pool ${total} lines over the ${COACH_CAP} cap`);
}
if (problems.length) {
  console.error(`practice-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(`practice-check: OK — ${files.length} curricula, v2 schema green.`);
