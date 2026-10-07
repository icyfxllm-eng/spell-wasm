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
// Freshness is a law here too, on Eric's call: a row whose word has left the
// bank fails the check. That means editing a word list makes the pools stale
// by definition and blocks the next push until they are rebuilt — which is a
// rate-limited fetch measured in hours. Start it in the background early. The
// 6,804 such rows the CC-CONTRIBUTE census counted are what this prevents.
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

export function check(dir, bankDir) {
  const bad = [];
  // Mirrors build-wordlists.py: accent-sensitive for the languages whose marks
  // are lexical, lenient elsewhere. Kept in step by `exclusion_fold_parity`.
  const ACCENT_SENSITIVE = new Set(["vi"]);
  const foldFor = (lang) => (s) => {
    const base = s.normalize("NFC").toLowerCase().trim();
    return ACCENT_SENSITIVE.has(lang)
      ? base.replace(/\s+/g, "")
      : base.normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/\s+/g, "");
  };
  const exclusionSet = (lang) => {
    const p = path.join("assets", "words", "exclusions", `${lang}.txt`);
    if (!fs.existsSync(p)) return null;
    const fold = foldFor(lang);
    return new Set(
      fs.readFileSync(p, "utf8").split("\n")
        .map((l) => l.trim())
        .filter((l) => l && !l.startsWith("#"))
        .map(fold),
    );
  };
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
    // Freshness is a law, not a note (Eric, 2026-09-30): a row for a word the
    // bank no longer serves is a row no player can be shown and no contributor
    // should be asked to judge. A missing bank FAILS rather than skips —
    // a check that cannot verify must not report OK.
    const bank = bankDir === undefined ? null : bankWords(bankDir, lang);
    if (bank && !bank.size) bad.push(`${f}: no word bank at ${path.join(bankDir, lang)} — cannot verify freshness`);
    // A word excluded from the BANK must not survive in the DEFINITIONS. They
    // are separate files, so a word removed from one stayed in the other: when
    // the vi exclusion seed was first filled it caught `đéo` in the Easy bank
    // AND a medium definition reading "to have penetrative sex (with)", which
    // nothing would have removed. The bank's exclusion list is the one list.
    const excl = exclusionSet(lang);
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
        if (excl && excl.has(foldFor(lang)(r.word))) {
          bad.push(`${where}: on the ${lang} exclusion list — a word kept out of the bank must not keep a definition`);
        }
        if (bank && bank.size && !bank.has(r.word)) {
          offBank++;
          if (offBank <= 12) bad.push(`${where}: not in the ${lang} bank any more — rebuild to drop it`);
        }
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

// The bank as the pools must key on it. Chinese stores `pinyin|hanzi` pairs
// and the pools key on the hanzi alone, so take the half after the pipe —
// without this every zh row reads as off-bank.
function bankWords(bankDir, lang) {
  const out = new Set();
  for (const tier of TIERS) {
    const p = path.join(bankDir, lang, `${tier}.txt`);
    if (!fs.existsSync(p)) continue;
    for (const line of fs.readFileSync(p, "utf8").split("\n")) {
      const w = line.trim();
      if (!w || w.startsWith("#")) continue;
      out.add(w.includes("|") ? w.slice(w.lastIndexOf("|") + 1) : w);
    }
  }
  return out;
}

