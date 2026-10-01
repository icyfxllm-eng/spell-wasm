// Definition pools are the only player-facing CONTENT that reaches production
// without an app release or a service restart — /api/defpool opens the JSON
// per request — so nothing but this check stands between a bad row and a
// child reading it.
//
// WHY THIS EXISTS. 883 rows across all 15 languages shipped wrong, for weeks,
// and the hint path masked it because hints come from a different route:
//
//   * cat  -> "ISO 639-2 & ISO 639-3 language code for Catalan."
//   * sun  -> "ISO 639-2 & ISO 639-3 language code for Sundanese."
//     ...and run hat bed cup fox map pig zoo the and to in is was for are be,
//     187 in English alone, easy tier, kid_register true. Wiktionary lists a
//     word's Symbol sense FIRST, and the builder took the first sense.
//   * Arabic kana -> "to be .mw-parser-output .object-usage-tag{font-style:italic}"
//     A stylesheet rule, served as a definition.
//   * cat (after the first fix) -> "Terms relating to animals.\nA mammal..."
//     Wiktionary's topical label, kept as if it were the gloss.
//
// Fixed in scripts/build-def-pools.py 2026-09-30. This check is the part that
// makes the fix stay fixed: it reads the shipped artifact, not the builder, so
// a hand-edited pool or a future scraper change is caught the same way.
//
//   node scripts/def-pool-check.mjs              # check the real pools
//   node scripts/def-pool-check.mjs --selftest   # prove it bites
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const TIERS = ["easy", "medium", "hard", "expert"];

// A definition must describe the WORD, never the string's other lives as a
// code, nor the encyclopedia's own furniture.
const JUNK = [
  [/^(?:Former )?ISO \d/i, "an ISO code, not a meaning"],
  [/^Symbol for\b/i, "a symbol gloss, not a meaning"],
  [/\bISO 639\b/i, "an ISO 639 language code"],
  [/mw-parser-output/, "inlined Wiktionary CSS"],
  [/\{[^}]*:[^}]*\}/, "a CSS rule"],
  [/^Terms relating to\b/i, "Wiktionary's topical label, not the gloss"],
  [/^\s*$/, "empty"],
];

export function check(dir) {
  const bad = [];
  let rows = 0, files = 0, offBank = 0;
  const files_ = fs.existsSync(dir)
    ? fs.readdirSync(dir).filter((f) => f.endsWith(".json")).sort()
    : [];
  if (!files_.length) return { bad: [`${dir}: no pool files at all`], rows, files, offBank };

  for (const f of files_) {
    files++;
    const lang = f.replace(/\.json$/, "");
    let doc;
    try {
      doc = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8"));
    } catch (e) {
      bad.push(`${f}: not valid JSON — ${e.message}`);
      continue;
    }
    if (doc.lang !== lang) bad.push(`${f}: lang field '${doc.lang}' does not match the filename`);
    if (!doc.tiers || typeof doc.tiers !== "object") {
      bad.push(`${f}: no tiers object`);
      continue;
    }
    for (const tier of Object.keys(doc.tiers)) {
      if (!TIERS.includes(tier)) bad.push(`${f}: unknown tier '${tier}'`);
    }
    for (const tier of TIERS) {
      const list = doc.tiers[tier];
      if (list === undefined) continue;
      if (!Array.isArray(list)) { bad.push(`${f}:${tier}: not an array`); continue; }
      for (const r of list) {
        rows++;
        const where = `${lang}/${tier} '${r && r.word}'`;
        if (!r || typeof r.word !== "string" || !r.word) { bad.push(`${where}: missing word`); continue; }
        if (typeof r.definition !== "string") { bad.push(`${where}: missing definition`); continue; }
        if (typeof r.prompt_grade !== "boolean") bad.push(`${where}: prompt_grade is not a boolean`);
        if (typeof r.kid_register !== "boolean") bad.push(`${where}: kid_register is not a boolean`);

        const d = r.definition;
        // The regression class: text that is not a meaning.
        for (const [re, why] of JUNK) {
          if (re.test(d)) { bad.push(`${where}: ${why} — ${JSON.stringify(d.slice(0, 70))}`); break; }
        }
        // One sense per row. A flattened multi-sense blob reads as nonsense on
        // a card and is how the topical label got in.
        if (d.includes("\n")) bad.push(`${where}: more than one line — ${JSON.stringify(d.slice(0, 70))}`);

        // The builder's own prescreen laws, re-checked on the artifact so a
        // hand-edit cannot promote a bad row. cat shipped kid_register TRUE.
        if (r.prompt_grade) {
          if (d.length > 90) bad.push(`${where}: prompt_grade but ${d.length} chars (max 90)`);
          const leak = new RegExp(`(?<!\\p{L})${r.word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?!\\p{L})`, "iu");
          if (leak.test(d)) bad.push(`${where}: prompt_grade but the definition contains the word`);
        }
        if (r.kid_register) {
          if (!["easy", "medium"].includes(tier)) bad.push(`${where}: kid_register outside easy/medium`);
          if (d.length > 60) bad.push(`${where}: kid_register but ${d.length} chars (max 60)`);
        }
      }
      // Two identical definitions in one tier are two right answers.
      const byDef = new Map();
      for (const r of list) {
        if (!r || typeof r.definition !== "string") continue;
        const k = r.definition.toLowerCase();
        byDef.set(k, (byDef.get(k) || []).concat(r));
      }
      for (const [, group] of byDef) {
        const pg = group.filter((r) => r.prompt_grade);
        if (group.length > 1 && pg.length) {
          bad.push(`${lang}/${tier}: ${pg.length} prompt_grade rows share one definition — ${pg.map((r) => r.word).join(", ")}`);
        }
      }
    }
  }
  return { bad, rows, files, offBank };
}

