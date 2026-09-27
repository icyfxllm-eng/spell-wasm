// The dead-id law.
//
// dom::el panics on a missing element, deliberately: the Rust/DOM layer
// addresses everything by id, and a typo should be loud. What nothing was
// checking is the other direction -- an id that WAS right and then went
// away. Rename the element in index.html, leave the string in a .rs file,
// and the compiler is happy, the gate is green, and the app aborts the
// first time that line runs.
//
// It had happened twice, unnoticed, before this check existed:
//
//   * #wpDone. The reveal card became #wpReveal in the FINALE commit
//     (484aca21, Jul 31). close_play kept clearing #wpDone, so every exit
//     from Spell Pic panicked and took the whole app with it. It shipped
//     that way for two months.
//   * #drawHint. Three sites in drawing.rs toggled a class on an id that
//     exists in no markup at all, and clear_canvas runs when the ink pad
//     OPENS -- so zh and ja ink practice crashed on the first tap.
//
// Both are the same shape, and both are invisible to every other gate:
// unit tests do not have a DOM, and the e2e suites never opened those two
// screens. A string is all that connects Rust to markup, so a string is
// what has to be checked.
//
// The law: every element id a PANICKING dom:: helper is given as a literal
// must exist -- in index.html, or created by Rust at runtime, or guarded by
// dom::exists in the same file.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SELFTEST = process.argv.includes("--selftest");

// Only modules lib.rs actually declares. An orphaned .rs file (src/mic.rs is
// one -- recovered from a dead machine, never wired back in) cannot crash
// anything, and flagging it would teach people to bypass this check.
function compiledModules(libRs) {
  return [...libRs.matchAll(/^\s*(?:pub\s+)?mod\s+([a-z_0-9]+)\s*;/gm)].map((m) => m[1]);
}

// Which dom:: helpers panic is not a list to maintain by hand -- it is a
// property of dom.rs. A helper panics when it reaches el(id); the ones that
// look the element up themselves and bail (`let Some(..) else { return }`)
// do not. Derive it, so a new helper is covered the day it is written.
function panickingHelpers(domRs) {
  const out = new Set();
  for (const m of domRs.matchAll(/pub fn ([a-z_0-9]+)\(\s*id: &str[^)]*\)[^{]*\{([\s\S]*?)\n\}/g)) {
    if (/\bel\(id\)/.test(m[2])) out.add(m[1]);
  }
  return out;
}

function markupIds(html) {
  const markup = html
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<script\b[\s\S]*?<\/script>/gi, "")
    .replace(/<style\b[\s\S]*?<\/style>/gi, "");
  return new Set([...markup.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]));
}

