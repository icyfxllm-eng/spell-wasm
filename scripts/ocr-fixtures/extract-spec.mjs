// CC-PHOTO-IMPORT Phase 7 — extract the fixture word lists from the word banks.
// 10 easy-tier words per launch language (zh uses the hanzi side, since that is
// what a photographed page carries). Output: scripts/ocr-fixtures/spec.json,
// consumed by gen-ocr-fixtures.swift. Words come FROM the banks so the fixture
// ground truth is also an InDictionary classification check.
import fs from "node:fs";

const src =
  fs.readFileSync("src/word_data.rs", "utf8") + fs.readFileSync("src/words.rs", "utf8");
const langs = {
  en: "EN_EASY", es: "ES_EASY", fr: "FR_EASY", de: "DE_EASY", pt: "PT_EASY",
  pl: "PL_EASY", vi: "VI_EASY", ko: "KO_EASY", ja: "JA_EASY", fil: "FIL_EASY",
  zh: "ZH_EASY", ru: "RU_EASY", sw: "SW_EASY",
};
const out = {};
for (const [lang, name] of Object.entries(langs)) {
  const m = src.match(new RegExp(`const ${name}[^=]*= &\\[([\\s\\S]*?)\\];`));
  if (!m) throw new Error("bank not found: " + name);
  let words = [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]);
  if (lang === "zh") words = words.map((w) => w.split("|")[1] || w);
  const min = ["ko", "ja", "zh"].includes(lang) ? 1 : 3;
  words = words.filter((w) => [...w].length >= min).slice(0, 10);
  if (words.length < 10) throw new Error(`${lang} only has ${words.length}`);
  out[lang] = words;
}
fs.writeFileSync("scripts/ocr-fixtures/spec.json", JSON.stringify(out, null, 2) + "\n");
console.log(
  Object.entries(out)
    .map(([l, w]) => `${l}: ${w.slice(0, 3).join(",")}…`)
    .join("\n"),
);
