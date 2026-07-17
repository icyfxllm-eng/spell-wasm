#!/usr/bin/env node
// License gate for the word-list provenance registry (CC-WORDLIST-SOURCES).
//
//   node scripts/license-gate.mjs
//
// FAILS the build (exit 1) if any of the following is true:
//   (a) a wordlists/<lang>.txt exists without a COMPLETE sources/<lang>/ entry
//       (registry entry + fetch.sh + LICENSE + PROVENANCE.md);
//   (b) a registry license tier is not A/B, or a Tier C source is used to
//       produce a shipped/derived list (Tier C is unmodified/server-side only);
//   (c) the generated credits file (credits.json) is missing a required
//       attribution for any Tier B (attribution) source that has a wordlist.
//
// PASSES (exit 0) only when every wordlist is fully backed. There is no silent
// fallback: a missing piece is a hard error.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const rel = (p) => p.replace(ROOT + "/", "");

const errors = [];
const notes = [];
const fail = (m) => errors.push(m);

// --- load registry ----------------------------------------------------------
const regPath = join(ROOT, "sources", "registry.json");
if (!existsSync(regPath)) {
  console.error("license-gate: FAIL — sources/registry.json is missing.");
  process.exit(1);
}
const registry = JSON.parse(readFileSync(regPath, "utf8"));
const sources = registry.sources || {};
const shipTiers = new Set(registry.allowed_tiers_for_ship || ["A", "B"]);
const knownTiers = new Set(Object.keys(registry.tiers || { A: 1, B: 1, C: 1 }));

// --- credits (attribution surface) ------------------------------------------
const creditsPath = join(ROOT, "credits.json");
let creditsSources = [];
if (existsSync(creditsPath)) {
  try {
    creditsSources = JSON.parse(readFileSync(creditsPath, "utf8")).sources || [];
  } catch (e) {
    fail(`credits.json is present but not valid JSON: ${e.message}`);
  }
} else {
  notes.push("credits.json not found (run `node scripts/gen-credits.mjs`).");
}
const creditedUrls = new Set(creditsSources.map((s) => s.url));

// --- enumerate emitted wordlists --------------------------------------------
const wlDir = join(ROOT, "wordlists");
const wordlistLangs = existsSync(wlDir)
  ? readdirSync(wlDir)
      .filter((f) => f.endsWith(".txt"))
      .map((f) => f.slice(0, -4))
  : [];

if (wordlistLangs.length === 0) {
  notes.push("No wordlists/*.txt present — nothing to gate.");
}

for (const lang of wordlistLangs) {
  const src = sources[lang];

  // (a) complete registry + filesystem entry
  if (!src) {
    fail(`wordlists/${lang}.txt exists but sources/registry.json has no "${lang}" entry.`);
    continue;
  }
  const dir = join(ROOT, "sources", lang);
  for (const req of ["fetch.sh", "LICENSE", "PROVENANCE.md"]) {
    const p = join(dir, req);
    if (!existsSync(p)) {
      fail(`wordlists/${lang}.txt: incomplete registry — missing ${rel(p)}.`);
    }
  }
  for (const field of ["name", "url", "license_spdx", "tier"]) {
    if (!src[field]) fail(`registry "${lang}": missing required field "${field}".`);
  }

  // (b) tier validity
  const tier = src.tier;
  if (!knownTiers.has(tier)) {
    fail(`registry "${lang}": unknown license tier "${tier}" (expected A/B/C).`);
  } else if (!shipTiers.has(tier)) {
    // Tier C: only allowed unmodified/server-side. A generated wordlist is a
    // DERIVED, modified artifact, so a Tier C source backing one is a failure.
    fail(`registry "${lang}": tier ${tier} source may not back a derived wordlist ` +
         `(Tier C is unmodified/server-side use only). STOP AND ASK.`);
  }

  // (c) attribution present for attribution-required tiers (B)
  if (tier === "B") {
    if (!creditedUrls.has(src.url)) {
      fail(`wordlists/${lang}.txt: Tier B source "${src.name}" requires attribution, ` +
           `but credits.json has no entry for ${src.url}. Regenerate credits.json.`);
    }
  }
}