if (process.argv.includes("--selftest")) {
  const row = (o = {}) => ({ word: "apple", definition: "A round fruit.", pos: "noun", prompt_grade: true, kid_register: true, ...o });
  // Each case asserts the law it is ABOUT, not merely that something failed.
  // Without that, every case here would pass on the freshness law alone the
  // moment its word was missing from the fixture bank, and a broken ISO regex
  // would still report "caught".
  const cases = {
    clean: { rows: [row()], want: null },
    iso_code: { rows: [row({ word: "cat", definition: "ISO 639-2 & ISO 639-3 language code for Catalan." })], want: /an ISO code/ },
    iso_former: { rows: [row({ word: "in", definition: "Former ISO 639-1 language code for Indonesian." })], want: /an ISO code/ },
    symbol_for: { rows: [row({ word: "as", definition: "Symbol for attosecond, an SI unit of time." })], want: /a symbol gloss/ },
    inlined_css: { rows: [row({ word: "kana", definition: "to be .mw-parser-output .object-usage-tag{font-style:italic}" })], want: /inlined Wiktionary CSS/ },
    topical_label: { rows: [row({ word: "cat", definition: "Terms relating to animals." })], want: /topical label/ },
    two_lines: { rows: [row({ definition: "shirt\n dress shirt" })], want: /more than one line/ },
    empty_def: { rows: [row({ definition: "" })], want: /empty/ },
    long_prompt_grade: { rows: [row({ definition: "A".repeat(91) })], want: /prompt_grade but 91 chars/ },
    self_leak: { rows: [row({ definition: "An apple is a fruit." })], want: /contains the word/ },
    kid_too_long: { rows: [row({ definition: "A round fruit that grows on a tree and is eaten raw or cooked." })], want: /kid_register but 62 chars/ },
    bad_flag_type: { rows: [row({ prompt_grade: "yes" })], want: /prompt_grade is not a boolean/ },
    duplicate_def: { rows: [row({ word: "apple" }), row({ word: "pear" })], want: /share one definition/ },
    off_bank: { rows: [row({ word: "quince", definition: "A hard yellow fruit." })], want: /not in the en bank/ },
    missing_bank: { rows: [row()], want: /cannot verify freshness/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "defpool-check-"));
    fs.writeFileSync(path.join(d, "en.json"), JSON.stringify({ lang: "en", tiers: { easy: c.rows }, exclusions: {} }));
    const bankDir = path.join(d, "bank");
    if (name === "missing_bank") {
      fs.mkdirSync(bankDir, { recursive: true });
    } else {
      // Every word used by a case EXCEPT the off-bank one, so each case fails
      // on its own law and not on freshness.
      fs.mkdirSync(path.join(bankDir, "en"), { recursive: true });
      fs.writeFileSync(path.join(bankDir, "en", "easy.txt"), "# a comment\napple\npear\ncat\nin\nas\nkana\n");
    }
    const { bad } = check(d, bankDir);
    let ok, why = "";
    if (c.want === null) {
      ok = bad.length === 0;
      why = bad[0] || "";
    } else {
      ok = bad.some((b) => c.want.test(b));
      why = ok ? bad.find((b) => c.want.test(b)) : `wanted ${c.want}, got ${bad.length ? bad.join(" | ") : "nothing"}`;
    }
    console.log(`  ${ok ? (c.want ? "caught " : "clean  ") : "MISSED "} ${name}${why ? " — " + why.slice(0, 74) : ""}`);
    if (!ok) failed++;
    fs.rmSync(d, { recursive: true });
  }
  // zh keys on the hanzi while the bank stores `pinyin|hanzi`. Splitting wrong
  // would mark every Chinese row off-bank, so prove both directions.
  {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "defpool-check-zh-"));
    fs.mkdirSync(path.join(d, "bank", "zh"), { recursive: true });
    fs.writeFileSync(path.join(d, "bank", "zh", "easy.txt"), "ai4|\u7231\nba1|\u516b\n");
    const mk = (w) => JSON.stringify({ lang: "zh", tiers: { easy: [{ word: w, definition: "love", pos: "noun", prompt_grade: true, kid_register: true }] }, exclusions: {} });
    fs.writeFileSync(path.join(d, "zh.json"), mk("\u7231"));
    const hanzi = check(d, path.join(d, "bank")).bad.length === 0;
    fs.writeFileSync(path.join(d, "zh.json"), mk("ai4|\u7231"));
    const pair = check(d, path.join(d, "bank")).bad.some((b) => /not in the zh bank/.test(b));
    console.log(`  ${hanzi ? "clean  " : "MISSED "} zh_hanzi_key_accepted`);
    console.log(`  ${pair ? "caught " : "MISSED "} zh_pipe_key_rejected`);
    if (!hanzi || !pair) failed++;
    fs.rmSync(d, { recursive: true });
  }
  // A word excluded from the bank must not keep a definition. The vi seed
  // caught exactly this: `đéo` was gone from the bank and its medium definition
  // -- "to have penetrative sex (with)" -- was still there, in a children's
  // spelling game, with nothing to remove it.
  {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "defpool-check-excl-"));
    fs.mkdirSync(path.join(d, "assets", "words", "exclusions"), { recursive: true });
    const cwd = process.cwd();
    process.chdir(d);
    fs.writeFileSync(path.join("assets", "words", "exclusions", "vi.txt"), "# seed\n\u0111\u00e9o\n");
    const row = (w) => JSON.stringify({ lang: "vi", tiers: { easy: [{ word: w, definition: "a meaning", pos: "noun", prompt_grade: true, kid_register: true }] }, exclusions: {} });
    fs.writeFileSync(path.join(d, "vi.json"), row("\u0111\u00e9o"));
    const caught = check(d, undefined).bad.some((b) => /on the vi exclusion list/.test(b));
    fs.writeFileSync(path.join(d, "vi.json"), row("\u0111i"));
    // vi matches ACCENT-SENSITIVELY: đi (to go) must survive a seed holding đĩ.
    fs.writeFileSync(path.join("assets", "words", "exclusions", "vi.txt"), "\u0111\u0129\n");
    const spared = check(d, undefined).bad.every((b) => !/exclusion list/.test(b));
    process.chdir(cwd);
    console.log(`  ${caught ? "caught " : "MISSED "} excluded_word_keeps_a_definition`);
    console.log(`  ${spared ? "clean  " : "MISSED "} accent_sensitive_spares_di`);
    if (!caught || !spared) failed++;
    fs.rmSync(d, { recursive: true });
  }
  if (failed) { console.error(`def-pool-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log("def-pool-check selftest: OK");
  process.exit(0);
}

const POOLS = `${ROOT}/backend/def_pools`;
const { bad, rows, files, offBank } = check(POOLS, `${ROOT}/assets/words`);
if (bad.length) {
  console.error("def-pool-check: FAILED");
  for (const b of bad.slice(0, 40)) console.error("  " + b);
  if (bad.length > 40) console.error(`  ...and ${bad.length - 40} more`);
  if (offBank) {
    console.error(`\n  ${offBank} row(s) name words the bank no longer serves.`);
    console.error("  Editing a word list makes the pools stale by definition — the rebuild");
    console.error("  is a rate-limited fetch and takes hours, so start it before you need it:");
    console.error("    python3 scripts/build-def-pools.py <lang>   # backgrounded");
  }
  console.error("\n  Rebuild with: python3 scripts/build-def-pools.py <lang>");
  process.exit(1);
}
console.log(`def-pool-check: OK — ${rows} rows in ${files} language(s), every definition a meaning, every word still in the bank`);
