#!/usr/bin/env node
// The attribution-reaches-a-player law.
//
// NOTICES.md held every third-party data licence this app depends on --
// Leipzig CC BY 4.0, CC-CEDICT CC BY-SA 4.0, Wiktionary CC BY-SA 4.0,
// KANJIDIC2, JMdict-derived JLPT meanings, OpenDyslexic CC BY 3.0 -- and
// shipped NOWHERE. Not in dist/, not in the iOS bundle, not served by the
// site. index.html named it in a CSS comment and nowhere else. The fonts were
// the single part already covered, and only by accident: fonts/OFL.txt lives
// inside the fonts/ directory the builds copy wholesale.
//
// It went unnoticed because nothing could notice it. assets/words/LICENSES.md
// asserted the opposite -- that no third-party lexicon licence applied at all
// -- and docs/census/bank_provenance.md, written to correct that, missed
// NOTICES.md and concluded the banks were credited nowhere in the repository
// either. Three documents, no gate, and a page no player could open.
//
// This is the gate. Five laws, each the shape of a way the fix could rot:
//
//   1. PARITY     notices.html is exactly what NOTICES.md renders to. It is a
//                 committed artifact because the site's serve stage is
//                 caddy:2-alpine with neither Node nor Python, so drift is
//                 possible in a way it is not for a build-time file.
//   2. BUILDS     every script that assembles a dist copies it. There are five
//                 and they each duplicate their own copy list.
//   3. SITE       the Dockerfile copies it to /srv. The Dockerfile is the
//                 source of truth for spellgame.net, separately from the five.
//   4. REACHABLE  index.html links to it. A shipped file nobody can open is
//                 the bug this check exists for, in a new costume.
//   5. RENDERABLE NOTICES.md uses only what the renderer handles. An
//                 unsupported construct would reach a player as raw markdown
//                 on the one page that has to be unambiguous.
//
//   node scripts/notices-ship-check.mjs
//   node scripts/notices-ship-check.mjs --selftest
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { render } from "./build-notices.mjs";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";