// Freshness is reported, never enforced: a bank edit must not block an
// unrelated push behind a two-and-a-half-hour refetch. Correctness of the text
// that ships is enforced above, and that never depends on the bank.
function staleRows(poolDir, bankDir) {
  if (!fs.existsSync(bankDir)) return null;
  let off = 0, total = 0;
  for (const f of fs.readdirSync(poolDir).filter((x) => x.endsWith(".json"))) {
    const lang = f.replace(/\.json$/, "");
    const bank = new Set();
    for (const tier of TIERS) {
      const p = path.join(bankDir, lang, `${tier}.txt`);
      if (!fs.existsSync(p)) continue;
      for (const line of fs.readFileSync(p, "utf8").split("\n")) {
        const w = line.trim();
        if (!w || w.startsWith("#")) continue;
        bank.add(w.includes("|") ? w.slice(w.lastIndexOf("|") + 1) : w);
      }
    }
    if (!bank.size) continue;
    const doc = JSON.parse(fs.readFileSync(path.join(poolDir, f), "utf8"));
    for (const tier of TIERS) {
      for (const r of doc.tiers?.[tier] || []) {
        total++;
        if (!bank.has(r.word)) off++;
      }
    }
  }
  return { off, total };
}

if (process.argv.includes("--selftest")) {
  const row = (o = {}) => ({ word: "apple", definition: "A round fruit.", pos: "noun", prompt_grade: true, kid_register: true, ...o });
  const cases = {
    clean: [row()],
    iso_code: [row({ word: "cat", definition: "ISO 639-2 & ISO 639-3 language code for Catalan." })],
    iso_former: [row({ word: "in", definition: "Former ISO 639-1 language code for Indonesian." })],
    symbol_for: [row({ word: "as", definition: "Symbol for attosecond, an SI unit of time." })],
    inlined_css: [row({ word: "kana", definition: "to be .mw-parser-output .object-usage-tag{font-style:italic}" })],
    topical_label: [row({ word: "cat", definition: "Terms relating to animals." })],
    two_lines: [row({ definition: "shirt\n dress shirt" })],
    empty_def: [row({ definition: "" })],
    long_prompt_grade: [row({ definition: "A".repeat(91) })],
    self_leak: [row({ definition: "An apple is a fruit." })],
    kid_too_long: [row({ definition: "A round fruit that grows on a tree and is eaten raw or cooked." })],
    bad_flag_type: [row({ prompt_grade: "yes" })],
    duplicate_def: [row({ word: "apple" }), row({ word: "pear" })],
  };
  let failed = 0;
  for (const [name, rows] of Object.entries(cases)) {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "defpool-check-"));
    // kid_too_long must fail on length alone, not on tier.
    fs.writeFileSync(path.join(d, "en.json"), JSON.stringify({ lang: "en", tiers: { easy: rows }, exclusions: {} }));
    const { bad } = check(d);
    const want = name === "clean" ? 0 : 1;
    const ok = want === 0 ? bad.length === 0 : bad.length > 0;
    console.log(`  ${ok ? (want ? "caught " : "clean  ") : "MISSED "} ${name}${bad.length ? " — " + bad[0].slice(0, 76) : ""}`);
    if (!ok) failed++;
    fs.rmSync(d, { recursive: true });
  }
  if (failed) { console.error(`def-pool-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log("def-pool-check selftest: OK");
  process.exit(0);
}

const POOLS = `${ROOT}/backend/def_pools`;
const { bad, rows, files } = check(POOLS);
if (bad.length) {
  console.error("def-pool-check: FAILED");
  for (const b of bad.slice(0, 40)) console.error("  " + b);
  if (bad.length > 40) console.error(`  ...and ${bad.length - 40} more`);
  console.error("\n  Rebuild with: python3 scripts/build-def-pools.py <lang>");
  process.exit(1);
}
const fresh = staleRows(POOLS, `${ROOT}/assets/words`);
let note = "";
if (fresh && fresh.off) {
  note = ` — NOTE ${fresh.off} row(s) name words no longer in the bank; rebuild to clear (not a failure)`;
}
console.log(`def-pool-check: OK — ${rows} rows in ${files} language(s), every definition a meaning${note}`);
