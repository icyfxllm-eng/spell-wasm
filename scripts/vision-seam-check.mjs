#!/usr/bin/env node
// CC-SCAN-STACK v1.1 #10 -- the vision layer is the ONLY per-platform seam.
//
// Module 1 may use Vision all it likes internally. What it may not do is let
// a Vision type escape: the moment a VN* appears in a public signature,
// every downstream module is welded to Apple and section C's substitution
// map becomes fiction. This is the grep-level check Done #3 asks for.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const SRC = "ios/ScanStack/Sources";
// Vision's own types all carry the VN prefix. Anything else in a public
// signature is Foundation/CoreGraphics, which is portable data by design.
const VISION_TYPE = /\bVN[A-Z]\w*/;
const PUBLIC_DECL = /^\s*(public|open)\s+(?!import)/;

let bad = [];
for (const mod of readdirSync(SRC)) {
  for (const f of readdirSync(join(SRC, mod)).filter(n => n.endsWith(".swift"))) {
    const path = join(SRC, mod, f);
    readFileSync(path, "utf8").split("\n").forEach((line, i) => {
      // Comments explain the seam; they are not the seam.
      const code = line.replace(/\/\/.*$/, "");
      if (PUBLIC_DECL.test(code) && VISION_TYPE.test(code)) {
        bad.push(`${path}:${i + 1}: ${line.trim()}`);
      }
    });
  }
}

if (bad.length) {
  console.error("Vision types escaping the module 1 seam:\n" + bad.join("\n"));
  console.error("\nKeep VN* inside the module; emit masks, point sets and rects.");
  process.exit(1);
}
console.log("vision seam: no VN* type in any public signature — OK");
