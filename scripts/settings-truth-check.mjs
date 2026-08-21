// CC-AUG6-AUDITPASS F8 — the settings-truth gate.
//
// Eric's Aug 6 device audit found a class of settings that render and do
// nothing. Diagnosing all seventeen produced a surprise that shapes this
// file: EVERY control was wired. There was no unconnected switch
// anywhere. They failed in three other ways —
//
//   * suppressed  — alive but overridden by a mode (Kid Mode forced
//                   extra attempts ON, forced reminders OFF)
//   * too small   — Slower Voice moved duration 1.29x, imperceptible
//   * unserved    — offline packs' client is complete; the /packs route
//                   does not exist in backend/ at all
//
// So a connectivity check would have gone green on all seventeen while
// every symptom Eric found stayed broken. That is why this gate refuses
// to check wiring. It checks that each control names an OBSERVABLE
// EFFECT and a TEST THAT EXISTS, and that a mode-overridden control
// declares its suppressor.
//
// Laws:
//   1. every rendered control has a manifest entry (no undeclared switch)
//   2. every manifest entry names a real rendered control (no ghosts)
//   3. every entry states an observable effect
//   4. every entry names an effect test that EXISTS in the tree — a Rust
//      `fn <name>` or an e2e `suite.test('<name>...')`. dev-only controls
//      are exempt and must say so.
//   5. `suppressed_by` is required to be present (null is a claim too) —
//      a control a mode overrides must say which mode.
//
//   node scripts/settings-truth-check.mjs
//   node scripts/settings-truth-check.mjs --selftest   # prove it bites
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SELFTEST = process.argv.includes("--selftest");

let html = fs.readFileSync(`${ROOT}/index.html`, "utf8");
const manifest = JSON.parse(fs.readFileSync(`${ROOT}/config/settings-effects.json`, "utf8"));

// A deliberate dead toggle, injected only under --selftest. F8 asks for
// proof the gate bites; a gate nobody has seen fail is a gate nobody
// knows works.
if (SELFTEST) {
  html = html.replace(
    "</body>",
    '<input type="checkbox" class="switch" id="selftestDeadToggle" /></body>'
  );
}

// Every rendered control the player can operate.
const rendered = new Set();
for (const m of html.matchAll(/<input[^>]*>/g)) {
  const tag = m[0];
  const type = (tag.match(/type="([^"]+)"/) || [])[1];
  const id = (tag.match(/\bid="([^"]+)"/) || [])[1];
  if (!id || !["checkbox", "range"].includes(type)) continue;
  rendered.add(id);
}
// CC-ZH-TONE F4: the tone buttons are controls too. Only switches and sliders
// used to count, because those were the only kinds that existed — but a tone
// button that does nothing is exactly the failure this gate is for, and a
// <button> would have sailed past a scan that only reads <input>. Matched by
// the class rather than by id pattern, so a sixth button cannot be added
// without also being declared.
for (const m of html.matchAll(/<button[^>]*class="[^"]*\btone-btn\b[^"]*"[^>]*>/g)) {
  const id = (m[0].match(/\bid="([^"]+)"/) || [])[1];
  if (id) rendered.add(id);
}

// Every test name the tree actually defines.
const testNames = new Set();
const walk = (dir, re, take) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { if (e.name !== "node_modules") walk(p, re, take); continue; }
    if (!re.test(e.name)) continue;
    take(fs.readFileSync(p, "utf8"));
  }
};
walk(`${ROOT}/src`, /\.rs$/, (s) => {
  for (const m of s.matchAll(/fn\s+([a-z0-9_]+)\s*\(/g)) testNames.add(m[1]);
});
walk(`${ROOT}/tests`, /\.mjs$/, (s) => {
  for (const m of s.matchAll(/suite\.test\(\s*['"`]([^'"`]+)/g)) testNames.add(m[1]);
});

const problems = [];
const byKey = new Map();
for (const c of manifest.controls) {
  if (byKey.has(c.key)) problems.push(`${c.key}: listed twice in the manifest`);
  byKey.set(c.key, c);
}

// Law 1
for (const id of rendered) {
  if (!byKey.has(id)) {
    problems.push(
      `${id} is RENDERED but absent from config/settings-effects.json — ` +
        `an undeclared switch is how a dead one hides`
    );
  }
}
// Law 2
for (const key of byKey.keys()) {
  if (!rendered.has(key)) problems.push(`${key}: in the manifest but never rendered`);
}
// Laws 3-5
for (const c of manifest.controls) {
  if (!rendered.has(c.key)) continue;
  if (!c.effect || !c.effect.trim()) {
    problems.push(`${c.key}: no observable effect stated`);
  }
  if (!("suppressed_by" in c)) {
    problems.push(
      `${c.key}: no suppressed_by field — say null explicitly. Five of the ` +
        `seventeen controls are overridden by Kid Mode; silence is not an answer`
    );
  }
  if (c.dev_only) continue;
  // A tool gated on Avail::Native or Avail::Server cannot be proven in a
  // headless browser: native_lang::available() is false there, so
  // flipping the flag changes nothing observable and a web assertion
  // would pass for the wrong reason. Those entries declare
  // harness:"device" and are satisfied by a Maestro flow instead.
  if (c.harness === "device") {
    if (!c.test) problems.push(`${c.key}: device-owned but names no Maestro flow`);
    continue;
  }
  if (!c.test) {
    problems.push(`${c.key}: names no effect test (F8: a switch with no effect test fails the build)`);
  } else if (!testNames.has(c.test) && ![...testNames].some((t) => t.startsWith(c.test))) {
    problems.push(`${c.key}: effect test '${c.test}' does not exist in src/ or tests/`);
  }
}

// CC-ZH-TONE F4 — a declared effect test is not proof of a live button.
//
// Found the hard way: unwiring the click handler left the build green, because
// the effect tests exercise the pure transform and never touch the wiring. A
// rendered, declared, fully-tested button can still be inert. So the handler
// itself is asserted by name.
{
  const wiring = fs.readFileSync(`${ROOT}/src/lib.rs`, "utf8");
  const looped = /on_click\(&format!\("toneBtn\{tone\}"\)/.test(wiring) &&
    /game::tap_tone\(/.test(wiring);
  for (const id of [...rendered].filter((r) => /^toneBtn\d$/.test(r))) {
    const literal = new RegExp(`on_click\\("${id}"`).test(wiring);
    if (!looped && !literal) {
      problems.push(
        `${id} is rendered and declared but nothing wires it — its effect test ` +
          `proves the transform, not the button (F4)`,
      );
    }
  }
}

if (SELFTEST) {
  const bit = problems.some((p) => p.startsWith("selftestDeadToggle"));
  if (!bit) {
    console.log("settings-truth-check: SELFTEST FAILED — a dead toggle slipped through");
    process.exit(1);
  }
  console.log("settings-truth-check: selftest OK — the deliberate dead toggle fails the build");
  process.exit(0);
}

if (problems.length) {
  console.log("settings-truth-check: FAILED\n  " + problems.join("\n  "));
  process.exit(1);
}
const suppressed = manifest.controls.filter((c) => c.suppressed_by).length;
console.log(
  `settings-truth-check: OK — ${rendered.size} rendered controls, all declared, ` +
    `all with a live effect test (${suppressed} declare a suppressor)`
);
