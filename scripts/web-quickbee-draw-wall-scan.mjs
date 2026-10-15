#!/usr/bin/env node
// CC-QUICKBEE-DRAW -- web-bundle symbol scan for the the drawn Quick Bee mode (a copy of the the drawn Quick Bee wall scan).
// Run against a SITE build (`npm run build:web`); the sibling of
// web-picture-wall-scan.mjs, for the same reason: the cfg keeps the mode out of
// the wasm, but the markup, the CSS and the locale strings are DATA that no
// cargo feature can reach, so the claim "the site carries no the drawn Quick Bee" has to
// be checked on the artifact.
//
//   node scripts/web-quickbee-draw-wall-scan.mjs dist
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

for (const sym of ["quickbee_draw", "QUICKBEEDRAW", "qbdOpenBtn", "qbd-", "spell_flag_quickbee_draw"]) {
  add(`symbol ${sym}`, new RegExp(sym.replace(/[-]/g, "\\-"), "g"));
}
// The locale namespace: the keys themselves, as they sit in the wasm's tables.
add("locale key namespace", /"qbd\.[A-Za-z]+(\.[A-Za-z]+)?"/g);

if (existsSync(LOCALES)) {
  for (const f of readdirSync(LOCALES).filter((n) => n.endsWith(".json"))) {
    const t = JSON.parse(readFileSync(join(LOCALES, f), "utf8"));
    // A bg. value that is word for word a string the site legitimately ships
    // under another key (a shared phrase such as "not available in this
    // language yet") cannot prove a leak, so only values unique to the mode
    // are scanned. The KEYS are scanned regardless, below.
    const shared = new Set(Object.entries(t).filter(([k]) => !k.startsWith("qbd.")).map(([, v]) => String(v).trim()));
    for (const [k, v] of Object.entries(t)) {
      if (!k.startsWith("qbd.")) continue;
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
  console.error(`web-quickbee-draw-wall-scan: ${DIST} does not exist — build it first`);
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

// "No the drawn Quick Bee" and "still a working site" are two claims; a sentinel cut that
// runs long takes shared UI with it.
const CORE = [".kb-key{", ".orb-wrap{", ".stage{", ".launch{", ".timer-ring{", "id=\"langSel\"", "id=\"gameKeyboard\""];
const html = join(DIST, "index.html");
if (existsSync(html)) {
  const body = readFileSync(html, "utf8");
  const missing = CORE.filter((c) => !body.includes(c));
  if (missing.length) {
    console.error(`web-quickbee-draw-wall-scan: the strip removed core UI, not just the the drawn Quick Bee:\n` +
      missing.map((m) => "  missing " + m).join("\n") +
      `\n\nCheck the QUICKBEEDRAW sentinel boundaries in index.html.`);
    process.exit(1);
  }
}

if (hits.length) {
  console.error(`web-quickbee-draw-wall-scan: I13 VIOLATED — ${hits.length} hit(s) in ${DIST}\n`);
  for (const h of hits.slice(0, 40)) console.error("  " + h);
  if (hits.length > 40) console.error(`  … and ${hits.length - 40} more`);
  console.error("\nThe site must contain no the drawn Quick Bee code, markup, asset or string.");
  process.exit(1);
}
console.log(`web-quickbee-draw-wall-scan: OK — ${scanned} files, ${patterns.length} generated ` +
            `patterns, zero the drawn Quick Bee traces in ${DIST}`);