// Constructs NOTICES.md must stay clear of, because build-notices.mjs renders
// headings, bold, inline code, bullets, bare URLs and rules -- and nothing else.
const UNSUPPORTED = [
  [/^\s*>/m, "a blockquote"],
  [/^\s*\|/m, "a table"],
  [/^\s*```/m, "a fenced code block"],
  [/!\[[^\]]*\]\(/, "an image"],
  [/(?<!!)\[[^\]]+\]\([^)]+\)/, "an inline [text](url) link — write the bare URL"],
  [/^\s*\d+\.\s/m, "a numbered list"],
];

export function check(root) {
  const bad = [];
  const read = (rel) => {
    const p = path.join(root, rel);
    return fs.existsSync(p) ? fs.readFileSync(p, "utf8") : null;
  };

  const md = read("NOTICES.md");
  if (!md || !md.trim()) return [`NOTICES.md is missing or empty at ${root}`];

  // 1. PARITY
  const html = read("notices.html");
  if (html === null) {
    bad.push("notices.html does not exist — run: node scripts/build-notices.mjs");
  } else if (html !== render(md)) {
    bad.push("notices.html is STALE against NOTICES.md — run: node scripts/build-notices.mjs");
  }

  // 2. BUILDS
  const builds = fs.existsSync(path.join(root, "scripts"))
    ? fs.readdirSync(path.join(root, "scripts")).filter((f) => /^build-web.*\.sh$/.test(f)).sort()
    : [];
  if (!builds.length) bad.push("no scripts/build-web*.sh found — cannot verify the app bundle ships it");
  for (const b of builds) {
    if (!(read(`scripts/${b}`) || "").includes("notices.html")) {
      bad.push(`scripts/${b} assembles a dist but never copies notices.html`);
    }
  }

  // 3. SITE
  const docker = read("Dockerfile");
  if (docker === null) bad.push("no Dockerfile — cannot verify the site ships it");
  else if (!/^COPY\b.*\bnotices\.html\b/m.test(docker)) {
    bad.push("the Dockerfile never COPYs notices.html to /srv — the site would serve a 404");
  }

  // 4. REACHABLE
  const index = read("index.html");
  if (index === null) bad.push("no index.html — cannot verify the page is reachable");
  else if (!/href="notices\.html"/.test(index)) {
    bad.push('index.html has no href="notices.html" — the page ships but nothing opens it');
  }

  // 5. RENDERABLE
  for (const [re, why] of UNSUPPORTED) {
    if (re.test(md)) bad.push(`NOTICES.md contains ${why}, which build-notices.mjs does not render`);
  }
  return bad;
}

if (process.argv.includes("--selftest")) {
  let failed = 0;
  const base = {
    "NOTICES.md": "# Notices\n\nSome **real** text.\n",
    "index.html": '<a href="notices.html">Open</a>',
    "Dockerfile": "COPY index.html notices.html /srv/\n",
    "scripts/build-web.sh": 'cp sw.js notices.html "$DIST/"\n',
  };
  const cases = {
    clean:        { mut: (f) => f, want: null },
    stale_html:   { mut: (f) => ({ ...f, "notices.html": "<!doctype html><p>old</p>" }), want: /STALE/ },
    missing_html: { mut: (f) => { const g = { ...f }; delete g.__render; g.__skip = true; return g; }, want: /does not exist/ },
    build_drops:  { mut: (f) => ({ ...f, "scripts/build-web.sh": 'cp sw.js "$DIST/"\n' }), want: /never copies notices.html/ },
    docker_drops: { mut: (f) => ({ ...f, Dockerfile: "COPY index.html /srv/\n" }), want: /never COPYs notices.html/ },
    unlinked:     { mut: (f) => ({ ...f, "index.html": "<p>no link here</p>" }), want: /nothing opens it/ },
    md_table:     { mut: (f) => ({ ...f, "NOTICES.md": "# N\n\n| a | b |\n" }), want: /a table/ },
    md_inline_link: { mut: (f) => ({ ...f, "NOTICES.md": "# N\n\nSee [the terms](http://x.test).\n" }), want: /inline \[text\]\(url\) link/ },
    md_empty:     { mut: (f) => ({ ...f, "NOTICES.md": "   \n" }), want: /missing or empty/ },
  };
  for (const [name, c] of Object.entries(cases)) {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "notices-check-"));
    const files = c.mut({ ...base });
    const skip = files.__skip; delete files.__skip; delete files.__render;
    for (const [rel, body] of Object.entries(files)) {
      fs.mkdirSync(path.join(d, path.dirname(rel)), { recursive: true });
      fs.writeFileSync(path.join(d, rel), body);
    }
    // Every case gets a CORRECT notices.html unless it is the one about that.
    if (!skip && !files["notices.html"]) {
      fs.writeFileSync(path.join(d, "notices.html"), render(files["NOTICES.md"]));
    }
    const bad = check(d);
    const ok = c.want === null ? bad.length === 0 : bad.some((b) => c.want.test(b));
    const why = c.want === null ? (bad[0] || "") : (ok ? bad.find((b) => c.want.test(b))
                 : `wanted ${c.want}, got ${bad.length ? bad.join(" | ") : "nothing"}`);
    console.log(`  ${ok ? (c.want ? "caught " : "clean  ") : "MISSED "} ${name}${why ? " — " + why.slice(0, 72) : ""}`);
    if (!ok) failed++;
    fs.rmSync(d, { recursive: true });
  }
  if (failed) { console.error(`notices-ship-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log("notices-ship-check selftest: OK");
  process.exit(0);
}

const bad = check(ROOT);
if (bad.length) {
  for (const b of bad) console.error(`  ${b}`);
  console.error("notices-ship-check: FAILED");
  process.exit(1);
}
console.log("notices-ship-check: OK — NOTICES.md renders to notices.html, and it ships in 5 build(s), the site image, and a link in index.html");
