#!/usr/bin/env node
// v8 F6 / I: the reserved chrome class is UI-only. A picture stroke that
// borrows the interface accent is a build failure — that ambiguity is
// exactly what made Eric unable to tell rails from artwork.
import fs from "node:fs";
const html = fs.readFileSync(new URL("../index.html", import.meta.url), "utf8");
const css = html.split("</style>")[0];
const problems = [];
// 1. the reserved variable exists
if (!/--ui-chrome\s*:/.test(css)) problems.push("--ui-chrome is not defined");
// 2. every picture-stroke class avoids it
for (const cls of ["wp-outline", "wp-guide", "wp-word", "wp-feature"]) {
  const rules = [...css.matchAll(new RegExp(`\\.${cls}[^{]*\\{([^}]*)\\}`, "g"))].map((m) => m[1]);
  for (const body of rules) {
    if (/--ui-chrome|var\(--accent/.test(body))
      problems.push(`.${cls} uses reserved chrome styling: ${body.trim().slice(0, 60)}`);
  }
}
// 3. chrome classes DO use it (so the separation is real, not vacuous)
const slots = css.match(/\.wp-slots\{([^}]*)\}/);
if (!slots || !/--ui-chrome/.test(slots[1])) problems.push(".wp-slots must use the reserved chrome accent");
if (problems.length) {
  console.error("chrome-separation: FAILED");
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log("chrome-separation: OK — interface owns the accent; no picture stroke borrows it.");