// Ids Rust mints itself, in generated HTML or by set_id. These are as real
// as markup ids -- calendar_ui writes <div id="calDetail"> and then fills it.
function rustBornIds(sources) {
  const out = new Set();
  for (const src of sources.values()) {
    for (const m of src.matchAll(/id=\\?["']([A-Za-z][\w-]*)\\?["']/g)) out.add(m[1]);
    for (const m of src.matchAll(/set_id\("([A-Za-z][\w-]*)"\)/g)) out.add(m[1]);
    // An id held in a const and used through it -- drawer.rs's BURGER.
    for (const m of src.matchAll(/const [A-Z_0-9]+: &str = "([A-Za-z][\w-]*)";/g)) out.add(m[1]);
  }
  return out;
}

/// Every literal id a panicking helper is handed, with its line.
function referencedIds(src, panicky) {
  const hits = [];
  const names = [...panicky].join("|");
  if (!names) return hits;
  const lines = src.split("\n");

  // Direct: dom::set_text("feedback", ..)
  const direct = new RegExp(`\\bdom::(?:${names})\\(\\s*"([A-Za-z][\\w-]*)"`, "g");
  lines.forEach((ln, i) => {
    for (const m of ln.matchAll(direct)) hits.push({ id: m[1], line: i + 1 });
  });

  // Indirect: `for id in ["wpHow", "wpConfirm", "wpReveal"] { dom::..(id, ..) }`
  // This is the form that hid #wpDone for two months -- the ids never appear
  // next to a dom:: call at all, so a direct scan walks straight past them.
  const loop = new RegExp(
    `for\\s+(\\w+)\\s+in\\s+(?:&\\s*)?\\[([^\\]]*)\\]\\s*\\{([\\s\\S]{0,400}?)\\n\\s*\\}`,
    "g",
  );
  for (const m of src.matchAll(loop)) {
    const [, varName, arrayBody, loopBody] = m;
    const used = new RegExp(`\\bdom::(?:${names})\\(\\s*${varName}\\b`);
    if (!used.test(loopBody)) continue;
    const line = src.slice(0, m.index).split("\n").length;
    for (const lit of arrayBody.matchAll(/"([A-Za-z][\w-]*)"/g)) hits.push({ id: lit[1], line });
  }
  return hits;
}

function evaluate({ html, libRs, domRs, sources }) {
  const fails = [];
  const panicky = panickingHelpers(domRs);
  if (panicky.size === 0) {
    fails.push(
      "no panicking dom:: helper found in src/dom.rs — this check is anchored to helpers\n" +
        "  that reach el(id); if dom.rs was restructured, re-anchor it",
    );
    return fails;
  }
  const known = markupIds(html);
  for (const id of rustBornIds(sources)) known.add(id);

  const mods = new Set(compiledModules(libRs));
  for (const [file, src] of sources) {
    const modName = path.basename(file, ".rs");
    if (modName !== "lib" && !mods.has(modName)) continue; // not compiled
    // Comments name dead ids on purpose (this repo explains its own history).
    const code = src.replace(/^[ \t]*\/\/.*$/gm, "");
    for (const { id, line } of referencedIds(code, panicky)) {
      if (known.has(id)) continue;
      // An author who wrote dom::exists("x") in this file knows x is optional.
      if (new RegExp(`dom::exists\\("${id}"\\)`).test(code)) continue;
      fails.push(
        `src/${file}:${line} — #${id} is handed to a panicking dom:: helper but exists in no\n` +
          "  markup and is created nowhere; that line aborts the app when it runs",
      );
    }
  }
  return fails;
}

function load() {
  const srcDir = `${ROOT}/src`;
  const sources = new Map();
  for (const f of fs.readdirSync(srcDir)) {
    if (f.endsWith(".rs")) sources.set(f, fs.readFileSync(`${srcDir}/${f}`, "utf8"));
  }
  return {
    html: fs.readFileSync(`${ROOT}/index.html`, "utf8"),
    libRs: sources.get("lib.rs") ?? "",
    domRs: sources.get("dom.rs") ?? "",
    sources,
  };
}

const world = load();

if (!SELFTEST) {
  const fails = evaluate(world);
  if (fails.length) {
    console.error("dom-id-live-check: FAILED\n" + fails.map((f) => "  " + f).join("\n"));
    process.exit(1);
  }
  console.log("dom-id-live-check: OK — every literal element id reaches a real element");
  process.exit(0);
}

// A check that cannot fail is decoration. Each lesion is one of the two real
// bugs, reintroduced, plus the ways they could hide.
const LESIONS = [
  {
    name: "the wpDone bug itself — a dead id inside a for-loop array",
    mutate: (w) => ({
      ...w,
      sources: new Map(w.sources).set(
        "wordpic_screen.rs",
        'fn close_play() {\n    for id in ["wpHow", "wpGoneForever"] {\n        dom::remove_class(id, "show");\n    }\n}\n',
      ),
    }),
  },
  {
    name: "the drawHint bug itself — a dead id in a direct call",
    mutate: (w) => ({
      ...w,
      sources: new Map(w.sources).set("drawing.rs", 'fn r() {\n    dom::add_class("drawHintGone", "gone");\n}\n'),
    }),
  },
  {
    name: "a dead id reached through a helper OTHER than the ones named here",
    mutate: (w) => ({
      ...w,
      sources: new Map(w.sources).set("game.rs", 'fn r() {\n    dom::set_text("noSuchElementAnywhere", "hi");\n}\n'),
    }),
  },
  {
    name: "dom.rs restructured so no helper is recognised as panicking",
    mutate: (w) => ({ ...w, domRs: "pub fn nothing() {}\n" }),
  },
];

let bad = 0;
if (evaluate(world).length !== 0) {
  console.error("dom-id-live-check: the REAL tree already fails; fix that before trusting the selftest");
  process.exit(1);
}
for (const { name, mutate } of LESIONS) {
  const mutated = mutate(world);
  // lib.rs must declare the module a lesion writes into, or the lesion is
  // skipped as uncompiled and "passes" for the wrong reason.
  for (const f of mutated.sources.keys()) {
    const m = path.basename(f, ".rs");
    if (m !== "lib" && !new RegExp(`mod ${m};`).test(mutated.libRs)) mutated.libRs += `\nmod ${m};`;
  }
  if (evaluate(mutated).length === 0) {
    console.error(`  SURVIVED: ${name}`);
    bad++;
  }
}
if (bad) {
  console.error(`dom-id-live-check: FAILED\n  ${bad} of ${LESIONS.length} lesions were not caught`);
  process.exit(1);
}
console.log(`dom-id-live-check: selftest OK — all ${LESIONS.length} lesions fail the build`);