// --- also sanity-check every registered source declares a known tier --------
for (const [lang, src] of Object.entries(sources)) {
  if (!knownTiers.has(src.tier)) {
    fail(`registry "${lang}": tier "${src.tier}" is not one of A/B/C ` +
         `(STOP AND ASK — see D1).`);
  }
}

// --- (d) provenance: every SHIPPED curated word must be source-backed --------
// For each language with BOTH a registry entry and shipped curated tiers
// (assets/words/<lang>/{easy,medium,hard,expert}.txt), every curated word must
// EXIST in the raw source surface index (sources/<lang>/surface-index.txt), OR
// be a reviewed entry in sources/<lang>/curated-exceptions.txt. Anything else is
// a build failure. A missing surface index is a HARD ERROR — never assume backed.
const TIERS = ["easy", "medium", "hard", "expert"];
const nfcLower = (s) => s.normalize("NFC").toLowerCase();
const readLines = (p) =>
  readFileSync(p, "utf8").split("\n").map((l) => l.replace(/\r$/, ""));
const compoundTokens = (w) =>
  nfcLower(w).split(/[\s-]+/).map((t) => t.trim()).filter((t) => t.length > 0);

for (const lang of Object.keys(sources)) {
  // Provenance-backing only means something for an INGESTED source: it asks
  // "does every curated word exist in the licensed lexicon we took it from?".
  // For original curation there IS no external lexicon — we authored the words,
  // so they ARE the source, and demanding a surface index would be asking the
  // list to prove it descends from itself. Its licence question is answered by
  // permitted_use/verified_by in check (e) instead, which is the real question
  // for owned content.
  if (sources[lang]?.kind === "original") continue;

  const curatedDir = join(ROOT, "assets", "words", lang);
  const tierPaths = TIERS.map((t) => join(curatedDir, `${t}.txt`));
  const presentTiers = tierPaths.filter((p) => existsSync(p));
  if (presentTiers.length === 0) continue; // no shipped curated list for this lang

  const indexPath = join(ROOT, "sources", lang, "surface-index.txt");
  if (!existsSync(indexPath)) {
    fail(`provenance: ${lang} ships curated words but ${rel(indexPath)} is missing. ` +
         `Build it (scripts/surface-index.sh ${lang}); NO silent fallback.`);
    continue;
  }
  const surface = new Set(readLines(indexPath).filter((l) => l !== ""));
  if (surface.size === 0) {
    fail(`provenance: ${rel(indexPath)} is empty.`);
    continue;
  }

  const exceptions = new Set();
  const excPath = join(ROOT, "sources", lang, "curated-exceptions.txt");
  if (existsSync(excPath)) {
    for (let line of readLines(excPath)) {
      const h = line.indexOf("#");
      if (h !== -1) line = line.slice(0, h);
      line = line.trim();
      if (line !== "") exceptions.add(nfcLower(line));
    }
  }

  const unbacked = [];
  for (const p of presentTiers) {
    for (const raw of readLines(p).map((w) => w.trim()).filter(Boolean)) {
      const toks = compoundTokens(raw);
      const backed = toks.length > 0 && toks.every((t) => surface.has(t));
      if (!backed && !exceptions.has(nfcLower(raw))) unbacked.push(nfcLower(raw));
    }
  }
  if (unbacked.length) {
    fail(`provenance: ${lang} — ${unbacked.length} curated word(s) neither in the raw ` +
         `source index nor in ${rel(excPath)}: ${unbacked.slice(0, 10).join(", ")}` +
         `${unbacked.length > 10 ? ", …" : ""}. Add a justified exception or ask Eric.`);
  } else {
    notes.push(`provenance ${lang}: all curated words backed ` +
               `(${surface.size} source forms, ${exceptions.size} reviewed exceptions).`);
  }
}


