// CC-SPELLPIC F0 — the unique-ID law.
//
// The Aug 8 device audit's headline bug ("the dragon renders as the fish")
// was an ID collision: the SVG renderer minted `<path id="sl0">` from a bare
// loop index, and an `href="#sl0"` resolves against the DOCUMENT, not the
// enclosing <svg>. With #wpStage and #wpRevealStage both permanently in
// #wpPlay -- and the gallery concatenating one SVG per trophy -- the second
// picture's <textPath>s bound to the FIRST picture's geometry. That half is
// now pinned by a Rust test (two_renders_never_share_element_ids).
//
// Tracing it turned up the same defect hand-written in the shell: #wpGallery
// declared twice. getElementById returns the first, so the second was dead
// markup that every dom:: helper silently skipped. Nothing caught it, because
// nothing was looking.
//
// This is the static half of that law. Duplicate IDs are never intentional
// here: the whole Rust/DOM layer addresses elements by ID, so a duplicate
// means some call site is reaching an element the author did not mean.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const FILES = ["index.html"];

let failed = false;

for (const rel of FILES) {
  const raw = fs.readFileSync(`${ROOT}/${rel}`, "utf8");

  // Comments, scripts and styles are not markup. An `id="x"` inside a JS
  // string or a CSS selector is not an element and must not be counted --
  // counting them is how a lint like this earns a reputation for crying
  // wolf and then gets bypassed.
  const markup = raw
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<script\b[\s\S]*?<\/script>/gi, "")
    .replace(/<style\b[\s\S]*?<\/style>/gi, "");

  // Line numbers come from the ORIGINAL text, so the message points at
  // something the author can actually open.
  const seen = new Map();
  for (const m of markup.matchAll(/\sid=["']([^"']+)["']/g)) {
    seen.set(m[1], (seen.get(m[1]) ?? 0) + 1);
  }

  const dupes = [...seen].filter(([, n]) => n > 1);
  if (!seen.size) {
    console.error(`FAIL ${rel}: no element IDs found — this check is vacuous`);
    failed = true;
    continue;
  }
  for (const [id, n] of dupes) {
    const lines = raw
      .split("\n")
      .map((l, i) => (new RegExp(`\\sid=["']${id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}["']`).test(l) ? i + 1 : 0))
      .filter(Boolean);
    console.error(`FAIL ${rel}: id="${id}" declared ${n}x (lines ${lines.join(", ")})`);
    failed = true;
  }
  if (!dupes.length) {
    console.log(`ok ${rel}: ${seen.size} element IDs, all unique`);
  }
}

process.exit(failed ? 1 : 0);
