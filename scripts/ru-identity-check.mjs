// CC-RUSSIAN-STRESS v3 — entry identity, held at option 1.
//
// v3 F6 resolves entry identity to (languageCode, canonicalForm,
// senseDiscriminator) so a homograph can carry two definitions. Definition
// Match makes that urgent: show за́мок with four definition cards and, under
// one-sense-per-spelling, TWO of them are correct and the round is unwinnable.
//
// BUT I8 -- "no store keys on a spelling string" -- is not satisfied yet.
// §0 found three persisted stores keyed on `lang::word`: misses, wordstats and
// tone_drill. word_id() now carries a sense, and sense 0 reproduces the legacy
// key byte for byte, so nothing migrates. What is NOT yet true is that a
// non-zero sense can be created at all: nothing authors one, and until an
// entry can declare its sense, a duplicate spelling in the bank would collide
// in all three stores -- one miss record, one stats row, one tone drill shared
// between two different words.
//
// So this is Eric's option 1: SHIP THE CONSTRAINT, FORBID THE COLLISION. The
// constraint is the cheap half and it is what stops an auditor row from
// introducing the first homograph while the identity work is still landing.
// At zero collisions it costs nothing; the cost arrives with the first one,
// which is precisely when someone should be forced to think.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SELFTEST = process.argv.includes("--selftest");

function ruBank(src) {
  const out = [];
  for (const tier of ["EASY", "MEDIUM", "HARD", "EXPERT"]) {
    const m = src.match(new RegExp(`pub const RU_${tier}: &\\[&str\\] = &\\[(.*?)\\];`, "s"));
    if (!m) continue;
    for (const w of m[1].matchAll(/"([^"]+)"/g)) out.push([w[1].split("|")[0], tier.toLowerCase()]);
  }
  return out;
}

function evaluate(src, wordIdSrc) {
  const fails = [];
  const bank = ruBank(src);
  if (!bank.length) fails.push("no RU_* bank found — re-anchor this gate");

  // THE CONSTRAINT. One spelling, one entry, until a sense can be declared.
  const seen = new Map();
  for (const [w, tier] of bank) {
    const k = w.toLowerCase();
    if (seen.has(k)) {
      fails.push(
        `ru bank has DUPLICATE SPELLING ${w!== k ? `${w} / ` : ""}${k} ` +
        `(${seen.get(k)} and ${tier}). That is a homograph, and a homograph needs a ` +
        `senseDiscriminator (v3 F6). Nothing can author one yet, so both senses would ` +
        `share ONE miss record, ONE stats row and ONE tone drill — get castle wrong and ` +
        `lock enters your misses. Split it only once I8 is satisfied.`
      );
    }
    seen.set(k, tier);
  }

  // Sense 0 must stay byte-identical to the legacy key, or every record in the
  // field is orphaned. This is the property that makes the change migration-free.
  if (!/if sense == 0 \{\s*\n\s*base/.test(wordIdSrc)) {
    fails.push(
      "word_id no longer returns the bare legacy key for sense 0. Every miss record, " +
      "word stat and tone drill in the field is keyed on that exact string; changing it " +
      "orphans them all and no migration exists."
    );
  }
  return fails;
}

const src = fs.readFileSync(path.join(ROOT, "src/word_data.rs"), "utf8")
  + fs.readFileSync(path.join(ROOT, "src/words.rs"), "utf8");
const wordIdSrc = fs.readFileSync(path.join(ROOT, "src/word_id.rs"), "utf8");

if (SELFTEST) {
  const lesions = [
    ["a duplicate ru spelling", () => src.replace(
      /pub const RU_EASY: &\[&str\] = &\["([^"]+)"/,
      (m, first) => m + `,"${first}"`)],
    ["word_id stops emitting the legacy key for sense 0",
      () => src, (w) => w.replace(/if sense == 0 \{\s*\n\s*base/, "if false {\n        base")],
  ];
  let bad = 0;
  for (const [name, mutSrc, mutWid] of lesions) {
    const s2 = mutSrc ? mutSrc() : src;
    const w2 = mutWid ? mutWid(wordIdSrc) : wordIdSrc;
    if (evaluate(s2, w2).length === 0) { console.error(`  SURVIVED: ${name}`); bad++; }
  }
  if (bad) { console.error(`ru-identity-check: FAILED — ${bad} lesion(s) not caught`); process.exit(1); }
  console.log(`ru-identity-check: selftest OK — all ${lesions.length} lesions fail the build`);
  process.exit(0);
}

const fails = evaluate(src, wordIdSrc);
if (fails.length) {
  console.error("ru-identity-check: FAILED");
  for (const f of fails) console.error("  " + f);
  process.exit(1);
}
console.log(`ru-identity-check: OK — ${ruBank(src).length} ru entries, one spelling each; sense 0 keeps the legacy key`);
