// CC-BANK-TRANSLATE ingest validation. The gloss pivot is the whole
// safety story of the translator, so its data is checked like data that
// ships to children: every key a live bank word, every concept a live
// ENGLISH bank word, and `audited` a human claim no tool may set.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const src = fs.readFileSync(`${ROOT}/src/word_data.rs`, "utf8");
// Chinese lives in its own file because it stores `pinyin|hanzi` pairs rather
// than bare words. Its gloss keys are now the FULL pair, not the pinyin half.
//
// Keying on pinyin alone silently conflated homophones: 九 (nine) and 酒 (wine)
// are both `jiu3`, as are 班/搬, 新/心, 要/药, 前/钱, 页/夜 and 下/夏. Those are
// unrelated words, but only one of each pair could hold the key, so seven
// concepts were unreachable for a reason that was never about Chinese — the
// bank already stored the character that disambiguates, and the gloss threw it
// away. The full `pinyin|hanzi` string is exactly the bank's stored form, which
// is what the "key is a live bank word" law wants anyway.
const zhSrc = fs.readFileSync(`${ROOT}/src/words.rs`, "utf8");

function pool(lang) {
  const out = new Set();
  const from = lang === "zh" ? zhSrc : src;
  for (const tier of ["EASY", "MEDIUM", "HARD", "EXPERT"]) {
    const m = from.match(new RegExp(`pub const ${lang.toUpperCase()}_${tier}: &\\[&str\\] = &\\[([^;]*)\\];`, "s"));
    if (!m) continue;
    for (const w of m[1].matchAll(/"([^"]+)"/g)) out.add(w[1]);
  }
  return out;
}

const en = pool("en");
const problems = [];
let rows = 0, audited = 0;
const dir = `${ROOT}/config/gloss`;
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith(".json"))) {
  const doc = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8"));
  const lang = doc.lang;
  if (`${lang}.json` !== f) problems.push(`${f}: lang field '${lang}' does not match the filename`);
  const p = pool(lang);
  if (!p.size) problems.push(`${f}: no word bank for '${lang}'`);
  for (const [word, concept] of Object.entries(doc.rows || {})) {
    rows++;
    if (!p.has(word)) problems.push(`${f}: '${word}' is not a live ${lang} bank word`);
    if (!en.has(concept)) problems.push(`${f}: concept '${concept}' is not a live EN bank word`);
  }
  if (doc.audited) {
    audited++;
    if (!doc.auditor) problems.push(`${f}: audited with no named auditor — the claim needs an owner`);
    if (!Object.keys(doc.rows || {}).length) problems.push(`${f}: audited but empty`);
  }
}
if (problems.length) {
  console.log("gloss-check: FAILED\n  " + problems.join("\n  "));
  process.exit(1);
}
console.log(`gloss-check: OK — ${rows} rows across ${fs.readdirSync(dir).filter((x) => x.endsWith(".json")).length} languages, ${audited} audited (dark until a native signs)`);
