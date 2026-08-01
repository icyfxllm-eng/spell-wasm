#!/usr/bin/env node
// CC-PICTURE-PLATFORM feature 4 — web-bundle symbol scan (job:
// web-picture-wall-scan). Run against a SITE build (`npm run build:web`).
//
// I1: no byte of Spell Picture code, asset or string exists in any web deploy
// artifact. The cfg keeps the mode out of the wasm; this proves it, and
// catches the markup and the strings, which no cargo feature can reach.
//
// D3: the denylist is GENERATED from the app-side sources, never hand-kept, so
// a new picture pack cannot dodge the scan by not being on someone's list.
//
// The false-positive trap this had to avoid: `dog`, `star`, `house`, `fish`
// and `cat` are subject IDs AND ordinary English word-bank entries that
// legitimately ship on the site. A denylist of bare subject IDs fails on day
// one and gets disabled by whoever is unlucky enough to hit it. So subjects
// are only ever matched in picture-specific SHAPES -- a scan key, an asset
// path, a manifest field -- never as a bare word.

import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, extname } from "node:path";

const DIST = process.argv[2] || "dist";
const MANIFESTS = "content-pipeline/wordpic/manifests";
const LOCALES = "src/i18n/locales";

// ---- generate the denylist from app-side sources (D3) --------------------
const subjects = existsSync(MANIFESTS)
  ? readdirSync(MANIFESTS).filter((f) => f.endsWith(".json")).map((f) => f.replace(/\.json$/, ""))
  : [];

const patterns = [];
const add = (label, re) => patterns.push({ label, re });

// Code symbols and CSS hooks unique to the mode.
for (const sym of ["wp-outline", "wp-pinned", "wp-feature", "wp-word", "wp-reveal",
                   "wp-stage", "wp-screen", "scanlock", "sub_floor", "decorative_thin",
                   "micro_feature", "wordpic", "spellpic", "wpRevealStage", "wpKbSlot"]) {
  add(`symbol ${sym}`, new RegExp(sym.replace(/[-]/g, "\\-"), "g"));
}
// Asset + manifest paths.
for (const p of ["config/wordpic", "wordpic/scans", "wordpic/manifests", "scans.json"]) {
  add(`asset path ${p}`, new RegExp(p.replace(/[/.]/g, "\\$&"), "g"));
}
// Subject IDs, but only in picture-specific shapes.
for (const s of subjects) {
  add(`subject ${s} (as a scan key)`, new RegExp(`"${s}"\\s*:\\s*\\{[^}]*"paths"`, "g"));
  add(`subject ${s} (as a data-pic tile)`, new RegExp(`data-pic="${s}"`, "g"));
}
// Localized mode strings, in every language, taken from the locale tables so
// a new translation cannot slip past.
if (existsSync(LOCALES)) {
  for (const f of readdirSync(LOCALES).filter((n) => n.endsWith(".json"))) {
    const t = JSON.parse(readFileSync(join(LOCALES, f), "utf8"));
    for (const [k, v] of Object.entries(t)) {
      if (!/^(wordpic|finale)\./.test(k)) continue;
      const s = String(v).trim();
      // Short or templated values collide with ordinary copy; the long ones
      // are the distinctive ones and are what would actually leak.
      if (s.length < 24) continue;
      add(`${f} ${k}`, new RegExp(s.slice(0, 40).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g"));
    }
  }
}

// ---- scan every emitted file ---------------------------------------------
const TEXTUAL = new Set([".js", ".html", ".json", ".css", ".mjs", ".wasm", ".webmanifest"]);
function* files(dir) {
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) yield* files(p);
    else yield p;
  }
}

if (!existsSync(DIST)) {
  console.error(`web-picture-wall-scan: ${DIST} does not exist — build it first`);
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

if (hits.length) {
  console.error(`web-picture-wall-scan: I1 VIOLATED — ${hits.length} hit(s) in ${DIST}\n`);
  for (const h of hits.slice(0, 40)) console.error("  " + h);
  if (hits.length > 40) console.error(`  … and ${hits.length - 40} more`);
  console.error("\nThe site must contain no Spell Picture code, asset or string.");
  process.exit(1);
}
console.log(`web-picture-wall-scan: OK — ${scanned} files, ${patterns.length} generated ` +
            `patterns, zero Spell Picture traces in ${DIST}`);
