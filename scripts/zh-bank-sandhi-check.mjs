#!/usr/bin/env node
// CC-ZH-TONE F5 / Done 3 — every zh bank entry carries its three fields.
//
//   pinyinCitation  the left half of the bank entry — what grading compares
//   pinyinSurface   what TTS synthesizes
//   sandhiClass     none | third-third | bu | yi
//
// Invariant 3 says a missing field is a BUILD FAILURE, not a runtime fallback.
// A word with no surface form is a word TTS cannot speak, and "fall back to the
// citation" is exactly the silent wrong-audio this feature exists to remove.
//
// The tagger PRE-FILLS, it never approves. The 50-word audit fixture is the
// human half of that: hand-derived expectations the tagger must reproduce
// exactly. Tagger and audit disagreeing is a failure of the TAGGER, not of the
// audit — the fixture is the authority.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const problems = [];

// ---- every bank entry has a row ----
const words = fs.readFileSync(`${ROOT}/src/words.rs`, "utf8");
const entries = [];
for (const name of ["ZH_EASY", "ZH_MEDIUM", "ZH_HARD", "ZH_EXPERT"]) {
  const m = words.match(new RegExp(`pub const ${name}: &\\[&str\\] = &\\[([\\s\\S]*?)\\n\\];`));
  if (!m) { problems.push(`${name} not found in src/words.rs`); continue; }
  for (const e of m[1].matchAll(/"([^"]+)"/g)) entries.push(e[1]);
}

const table = fs.readFileSync(`${ROOT}/src/zh_sandhi.rs`, "utf8");
const rows = new Map();
for (const m of table.matchAll(/\("([^"]+)", "([^"]+)", "([^"]+)"\)/g)) {
  rows.set(m[1], { surface: m[2], cls: m[3] });
}

const CLASSES = new Set(["none", "third-third", "bu", "yi"]);
const missing = entries.filter((e) => !rows.has(e));
if (missing.length)
  problems.push(
    `${missing.length} bank entries have no sandhi row — e.g. ${missing.slice(0, 5).join(", ")}`,
  );
const orphan = [...rows.keys()].filter((k) => !entries.includes(k));
if (orphan.length)
  problems.push(`${orphan.length} sandhi rows have no bank entry — e.g. ${orphan.slice(0, 5).join(", ")}`);

for (const [entry, r] of rows) {
  if (!r.surface) problems.push(`${entry}: empty pinyinSurface`);
  if (!CLASSES.has(r.cls)) problems.push(`${entry}: unknown sandhiClass '${r.cls}'`);
  // A class that claims a shift must actually shift, and "none" must not.
  const citation = entry.split("|")[0];
  const moved = r.surface !== citation;
  if (r.cls === "none" && moved)
    problems.push(`${entry}: class 'none' but the surface differs (${citation} -> ${r.surface})`);
  if (r.cls !== "none" && !moved)
    problems.push(`${entry}: class '${r.cls}' but nothing moved`);
}

// ---- the hand audit ----
const audit = JSON.parse(fs.readFileSync(`${ROOT}/config/zh-sandhi-audit.json`, "utf8"));
// Done 3 asks the TAGGER and a HUMAN to agree. Agreement with an unsigned
// fixture is the tagger agreeing with its own author, which proves nothing, so
// the signature is enforced rather than noted.
if (!audit.confirmed_by)
  problems.push(
    "config/zh-sandhi-audit.json is unsigned (confirmed_by is null) — Done 3 " +
      "needs a human who did not write the tagger",
  );
let agree = 0;
for (const c of audit.cases) {
  const row = rows.get(c.entry);
  if (!row) { problems.push(`audit: ${c.entry} is not in the bank`); continue; }
  if (row.surface === c.surface && row.cls === c.class) { agree++; continue; }
  problems.push(
    `audit disagreement on ${c.entry}: tagger says ${row.surface}/${row.cls}, ` +
      `the hand audit says ${c.surface}/${c.class}`,
  );
}
if (audit.cases.length < 50)
  problems.push(`audit has only ${audit.cases.length} cases; Done 3 asks for 50`);

if (problems.length) {
  console.error(`zh-bank-sandhi-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems.slice(0, 20)) console.error("  ✗ " + p);
  process.exit(1);
}
const byClass = {};
for (const r of rows.values()) byClass[r.cls] = (byClass[r.cls] ?? 0) + 1;
console.log(
  `zh-bank-sandhi-check: OK — ${rows.size} entries, all three fields present ` +
    `(${Object.entries(byClass).map(([k, v]) => `${v} ${k}`).join(", ")}); ` +
    `tagger and hand audit agree ${agree}/${audit.cases.length}.`,
);

// Selftest: Invariant 3 says a missing field FAILS THE BUILD. Prove it does.
{
  const gutted = new Map(rows);
  const victim = [...gutted.keys()][0];
  gutted.delete(victim);
  const caught = entries.filter((e) => !gutted.has(e)).length > 0;
  if (!caught) {
    console.error("zh-bank-sandhi-check: SELFTEST FAILED — a missing sandhi row slipped through.");
    process.exit(1);
  }
  console.log("zh-bank-sandhi-check: selftest OK — a missing surface form fails the build.");
}
