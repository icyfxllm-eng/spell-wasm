#!/usr/bin/env node
// CC-PICTURE-PLATFORM I3 — the dependency runs ONE way.
//
//   "Spell Picture may import shared code; shared code may never import
//    Spell Picture."
//
// This is the lint rule feature 2 asks for. It matters beyond tidiness: every
// inbound reference is a place the `picture` cargo feature cannot compile the
// mode out without breaking the base game. Before the surface_hooks inversion
// there were six, in game.rs, spell_aloud.rs and share.rs.
//
// Exempt: lib.rs (it wires every module by definition) and testseam.rs (a
// dev-only observation surface, itself absent from production builds).

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const SRC = "src";
const PICTURE = /^(wordpic|spellpic)/;           // the subtree, by module name
const REFERENCE = /\bcrate::(wordpic\w*|spellpic\w*)\b/g;
const EXEMPT = new Set(["lib.rs", "testseam.rs"]);

const offenders = [];
for (const f of readdirSync(SRC).filter((n) => n.endsWith(".rs"))) {
  const mod = f.replace(/\.rs$/, "");
  if (PICTURE.test(mod) || EXEMPT.has(f)) continue;
  readFileSync(join(SRC, f), "utf8").split("\n").forEach((line, i) => {
    const code = line.replace(/\/\/.*$/, "");   // a comment may name the mode
    for (const m of code.matchAll(REFERENCE)) {
      offenders.push(`${SRC}/${f}:${i + 1}: ${m[0]}  —  ${line.trim().slice(0, 72)}`);
    }
  });
}

if (offenders.length) {
  console.error("Shared code imports the Spell Picture subtree (I3):\n" + offenders.join("\n"));
  console.error(
    "\nInvert it: have the picture register with shared code (src/surface_hooks.rs)" +
    "\nrather than shared code reaching into the picture.");
  process.exit(1);
}
// CC-SCAN-STACK boundary, same spirit as I3 and v1.2 Done #3: the tonal
// crate (kornia, the suggester) is TOOL-SIDE. The day spell_wasm links it,
// scan-stack machinery rides into the app bundle past every gate that
// polices source text — so ask the dependency graph itself.
import { execSync } from "node:child_process";
const tree = execSync("cargo tree -p spell_wasm --edges normal", { encoding: "utf8" });
for (const banned of ["kornia", "tonal"]) {
  if (tree.split("\n").some((l) => l.includes(banned))) {
    console.error(`picture-import: spell_wasm depends on ${banned} — scan-stack is tool-side only`);
    process.exit(1);
  }
}

console.log(`picture-import: OK — no shared module imports the picture subtree (I3), app links no tool crate`);