// --- (e) LICENSE VERDICTS + ORPHAN CONTENT (CC-WORDLIST-SOURCES addendum) ----
// The addendum's claim, verbatim, is what has to be machine-true: "every word
// list traces to a documented source whose license permits this use, and the
// build fails otherwise."
//
// Severity mirrors the activation-gate philosophy the addendum asks for —
// enforcement tightens at the point content becomes REACHABLE:
//   * a language that is ACTIVE ships to players  -> UNKNOWN/PROHIBITED FAILS.
//   * a language still gated (audit / RTL / coming soon) -> WARNS. Its content
//     exists but nobody can reach it, so it must not block the build.
// Active-ness is read from src/consts.rs BUILTIN_LANGS, which is the single
// source of truth every other surface reads — not a second list here that could
// drift out of step with it.
const warnings = [];
const warn = (m) => warnings.push(m);

const VERDICTS = new Set(["PERMITTED", "PERMITTED_WITH_ATTRIBUTION", "PROHIBITED", "UNKNOWN"]);

function activeLangs() {
  const p = join(ROOT, "src", "consts.rs");
  if (!existsSync(p)) return null;
  const m = readFileSync(p, "utf8").match(/pub const BUILTIN_LANGS[\s\S]*?\n\];/);
  if (!m) return null;
  const out = new Set();
  for (const line of m[0].split("\n")) {
    if (!/\bActive\b/.test(line)) continue;
    const c = line.match(/^\s*\(([A-Z_]+),/);
    if (c) out.add(c[1].toLowerCase());
  }
  return out;
}
const active = activeLangs();
if (active === null) {
  fail("cannot read BUILTIN_LANGS from src/consts.rs — refusing to guess which languages are active.");
}

// Orphan check: every language that SHIPS a word bank must be covered.
const wordsDir = join(ROOT, "assets", "words");
const shipped = existsSync(wordsDir)
  ? readdirSync(wordsDir).filter((d) => existsSync(join(wordsDir, d, "easy.txt")))
  : [];

for (const lang of shipped) {
  const src = sources[lang];
  const isActive = active ? active.has(lang) : false;
  const where = isActive ? "ACTIVE" : "gated";
  const say = isActive ? fail : warn;

  if (!src) {
    say(`orphan content: assets/words/${lang}/ ships a word bank but sources/registry.json ` +
        `has no "${lang}" entry (${where}).`);
    continue;
  }
  const use = src.permitted_use;
  if (!VERDICTS.has(use)) {
    say(`registry "${lang}": permitted_use must be one of ${[...VERDICTS].join("|")}, got ` +
        `${JSON.stringify(use)} (${where}).`);
    continue;
  }
  if (use === "PROHIBITED") {
    say(`registry "${lang}": permitted_use is PROHIBITED — it may not back a shipped list (${where}).`);
  } else if (use === "UNKNOWN") {
    say(`registry "${lang}": permitted_use is UNKNOWN — no human has verified this source (${where}). ` +
        `Fail-closed is non-negotiable: fix the manifest entry, never the gate.`);
  } else {
    // A verdict only counts if a PERSON signed it. The addendum: "verified_by
    // must name a person"; the generator may pre-fill UNKNOWN, only a human
    // promotes.
    if (!src.verified_by || String(src.verified_by).trim() === "") {
      say(`registry "${lang}": permitted_use is ${use} but verified_by names nobody (${where}). ` +
          `A verdict without a verifier is not a verdict.`);
    }
    if (!src.verified_date) {
      say(`registry "${lang}": permitted_use is ${use} but verified_date is missing (${where}).`);
    }
    if (use === "PERMITTED_WITH_ATTRIBUTION" && !creditedUrls.has(src.url)) {
      say(`registry "${lang}": PERMITTED_WITH_ATTRIBUTION but credits.json has no entry for ${src.url}.`);
    }
  }
}

// --- report -----------------------------------------------------------------
for (const n of notes) console.log(`license-gate: note — ${n}`);
for (const w of warnings) console.log(`license-gate: WARN — ${w}`);
if (warnings.length) {
  console.log(`license-gate: ${warnings.length} warning(s) — activation-gated languages, not blocking.`);
}
if (errors.length) {
  console.error("\nlicense-gate: FAIL");
  for (const e of errors) console.error("  ✗ " + e);
  process.exit(1);
}
console.log(`license-gate: PASS — ${wordlistLangs.length} wordlist(s) fully backed ` +
            `(${wordlistLangs.join(", ") || "none"}).`);
