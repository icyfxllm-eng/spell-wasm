#!/usr/bin/env node
// CC-SPELLUZZLE O1 -- web-bundle symbol scan for the Spelluzzle mode (a copy of the Spelluzzle wall scan).
// Run against a SITE build (`npm run build:web`); the sibling of
// web-picture-wall-scan.mjs, for the same reason: the cfg keeps the mode out of
// the wasm, but the markup, the CSS and the locale strings are DATA that no
// cargo feature can reach, so the claim "the site carries no Spelluzzle" has to
// be checked on the artifact.
//
//   node scripts/web-spelluzzle-wall-scan.mjs dist
//
// The denylist is generated from the app-side sources: every `bg.` locale key
// and its localized value (in all fifteen languages) is read from the tables, so
// a new string cannot dodge the scan by not being on a hand-kept list.
import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, extname } from "node:path";

const DIST = process.argv[2] || "dist";
const LOCALES = "src/i18n/locales";

const patterns = [];
const add = (label, re) => patterns.push({ label, re });

for (const sym of ["spelluzzle", "SPELLUZZLE", "szOpenBtn", "szScreen", "sz-screen", "sz-key", "sz-cell", "sz-rune",
                   "szBoard", "szKb", "szStartPane", "spell_flag_spelluzzle", "spell_spz_", "rune-0", "rune-27"]) {
  add(`symbol ${sym}`, new RegExp(sym.replace(/[-]/g, "\\-"), "g"));
}
// The locale namespace: the keys themselves, as they sit in the wasm's tables.
add("locale key namespace", /"spz\.[A-Za-z]+(\.[A-Za-z]+)?"/g);

if (existsSync(LOCALES)) {
  for (const f of readdirSync(LOCALES).filter((n) => n.endsWith(".json"))) {
    const t = JSON.parse(readFileSync(join(LOCALES, f), "utf8"));
    // A bg. value that is word for word a string the site legitimately ships
    // under another key (a shared phrase such as "not available in this
    // language yet") cannot prove a leak, so only values unique to the mode
    // are scanned. The KEYS are scanned regardless, below.
    const shared = new Set(Object.entries(t).filter(([k]) => !k.startsWith("spz.")).map(([, v]) => String(v).trim()));
    for (const [k, v] of Object.entries(t)) {
      if (!k.startsWith("spz.")) continue;
      const s = String(v).trim();
      if (shared.has(s)) continue;
      // The scan matches the first 40 characters, so a value that is only the START of a
      // string the site legitimately ships (fil "Burahin ang salita" inside "Burahin ang
      // salita ko") cannot prove a leak either.
      if ([...shared].some((v) => v.includes(s.slice(0, 40)))) continue;
      // Short values collide with ordinary copy; the long ones are what leaks.
      if (s.length < 14) continue;
      add(`${f} ${k}`, new RegExp(s.slice(0, 40).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g"));
    }
  }
}

const TEXTUAL = new Set([".js", ".html", ".json", ".css", ".mjs", ".wasm", ".webmanifest"]);
function* files(dir) {
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) yield* files(p);
    else yield p;
  }
}

if (!existsSync(DIST)) {
  console.error(`web-spelluzzle-wall-scan: ${DIST} does not exist — build it first`);
  process.exit(1);
}

const hits = [];
let scanned = 0;
for (const f of files(DIST)) {
  if (!TEXTUAL.has(extname(f))) continue;
  scanned++;
  const body = readFileSync(f, "latin1"); // wasm is binary; latin1 keeps bytes 1:1
  for (const { label, re } of patterns) {
    re.lastIndex = 0;
    const m = re.exec(body);
    if (m) hits.push(`${f}: ${label}  (…${body.slice(Math.max(0, m.index - 24), m.index + 40).replace(/\s+/g, " ")}…)`);
  }
}

// "No Spelluzzle" and "still a working site" are two claims; a sentinel cut that
// runs long takes shared UI with it.
const CORE = [".kb-key{", ".orb-wrap{", ".stage{", ".launch{", ".timer-ring{", "id=\"langSel\"", "id=\"gameKeyboard\""];
const html = join(DIST, "index.html");
if (existsSync(html)) {
  const body = readFileSync(html, "utf8");
  const missing = CORE.filter((c) => !body.includes(c));
  if (missing.length) {
    console.error(`web-spelluzzle-wall-scan: the strip removed core UI, not just the Spelluzzle:\n` +
      missing.map((m) => "  missing " + m).join("\n") +
      `\n\nCheck the BOARDGAME sentinel boundaries in index.html.`);
    process.exit(1);
  }
}

if (hits.length) {
  console.error(`web-spelluzzle-wall-scan: I13 VIOLATED — ${hits.length} hit(s) in ${DIST}\n`);
  for (const h of hits.slice(0, 40)) console.error("  " + h);
  if (hits.length > 40) console.error(`  … and ${hits.length - 40} more`);
  console.error("\nThe site must contain no Spelluzzle code, markup, asset or string.");
  process.exit(1);
}
console.log(`web-spelluzzle-wall-scan: OK — ${scanned} files, ${patterns.length} generated ` +
            `patterns, zero Spelluzzle traces in ${DIST}`);
